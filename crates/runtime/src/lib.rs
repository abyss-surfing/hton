//! Trusted independent runtime. Host bindings must enforce port deadlines and identity.
#![forbid(unsafe_code)]
pub mod actions;
pub mod guardian;
pub mod host;
pub mod manifest;
pub mod probes;
pub mod state;

use hton_runtime_contract::*;

pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}
pub trait Cancellation: Send + Sync {
    fn cancelled(&self, operation: &OperationContext) -> bool;
}

pub fn error(code: ErrorCode, context: &OperationContext) -> RuntimeError {
    RuntimeError {
        code,
        operation_id: context.operation_id.clone(),
        resource_id: None,
        service_id: None,
        retryable: matches!(code, ErrorCode::StaleInput | ErrorCode::DeadlineExceeded),
        repair_hint: RepairHint::InspectLocalJournal,
        retained_bundle: None,
    }
}

pub fn check_budget(
    context: &OperationContext,
    clock: &dyn Clock,
    cancellation: &dyn Cancellation,
) -> Result<(), RuntimeError> {
    if cancellation.cancelled(context) {
        return Err(error(ErrorCode::Cancelled, context));
    }
    if clock.now_ms() >= context.budget.deadline_ms {
        return Err(error(ErrorCode::DeadlineExceeded, context));
    }
    // These are runtime authority limits, regardless of what a caller supplied.
    if context.budget.max_concurrency.get() > 16
        || context.budget.max_bytes.get() > 16 * 1024 * 1024
        || context.budget.max_observations.get() > 4096
    {
        return Err(error(ErrorCode::InvalidManifest, context));
    }
    Ok(())
}

