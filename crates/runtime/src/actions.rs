//! Fixed native operations, bounded by trusted identity and short effect locks.
use crate::{
    Cancellation, Clock, check_budget, error,
    manifest::AdmittedBundle,
    state::{StateRepository, write_transition},
};
use hton_runtime_contract::*;
use std::collections::{BTreeMap, BTreeSet};

pub trait EffectGuard {}
/// A separate short-held runtime lock; never a management/config lock.
pub trait EffectGate: Send + Sync {
    fn acquire(
        &self,
        context: &OperationContext,
    ) -> Result<Box<dyn EffectGuard + '_>, RuntimeError>;
}

pub struct NativeRequest<'a> {
    action: FixedAction,
    resource: &'a ResourceIdentity,
    bundle: Option<&'a AdmittedBundle>,
    run_epoch: u64,
}
impl NativeRequest<'_> {
    pub fn action(&self) -> &FixedAction {
        &self.action
    }
    pub fn resource(&self) -> &ResourceIdentity {
        self.resource
    }
    pub fn bundle(&self) -> Option<&AdmittedBundle> {
        self.bundle
    }
    pub fn run_epoch(&self) -> u64 {
        self.run_epoch
    }
}
/// Must enforce context deadline, fixed enum, supplied exact identity and output cap.
/// Return uncertain if the child may have effected a change before timing out.
pub trait NativeAction: Send + Sync {
    /// Confirm exact Owned container deletion. Implementations must durably
    /// record intent before removal and distinguish unexplained absence from
    /// interrupted completion. No data, image or volume deletion authority.
    fn remove_stopped_owned(
        &self,
        _resource: &ResourceIdentity,
        context: &OperationContext,
    ) -> Result<(), RuntimeError> {
        Err(error(ErrorCode::NativeEvidenceMissing, context))
    }
    fn run(
        &self,
        request: &NativeRequest<'_>,
        context: &OperationContext,
    ) -> Result<ResourceReceipt, RuntimeError>;
    fn observe(
        &self,
        resource: &ResourceIdentity,
        context: &OperationContext,
    ) -> Result<ResourceReceipt, RuntimeError>;
    /// Compare observed native launch/configuration with exact admitted artifacts;
    /// status alone cannot establish that a healthy service is the accepted service.
    fn matches_admitted(
        &self,
        bundle: &AdmittedBundle,
        resource: &ResourceIdentity,
        observed: &ResourceReceipt,
        context: &OperationContext,
    ) -> Result<bool, RuntimeError>;
}

#[derive(Clone)]
pub struct ShutdownResource {
    pub resource: ResourceIdentity,
    pub service_id: ServiceId,
    pub dependencies: Vec<ResourceId>,
}
/// Reads exact associated resources and restart owners from protected authority;
/// discovery alone is never sufficient. Includes timers/watchers/systemd owners.
pub trait ShutdownAuthority: Send + Sync {
    fn resources(
        &self,
        instance: &InstanceId,
        context: &OperationContext,
    ) -> Result<Vec<ShutdownResource>, RuntimeError>;
}

