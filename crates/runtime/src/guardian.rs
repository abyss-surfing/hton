//! Durable activation lease supervision independent of management process lifetime.
use crate::{
    Cancellation, Clock,
    actions::ActionExecutor,
    error,
    manifest::{self, AdmissionPorts, AdmittedBundle},
    probes::{self, ProbePort},
    state::{StateRepository, write_transition},
};
use hton_runtime_contract::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonitorOutcome {
    Waiting,
    Committed,
    Compensated,
    StoppedWithArtifacts,
    RecoveryRequired,
}

/// Verifies that an independently supervised worker has acknowledged the durable
/// lease before any native promotion. It must not be a management-owned task.
pub trait GuardianSupervisor: Send + Sync {
    fn ensure_monitoring(
        &self,
        instance: &InstanceId,
        lease: &Lease,
        context: &OperationContext,
    ) -> Result<(), RuntimeError>;
}

pub struct RuntimeGuardian<'a> {
    pub store: &'a dyn StateRepository,
    pub admission: &'a dyn AdmissionPorts,
    pub actions: &'a ActionExecutor<'a>,
    pub probes: &'a dyn ProbePort,
    pub clock: &'a dyn Clock,
    pub cancellation: &'a dyn Cancellation,
    pub supervisor: &'a dyn GuardianSupervisor,
}
impl RuntimeGuardian<'_> {
    pub fn arm(
        &self,
        bundle: &AdmittedBundle,
        fallback: &FallbackCompatibility,
        lease_id: LeaseId,
        deadline_ms: u64,
        expected_epoch: u64,
        context: &OperationContext,
    ) -> Result<Lease, RuntimeError> {
        // Serialize durable trial admission with fixed runtime actions. A service
        // restart holding this same gate either completes first or observes Armed.
        let effect_guard = self.actions.gate.acquire(context)?;
        let state = self.store.read(&bundle.candidate().instance_id, context)?;
        if state.run_epoch != expected_epoch || state.run_intent != RunIntent::Running {
            return Err(error(ErrorCode::Stopped, context));
        }
        if deadline_ms <= self.clock.now_ms()
            || deadline_ms > self.clock.now_ms().saturating_add(300_000)
            || fallback.fences.credential_revision < state.fences.credential_revision
            || !self.admission.fallback_valid(fallback, context)?
        {
            return Err(error(ErrorCode::FallbackIncompatible, context));
        }
        let lease = Lease {
            started_from_stopped: false,
            lease_id,
            candidate: bundle.candidate().bundle_id.clone(),
            fallback: fallback.clone(),
            run_epoch: state.run_epoch,
            deadline_ms,
            state: LeaseState::Armed,
        };
        write_transition(
            self.store,
            &state,
            StateTransition::ArmTrial {
                lease: lease.clone(),
            },
            context,
        )?;
        drop(effect_guard);
        self.supervisor
            .ensure_monitoring(&state.instance_id, &lease, context)?;
        Ok(lease)
    }

    pub fn monitor(
        &self,
        instance: &InstanceId,
        lease_id: &LeaseId,
        context: &OperationContext,
    ) -> Result<MonitorOutcome, RuntimeError> {
        let state = self.store.read(instance, context)?;
        let lease = state
            .lease
            .as_ref()
            .filter(|l| &l.lease_id == lease_id)
            .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
        match lease.state {
            LeaseState::Committed => return Ok(MonitorOutcome::Committed),
            LeaseState::Compensated => return Ok(MonitorOutcome::Compensated),
            LeaseState::StoppedWithArtifacts => return Ok(MonitorOutcome::StoppedWithArtifacts),
            LeaseState::RecoveryRequired => return Ok(MonitorOutcome::RecoveryRequired),
            _ => (),
        }
        if state.run_intent == RunIntent::Running
            && state.run_epoch == lease.run_epoch
            && self.clock.now_ms() < lease.deadline_ms
        {
            if lease.state == LeaseState::Verifying {
                let candidate = manifest::admit_trial(&state, self.admission, self.clock, context)?;
                let report = probes::verify(
                    &candidate,
                    self.probes,
                    self.clock,
                    self.cancellation,
                    context,
                )?;
                if !report.blocking {
                    if self.clock.now_ms() >= lease.deadline_ms {
                        return self.recover(instance, lease_id, context);
                    }
                    // CAS refuses a concurrent stop or another lease transition.
                    write_transition(
                        self.store,
                        &state,
                        StateTransition::CommitTrial {
                            lease_id: lease_id.clone(),
                        },
                        context,
                    )?;
                    return Ok(MonitorOutcome::Committed);
                }
            }
            return Ok(MonitorOutcome::Waiting);
        }
        self.recover(instance, lease_id, context)
    }

    pub fn recover(
        &self,
        instance: &InstanceId,
        lease_id: &LeaseId,
        context: &OperationContext,
    ) -> Result<MonitorOutcome, RuntimeError> {
        self.recover_inner(instance, lease_id, false, None, context)
    }

    /// Restore the exact previous-compatible target retained by the currently
    /// committed activation lease. The caller supplies only the target ID; all
    /// fallback proof, digest, resource and certificate checks come from protected
    /// runtime state and admission ports.
    pub fn rollback_accepted(
        &self,
        instance: &InstanceId,
        target: &BundleId,
        context: &OperationContext,
    ) -> Result<MonitorOutcome, RuntimeError> {
        let state = self.store.read(instance, context)?;
        let lease = state
            .lease
            .as_ref()
            .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
        if state.run_intent != RunIntent::Running
            || state.maintenance.is_some()
            || state.run_epoch != lease.run_epoch
            || state.accepted.as_ref() != Some(&lease.candidate)
            || state.previous_compatible.as_ref() != Some(target)
            || !matches!(
                lease.state,
                LeaseState::Committed | LeaseState::RecoveryRequired
            )
            || !matches!(
                &lease.fallback.target,
                RollbackTarget::Accepted { bundle_id, .. } if bundle_id == target
            )
        {
            return Err(error(ErrorCode::FallbackIncompatible, context));
        }
        self.recover_inner(instance, &lease.lease_id, true, Some(target), context)
    }

    fn recover_inner(
        &self,
        instance: &InstanceId,
        lease_id: &LeaseId,
        explicit_rollback: bool,
        requested_target: Option<&BundleId>,
        context: &OperationContext,
    ) -> Result<MonitorOutcome, RuntimeError> {
        let state = self.store.read(instance, context)?;
        let lease = state
            .lease
            .as_ref()
            .filter(|l| &l.lease_id == lease_id)
            .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
        if matches!(
            lease.state,
            LeaseState::Committed | LeaseState::Compensated | LeaseState::StoppedWithArtifacts
        ) && !explicit_rollback
        {
            return Err(error(ErrorCode::LeaseConflict, context));
        }
        if explicit_rollback
            && (requested_target.is_none_or(|id| state.previous_compatible.as_ref() != Some(id))
                || state.run_intent != RunIntent::Running
                || state.run_epoch != lease.run_epoch
                || state.accepted.as_ref() != Some(&lease.candidate)
                || !matches!(
                    lease.state,
                    LeaseState::Committed | LeaseState::RecoveryRequired
                ))
        {
            return Err(error(ErrorCode::FallbackIncompatible, context));
        }
        let qualified_fallback =
            manifest::admit_fallback(&state, self.admission, self.clock, context)?;
        if explicit_rollback
            && qualified_fallback
                .as_ref()
                .is_none_or(|bundle| Some(&bundle.candidate().bundle_id) != requested_target)
        {
            return Err(error(ErrorCode::FallbackIncompatible, context));
        }
        let target = lease.fallback.target.clone();
        let lease_epoch = lease.run_epoch;
        let recovering = if explicit_rollback {
            write_transition(
                self.store,
                &state,
                StateTransition::BeginRollback {
                    lease_id: lease_id.clone(),
                    target_bundle_id: requested_target
                        .expect("explicit target is rechecked above")
                        .clone(),
                },
                context,
            )?
        } else {
            write_transition(
                self.store,
                &state,
                StateTransition::SetLeaseState {
                    lease_id: lease_id.clone(),
                    state: LeaseState::Compensating,
                },
                context,
            )?
        };
        let effects = || -> Result<(), RuntimeError> {
            let candidate =
                manifest::admit_cleanup(&recovering, self.admission, self.clock, context)?;
            // Validate fallback before touching any candidate state.
            let fallback =
                manifest::admit_fallback(&recovering, self.admission, self.clock, context)?;
            let receipts =
                self.actions
                    .compensate(&candidate, fallback.as_ref(), lease_epoch, context)?;
            if receipts.iter().any(|r| {
                matches!(
                    r.outcome,
                    ActionOutcome::Failed { .. } | ActionOutcome::Uncertain { .. }
                )
            }) {
                return Err(error(ErrorCode::RecoveryRequired, context));
            }
            let current = self.store.read(instance, context)?;
            if let Some(fallback) = fallback {
                let receipts = self
                    .actions
                    .restore_fallback(&fallback, lease_epoch, context)?;
                if receipts.iter().any(|r| {
                    matches!(
                        r.outcome,
                        ActionOutcome::Failed { .. } | ActionOutcome::Uncertain { .. }
                    )
                }) {
                    return Err(error(ErrorCode::RecoveryRequired, context));
                }
                // Revalidate the immutable fallback after effects as expiry/fence
                // changes can occur during a long compensation.
                let revalidated = manifest::admit_fallback(
                    &self.store.read(instance, context)?,
                    self.admission,
                    self.clock,
                    context,
                )?
                .ok_or_else(|| error(ErrorCode::FallbackIncompatible, context))?;
                if revalidated.candidate().bundle_id != fallback.candidate().bundle_id {
                    return Err(error(ErrorCode::FallbackIncompatible, context));
                }
                let after = self.store.read(instance, context)?;
                if after.run_intent == RunIntent::Running
                    && after.run_epoch == lease_epoch
                    && probes::verify(
                        &revalidated,
                        self.probes,
                        self.clock,
                        self.cancellation,
                        context,
                    )?
                    .blocking
                {
                    return Err(error(ErrorCode::RecoveryRequired, context));
                }
            }
            Ok(())
        };
        if effects().is_err() {
            let current = self.store.read(instance, context)?;
            write_transition(
                self.store,
                &current,
                StateTransition::SetLeaseState {
                    lease_id: lease_id.clone(),
                    state: LeaseState::RecoveryRequired,
                },
                context,
            )?;
            return Ok(MonitorOutcome::RecoveryRequired);
        }
        let current = self.store.read(instance, context)?;
        let result = write_transition(
            self.store,
            &current,
            StateTransition::FinishCompensation {
                lease_id: lease_id.clone(),
                restored: target,
            },
            context,
        )?;
        Ok(
            if result.run_intent == RunIntent::Stopped || result.run_epoch != lease_epoch {
                MonitorOutcome::StoppedWithArtifacts
            } else {
                MonitorOutcome::Compensated
            },
        )
    }
}