/// Runtime dispatcher over already composed trusted ports, independent of Core.
pub struct RuntimeExecutor<'a> {
    pub store: &'a dyn state::StateRepository,
    pub admission: &'a dyn manifest::AdmissionPorts,
    pub actions: &'a actions::ActionExecutor<'a>,
    pub guardian: &'a guardian::RuntimeGuardian<'a>,
    pub shutdown: &'a dyn actions::ShutdownAuthority,
    pub probes: &'a dyn probes::ProbePort,
    pub clock: &'a dyn Clock,
    pub cancellation: &'a dyn Cancellation,
}
impl RuntimeExecutor<'_> {
    pub fn execute(&self, request: &ExecuteRequest) -> Result<RuntimeResult, RuntimeError> {
        let context = &request.context;
        check_budget(context, self.clock, self.cancellation)?;
        let before = self.store.read(&request.instance_id, context)?;
        if before.run_epoch != request.expected_run_epoch {
            return Err(error(ErrorCode::StaleInput, context));
        }
        let mut receipts = Vec::new();
        let mut errors = Vec::new();
        let mut survivors = Vec::new();
        match &request.action {
            ExecutorAction::StopAll => {
                return self.actions.stop_all(
                    &request.instance_id,
                    request.expected_run_epoch,
                    self.shutdown,
                    context,
                );
            }
            ExecutorAction::UninstallOwned { resources } => {
                return self.actions.uninstall_owned(
                    &request.instance_id,
                    request.expected_run_epoch,
                    resources.as_slice(),
                    self.shutdown,
                    context,
                );
            }
            ExecutorAction::Observe => (),
            ExecutorAction::StartAccepted { bundle_id } => {
                // Explicit local startup qualifies before changing durable intent.
                // Cold boot calls this only while already Running.
                let mut prospective = before.clone();
                if before.run_intent == RunIntent::Stopped {
                    prospective.run_intent = RunIntent::Running;
                    prospective.run_epoch = before
                        .run_epoch
                        .checked_add(1)
                        .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?;
                }
                let _qualified = manifest::admit_accepted(
                    bundle_id,
                    &prospective,
                    self.admission,
                    self.clock,
                    context,
                )?;
                let started = if before.run_intent == RunIntent::Stopped {
                    state::write_transition(
                        self.store,
                        &before,
                        StateTransition::StartIntent,
                        context,
                    )?
                } else {
                    before.clone()
                };
                let bundle = manifest::admit_accepted(
                    bundle_id,
                    &started,
                    self.admission,
                    self.clock,
                    context,
                )?;
                receipts = self
                    .actions
                    .start_accepted(&bundle, started.run_epoch, context)?;
            }
            ExecutorAction::Activate { bundle_id, proof } => {
                let bundle = manifest::admit(
                    bundle_id,
                    proof,
                    &before,
                    self.admission,
                    self.clock,
                    context,
                )?;
                let lease_id = LeaseId::new(format!(
                    "trial-{}",
                    &manifest::hash(context.operation_id.as_str().as_bytes()).as_str()[7..]
                ))
                .map_err(|_| error(ErrorCode::InvalidManifest, context))?;
                let deadline = context
                    .budget
                    .deadline_ms
                    .min(self.clock.now_ms().saturating_add(120_000));
                self.guardian.arm(
                    &bundle,
                    &proof.fallback,
                    lease_id.clone(),
                    deadline,
                    before.run_epoch,
                    context,
                )?;
                let current = self.store.read(&request.instance_id, context)?;
                state::write_transition(
                    self.store,
                    &current,
                    StateTransition::SetLeaseState {
                        lease_id: lease_id.clone(),
                        state: LeaseState::Activating,
                    },
                    context,
                )?;
                receipts = self.actions.apply(&bundle, before.run_epoch, context)?;
                if receipts.iter().any(|r| {
                    matches!(
                        r.outcome,
                        ActionOutcome::Failed { .. } | ActionOutcome::Uncertain { .. }
                    )
                }) {
                    let _ = self
                        .guardian
                        .recover(&request.instance_id, &lease_id, context);
                } else {
                    let current = self.store.read(&request.instance_id, context)?;
                    state::write_transition(
                        self.store,
                        &current,
                        StateTransition::SetLeaseState {
                            lease_id: lease_id.clone(),
                            state: LeaseState::Verifying,
                        },
                        context,
                    )?;
                    if self
                        .guardian
                        .monitor(&request.instance_id, &lease_id, context)?
                        != guardian::MonitorOutcome::Committed
                    {
                        errors.push(error(ErrorCode::Partial, context));
                    }
                }
            }
            ExecutorAction::CommitTrial { lease_id } => {
                if self
                    .guardian
                    .monitor(&request.instance_id, lease_id, context)?
                    != guardian::MonitorOutcome::Committed
                {
                    errors.push(error(ErrorCode::Partial, context));
                }
            }
            ExecutorAction::RollbackAccepted { bundle_id } => {
                let outcome =
                    self.guardian
                        .rollback_accepted(&request.instance_id, bundle_id, context)?;
                match outcome {
                    guardian::MonitorOutcome::Compensated => (),
                    guardian::MonitorOutcome::StoppedWithArtifacts => {
                        errors.push(error(ErrorCode::Partial, context));
                    }
                    _ => errors.push(error(ErrorCode::RecoveryRequired, context)),
                }
            }
            ExecutorAction::RestartAccepted {
                bundle_id,
                expected_run_epoch,
                service_ids,
            } => {
                if expected_run_epoch != &before.run_epoch {
                    return Err(error(ErrorCode::StaleInput, context));
                }
                if service_ids.as_slice().is_empty()
                    || before.run_intent != RunIntent::Running
                    || before.trial.is_some()
                    || before.lease.as_ref().is_some_and(|lease| {
                        matches!(
                            lease.state,
                            LeaseState::Armed
                                | LeaseState::Activating
                                | LeaseState::Verifying
                                | LeaseState::Compensating
                                | LeaseState::RollingBack
                                | LeaseState::RecoveryRequired
                        )
                    })
                    || before.maintenance.is_some()
                {
                    return Err(error(ErrorCode::LeaseConflict, context));
                }
                if before.accepted.as_ref() != Some(bundle_id) {
                    return Err(error(ErrorCode::StaleInput, context));
                }
                let bundle = manifest::admit_accepted(
                    bundle_id,
                    &before,
                    self.admission,
                    self.clock,
                    context,
                )?;
                receipts = self.actions.restart_accepted(
                    &bundle,
                    service_ids.as_slice(),
                    before.run_epoch,
                    context,
                )?;
                survivors.extend(
                    receipts
                        .iter()
                        .filter_map(|receipt| match &receipt.outcome {
                            ActionOutcome::Completed { receipt }
                            | ActionOutcome::Unchanged { receipt }
                                if receipt.status == ResourceStatus::Running =>
                            {
                                Some(receipt.resource.resource_id.clone())
                            }
                            _ => None,
                        }),
                );
                match probes::verify_selected(
                    &bundle,
                    service_ids.as_slice(),
                    self.probes,
                    self.clock,
                    self.cancellation,
                    context,
                ) {
                    Ok(report) if report.blocking => {
                        // The restart effects have already been durably recorded.
                        // Preserve their receipts and return partial instead of an IPC
                        // error that would hide the completed native effects.
                        errors.push(error(ErrorCode::NativeEvidenceMissing, context));
                    }
                    Ok(_) => (),
                    Err(probe_error) => errors.push(probe_error),
                }
            }
            ExecutorAction::StartCandidate { .. }
            | ExecutorAction::ReserveMaintenance { .. }
            | ExecutorAction::ReleaseMaintenance { .. }
            | ExecutorAction::RestoreStopped { .. }
            | ExecutorAction::AdvanceFence { .. } => {
                // These transitions require the installed host's independent
                // metadata/artifact qualification, not a generic executor bypass.
                return Err(error(ErrorCode::ApprovalRequired, context));
            }
            ExecutorAction::Recover { lease_id } => {
                if self
                    .guardian
                    .recover(&request.instance_id, lease_id, context)?
                    == guardian::MonitorOutcome::RecoveryRequired
                {
                    errors.push(error(ErrorCode::RecoveryRequired, context));
                }
            }
        }
        for receipt in &receipts {
            if let ActionOutcome::Failed { error } | ActionOutcome::Uncertain { error } =
                &receipt.outcome
            {
                errors.push(error.clone());
            }
        }
        let state = self.store.read(&request.instance_id, context)?;
        let status = if errors.iter().any(|e| e.code == ErrorCode::RecoveryRequired) {
            ResultStatus::RecoveryRequired
        } else if !errors.is_empty() {
            ResultStatus::Partial
        } else {
            ResultStatus::Succeeded
        };
        Ok(RuntimeResult {
            contract_version: ContractVersion::V1,
            operation_id: context.operation_id.clone(),
            status,
            run_intent: state.run_intent,
            run_epoch: state.run_epoch,
            state_revision: state.revision,
            accepted: state.accepted,
            receipts: Items::new(receipts)
                .map_err(|_| error(ErrorCode::InvalidManifest, context))?,
            survivors: Items::new(survivors)
                .map_err(|_| error(ErrorCode::InvalidManifest, context))?,
            errors: Items::new(errors).map_err(|_| error(ErrorCode::InvalidManifest, context))?,
        })
    }
}