pub struct ActionExecutor<'a> {
    pub store: &'a dyn StateRepository,
    pub gate: &'a dyn EffectGate,
    pub native: &'a dyn NativeAction,
    pub clock: &'a dyn Clock,
    pub cancellation: &'a dyn Cancellation,
    pub logs: &'a hton_logging::LogSink,
}
impl ActionExecutor<'_> {
    fn execute_one(
        &self,
        instance: &InstanceId,
        expected_epoch: u64,
        action: &FixedAction,
        resource: &ResourceIdentity,
        bundle: Option<&AdmittedBundle>,
        allow_stopped: bool,
        context: &OperationContext,
    ) -> ActionOutcome {
        let attempt = || -> Result<(ResourceReceipt, bool), RuntimeError> {
            check_budget(context, self.clock, self.cancellation)?;
            let _lock = self.gate.acquire(context)?;
            let state = self.store.read(instance, context)?;
            let restart_blocked = action.kind == NativeActionKind::Restart
                && (state.trial.is_some()
                    || state.lease.as_ref().is_some_and(|lease| {
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
                    || bundle.is_none_or(|bundle| {
                        state.accepted.as_ref() != Some(&bundle.candidate().bundle_id)
                    }));
            if state.run_epoch != expected_epoch
                || (!allow_stopped && state.run_intent != RunIntent::Running)
                || restart_blocked
                || (state.maintenance.is_some()
                    && !(allow_stopped
                        && matches!(
                            action.kind,
                            NativeActionKind::Stop | NativeActionKind::DisableAutostart
                        )))
            {
                return Err(error(ErrorCode::Stopped, context));
            }
            let bound_resource = state.resources.as_slice().iter().find(|r| {
                r.resource.resource_id == resource.resource_id
                    && bundle.is_some_and(|b| {
                        r.effective_bundle.as_ref() == Some(&b.candidate().bundle_id)
                    })
            });
            let resource = match bound_resource {
                Some(receipt)
                    if receipt.resource.kind == resource.kind
                        && receipt.resource.ownership == resource.ownership =>
                {
                    &receipt.resource
                }
                _ => resource,
            };
            let mut bounded = context.clone();
            bounded.budget.deadline_ms = bounded.budget.deadline_ms.min(
                self.clock
                    .now_ms()
                    .saturating_add(u64::from(action.timeout_ms.get())),
            );
            let observed = self.native.observe(resource, &bounded)?;
            check_budget(&bounded, self.clock, self.cancellation)?;
            if !same_identity(&observed.resource, resource)
                || action
                    .before
                    .as_ref()
                    .is_some_and(|before| before != &observed.resource.fingerprint)
            {
                return Err(error(ErrorCode::ResourceDrift, context));
            }
            if action.kind == NativeActionKind::Restart {
                let admitted = bundle.ok_or_else(|| error(ErrorCode::InvalidManifest, context))?;
                if !self
                    .native
                    .matches_admitted(admitted, resource, &observed, &bounded)?
                {
                    return Err(error(ErrorCode::ResourceDrift, context));
                }
                check_budget(&bounded, self.clock, self.cancellation)?;
            }
            if action.before.is_none()
                && matches!(
                    action.kind,
                    NativeActionKind::Start | NativeActionKind::EnableAutostart
                )
            {
                let admitted = bundle.ok_or_else(|| error(ErrorCode::InvalidManifest, context))?;
                if !self
                    .native
                    .matches_admitted(admitted, resource, &observed, &bounded)?
                {
                    return Err(error(ErrorCode::ResourceDrift, context));
                }
                check_budget(&bounded, self.clock, self.cancellation)?;
            }
            let unchanged = match action.kind {
                NativeActionKind::Start => observed.status == ResourceStatus::Running,
                NativeActionKind::Stop => observed.status == ResourceStatus::Stopped,
                NativeActionKind::DisableAutostart => {
                    observed.restart_policy == RestartPolicy::Disabled
                }
                NativeActionKind::EnableAutostart => matches!(
                    observed.restart_policy,
                    RestartPolicy::Always | RestartPolicy::UnlessStopped | RestartPolicy::OnFailure
                ),
                _ => false,
            };
            let mut receipt = if unchanged {
                observed
            } else {
                self.native.run(
                    &NativeRequest {
                        action: action.clone(),
                        resource,
                        bundle,
                        run_epoch: expected_epoch,
                    },
                    &bounded,
                )?
            };
            check_budget(&bounded, self.clock, self.cancellation)?;
            if !unchanged {
                let actual = self.native.observe(&receipt.resource, &bounded)?;
                check_budget(&bounded, self.clock, self.cancellation)?;
                if !same_identity(&actual.resource, &receipt.resource)
                    || actual.resource.fingerprint != receipt.resource.fingerprint
                    || actual.status != receipt.status
                    || actual.restart_policy != receipt.restart_policy
                {
                    return Err(error(ErrorCode::ResourceDrift, context));
                }
                if action.kind == NativeActionKind::Restart {
                    let admitted =
                        bundle.ok_or_else(|| error(ErrorCode::InvalidManifest, context))?;
                    if !self.native.matches_admitted(
                        admitted,
                        &actual.resource,
                        &actual,
                        &bounded,
                    )? {
                        return Err(error(ErrorCode::ResourceDrift, context));
                    }
                    check_budget(&bounded, self.clock, self.cancellation)?;
                }
                // Effective bundle is an authenticated execution receipt binding;
                // independent observation confirms native state without inventing it.
                receipt.timestamp_ms = actual.timestamp_ms;
            }
            let restored_parked = action.kind==NativeActionKind::Stop
                && receipt.status==ResourceStatus::Stopped
                && receipt.effective_bundle.is_none()
                && state.lease.as_ref().is_some_and(|lease|matches!(&lease.fallback.target,RollbackTarget::PreInstall{closed_listeners,..} if closed_listeners.as_slice().contains(&resource.resource_id)));
            let recreated = matches!(
                action.kind,
                NativeActionKind::Recreate | NativeActionKind::Start | NativeActionKind::Stop
            ) && resource.ownership == Ownership::Owned
                && receipt.resource.resource_id == resource.resource_id
                && receipt.resource.kind == resource.kind
                && receipt.resource.ownership == Ownership::Owned
                && action.after.as_ref() == Some(&receipt.resource.fingerprint)
                && bundle.is_some_and(|b| {
                    receipt.effective_bundle.as_ref() == Some(&b.candidate().bundle_id)
                        || restored_parked
                });
            if (!same_identity(&receipt.resource, resource) && !recreated)
                || action
                    .after
                    .as_ref()
                    .is_some_and(|after| after != &receipt.resource.fingerprint)
                || (!unchanged && receipt.run_epoch != expected_epoch)
            {
                return Err(error(ErrorCode::ResourceDrift, context));
            }
            receipt.run_epoch = expected_epoch; // Epoch records this observation, not native process state.
            match action.kind {
                NativeActionKind::Stop if receipt.status != ResourceStatus::Stopped => {
                    return Err(error(ErrorCode::Partial, context));
                }
                NativeActionKind::DisableAutostart
                    if receipt.restart_policy != RestartPolicy::Disabled =>
                {
                    return Err(error(ErrorCode::Partial, context));
                }
                NativeActionKind::Start
                | NativeActionKind::Restart
                | NativeActionKind::Recreate
                    if receipt.status != ResourceStatus::Running =>
                {
                    return Err(error(ErrorCode::Partial, context));
                }
                _ => (),
            }
            let current = self.store.read(instance, context)?;
            if current.run_epoch != expected_epoch {
                return Err(error(ErrorCode::StaleInput, context));
            }
            write_transition(
                self.store,
                &current,
                StateTransition::RecordReceipts {
                    receipts: Items::new(vec![receipt.clone()])
                        .map_err(|_| error(ErrorCode::InvalidManifest, context))?,
                },
                context,
            )?;
            Ok((receipt, unchanged))
        };
        let result = attempt();
        let _ = self.logs.emit(
            hton_logging::Component::Runtime,
            hton_logging::EventContext {
                operation_id: hton_logging::CorrelationId::new(context.operation_id.as_str()).ok(),
                service_id: hton_logging::CorrelationId::new(action.service_id.as_str()).ok(),
                resource_id: hton_logging::CorrelationId::new(action.resource_id.as_str()).ok(),
                ..Default::default()
            },
            if result.is_ok() {
                hton_logging::Severity::Information
            } else {
                hton_logging::Severity::Error
            },
            hton_logging::EventCode::StepRecorded,
            hton_logging::SafeFields {
                reason: result.as_ref().err().map(|e| crate::logging_reason(e.code)),
                retryable: result.as_ref().err().map(|e| e.retryable),
                outcome: Some(if result.is_ok() {
                    hton_logging::Outcome::Succeeded
                } else {
                    hton_logging::Outcome::Failed
                }),
                ..Default::default()
            },
        );
        match result {
            Ok((receipt, true)) => ActionOutcome::Unchanged { receipt },
            Ok((receipt, false)) => ActionOutcome::Completed { receipt },
            Err(e)
                if matches!(
                    e.code,
                    ErrorCode::DeadlineExceeded
                        | ErrorCode::Io
                        | ErrorCode::RecoveryRequired
                        | ErrorCode::StaleInput
                ) =>
            {
                ActionOutcome::Uncertain { error: e }
            }
            Err(e) => ActionOutcome::Failed { error: e },
        }
    }

    pub fn apply(
        &self,
        bundle: &AdmittedBundle,
        expected_epoch: u64,
        context: &OperationContext,
    ) -> Result<Vec<ActionReceipt>, RuntimeError> {
        if !bundle.activation_ready() {
            return Err(error(ErrorCode::InvalidManifest, context));
        }
        let candidate = bundle.candidate();
        let compensation: BTreeSet<_> = candidate
            .actions
            .as_slice()
            .iter()
            .filter_map(|a| a.compensation.as_ref())
            .collect();
        let mut pending: Vec<_> = candidate
            .actions
            .as_slice()
            .iter()
            .filter(|a| !compensation.contains(&a.action_id))
            .collect();
        let mut completed = BTreeSet::new();
        let mut outcomes = Vec::new();
        while !pending.is_empty() {
            let index = pending
                .iter()
                .position(|a| {
                    a.dependencies
                        .as_slice()
                        .iter()
                        .all(|id| completed.contains(id))
                })
                .ok_or_else(|| error(ErrorCode::InvalidManifest, context))?;
            let action = pending.remove(index);
            let resource = bundle
                .resource(&action.resource_id)
                .ok_or_else(|| error(ErrorCode::ResourceDrift, context))?;
            let outcome = self.execute_one(
                &candidate.instance_id,
                expected_epoch,
                action,
                resource,
                Some(bundle),
                false,
                context,
            );
            let succeeded = matches!(
                outcome,
                ActionOutcome::Completed { .. } | ActionOutcome::Unchanged { .. }
            );
            outcomes.push(ActionReceipt {
                action_id: action.action_id.clone(),
                outcome,
            });
            if !succeeded {
                break;
            } // Guardian owns compensation; never continue dependent promotions.
            completed.insert(action.action_id.clone());
        }
        Ok(outcomes)
    }

    pub fn start_accepted(
        &self,
        bundle: &AdmittedBundle,
        expected_epoch: u64,
        context: &OperationContext,
    ) -> Result<Vec<ActionReceipt>, RuntimeError> {
        if !bundle.activation_ready() {
            return Err(error(ErrorCode::InvalidManifest, context));
        }
        let candidate = bundle.candidate();
        let mut remaining: Vec<_> = candidate.services.as_slice().iter().collect();
        let mut visited = BTreeSet::new();
        let mut failed = BTreeSet::new();
        let mut receipts = Vec::new();
        while !remaining.is_empty() {
            let index = remaining
                .iter()
                .position(|service| {
                    service
                        .dependencies
                        .as_slice()
                        .iter()
                        .all(|d| visited.contains(d))
                })
                .ok_or_else(|| error(ErrorCode::InvalidManifest, context))?;
            let service = remaining.remove(index);
            let blocked = service
                .dependencies
                .as_slice()
                .iter()
                .any(|id| failed.contains(id));
            let mut service_failed = blocked;
            let mut targets = vec![
                bundle
                    .resource(&service.resource.resource_id)
                    .ok_or_else(|| error(ErrorCode::ResourceDrift, context))?,
            ];
            for owner in service.restart_owner_resources.as_slice() {
                targets.push(
                    bundle
                        .resource(owner)
                        .ok_or_else(|| error(ErrorCode::ResourceDrift, context))?,
                );
            }
            for target in targets {
                for kind in [NativeActionKind::EnableAutostart, NativeActionKind::Start] {
                    // Restart owners (timers/units) are enabled explicitly; their Start is
                    // a separate admitted native action, never inferred Docker authority.
                    let action_id = ActionId::new(format!(
                        "boot-{}-{}",
                        &crate::manifest::hash(target.resource_id.as_str().as_bytes()).as_str()
                            [7..23],
                        kind as u8
                    ))
                    .map_err(|_| error(ErrorCode::InvalidManifest, context))?;
                    let action = FixedAction {
                        action_id: action_id.clone(),
                        kind,
                        resource_id: target.resource_id.clone(),
                        service_id: service.service_id.clone(),
                        dependencies: Items::default(),
                        before: None,
                        after: None,
                        timeout_ms: std::num::NonZeroU32::new(15_000).expect("nonzero"),
                        compensation: None,
                    };
                    let outcome = if blocked {
                        ActionOutcome::Failed {
                            error: error(ErrorCode::Partial, context),
                        }
                    } else {
                        self.execute_one(
                            &candidate.instance_id,
                            expected_epoch,
                            &action,
                            target,
                            Some(bundle),
                            false,
                            context,
                        )
                    };
                    service_failed |= matches!(
                        outcome,
                        ActionOutcome::Failed { .. } | ActionOutcome::Uncertain { .. }
                    );
                    receipts.push(ActionReceipt { action_id, outcome });
                }
            }
            visited.insert(service.service_id.clone());
            if service_failed {
                failed.insert(service.service_id.clone());
            }
        }
        Ok(receipts)
    }

    /// Restarts only services in the independently admitted current bundle.
    /// Selection must be explicit; callers resolve any default from the saved
    /// enabled optional/data-service set before crossing the runtime boundary.
    pub fn restart_accepted(
        &self,
        bundle: &AdmittedBundle,
        selected: &[ServiceId],
        expected_epoch: u64,
        context: &OperationContext,
    ) -> Result<Vec<ActionReceipt>, RuntimeError> {
        if !bundle.activation_ready() || selected.is_empty() {
            return Err(error(ErrorCode::InvalidManifest, context));
        }
        let candidate = bundle.candidate();
        let mut targets: Vec<_> = selected
            .iter()
            .map(|id| {
                candidate
                    .services
                    .as_slice()
                    .iter()
                    .find(|s| &s.service_id == id)
                    .ok_or_else(|| error(ErrorCode::MissingArtifact, context))
            })
            .collect::<Result<_, _>>()?;
        targets.sort_by(|a, b| a.service_id.cmp(&b.service_id));
        if targets
            .windows(2)
            .any(|pair| pair[0].service_id == pair[1].service_id)
            || targets
                .iter()
                .any(|service| service.resource.ownership != Ownership::Owned)
        {
            return Err(error(ErrorCode::ApprovalRequired, context));
        }
        let mut receipts = Vec::with_capacity(targets.len());
        for service in targets {
            check_budget(context, self.clock, self.cancellation)?;
            let resource = bundle
                .resource(&service.resource.resource_id)
                .ok_or_else(|| error(ErrorCode::ResourceDrift, context))?;
            if resource.ownership != Ownership::Owned || resource != &service.resource {
                return Err(error(ErrorCode::ResourceDrift, context));
            }
            let action_id = ActionId::new(format!(
                "restart-{}",
                &crate::manifest::hash(service.service_id.as_str().as_bytes()).as_str()[7..23]
            ))
            .map_err(|_| error(ErrorCode::InvalidManifest, context))?;
            let action = FixedAction {
                action_id: action_id.clone(),
                kind: NativeActionKind::Restart,
                resource_id: resource.resource_id.clone(),
                service_id: service.service_id.clone(),
                dependencies: Items::default(),
                before: None,
                after: None,
                timeout_ms: std::num::NonZeroU32::new(30_000).expect("nonzero"),
                compensation: None,
            };
            let outcome = self.execute_one(
                &candidate.instance_id,
                expected_epoch,
                &action,
                resource,
                Some(bundle),
                false,
                context,
            );
            receipts.push(ActionReceipt { action_id, outcome });
        }
        Ok(receipts)
    }

    /// Restore owned service artifact bindings without reviving a later stopped epoch.
    pub fn restore_fallback(
        &self,
        fallback: &AdmittedBundle,
        lease_epoch: u64,
        context: &OperationContext,
    ) -> Result<Vec<ActionReceipt>, RuntimeError> {
        let candidate = fallback.candidate();
        let current = self.store.read(&candidate.instance_id, context)?;
        let mut receipts = Vec::new();
        for service in candidate.services.as_slice() {
            if service.resource.ownership == Ownership::Associated {
                continue;
            }
            let action_id = ActionId::new(format!(
                "restore-{}",
                &crate::manifest::hash(service.service_id.as_str().as_bytes()).as_str()[7..23]
            ))
            .map_err(|_| error(ErrorCode::InvalidManifest, context))?;
            let action = FixedAction {
                action_id: action_id.clone(),
                kind: NativeActionKind::RestoreArtifacts,
                resource_id: service.resource.resource_id.clone(),
                service_id: service.service_id.clone(),
                dependencies: Items::default(),
                before: None,
                after: None,
                timeout_ms: std::num::NonZeroU32::new(15_000).expect("nonzero"),
                compensation: None,
            };
            let outcome = self.execute_one(
                &candidate.instance_id,
                current.run_epoch,
                &action,
                &service.resource,
                Some(fallback),
                true,
                context,
            );
            receipts.push(ActionReceipt { action_id, outcome });
        }
        if receipts.iter().any(|r| {
            matches!(
                r.outcome,
                ActionOutcome::Failed { .. } | ActionOutcome::Uncertain { .. }
            )
        }) {
            return Ok(receipts);
        }
        let state = self.store.read(&candidate.instance_id, context)?;
        if state.run_intent == RunIntent::Running && state.run_epoch == lease_epoch {
            receipts.extend(self.start_accepted(fallback, lease_epoch, context)?);
        }
        Ok(receipts)
    }

    /// Stop fence is persisted before attempts, even when desired YAML is broken.
    pub fn stop_all(
        &self,
        instance: &InstanceId,
        expected_epoch: u64,
        authority: &dyn ShutdownAuthority,
        context: &OperationContext,
    ) -> Result<RuntimeResult, RuntimeError> {
        let current = self.store.read(instance, context)?;
        if current.run_epoch != expected_epoch {
            return Err(error(ErrorCode::StaleInput, context));
        }
        let stopped = write_transition(self.store, &current, StateTransition::StopIntent, context)?;
        let mut resources = authority.resources(instance, context)?;
        // Registry scope stays authoritative, while a protected owned recreation
        // receipt carries its newer exact native ID. Never adopt associated drift.
        for member in &mut resources {
            if let Some(receipt) = current
                .resources
                .as_slice()
                .iter()
                .find(|r| r.resource.resource_id == member.resource.resource_id)
            {
                let approved_bundle = receipt.effective_bundle.as_ref().is_some_and(|id| {
                    current.accepted.as_ref() == Some(id) || current.trial.as_ref() == Some(id)
                });
                if approved_bundle
                    && member.resource.ownership == Ownership::Owned
                    && receipt.resource.ownership == Ownership::Owned
                    && member.resource.kind == receipt.resource.kind
                {
                    member.resource = receipt.resource.clone();
                }
            }
        }
        if resources.len() > 1024 {
            return Err(error(ErrorCode::InvalidManifest, context));
        }
        let mut unique = BTreeSet::new();
        if !resources
            .iter()
            .all(|r| unique.insert(r.resource.resource_id.clone()))
        {
            return Err(error(ErrorCode::InvalidManifest, context));
        }
        let mut ordered = Vec::new();
        let mut remaining: Vec<_> = resources.iter().collect();
        while !remaining.is_empty() {
            let index = remaining
                .iter()
                .position(|r| {
                    !remaining
                        .iter()
                        .any(|other| other.dependencies.contains(&r.resource.resource_id))
                })
                .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?;
            ordered.push(remaining.remove(index));
        }
        let mut receipts = Vec::new();
        let mut errors = Vec::new();
        let mut survivors = Vec::new();
        // Disable all restart owners before stops; each resource remains attempted after failures.
        for kind in [NativeActionKind::DisableAutostart, NativeActionKind::Stop] {
            for member in &ordered {
                let prefix = if kind == NativeActionKind::Stop {
                    "stop"
                } else {
                    "autostart"
                };
                let id = ActionId::new(format!(
                    "{prefix}-{}",
                    &crate::manifest::hash(member.resource.resource_id.as_str().as_bytes())
                        .as_str()[7..]
                ))
                .map_err(|_| error(ErrorCode::InvalidManifest, context))?;
                let action = FixedAction {
                    action_id: id.clone(),
                    kind,
                    resource_id: member.resource.resource_id.clone(),
                    service_id: member.service_id.clone(),
                    dependencies: Items::default(),
                    before: None,
                    after: None,
                    timeout_ms: std::num::NonZeroU32::new(15_000).expect("nonzero"),
                    compensation: None,
                };
                let outcome = self.execute_one(
                    instance,
                    stopped.run_epoch,
                    &action,
                    &member.resource,
                    None,
                    true,
                    context,
                );
                if let ActionOutcome::Failed { error: e } | ActionOutcome::Uncertain { error: e } =
                    &outcome
                {
                    errors.push(e.clone());
                }
                receipts.push(ActionReceipt {
                    action_id: id,
                    outcome,
                });
            }
        }
        let mut final_observations = Vec::new();
        for member in &ordered {
            let mut bounded = context.clone();
            bounded.budget.deadline_ms = bounded
                .budget
                .deadline_ms
                .min(self.clock.now_ms().saturating_add(15_000));
            let attempt = || -> Result<ResourceReceipt, RuntimeError> {
                check_budget(&bounded, self.clock, self.cancellation)?;
                let receipt = self.native.observe(&member.resource, &bounded)?;
                check_budget(&bounded, self.clock, self.cancellation)?;
                if !same_identity(&receipt.resource, &member.resource) {
                    return Err(error(ErrorCode::ResourceDrift, context));
                }
                let mut receipt = receipt;
                receipt.run_epoch = stopped.run_epoch;
                Ok(receipt)
            };
            match attempt() {
                Ok(receipt) => {
                    if receipt.status != ResourceStatus::Stopped
                        || receipt.restart_policy != RestartPolicy::Disabled
                    {
                        survivors.push(member.resource.resource_id.clone());
                    }
                    final_observations.push(receipt);
                }
                Err(e) => {
                    errors.push(e);
                    survivors.push(member.resource.resource_id.clone());
                    final_observations.push(ResourceReceipt {
                        resource: member.resource.clone(),
                        status: ResourceStatus::Unknown,
                        restart_policy: RestartPolicy::External,
                        effective_bundle: None,
                        timestamp_ms: self.clock.now_ms(),
                        run_epoch: stopped.run_epoch,
                    });
                }
            }
        }
        let current = self.store.read(instance, context)?;
        if current.run_epoch != stopped.run_epoch {
            return Err(error(ErrorCode::StaleInput, context));
        }
        write_transition(
            self.store,
            &current,
            StateTransition::RecordReceipts {
                receipts: Items::new(final_observations)
                    .map_err(|_| error(ErrorCode::InvalidManifest, context))?,
            },
            context,
        )?;
        let final_state = self.store.read(instance, context)?;
        Ok(RuntimeResult {
            contract_version: ContractVersion::V1,
            operation_id: context.operation_id.clone(),
            status: if errors.is_empty() && survivors.is_empty() {
                ResultStatus::Succeeded
            } else {
                ResultStatus::Partial
            },
            run_intent: final_state.run_intent,
            run_epoch: final_state.run_epoch,
            state_revision: final_state.revision,
            accepted: final_state.accepted,
            receipts: Items::new(receipts)
                .map_err(|_| error(ErrorCode::InvalidManifest, context))?,
            survivors: Items::new(survivors)
                .map_err(|_| error(ErrorCode::InvalidManifest, context))?,
            errors: Items::new(errors).map_err(|_| error(ErrorCode::InvalidManifest, context))?,
        })
    }

    /// Compensation executes only declared fixed compensation actions. A later down
    /// permits artifact restoration/stop only, never a start/reload/recreate.
    pub fn compensate(
        &self,
        candidate: &AdmittedBundle,
        fallback: Option<&AdmittedBundle>,
        lease_epoch: u64,
        context: &OperationContext,
    ) -> Result<Vec<ActionReceipt>, RuntimeError> {
        let bundle = candidate.candidate();
        let compensation_ids: BTreeSet<_> = bundle
            .actions
            .as_slice()
            .iter()
            .filter_map(|a| a.compensation.as_ref())
            .collect();
        let mut pending: Vec<_> = bundle
            .actions
            .as_slice()
            .iter()
            .filter(|a| !compensation_ids.contains(&a.action_id))
            .collect();
        let mut ordered = Vec::new();
        let mut visited = BTreeSet::new();
        while !pending.is_empty() {
            let index = pending
                .iter()
                .position(|a| {
                    a.dependencies
                        .as_slice()
                        .iter()
                        .all(|id| visited.contains(id))
                })
                .ok_or_else(|| error(ErrorCode::InvalidManifest, context))?;
            let action = pending.remove(index);
            visited.insert(action.action_id.clone());
            ordered.push(action);
        }
        let actions: BTreeMap<_, _> = bundle
            .actions
            .as_slice()
            .iter()
            .map(|a| (&a.action_id, a))
            .collect();
        let mut receipts = Vec::new();
        for forward in ordered.into_iter().rev() {
            let current = self.store.read(&bundle.instance_id, context)?;
            let before = forward
                .before
                .as_ref()
                .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?;
            let after = forward
                .after
                .as_ref()
                .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?;
            let original = candidate
                .resource(&forward.resource_id)
                .ok_or_else(|| error(ErrorCode::ResourceDrift, context))?;
            let resource = current
                .resources
                .as_slice()
                .iter()
                .find(|r| {
                    r.resource.resource_id == forward.resource_id
                        && (r.effective_bundle.as_ref() == Some(&bundle.bundle_id)
                            || fallback.is_some_and(|b| {
                                r.effective_bundle.as_ref() == Some(&b.candidate().bundle_id)
                            }))
                })
                .map(|r| &r.resource)
                .unwrap_or(original);
            let mut bounded = context.clone();
            bounded.budget.deadline_ms = bounded
                .budget
                .deadline_ms
                .min(self.clock.now_ms().saturating_add(15_000));
            check_budget(&bounded, self.clock, self.cancellation)?;
            let observed = self.native.observe(resource, &bounded)?;
            check_budget(&bounded, self.clock, self.cancellation)?;
            if !same_identity(&observed.resource, resource) {
                return Err(error(ErrorCode::ResourceDrift, context));
            }
            if &observed.resource.fingerprint == before {
                continue;
            } // No effect, including interruption before execution.
            if &observed.resource.fingerprint != after {
                return Err(error(ErrorCode::ResourceDrift, context));
            }
            let id = forward
                .compensation
                .as_ref()
                .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?;
            let mut action = (*actions
                .get(id)
                .ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?)
            .clone();
            let can_start =
                current.run_intent == RunIntent::Running && current.run_epoch == lease_epoch;
            let cleanup = matches!(
                action.kind,
                NativeActionKind::Stop | NativeActionKind::DisableAutostart
            );
            if !can_start && !cleanup && action.kind != NativeActionKind::RestoreArtifacts {
                continue;
            }
            // Non-cleanup inverses use only fully admitted fallback artifacts.
            let execution_bundle = if cleanup {
                Some(candidate)
            } else {
                Some(fallback.ok_or_else(|| error(ErrorCode::RecoveryRequired, context))?)
            };
            action.before = Some(after.clone());
            action.after = Some(before.clone());
            let outcome = self.execute_one(
                &bundle.instance_id,
                current.run_epoch,
                &action,
                resource,
                execution_bundle,
                !can_start,
                context,
            );
            receipts.push(ActionReceipt {
                action_id: action.action_id,
                outcome,
            });
        }
        Ok(receipts)
    }
}

/// Concrete separate effect-lock owner. Use a directory distinct from runtime state
/// and Core configuration locks, so no management lock can obstruct guardian effects.
pub struct DirectoryEffectGate {
    directory: hton_storage::SecureDir,
    clock: std::sync::Arc<dyn Clock>,
}
impl DirectoryEffectGate {
    pub fn new(directory: hton_storage::SecureDir, clock: std::sync::Arc<dyn Clock>) -> Self {
        Self { directory, clock }
    }
}
impl EffectGuard for hton_storage::LockedDir {}
impl EffectGate for DirectoryEffectGate {
    fn acquire(
        &self,
        context: &OperationContext,
    ) -> Result<Box<dyn EffectGuard + '_>, RuntimeError> {
        let remaining = context
            .budget
            .deadline_ms
            .saturating_sub(self.clock.now_ms())
            .min(15_000);
        if remaining == 0 {
            return Err(error(ErrorCode::DeadlineExceeded, context));
        }
        self.directory
            .lock(std::time::Instant::now() + std::time::Duration::from_millis(remaining))
            .map(|lock| Box::new(lock) as Box<dyn EffectGuard>)
            .map_err(|_| error(ErrorCode::DeadlineExceeded, context))
    }
}

