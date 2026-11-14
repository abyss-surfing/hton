//! Durable runtime state transitions. Storage implementations serialize read/CAS/write.
use crate::error;
use hton_runtime_contract::*;
use std::collections::BTreeMap;

/// Atomic persistence is a required port guarantee, not an in-memory success flag.
pub trait StateRepository: Send + Sync {
    fn read(
        &self,
        instance: &InstanceId,
        context: &OperationContext,
    ) -> Result<RuntimeState, RuntimeError>;
    fn write(
        &self,
        request: &StateWrite,
        context: &OperationContext,
    ) -> Result<RuntimeState, RuntimeError>;
}

pub fn initial(instance: InstanceId) -> RuntimeState {
    initial_state(instance)
}

fn active(state: LeaseState) -> bool {
    matches!(
        state,
        LeaseState::Armed
            | LeaseState::Activating
            | LeaseState::Verifying
            | LeaseState::Compensating
            | LeaseState::RollingBack
            | LeaseState::RecoveryRequired
    )
}

pub fn transition(
    current: &RuntimeState,
    request: &StateWrite,
    context: &OperationContext,
) -> Result<RuntimeState, RuntimeError> {
    if current.instance_id != request.instance_id
        || current.revision != request.expected_revision
        || current.run_epoch != request.expected_run_epoch
    {
        return Err(error(ErrorCode::StaleInput, context));
    }
    let mut next = current.clone();
    next.revision = current
        .revision
        .checked_add(1)
        .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?;
    match &request.transition {
        StateTransition::StartTrial { lease } => {
            let epoch = current
                .run_epoch
                .checked_add(1)
                .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?;
            if current.run_intent != RunIntent::Stopped
                || current.maintenance.is_some()
                || current.lease.as_ref().is_some_and(|old| active(old.state))
                || !lease.started_from_stopped
                || lease.run_epoch != epoch
                || lease.state != LeaseState::Armed
                || lease.fallback.fences.credential_revision < current.fences.credential_revision
            {
                return Err(error(ErrorCode::LeaseConflict, context));
            }
            next.run_intent = RunIntent::Running;
            next.run_epoch = epoch;
            next.trial = Some(lease.candidate.clone());
            next.lease = Some(lease.clone());
        }

        StateTransition::RestoreStopped {
            reservation_id,
            accepted,
            previous_compatible,
            credential_revision,
        } => {
            let reservation = current
                .maintenance
                .as_ref()
                .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
            if reservation.reservation_id != *reservation_id
                || reservation.operation_id != context.operation_id
                || reservation.run_epoch != current.run_epoch
                || current.run_intent != RunIntent::Stopped
                || current
                    .lease
                    .as_ref()
                    .is_some_and(|lease| active(lease.state))
                || current.resources.as_slice().iter().any(|resource| {
                    resource.status != ResourceStatus::Stopped
                        || resource.restart_policy != RestartPolicy::Disabled
                })
                || *credential_revision < current.fences.credential_revision
                || (accepted.is_none() && previous_compatible.is_some())
            {
                return Err(error(ErrorCode::RecoveryRequired, context));
            }
            next.accepted = accepted.clone();
            next.previous_compatible = previous_compatible.clone();
            next.trial = None;
            next.lease = None;
            next.fences.credential_revision = *credential_revision;
            // Retain actual current resource locators, stopped intent and epoch/reservation.
        }

        StateTransition::ReserveMaintenance { reservation_id } => {
            if current.run_intent != RunIntent::Stopped
                || current.maintenance.is_some()
                || current.lease.as_ref().is_some_and(|l| active(l.state))
                || current.resources.as_slice().iter().any(|r| {
                    r.status != ResourceStatus::Stopped
                        || r.restart_policy != RestartPolicy::Disabled
                })
            {
                return Err(error(ErrorCode::LeaseConflict, context));
            }
            next.run_epoch = current
                .run_epoch
                .checked_add(1)
                .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?;
            next.maintenance = Some(MaintenanceReservation {
                reservation_id: reservation_id.clone(),
                operation_id: context.operation_id.clone(),
                run_epoch: next.run_epoch,
            });
        }
        StateTransition::ReleaseMaintenance { reservation_id } => {
            let reservation = current
                .maintenance
                .as_ref()
                .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
            if &reservation.reservation_id != reservation_id
                || reservation.operation_id != context.operation_id
                || reservation.run_epoch != current.run_epoch
                || current.run_intent != RunIntent::Stopped
                || current.resources.as_slice().iter().any(|r| {
                    r.status != ResourceStatus::Stopped
                        || r.restart_policy != RestartPolicy::Disabled
                })
            {
                return Err(error(ErrorCode::LeaseConflict, context));
            }
            next.maintenance = None;
        }
        StateTransition::StopIntent => {
            next.run_intent = RunIntent::Stopped;
            next.run_epoch = current
                .run_epoch
                .checked_add(1)
                .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?;
            // Preserve the old lease and its original epoch for artifact compensation.
            if let Some(reservation) = &mut next.maintenance {
                reservation.run_epoch = next.run_epoch;
            }
        }
        StateTransition::StartIntent => {
            if current.maintenance.is_some()
                || current.lease.as_ref().is_some_and(|l| active(l.state))
            {
                return Err(error(ErrorCode::LeaseConflict, context));
            }
            if current.run_intent != RunIntent::Running {
                next.run_epoch = current
                    .run_epoch
                    .checked_add(1)
                    .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?;
            }
            next.run_intent = RunIntent::Running;
        }
        StateTransition::ArmTrial { lease } => {
            if current.run_intent != RunIntent::Running
                || current.maintenance.is_some()
                || lease.started_from_stopped
                || lease.run_epoch != current.run_epoch
                || current.lease.as_ref().is_some_and(|l| active(l.state))
                || lease.state != LeaseState::Armed
                || lease.fallback.fences.credential_revision < current.fences.credential_revision
            {
                return Err(error(ErrorCode::LeaseConflict, context));
            }
            next.trial = Some(lease.candidate.clone());
            next.lease = Some(lease.clone());
        }
        StateTransition::SetLeaseState { lease_id, state } => {
            let lease = next
                .lease
                .as_mut()
                .filter(|l| &l.lease_id == lease_id)
                .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
            let allowed = match (lease.state, *state) {
                (LeaseState::Armed, LeaseState::Activating | LeaseState::Compensating)
                | (
                    LeaseState::Activating,
                    LeaseState::Verifying | LeaseState::Compensating | LeaseState::RecoveryRequired,
                )
                | (
                    LeaseState::Verifying,
                    LeaseState::Compensating | LeaseState::RecoveryRequired,
                )
                | (LeaseState::Compensating, LeaseState::RecoveryRequired)
                | (LeaseState::Committed, LeaseState::RollingBack)
                | (LeaseState::RecoveryRequired, LeaseState::RollingBack)
                | (LeaseState::RollingBack, LeaseState::RecoveryRequired)
                | (LeaseState::RecoveryRequired, LeaseState::Compensating) => true,
                (a, b) if a == b => true,
                _ => false,
            };
            if !allowed
                || (matches!(state, LeaseState::Activating | LeaseState::Verifying)
                    && (lease.run_epoch != current.run_epoch
                        || current.run_intent != RunIntent::Running))
            {
                return Err(error(ErrorCode::Stopped, context));
            }
            lease.state = *state;
        }
        StateTransition::BeginRollback {
            lease_id,
            target_bundle_id,
        } => {
            let lease = next
                .lease
                .as_mut()
                .filter(|l| &l.lease_id == lease_id)
                .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
            let fallback_matches = matches!(
                &lease.fallback.target,
                RollbackTarget::Accepted { bundle_id, .. } if bundle_id == target_bundle_id
            );
            if !matches!(
                lease.state,
                LeaseState::Committed | LeaseState::RecoveryRequired
            ) || !fallback_matches
                || current.run_intent != RunIntent::Running
                || current.maintenance.is_some()
                || lease.run_epoch != current.run_epoch
                || current.accepted.as_ref() != Some(&lease.candidate)
                || current.previous_compatible.as_ref() != Some(target_bundle_id)
            {
                return Err(error(ErrorCode::LeaseConflict, context));
            }
            lease.state = LeaseState::RollingBack;
            next.trial = Some(lease.candidate.clone());
        }
        StateTransition::CommitTrial { lease_id } => {
            let lease = next
                .lease
                .as_mut()
                .filter(|l| &l.lease_id == lease_id)
                .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
            if current.run_intent != RunIntent::Running
                || lease.run_epoch != current.run_epoch
                || lease.state != LeaseState::Verifying
            {
                return Err(error(ErrorCode::Stopped, context));
            }
            next.previous_compatible = current.accepted.clone();
            next.accepted = Some(lease.candidate.clone());
            next.trial = None;
            lease.state = LeaseState::Committed;
        }
        StateTransition::FinishCompensation { lease_id, restored } => {
            let lease = next
                .lease
                .as_mut()
                .filter(|l| &l.lease_id == lease_id)
                .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
            if !matches!(
                lease.state,
                LeaseState::Compensating | LeaseState::RollingBack
            ) || restored != &lease.fallback.target
            {
                return Err(error(ErrorCode::LeaseConflict, context));
            }
            let explicit_rollback = lease.state == LeaseState::RollingBack;
            next.accepted = match restored {
                RollbackTarget::Accepted { bundle_id, .. } => Some(bundle_id.clone()),
                RollbackTarget::PreInstall { .. } => None,
            };
            if explicit_rollback {
                // This rollback consumed the durable one-step fallback. A later
                // rollback target must be established by another qualified commit.
                next.previous_compatible = None;
            }
            next.trial = None;
            lease.state = if current.run_intent == RunIntent::Stopped
                || current.run_epoch != lease.run_epoch
            {
                LeaseState::StoppedWithArtifacts
            } else {
                LeaseState::Compensated
            };
        }
        StateTransition::RecordReceipts { receipts } => {
            let mut resources: BTreeMap<_, _> = current
                .resources
                .as_slice()
                .iter()
                .map(|r| (r.resource.resource_id.clone(), r.clone()))
                .collect();
            for receipt in receipts.as_slice() {
                if receipt.run_epoch != current.run_epoch {
                    return Err(error(ErrorCode::StaleInput, context));
                }
                if current.run_intent == RunIntent::Stopped
                    && receipt.status == ResourceStatus::Running
                {
                    // Observation of a surviving service is allowed and must remain visible.
                }
                resources.insert(receipt.resource.resource_id.clone(), receipt.clone());
            }
            next.resources = Items::new(resources.into_values().collect())
                .map_err(|_| error(ErrorCode::InvalidManifest, context))?;
        }
        StateTransition::RecordOwnedRemovals { resources } => {
            if current.run_intent != RunIntent::Stopped
                || current.trial.is_some()
                || current.maintenance.is_some()
                || current
                    .lease
                    .as_ref()
                    .is_some_and(|lease| active(lease.state))
            {
                return Err(error(ErrorCode::LeaseConflict, context));
            }
            let mut seen = std::collections::BTreeSet::new();
            for resource in resources.as_slice() {
                if resource.ownership != Ownership::Owned
                    || resource.kind != ResourceKind::Container
                    || !seen.insert(resource.resource_id.clone())
                {
                    return Err(error(ErrorCode::ApprovalRequired, context));
                }
                if let Some(receipt) = current
                    .resources
                    .as_slice()
                    .iter()
                    .find(|receipt| receipt.resource.resource_id == resource.resource_id)
                {
                    if receipt.resource != *resource
                        || receipt.status != ResourceStatus::Stopped
                        || receipt.restart_policy != RestartPolicy::Disabled
                    {
                        return Err(error(ErrorCode::ResourceDrift, context));
                    }
                }
            }
            next.resources = Items::new(
                current
                    .resources
                    .as_slice()
                    .iter()
                    .filter(|receipt| !seen.contains(&receipt.resource.resource_id))
                    .cloned()
                    .collect(),
            )
            .map_err(|_| error(ErrorCode::InvalidManifest, context))?;
        }
        StateTransition::AdvanceFence {
            credential_revision,
        } => {
            if current
                .maintenance
                .as_ref()
                .is_some_and(|r| r.operation_id != context.operation_id)
                || *credential_revision < current.fences.credential_revision
                || current.lease.as_ref().is_some_and(|l| active(l.state))
            {
                return Err(error(ErrorCode::FenceConflict, context));
            }
            next.fences.credential_revision = *credential_revision;
        }
    }
    Ok(next)
}