fn logging_reason(code: ErrorCode) -> hton_logging::FailureReason {
    use hton_logging::FailureReason as R;
    match code {
        ErrorCode::UnsupportedVersion => R::UnsupportedFormat,
        ErrorCode::InvalidManifest => R::SemanticConflict,
        ErrorCode::StaleInput | ErrorCode::LeaseConflict => R::StaleInput,
        ErrorCode::ResourceDrift => R::ResourceDrift,
        ErrorCode::ApprovalRequired => R::ApprovalRequired,
        ErrorCode::NativeEvidenceMissing | ErrorCode::StagingEvidenceInvalid => R::NativeValidation,
        ErrorCode::MissingArtifact | ErrorCode::MissingImage => R::MissingArtifact,
        ErrorCode::SecretUnavailable => R::MissingSecret,
        ErrorCode::FenceConflict
        | ErrorCode::FallbackIncompatible
        | ErrorCode::RecoveryRequired => R::RecoveryRequired,
        ErrorCode::Stopped => R::Stopped,
        ErrorCode::Partial => R::Partial,
        ErrorCode::DeadlineExceeded | ErrorCode::LeaseExpired => R::DeadlineExceeded,
        ErrorCode::Cancelled => R::Cancelled,
        ErrorCode::Io => R::StorageUnavailable,
    }
}