fn same_identity(actual: &ResourceIdentity, expected: &ResourceIdentity) -> bool {
    actual.resource_id == expected.resource_id
        && actual.kind == expected.kind
        && actual.identity_digest == expected.identity_digest
        && actual.ownership == expected.ownership
}

impl ActionExecutor<'_> {
    /// Full-maintenance deletion of exact already-stopped Owned containers.
    /// Resource discovery and caller-supplied locators never grant authority.
    pub fn uninstall_owned(
        &self,
        instance: &InstanceId,
        expected_epoch: u64,
        selected: &[ResourceIdentity],
        authority: &dyn ShutdownAuthority,
        context: &OperationContext,
    ) -> Result<RuntimeResult, RuntimeError> {
        check_budget(context, self.clock, self.cancellation)?;
        let _gate = self.gate.acquire(context)?;
        let current = self.store.read(instance, context)?;
        if current.run_epoch != expected_epoch
            || current.run_intent != RunIntent::Stopped
            || current.trial.is_some()
            || current.maintenance.is_some()
            || current.lease.as_ref().is_some_and(|lease| {
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
        {
            return Err(error(ErrorCode::LeaseConflict, context));
        }
        let authorized = authority.resources(instance, context)?;
        let mut unique = BTreeSet::new();
        if selected.len() > 256
            || selected.iter().any(|resource| {
                resource.ownership != Ownership::Owned
                    || resource.kind != ResourceKind::Container
                    || !unique.insert(resource.resource_id.clone())
                    || !authorized.iter().any(|member| {
                        member.resource.resource_id == resource.resource_id
                            && member.resource.kind == resource.kind
                            && member.resource.ownership == Ownership::Owned
                    })
                    || current
                        .resources
                        .as_slice()
                        .iter()
                        .find(|receipt| receipt.resource.resource_id == resource.resource_id)
                        .is_some_and(|receipt| {
                            receipt.resource != *resource
                                || receipt.status != ResourceStatus::Stopped
                                || receipt.restart_policy != RestartPolicy::Disabled
                        })
            })
        {
            return Err(error(ErrorCode::ApprovalRequired, context));
        }
        let mut receipts = Vec::new();
        let mut survivors = Vec::new();
        let mut errors = Vec::new();
        for resource in selected {
            let mut bounded = context.clone();
            bounded.budget.deadline_ms = bounded
                .budget
                .deadline_ms
                .min(self.clock.now_ms().saturating_add(30_000));
            let outcome = (|| {
                check_budget(&bounded, self.clock, self.cancellation)?;
                self.native.remove_stopped_owned(resource, &bounded)?;
                let state = self.store.read(instance, context)?;
                if state.run_epoch != expected_epoch || state.run_intent != RunIntent::Stopped {
                    return Err(error(ErrorCode::StaleInput, context));
                }
                write_transition(
                    self.store,
                    &state,
                    StateTransition::RecordOwnedRemovals {
                        resources: Items::new(vec![resource.clone()])
                            .map_err(|_| error(ErrorCode::InvalidManifest, context))?,
                    },
                    context,
                )?;
                Ok(())
            })();
            let action_id = ActionId::new(format!(
                "remove-{}",
                &crate::manifest::hash(resource.resource_id.as_str().as_bytes()).as_str()[7..]
            ))
            .map_err(|_| error(ErrorCode::InvalidManifest, context))?;
            let outcome = match outcome {
                Ok(()) => ActionOutcome::Removed {
                    resource: resource.clone(),
                    timestamp_ms: self.clock.now_ms(),
                },
                Err(failure) => {
                    survivors.push(resource.resource_id.clone());
                    errors.push(failure.clone());
                    ActionOutcome::Uncertain { error: failure }
                }
            };
            receipts.push(ActionReceipt { action_id, outcome });
        }
        let state = self.store.read(instance, context)?;
        Ok(RuntimeResult {
            contract_version: ContractVersion::V1,
            operation_id: context.operation_id.clone(),
            status: if survivors.is_empty() {
                ResultStatus::Succeeded
            } else {
                ResultStatus::Partial
            },
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
