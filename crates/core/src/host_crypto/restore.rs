//! Restore epoch/reservation bridge; it never writes runtime state directly.
use crate::{
    backup::*,
    host::{HostApplication, runtime},
    journal::{Intent, TerminalStatus, failure},
};
use hton_contracts::operations::*;
use hton_runtime_contract as wire;

pub struct RuntimeRestore {
    app: HostApplication,
}
impl RuntimeRestore {
    pub fn new(app: &HostApplication) -> Self {
        Self { app: app.clone() }
    }
}
pub fn approval_id(
    archive: &ObjectId,
    manifest: &Sha256Digest,
    instance: &InstanceId,
) -> ApprovalId {
    let digest = crate::config::digest(
        &serde_json::to_vec(&(archive, manifest, instance)).expect("validated metadata"),
    );
    ApprovalId::new(format!("restore.{}", &digest.as_str()[7..])).expect("digest ID")
}
pub fn verify_guard(
    app: &HostApplication,
    guard: &RestoreFence,
    ctx: &OperationContext,
) -> Result<()> {
    let state = app.runtime(ctx)?;
    if app.control.now_ms() >= ctx.deadline.0
        || app.control.is_cancelled(&ctx.cancellation)
        || state.run_intent != wire::RunIntent::Stopped
        || state.run_epoch != guard.stopped_epoch.0
        || state.fences.credential_revision < guard.credential_fence
        || !state.maintenance.as_ref().is_some_and(|reservation| {
            reservation.reservation_id == guard.maintenance_id
                && reservation.operation_id.as_str() == ctx.operation_id.as_str()
                && reservation.run_epoch == state.run_epoch
        })
        || state.resources.as_slice().iter().any(|resource| {
            resource.status != wire::ResourceStatus::Stopped
                || resource.restart_policy != wire::RestartPolicy::Disabled
        })
    {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "restore stopped reservation or native policy fence changed",
        ));
    }
    Ok(())
}
impl RestoreRuntime for RuntimeRestore {
    fn stop_for_restore(&self, ctx: &OperationContext) -> Result<(OperationResult, RestoreFence)> {
        let stopped = runtime::execute(&self.app, wire::ExecutorAction::StopAll, ctx)?;
        let outcome = runtime::map_result(&stopped, ctx)?;
        if outcome.status != OperationStatus::Succeeded
            || outcome.run_intent != RunIntent::Stopped
            || !outcome.survivors.is_empty()
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "restore shutdown is incomplete",
            ));
        }
        let maintenance_id = wire::LeaseId::new(format!(
            "restore.{}",
            &crate::config::digest(ctx.operation_id.as_str().as_bytes()).as_str()[7..]
        ))
        .expect("digest ID");
        runtime::execute(
            &self.app,
            wire::ExecutorAction::ReserveMaintenance {
                reservation_id: maintenance_id.clone(),
            },
            ctx,
        )?;
        let state = self.app.runtime(ctx)?;
        let guard = RestoreFence {
            maintenance_id,
            stopped_epoch: RunEpoch(state.run_epoch),
            credential_fence: state.fences.credential_revision,
        };
        verify_guard(&self.app, &guard, ctx)?;
        Ok((outcome, guard))
    }
    fn release_completed(
        &self,
        plan: &RestorePlan,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<()> {
        let tx = self.app.journal.read(transaction, ctx)?;
        let expected = crate::config::digest(&serde_json::to_vec(plan).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "restore metadata invalid",
            )
        })?);
        if tx.operation_id != ctx.operation_id
            || tx.actor != ctx.actor
            || !matches!(tx.intent,Intent::Restore{request_digest}if request_digest==expected)
            || !self
                .app
                .journal
                .terminal(transaction, ctx)?
                .is_some_and(|record| record.status == TerminalStatus::Committed)
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "restore release requires exact committed terminal",
            ));
        }
        let state = self.app.runtime(ctx)?;
        let Some(reservation) = state.maintenance else {
            return Ok(());
        };
        if reservation.operation_id.as_str() != ctx.operation_id.as_str() {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "another operation owns maintenance reservation",
            ));
        }
        runtime::execute(
            &self.app,
            wire::ExecutorAction::ReleaseMaintenance {
                reservation_id: reservation.reservation_id,
            },
            ctx,
        )?;
        Ok(())
    }
    fn verify_approval(&self, plan: &RestorePlan, ctx: &OperationContext) -> Result<bool> {
        Ok(ctx.actor.source == ActorSource::Local
            && ctx
                .actor
                .granted_scopes
                .contains(&ScopeId::new("backup.restore").expect("scope"))
            && ctx.actor.local_approval_refs.contains(&plan.approval_id)
            && plan.approval_id
                == approval_id(
                    &plan.archive_id,
                    &plan.manifest_digest,
                    &self.app.installation.instance_id,
                ))
    }
    fn publish_restored(
        &self,
        plan: &RestorePlan,
        guard: &RestoreFence,
        ctx: &OperationContext,
    ) -> Result<()> {
        verify_guard(&self.app, guard, ctx)?;
        let mut restored = None;
        for tx in self.app.journal.unfinished(ctx)? {
            if tx.operation_id != ctx.operation_id || !matches!(tx.intent, Intent::Restore { .. }) {
                continue;
            }
            let directory = match self
                .app
                .work
                .child("restore")
                .and_then(|dir| dir.child(tx.transaction_id.as_str()))
            {
                Ok(directory) => directory,
                Err(hton_storage::Error::NotFound) => continue,
                Err(_) => {
                    return Err(failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "restored runtime metadata unavailable",
                    ));
                }
            };
            if let Some(bytes) = directory
                .read("runtime-state.json", wire::MAX_WIRE_BYTES)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "restored runtime metadata unreadable",
                    )
                })?
            {
                let state: wire::RuntimeState = wire::decode(&bytes).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "restored runtime metadata invalid",
                    )
                })?;
                if state.instance_id.as_str() != self.app.installation.instance_id.as_str()
                    || state.fences.credential_revision < guard.credential_fence
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "restored instance or credential floor incompatible",
                    ));
                }
                restored = Some(state);
            }
        }
        let accepted = restored.as_ref().and_then(|state| state.accepted.clone());
        let previous = restored
            .as_ref()
            .and_then(|state| state.previous_compatible.clone());
        let floor = plan.manifest.credential_fence.max(guard.credential_fence);
        runtime::execute(
            &self.app,
            wire::ExecutorAction::RestoreStopped {
                reservation_id: guard.maintenance_id.clone(),
                accepted,
                previous_compatible: previous,
                credential_revision: floor,
            },
            ctx,
        )?;
        verify_guard(
            &self.app,
            &RestoreFence {
                credential_fence: floor,
                ..guard.clone()
            },
            ctx,
        )
    }
    fn verify_stopped(&self, guard: &RestoreFence, ctx: &OperationContext) -> Result<bool> {
        Ok(verify_guard(&self.app, guard, ctx).is_ok())
    }
}