pub fn write_transition(
    store: &dyn StateRepository,
    current: &RuntimeState,
    transition: StateTransition,
    context: &OperationContext,
) -> Result<RuntimeState, RuntimeError> {
    store.write(
        &StateWrite {
            instance_id: current.instance_id.clone(),
            expected_revision: current.revision,
            expected_run_epoch: current.run_epoch,
            transition,
        },
        context,
    )
}

/// One protected durable directory per instance. Reads never create it or a lock.
pub struct RuntimeStateStore {
    directory: hton_storage::SecureDir,
    instance: InstanceId,
    clock: std::sync::Arc<dyn crate::Clock>,
}
impl RuntimeStateStore {
    pub fn new(
        directory: hton_storage::SecureDir,
        instance: InstanceId,
        clock: std::sync::Arc<dyn crate::Clock>,
    ) -> Self {
        Self {
            directory,
            instance,
            clock,
        }
    }
    /// Existing state only; absent is not a synthetic initialized instance.
    pub fn inspect(directory: &hton_storage::SecureDir) -> Result<Option<RuntimeState>, ErrorCode> {
        directory
            .read("state.json", MAX_WIRE_BYTES)
            .map_err(|_| ErrorCode::Io)?
            .map(|bytes| {
                let state: RuntimeState =
                    decode(&bytes).map_err(|_| ErrorCode::RecoveryRequired)?;
                validate_record(&state)?;
                Ok(state)
            })
            .transpose()
    }
    fn parse(
        &self,
        bytes: Option<&[u8]>,
        context: &OperationContext,
    ) -> Result<RuntimeState, RuntimeError> {
        let state: RuntimeState = match bytes {
            Some(bytes) => {
                decode(bytes).map_err(|_| error(ErrorCode::RecoveryRequired, context))?
            }
            None => initial(self.instance.clone()),
        };
        if state.instance_id != self.instance {
            return Err(error(ErrorCode::RecoveryRequired, context));
        }
        validate_record(&state).map_err(|code| error(code, context))?;
        Ok(state)
    }
}
impl StateRepository for RuntimeStateStore {
    fn read(
        &self,
        instance: &InstanceId,
        context: &OperationContext,
    ) -> Result<RuntimeState, RuntimeError> {
        if instance != &self.instance {
            return Err(error(ErrorCode::InvalidManifest, context));
        }
        let bytes = self
            .directory
            .read("state.json", MAX_WIRE_BYTES)
            .map_err(|_| error(ErrorCode::Io, context))?;
        self.parse(bytes.as_deref(), context)
    }
    fn write(
        &self,
        request: &StateWrite,
        context: &OperationContext,
    ) -> Result<RuntimeState, RuntimeError> {
        if request.instance_id != self.instance {
            return Err(error(ErrorCode::InvalidManifest, context));
        }
        let remaining = context
            .budget
            .deadline_ms
            .saturating_sub(self.clock.now_ms())
            .min(30_000);
        if remaining == 0 {
            return Err(error(ErrorCode::DeadlineExceeded, context));
        }
        let locked = self
            .directory
            .lock(std::time::Instant::now() + std::time::Duration::from_millis(remaining))
            .map_err(|_| error(ErrorCode::Io, context))?;
        let bytes = locked
            .read("state.json", MAX_WIRE_BYTES)
            .map_err(|_| error(ErrorCode::Io, context))?;
        let current = self.parse(bytes.as_deref(), context)?;
        let next = transition(&current, request, context)?;
        let replacement =
            serde_json::to_vec(&next).map_err(|_| error(ErrorCode::InvalidManifest, context))?;
        if replacement.len() > MAX_WIRE_BYTES {
            return Err(error(ErrorCode::InvalidManifest, context));
        }
        locked
            .compare_exchange("state.json", bytes.as_deref(), &replacement)
            .map_err(|_| error(ErrorCode::RecoveryRequired, context))?;
        Ok(next)
    }
}

fn validate_record(state: &RuntimeState) -> Result<(), ErrorCode> {
    if state
        .maintenance
        .as_ref()
        .is_some_and(|r| r.run_epoch != state.run_epoch || state.run_intent != RunIntent::Stopped)
        || state.trial.as_ref()
            != state
                .lease
                .as_ref()
                .filter(|l| active(l.state))
                .map(|l| &l.candidate)
        || state
            .lease
            .as_ref()
            .is_some_and(|l| l.run_epoch > state.run_epoch)
    {
        return Err(ErrorCode::RecoveryRequired);
    }
    Ok(())
}
