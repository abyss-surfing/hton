//! Single owner of module membership and whole-server lifecycle policy.
use crate::apply::EffectApplyPort;
use crate::catalog::ModuleCatalogue;
use crate::journal::{Intent, JournalStore, TerminalStatus, Transaction, ensure_mutating, failure};
use crate::registry::{RegistryPreconditions, RegistryStore};
use crate::selection::{SelectionCandidate, SelectionWriter};
use crate::setup::{SetupCoordinator, SetupResult};
use hton_contracts::{config::*, operations::*};
use hton_runtime_contract as runtime;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub struct LifecycleSnapshot {
    pub desired: Option<DesiredReadResult>,
    pub registry: RegistrySnapshot,
    pub runtime: runtime::RuntimeState,
    pub observations: RuntimeObservation,
}
/// Reads known protected inventories. Down/accepted startup never parse desired YAML.
pub trait LifecycleSnapshotPort {
    fn read(&self, include_desired: bool, ctx: &OperationContext) -> Result<LifecycleSnapshot>;
    fn registry_preconditions(
        &self,
        module: &ModuleId,
        ctx: &OperationContext,
    ) -> Result<RegistryPreconditions>;
}
#[derive(Default)]
pub struct PreparedOperation {
    pub selection: Option<SelectionCandidate>,
    pub setup: Option<SetupPlan>,
    pub effect: Option<EffectPlan>,
    pub update: Option<UpdatePlan>,
    pub association: Option<RegistryChange>,
    pub validation: Option<CheckReport>,
    pub target_graph: Option<DependencyGraph>,
    pub port_allocation_candidate: Option<PortAllocationCandidate>,
    pub purge_actions: Vec<Action>,
    pub verified_approval_ids: BTreeSet<ApprovalId>,
}
/// Supplies owner-produced candidates/evidence, not lifecycle decisions. The
/// optional baseline fixes only approved expiry/IDs while every actual input is
/// re-read/replanned through its owner. It must never accept baseline actions as truth.
pub trait LifecyclePreparationPort {
    fn prepare(
        &self,
        request: &LifecycleRequest,
        snapshot: &LifecycleSnapshot,
        baseline: Option<&LifecyclePlan>,
        ctx: &OperationContext,
    ) -> Result<PreparedOperation>;
}
pub trait RuntimeLifecyclePort {
    fn up(
        &self,
        expected: &ExpectedInputs,
        pending_bundle: Option<&BundleId>,
        ctx: &OperationContext,
    ) -> Result<OperationResult>;
    fn down(&self, expected: &ExpectedInputs, ctx: &OperationContext) -> Result<OperationResult>;
    fn start_accepted(
        &self,
        bundle: &BundleId,
        expected: &ExpectedInputs,
        ctx: &OperationContext,
    ) -> Result<OperationResult>;
    fn rollback_accepted(
        &self,
        bundle: &BundleId,
        expected: &ExpectedInputs,
        ctx: &OperationContext,
    ) -> Result<OperationResult>;
    fn stop_module(
        &self,
        module: &RegistryEntry,
        expected: &ExpectedInputs,
        ctx: &OperationContext,
    ) -> Result<OperationResult>;
}
pub struct UpdateResult {
    pub receipts: Vec<StepReceipt>,
    pub installed_versions: BTreeMap<ArtifactId, ImageDigest>,
}
pub trait ModuleUpdatePort {
    fn execute(&self, plan: &UpdatePlan, ctx: &OperationContext) -> Result<UpdateResult>;
}
pub trait ModulePurgePort {
    fn execute(
        &self,
        entry: &RegistryEntry,
        actions: &[Action],
        ctx: &OperationContext,
    ) -> Result<Vec<StepReceipt>>;
}
/// Commits the exact lifecycle-owned allocation candidate after the parent
/// lifecycle intent is durable and before selection or activation changes.
/// Implementations perform candidate-scoped approval, protected settings CAS,
/// and exact-intent recovery; they must not generate replacement entropy.
pub trait PortAllocationCommitPort {
    fn commit(&self, candidate: &PortAllocationCandidate, ctx: &OperationContext) -> Result<()>;
}
pub struct LifecycleService {
    pub catalogue: Arc<ModuleCatalogue>,
    pub registry: Arc<RegistryStore>,
    pub journal: Arc<JournalStore>,
    pub selection: Arc<SelectionWriter>,
    pub setup: Arc<SetupCoordinator>,
    pub snapshots: Arc<dyn LifecycleSnapshotPort>,
    pub preparation: Arc<dyn LifecyclePreparationPort>,
    pub runtime: Arc<dyn RuntimeLifecyclePort>,
    pub apply: Arc<dyn EffectApplyPort>,
    pub updates: Arc<dyn ModuleUpdatePort>,
    pub purge: Arc<dyn ModulePurgePort>,
    pub allocations: Arc<dyn PortAllocationCommitPort>,
}
impl LifecycleService {
    pub fn plan(&self, request: LifecycleRequest, ctx: &OperationContext) -> Result<LifecyclePlan> {
        let mut readonly = ctx.clone();
        readonly.execution_mode = ExecutionMode::ReadOnly;
        let snapshot = self.snapshots.read(needs_desired(&request), &readonly)?;
        let prepared = self
            .preparation
            .prepare(&request, &snapshot, None, &readonly)?;
        plan(request, &snapshot, &prepared, &self.catalogue, &readonly)
    }
    pub fn execute(
        &self,
        submitted: &LifecyclePlan,
        ctx: &OperationContext,
    ) -> Result<OperationResult> {
        ensure_mutating(ctx, self.journal.control())?;
        let intent = Intent::Lifecycle {
            plan: submitted.clone(),
        };
        if let Some(old) = self.journal.lookup_intent(&intent, ctx)? {
            if let Some(terminal) = self.journal.terminal(&old.transaction_id, ctx)? {
                if let Some(result) = terminal.operation_result {
                    return Ok(result);
                }
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "lifecycle terminal is missing its operation result",
                ));
            }
        }
        let mut readonly = ctx.clone();
        readonly.execution_mode = ExecutionMode::ReadOnly;
        let snapshot = self
            .snapshots
            .read(needs_desired(&submitted.request), &readonly)?;
        let prepared =
            self.preparation
                .prepare(&submitted.request, &snapshot, Some(submitted), &readonly)?;
        let exact = plan(
            submitted.request.clone(),
            &snapshot,
            &prepared,
            &self.catalogue,
            &readonly,
        )?;
        if exact != *submitted {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "lifecycle plan no longer matches its exact trusted inputs",
            ));
        }
        let pending = self.journal.unfinished(ctx)?;
        for old in &pending {
            if !resumable_add(old, &submitted.request, ctx)
                && !matches!(submitted.request, LifecycleRequest::Down)
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "unfinished operation must be reconciled before new lifecycle effects",
                ));
            }
        }
        for old in &pending {
            if matches!(old.intent, Intent::Selection { .. })
                && resumable_add(old, &submitted.request, ctx)
            {
                let mut original_ctx = ctx.clone();
                original_ctx.operation_id = old.operation_id.clone();
                original_ctx.actor = old.actor.clone();
                self.selection.reconcile(old, &original_ctx)?;
            }
        }
        if !submitted.issues.is_empty() {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "lifecycle plan has unresolved blockers",
            ));
        }
        let tx = self.journal.prepare_intent(intent, vec![], ctx)?;
        let mut receipts = Vec::new();
        let outcome = self.execute_prepared(submitted, &snapshot, &prepared, &mut receipts, ctx);
        finish_outcome(
            &self.journal,
            &tx,
            &submitted.request,
            &snapshot,
            receipts,
            outcome,
            ctx,
        )
    }

    fn execute_prepared(
        &self,
        plan: &LifecyclePlan,
        snapshot: &LifecycleSnapshot,
        prepared: &PreparedOperation,
        receipts: &mut Vec<StepReceipt>,
        ctx: &OperationContext,
    ) -> Result<OperationResult> {
        let mut result = result_for(snapshot, ctx);
        if matches!(
            &plan.request,
            LifecycleRequest::Add { .. } | LifecycleRequest::Enable { .. }
        ) {
            if let Some(candidate) = &plan.port_allocation_candidate {
                self.allocations.commit(candidate, ctx)?;
            }
        }
        match &plan.request {
            LifecycleRequest::Down => return self.runtime.down(&plan.expected_inputs, ctx),
            LifecycleRequest::StartAccepted { bundle_id, .. } => {
                return self
                    .runtime
                    .start_accepted(bundle_id, &plan.expected_inputs, ctx);
            }
            LifecycleRequest::RollbackAccepted { bundle_id } => {
                return self
                    .runtime
                    .rollback_accepted(bundle_id, &plan.expected_inputs, ctx);
            }
            LifecycleRequest::RestartServices { .. } => {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "service restart must use the journaled runtime request bridge",
                ));
            }
            LifecycleRequest::Up => {
                if let Some(effect) = &prepared.effect {
                    result = self.apply.stage_and_apply(effect, ctx)?;
                    if result.status != OperationStatus::Succeeded {
                        return Ok(result);
                    }
                }
                // Re-read after preparation/apply; up's runtime owner persists the running epoch.
                let now = self.snapshots.read(false, ctx)?;
                return self.runtime.up(
                    &expected_inputs(&now),
                    result.pending_bundle.as_ref(),
                    ctx,
                );
            }
            LifecycleRequest::Add { module_id, .. } => {
                if let Some(candidate) = &prepared.selection {
                    let saved = self.selection.commit(
                        candidate,
                        &candidate.change().expected_root_revision,
                        ctx,
                    )?;
                    result.desired_saved |= saved.changed;
                }
                if let Some(change @ RegistryChange::Adopt { .. }) = &prepared.association {
                    let pre = self.snapshots.registry_preconditions(module_id, ctx)?;
                    let current = self.registry.read(&snapshot.registry.instance_id, ctx)?;
                    let state = self.registry.transition(
                        &snapshot.registry.instance_id,
                        current.revision,
                        change.clone(),
                        &pre,
                        ctx,
                    )?;
                    let entry = state.entries.get(module_id).ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "adopted registry member missing",
                        )
                    })?;
                    result.member_results.push(MemberResult {
                        module_id: module_id.clone(),
                        status: OperationStatus::Succeeded,
                        resources: entry
                            .resources
                            .iter()
                            .map(|r| r.resource_id.clone())
                            .collect(),
                        error: None,
                    });
                    return Ok(result);
                }
                let setup = prepared.setup.as_ref().ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::ModuleIncomplete,
                        "add has no qualified setup plan",
                    )
                })?;
                let done = self.setup.resume(setup, ctx)?;
                result.status = if done.complete {
                    OperationStatus::Succeeded
                } else {
                    OperationStatus::Partial
                };
                result.member_results.push(setup_member(&done));
                if done.complete {
                    self.complete_prior_adds(module_id, &done.receipts, &result, ctx)?;
                }
                return Ok(result);
            }
            LifecycleRequest::Enable { save_only, .. }
            | LifecycleRequest::Disable { save_only, .. } => {
                if let Some(candidate) = &prepared.selection {
                    let saved = self.selection.commit(
                        candidate,
                        &candidate.change().expected_root_revision,
                        ctx,
                    )?;
                    result.desired_saved |= saved.changed;
                }
                if !save_only {
                    if let Some(effect) = &prepared.effect {
                        let saved = result.desired_saved;
                        result = self.apply.stage_and_apply(effect, ctx)?;
                        result.desired_saved |= saved;
                    }
                }
            }
            LifecycleRequest::Remove { module_id, .. } => {
                if let Some(candidate) = &prepared.selection {
                    result.desired_saved |= self
                        .selection
                        .commit(candidate, &candidate.change().expected_root_revision, ctx)?
                        .changed;
                }
                if let Some(effect) = &prepared.effect {
                    result = self.apply.stage_and_apply(effect, ctx)?;
                    result.desired_saved = true;
                    if result.status != OperationStatus::Succeeded {
                        return self.mark_cleanup(
                            module_id,
                            &snapshot.registry.instance_id,
                            result,
                            ctx,
                        );
                    }
                }
                let current = self.registry.read(&snapshot.registry.instance_id, ctx)?;
                let entry = current.entries.get(module_id).ok_or_else(|| {
                    failure(ctx, ErrorCode::ModuleUnknown, "module no longer registered")
                })?;
                let mut stopped = self.runtime.stop_module(
                    entry,
                    &expected_inputs(&self.snapshots.read(false, ctx)?),
                    ctx,
                )?;
                stopped.desired_saved |= result.desired_saved;
                if stopped.status != OperationStatus::Succeeded || !stopped.survivors.is_empty() {
                    return self.mark_cleanup(
                        module_id,
                        &snapshot.registry.instance_id,
                        stopped,
                        ctx,
                    );
                }
                let pre = self.snapshots.registry_preconditions(module_id, ctx)?;
                self.registry.transition(
                    &snapshot.registry.instance_id,
                    current.revision,
                    RegistryChange::Remove {
                        module_id: module_id.clone(),
                        retained_artifacts: retained_owned_artifacts(entry),
                    },
                    &pre,
                    ctx,
                )?;
            }
            LifecycleRequest::Purge { module_id, .. } => {
                let entry = snapshot.registry.entries.get(module_id).ok_or_else(|| {
                    failure(ctx, ErrorCode::ModuleUnknown, "module not registered")
                })?;
                let pre = self.snapshots.registry_preconditions(module_id, ctx)?;
                validate_purge(entry, &pre, ctx, self.journal.control().now_ms())?;
                *receipts = self.purge.execute(entry, &plan.actions, ctx)?;
                validate_effect_receipts(&plan.actions, receipts, ctx)?;
                let current = self.registry.read(&snapshot.registry.instance_id, ctx)?;
                let post = self.snapshots.registry_preconditions(module_id, ctx)?;
                self.registry.transition(
                    &snapshot.registry.instance_id,
                    current.revision,
                    RegistryChange::Purge {
                        module_id: module_id.clone(),
                    },
                    &post,
                    ctx,
                )?;
            }
            LifecycleRequest::Update {
                module_id,
                target_digest,
                ..
            } => {
                let update = prepared.update.as_ref().ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::ModuleUnsupported,
                        "module has no qualified update plan",
                    )
                })?;
                let tx = self.journal.prepare_intent(
                    Intent::Update {
                        plan: update.clone(),
                    },
                    vec![],
                    ctx,
                )?;
                let applied = self.updates.execute(update, ctx)?;
                validate_effect_receipts(&update.actions, &applied.receipts, ctx)?;
                let old = snapshot.registry.entries.get(module_id).ok_or_else(|| {
                    failure(ctx, ErrorCode::ModuleUnknown, "module is not registered")
                })?;
                if applied.installed_versions.is_empty()
                    || applied.installed_versions.iter().any(|(id, version)| {
                        old.installed_artifact_versions.get(id) != Some(version)
                            && version != target_digest
                    })
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "update returned undeclared installed artifact versions",
                    ));
                }
                let receipt = applied
                    .receipts
                    .last()
                    .ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "update has no verified receipt",
                        )
                    })?
                    .clone();
                let current = self.registry.read(&snapshot.registry.instance_id, ctx)?;
                self.registry.transition(
                    &snapshot.registry.instance_id,
                    current.revision,
                    RegistryChange::Update {
                        module_id: module_id.clone(),
                        installed_versions: applied.installed_versions,
                        receipt,
                    },
                    &RegistryPreconditions::default(),
                    ctx,
                )?;
                self.journal.finish(
                    &tx,
                    TerminalStatus::Committed,
                    applied.receipts.clone(),
                    ctx,
                )?;
                *receipts = applied.receipts;
            }
        }
        Ok(result)
    }
    fn mark_cleanup(
        &self,
        module: &ModuleId,
        instance: &InstanceId,
        mut result: OperationResult,
        ctx: &OperationContext,
    ) -> Result<OperationResult> {
        let current = self.registry.read(instance, ctx)?;
        let entry = current
            .entries
            .get(module)
            .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "cleanup module missing"))?;
        self.registry.transition(
            instance,
            current.revision,
            RegistryChange::CleanupRequired {
                module_id: module.clone(),
                resources: entry
                    .resources
                    .iter()
                    .map(|r| r.resource_id.clone())
                    .collect(),
            },
            &RegistryPreconditions::default(),
            ctx,
        )?;
        result.status = if result.status == OperationStatus::RecoveryRequired {
            result.status
        } else {
            OperationStatus::Partial
        };
        Ok(result)
    }
    fn complete_prior_adds(
        &self,
        module: &ModuleId,
        _fresh: &[StepReceipt],
        result: &OperationResult,
        ctx: &OperationContext,
    ) -> Result<()> {
        for old in self.journal.unfinished(ctx)? {
            if old.operation_id == ctx.operation_id
                || !resumable_add(
                    &old,
                    &LifecycleRequest::Add {
                        module_id: module.clone(),
                        approval_refs: BTreeSet::new(),
                    },
                    ctx,
                )
            {
                continue;
            }
            let mut original_ctx = ctx.clone();
            original_ctx.operation_id = old.operation_id.clone();
            original_ctx.actor = old.actor.clone();
            match &old.intent {
                Intent::Lifecycle { plan } if plan.actions.is_empty() => {
                    let mut previous = result.clone();
                    previous.operation_id = old.operation_id.clone();
                    previous.transaction_id = Some(old.transaction_id.clone());
                    if self.journal.terminal(&old.transaction_id, ctx)?.is_some() {
                        self.journal.resolve_operation(
                            &old.transaction_id,
                            TerminalStatus::Committed,
                            vec![],
                            Some(&previous),
                            &original_ctx,
                        )?;
                    } else {
                        self.journal.finish_operation(
                            &old.transaction_id,
                            TerminalStatus::Committed,
                            vec![],
                            &previous,
                            &original_ctx,
                        )?;
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }
}

pub fn plan(
    request: LifecycleRequest,
    snapshot: &LifecycleSnapshot,
    prepared: &PreparedOperation,
    catalogue: &ModuleCatalogue,
    ctx: &OperationContext,
) -> Result<LifecyclePlan> {
    let request = resolve_restart_selection(request, snapshot, catalogue, ctx)?;
    if snapshot.runtime.maintenance.is_some() && !matches!(request, LifecycleRequest::Down) {
        return Err(failure(
            ctx,
            ErrorCode::OperationConflict,
            "runtime maintenance reservation blocks lifecycle changes",
        ));
    }
    let expected = expected_inputs(snapshot);
    let mut planned = LifecyclePlan {
        request: request.clone(),
        expected_inputs: expected,
        actions: vec![],
        registry_changes: vec![],
        selection_change: None,
        setup_plan: None,
        effect_plan: None,
        port_allocation_candidate: prepared.port_allocation_candidate.clone(),
        issues: vec![],
        expected_disruption: Disruption::None,
    };
    if let Some(module) = request_module(&request) {
        let descriptor = catalogue
            .describe(module)
            .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "unknown module"))?;
        if module.as_str() == "core"
            || !descriptor.implemented
            || descriptor.descriptor.phase != ReleasePhase::R1
        {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "module lifecycle is outside the selected release",
            ));
        }
        for approval in request_approvals(&request) {
            if !ctx.actor.local_approval_refs.contains(approval)
                || !prepared.verified_approval_ids.contains(approval)
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "lifecycle approval is not trusted local authority",
                ));
            }
        }
    }
    match &request {
        LifecycleRequest::Down => {
            planned.expected_disruption = Disruption::CompositionStopped;
        }
        LifecycleRequest::StartAccepted { bundle_id, .. } => {
            if snapshot.runtime.accepted.as_ref().map(|id| id.as_str()) != Some(bundle_id.as_str())
            {
                return Err(failure(
                    ctx,
                    ErrorCode::PlanStale,
                    "requested bundle is not the durable accepted bundle",
                ));
            }
        }
        LifecycleRequest::RollbackAccepted { bundle_id } => {
            let lease = snapshot.runtime.lease.as_ref();
            let exact_fallback = lease.is_some_and(|lease| {
                matches!(
                    &lease.fallback.target,
                    runtime::RollbackTarget::Accepted { bundle_id: id, .. } if id == bundle_id
                )
            });
            if snapshot.runtime.run_intent != runtime::RunIntent::Running
                || snapshot.runtime.accepted.as_ref() == Some(bundle_id)
                || snapshot.runtime.previous_compatible.as_ref() != Some(bundle_id)
                || !exact_fallback
                || lease.is_none_or(|lease| {
                    lease.run_epoch != snapshot.runtime.run_epoch
                        || !matches!(
                            lease.state,
                            runtime::LeaseState::Committed | runtime::LeaseState::RecoveryRequired
                        )
                })
            {
                return Err(failure(
                    ctx,
                    ErrorCode::PlanStale,
                    "rollback target is not the exact durable previous-compatible bundle",
                ));
            }
            planned.expected_disruption = Disruption::ConnectionsInterrupted;
        }
        LifecycleRequest::RestartServices {
            service_ids,
            wait_seconds,
            bundle_id,
        } => {
            if snapshot.runtime.run_intent != runtime::RunIntent::Running
                || snapshot.runtime.accepted.is_none()
                || snapshot
                    .runtime
                    .accepted
                    .as_ref()
                    .is_some_and(|id| id.as_str() != bundle_id.as_str())
                || snapshot.runtime.trial.is_some()
                || snapshot.runtime.lease.as_ref().is_some_and(|lease| {
                    matches!(
                        lease.state,
                        runtime::LeaseState::Armed
                            | runtime::LeaseState::Activating
                            | runtime::LeaseState::Verifying
                            | runtime::LeaseState::Compensating
                            | runtime::LeaseState::RollingBack
                            | runtime::LeaseState::RecoveryRequired
                    )
                })
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "service restart requires a running accepted composition without a trial lease",
                ));
            }
            if service_ids.iter().collect::<BTreeSet<_>>().len() != service_ids.len() {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "restart service selection is duplicated",
                ));
            }
            if service_ids.is_empty() {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "no registered owned service is eligible for restart",
                ));
            }
            if !(1..=300).contains(wait_seconds) {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "restart wait bound is invalid",
                ));
            }
            for service_id in service_ids {
                if !catalogue.services().contains_key(service_id) {
                    return Err(failure(
                        ctx,
                        ErrorCode::ModuleUnknown,
                        "unknown catalogue service",
                    ));
                }
                let owned = snapshot.registry.entries.values().any(|entry| {
                    entry.ownership == Ownership::Owned
                        && entry.membership == MembershipState::Installed
                        && entry.setup_state == SetupState::Complete
                        && entry
                            .resources
                            .iter()
                            .any(|resource| resource.service_id.as_str() == service_id.as_str())
                });
                let accepted_receipt = snapshot.runtime.accepted.as_ref().is_some_and(|accepted| {
                    snapshot
                        .registry
                        .entries
                        .values()
                        .filter(|entry| {
                            entry.ownership == Ownership::Owned
                                && entry.membership == MembershipState::Installed
                                && entry.setup_state == SetupState::Complete
                        })
                        .flat_map(|entry| entry.resources.iter())
                        .filter(|resource| resource.service_id == *service_id)
                        .any(|resource| {
                            snapshot.runtime.resources.as_slice().iter().any(|receipt| {
                                receipt.resource.ownership == runtime::Ownership::Owned
                                    && receipt.resource.resource_id.as_str()
                                        == resource.resource_id.as_str()
                                    && receipt.effective_bundle.as_ref() == Some(accepted)
                                    && receipt.run_epoch == snapshot.runtime.run_epoch
                            })
                        })
                });
                if !owned || !accepted_receipt {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "restart target is not an exact registered owned resource in the accepted bundle",
                    ));
                }
            }
            planned.expected_disruption = Disruption::ConnectionsInterrupted;
        }
        LifecycleRequest::Up => {
            require_validation(prepared, ctx)?;
            planned.effect_plan = prepared.effect.clone();
        }
        LifecycleRequest::Add { module_id, .. } => {
            require_desired(snapshot, ctx)?;
            let existing = snapshot.registry.entries.get(module_id);
            if existing.is_some_and(|e| e.membership == MembershipState::CleanupRequired) {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "module cleanup is incomplete",
                ));
            }
            if existing.is_none() {
                let selection = prepared.selection.as_ref().ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::ModuleIncomplete,
                        "new module must be saved disabled before setup",
                    )
                })?;
                ensure_selection(selection, module_id, false, snapshot, ctx)?;
                planned.selection_change = Some(selection.change().clone());
            } else if prepared.selection.is_some() {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "re-add cannot silently change existing participation",
                ));
            }
            if let Some(change @ RegistryChange::Adopt { entry }) = &prepared.association {
                if existing.is_some()
                    || entry.module_id != *module_id
                    || entry.ownership != Ownership::Associated
                    || entry.setup_state != SetupState::Complete
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "association candidate is not a new qualified native member",
                    ));
                }
                planned.registry_changes.push(change.clone());
            } else {
                let setup = prepared.setup.as_ref().ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::ModuleIncomplete,
                        "module setup plan missing",
                    )
                })?;
                if setup.module_id != *module_id
                    || setup.expected_registry_revision != snapshot.registry.revision
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::PlanStale,
                        "setup registry input changed",
                    ));
                }
                crate::setup::validate_plan(setup, ctx)?;
                planned.setup_plan = Some(setup.clone());
            }
        }
        LifecycleRequest::Enable {
            module_id,
            save_only,
            ..
        }
        | LifecycleRequest::Disable {
            module_id,
            save_only,
            ..
        } => {
            require_desired(snapshot, ctx)?;
            let enabled = matches!(request, LifecycleRequest::Enable { .. });
            if enabled
                && prepared
                    .target_graph
                    .as_ref()
                    .is_some_and(|graph| !graph.allocation_bindings.is_empty())
                && prepared.port_allocation_candidate.is_none()
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "enabled VPN services require a prepared persistent port allocation candidate",
                ));
            }
            let entry = installed(snapshot, module_id, ctx)?;
            if enabled && entry.setup_state != SetupState::Complete {
                return Err(failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "incomplete module cannot be enabled",
                ));
            }
            let selection = prepared.selection.as_ref().ok_or_else(|| {
                failure(ctx, ErrorCode::ConfigSyntax, "selection candidate missing")
            })?;
            ensure_selection(selection, module_id, enabled, snapshot, ctx)?;
            require_validation(prepared, ctx)?;
            planned.selection_change = Some(selection.change().clone());
            if !save_only {
                planned.effect_plan = Some(prepared.effect.clone().ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::ModuleIncomplete,
                        "participation change has no exact effect plan",
                    )
                })?);
            }
        }
        LifecycleRequest::Remove { module_id, .. } => {
            require_desired(snapshot, ctx)?;
            let entry = installed(snapshot, module_id, ctx)?;
            if snapshot.registry.entries.iter().any(|(id, e)| {
                id != module_id
                    && e.membership == MembershipState::Installed
                    && enabled(snapshot, id)
                    && catalogue
                        .describe(id)
                        .is_some_and(|d| d.descriptor.dependencies.contains(module_id))
            }) {
                return Err(failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "enabled dependents require this module",
                ));
            }
            let selection = prepared.selection.as_ref().ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "remove requires explicit disabled selection",
                )
            })?;
            ensure_selection(selection, module_id, false, snapshot, ctx)?;
            require_validation(prepared, ctx)?;
            reject_shared_feature_ownership(entry, prepared.target_graph.as_ref(), ctx)?;
            planned.selection_change = Some(selection.change().clone());
            planned.effect_plan = prepared.effect.clone();
            planned.registry_changes.push(RegistryChange::Remove {
                module_id: module_id.clone(),
                retained_artifacts: retained_owned_artifacts(entry),
            });
        }
        LifecycleRequest::Purge {
            module_id,
            approval_refs,
        } => {
            let entry =
                snapshot.registry.entries.get(module_id).ok_or_else(|| {
                    failure(ctx, ErrorCode::ModuleUnknown, "module not registered")
                })?;
            if entry.ownership != Ownership::Owned
                || entry.membership != MembershipState::RemovedRetained
                || approval_refs.is_empty()
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "purge requires explicit approval for an already removed owned module",
                ));
            }
            for action in &prepared.purge_actions {
                if action.kind != ActionKind::RemoveOwnedResource || !entry.resources.iter().any(|r| r.resource_id == action.resource_id)
                    || !action.preconditions.iter().any(|condition| matches!(condition,Condition::Approval{approval_id} if approval_refs.contains(approval_id))) {
                    return Err(failure(ctx, ErrorCode::OperationDenied, "purge action escapes removed module resources or approval"));
                }
            }
            let targets: BTreeSet<_> = prepared
                .purge_actions
                .iter()
                .map(|action| action.resource_id.clone())
                .collect();
            if targets
                != entry
                    .resources
                    .iter()
                    .map(|resource| resource.resource_id.clone())
                    .collect()
                || (targets.is_empty() && !entry.retained_artifacts.is_empty())
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "purge lacks an exact action for every retained owned resource",
                ));
            }
            planned.actions = prepared.purge_actions.clone();
            planned.registry_changes.push(RegistryChange::Purge {
                module_id: module_id.clone(),
            });
        }
        LifecycleRequest::Update {
            module_id,
            target_digest,
            ..
        } => {
            let entry = installed(snapshot, module_id, ctx)?;
            let update = prepared.update.as_ref().ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "module update capability unavailable",
                )
            })?;
            if entry.ownership != Ownership::Owned
                || update.module_id != *module_id
                || update.target_digest != *target_digest
                || update.compatibility != Compatibility::Compatible
                || !entry
                    .installed_artifact_versions
                    .values()
                    .any(|v| v == &update.installed_digest)
                || update.actions.is_empty()
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "update compatibility or installed image evidence is invalid",
                ));
            }
            planned.actions = update.actions.clone();
        }
    }
    if let Some(effect) = &planned.effect_plan {
        planned.expected_disruption = effect.expected_disruption;
    }
    Ok(planned)
}
fn needs_desired(request: &LifecycleRequest) -> bool {
    !matches!(
        request,
        LifecycleRequest::Down
            | LifecycleRequest::StartAccepted { .. }
            | LifecycleRequest::RollbackAccepted { .. }
            | LifecycleRequest::Purge { .. }
    )
}

fn resolve_restart_selection(
    request: LifecycleRequest,
    snapshot: &LifecycleSnapshot,
    catalogue: &ModuleCatalogue,
    ctx: &OperationContext,
) -> Result<LifecycleRequest> {
    let LifecycleRequest::RestartServices {
        service_ids,
        wait_seconds,
        bundle_id,
    } = request
    else {
        return Ok(request);
    };
    if !(1..=300).contains(&wait_seconds) {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "restart wait bound is invalid",
        ));
    }
    if snapshot.runtime.accepted.as_ref() != Some(&bundle_id) {
        return Err(failure(
            ctx,
            ErrorCode::PlanStale,
            "restart bundle is not currently accepted",
        ));
    }
    let desired = snapshot.desired.as_ref().ok_or_else(|| {
        failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "restart requires the saved enabled selection",
        )
    })?;
    if !desired.errors.is_empty() {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "saved enabled selection cannot be read",
        ));
    }
    let desired = desired.snapshot.as_ref().ok_or_else(|| {
        failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "saved enabled selection is unavailable",
        )
    })?;
    let normalized = crate::config::normalize(desired, catalogue);
    let normalized = normalized.normalized_snapshot.as_ref().ok_or_else(|| {
        failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "saved enabled selection is invalid",
        )
    })?;
    let graph = catalogue.resolve(
        normalized,
        ReleasePhase::R1,
        &crate::catalog::ResolutionEvidence::default(),
    );
    let enabled_services: BTreeSet<_> = graph
        .service_consumers
        .iter()
        .filter(|(_, consumers)| consumers.iter().any(|module| module.as_str() != "core"))
        .map(|(service, _)| service.clone())
        .collect();
    let eligible = |service: &ServiceId| {
        enabled_services.contains(service)
            && snapshot.registry.entries.values().any(|entry| {
                entry.module_id.as_str() != "core"
                    && entry.ownership == Ownership::Owned
                    && entry.membership == MembershipState::Installed
                    && entry.setup_state == SetupState::Complete
                    && entry.resources.iter().any(|resource| {
                        resource.service_id == *service
                            && snapshot.runtime.resources.as_slice().iter().any(|receipt| {
                                receipt.resource.ownership == runtime::Ownership::Owned
                                    && receipt.resource.resource_id.as_str()
                                        == resource.resource_id.as_str()
                                    && receipt.effective_bundle.as_ref() == Some(&bundle_id)
                                    && receipt.run_epoch == snapshot.runtime.run_epoch
                            })
                    })
            })
    };
    let selected: Vec<_> = if service_ids.is_empty() {
        enabled_services
            .iter()
            .filter(|service| eligible(service))
            .cloned()
            .collect()
    } else {
        if service_ids.iter().collect::<BTreeSet<_>>().len() != service_ids.len()
            || service_ids.iter().any(|service| !eligible(service))
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "restart selection is not enabled, accepted and owned",
            ));
        }
        service_ids
    };
    if selected.is_empty() {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "no enabled accepted owned optional/data service is eligible for restart",
        ));
    }
    Ok(LifecycleRequest::RestartServices {
        service_ids: selected,
        wait_seconds,
        bundle_id,
    })
}
fn expected_inputs(snapshot: &LifecycleSnapshot) -> ExpectedInputs {
    ExpectedInputs {
        read_set: snapshot
            .desired
            .as_ref()
            .map(|d| d.read_set.clone())
            .unwrap_or(ReadSet {
                objects: BTreeMap::new(),
                absent_objects: BTreeSet::new(),
                secret_versions: BTreeMap::new(),
                coherence: ReadCoherence::Unverified,
            }),
        registry_revision: snapshot.registry.revision,
        runtime_revision: StateRevision(snapshot.runtime.revision),
        run_epoch: RunEpoch(snapshot.runtime.run_epoch),
        resources: snapshot
            .observations
            .resources
            .iter()
            .map(|(id, o)| (id.clone(), o.identity.fingerprint.clone()))
            .collect(),
    }
}
fn require_desired<'a>(
    snapshot: &'a LifecycleSnapshot,
    ctx: &OperationContext,
) -> Result<&'a DesiredSnapshot> {
    let read = snapshot.desired.as_ref().ok_or_else(|| {
        failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "desired configuration is unavailable",
        )
    })?;
    if !read.errors.is_empty() || read.read_set.coherence != ReadCoherence::Coherent {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "desired configuration is invalid; repair it before this operation",
        ));
    }
    read.snapshot.as_ref().ok_or_else(|| {
        failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "desired configuration has no valid snapshot",
        )
    })
}
fn installed<'a>(
    snapshot: &'a LifecycleSnapshot,
    module: &ModuleId,
    ctx: &OperationContext,
) -> Result<&'a RegistryEntry> {
    snapshot
        .registry
        .entries
        .get(module)
        .filter(|e| e.membership == MembershipState::Installed)
        .ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::ModuleIncomplete,
                "module is not installed or cleanup is incomplete",
            )
        })
}
fn enabled(snapshot: &LifecycleSnapshot, module: &ModuleId) -> bool {
    snapshot
        .desired
        .as_ref()
        .and_then(|d| d.snapshot.as_ref())
        .and_then(|s| s.features.get(module))
        .is_some_and(|s| s.enabled)
}
fn ensure_selection(
    candidate: &SelectionCandidate,
    module: &ModuleId,
    enable: bool,
    snapshot: &LifecycleSnapshot,
    ctx: &OperationContext,
) -> Result<()> {
    let root = snapshot
        .desired
        .as_ref()
        .and_then(|d| {
            d.read_set
                .objects
                .get(&ObjectId::new("root").expect("fixed ID"))
        })
        .map(|r| &r.revision);
    if candidate.change().module_ids != BTreeSet::from([module.clone()])
        || candidate.change().enabled != enable
        || root != Some(&candidate.change().expected_root_revision)
    {
        return Err(failure(
            ctx,
            ErrorCode::PlanStale,
            "selection candidate has a different module, target or root baseline",
        ));
    }
    Ok(())
}
fn require_validation(prepared: &PreparedOperation, ctx: &OperationContext) -> Result<()> {
    if !prepared
        .validation
        .as_ref()
        .is_some_and(|v| !v.blocking && v.normalized_snapshot.is_some())
        || !prepared
            .target_graph
            .as_ref()
            .is_some_and(|g| g.conflicts.is_empty())
    {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "prospective composition lacks successful module/global validation",
        ));
    }
    if let Some(candidate) = &prepared.selection {
        let target = prepared
            .validation
            .as_ref()
            .and_then(|v| v.normalized_snapshot.as_ref())
            .expect("validated above");
        if target.desired.read_set.objects != candidate.read_result().read_set.objects {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "validation is not bound to the exact prospective selection bytes",
            ));
        }
    }
    Ok(())
}
fn reject_shared_feature_ownership(
    entry: &RegistryEntry,
    graph: Option<&DependencyGraph>,
    ctx: &OperationContext,
) -> Result<()> {
    if entry.resources.iter().any(|resource| {
        graph
            .and_then(|g| g.service_consumers.get(&resource.service_id))
            .is_some_and(|consumers| consumers.iter().any(|m| m != &entry.module_id))
    }) {
        return Err(failure(
            ctx,
            ErrorCode::ResourceDrift,
            "feature registry incorrectly owns a shared live service; reconcile the persistent service owner",
        ));
    }
    Ok(())
}
fn validate_purge(
    entry: &RegistryEntry,
    pre: &RegistryPreconditions,
    ctx: &OperationContext,
    now: u64,
) -> Result<()> {
    if entry.ownership != Ownership::Owned
        || entry.membership != MembershipState::RemovedRetained
        || !pre.recovery_released.contains(&entry.module_id)
        || entry.resources.iter().any(|r| {
            !pre.no_shared_consumers.contains(&r.resource_id)
                || !pre.observed.get(&r.resource_id).is_some_and(|o| {
                    o.identity == *r && o.state == ObservedState::Stopped && o.valid_until.0 > now
                })
        })
    {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "purge lacks fresh stopped/exclusive/recovery-release evidence",
        ));
    }
    Ok(())
}
fn validate_effect_receipts(
    actions: &[Action],
    receipts: &[StepReceipt],
    ctx: &OperationContext,
) -> Result<()> {
    let expected: BTreeSet<_> = actions.iter().map(|a| a.action_id.clone()).collect();
    let actual: BTreeSet<_> = receipts.iter().map(|r| r.step_id.clone()).collect();
    if actual != expected
        || actual.len() != receipts.len()
        || receipts
            .iter()
            .any(|r| r.status != StepStatus::Verified || r.error.is_some())
    {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "module effect receipts do not prove the full planned action set",
        ));
    }
    Ok(())
}
fn resumable_add(
    transaction: &Transaction,
    request: &LifecycleRequest,
    ctx: &OperationContext,
) -> bool {
    let LifecycleRequest::Add { module_id, .. } = request else {
        return false;
    };
    if transaction.actor.actor_id != ctx.actor.actor_id {
        return false;
    }
    match &transaction.intent {
        Intent::Setup { plan } => &plan.module_id == module_id,
        Intent::Selection {
            module_ids,
            enabled,
            ..
        } => !enabled && module_ids == &BTreeSet::from([module_id.clone()]),
        Intent::Lifecycle { plan } => {
            matches!(&plan.request, LifecycleRequest::Add{module_id:id,..} if id == module_id)
                && plan.actions.is_empty()
        }
        _ => false,
    }
}
fn request_module(request: &LifecycleRequest) -> Option<&ModuleId> {
    match request {
        LifecycleRequest::Add { module_id, .. }
        | LifecycleRequest::Enable { module_id, .. }
        | LifecycleRequest::Disable { module_id, .. }
        | LifecycleRequest::Remove { module_id, .. }
        | LifecycleRequest::Purge { module_id, .. }
        | LifecycleRequest::Update { module_id, .. } => Some(module_id),
        _ => None,
    }
}
fn request_approvals(request: &LifecycleRequest) -> &BTreeSet<ApprovalId> {
    match request {
        LifecycleRequest::Add { approval_refs, .. }
        | LifecycleRequest::Enable { approval_refs, .. }
        | LifecycleRequest::Disable { approval_refs, .. }
        | LifecycleRequest::Remove { approval_refs, .. }
        | LifecycleRequest::Purge { approval_refs, .. }
        | LifecycleRequest::Update { approval_refs, .. }
        | LifecycleRequest::StartAccepted { approval_refs, .. } => approval_refs,
        _ => {
            static EMPTY: std::sync::OnceLock<BTreeSet<ApprovalId>> = std::sync::OnceLock::new();
            EMPTY.get_or_init(BTreeSet::new)
        }
    }
}
fn result_for(snapshot: &LifecycleSnapshot, ctx: &OperationContext) -> OperationResult {
    OperationResult {
        operation_id: ctx.operation_id.clone(),
        status: OperationStatus::Succeeded,
        transaction_id: None,
        desired_saved: false,
        pending_bundle: None,
        accepted_bundle: snapshot
            .runtime
            .accepted
            .as_ref()
            .map(|id| BundleId::new(id.as_str()).expect("wire IDs have the same grammar")),
        run_intent: if snapshot.runtime.run_intent == runtime::RunIntent::Running {
            RunIntent::Running
        } else {
            RunIntent::Stopped
        },
        member_results: vec![],
        survivors: vec![],
        issues: vec![],
    }
}
fn setup_member(setup: &SetupResult) -> MemberResult {
    MemberResult {
        module_id: setup.module_id.clone(),
        status: if setup.complete {
            OperationStatus::Succeeded
        } else {
            OperationStatus::Partial
        },
        resources: setup
            .receipts
            .iter()
            .flat_map(|r| r.resources.iter().map(|r| r.resource_id.clone()))
            .collect(),
        error: setup.error.clone(),
    }
}
fn terminal_status(status: OperationStatus) -> TerminalStatus {
    match status {
        OperationStatus::Succeeded => TerminalStatus::Committed,
        OperationStatus::Partial => TerminalStatus::Partial,
        OperationStatus::Failed => TerminalStatus::Failed,
        OperationStatus::RecoveryRequired => TerminalStatus::RecoveryRequired,
    }
}
fn core_id() -> ModuleId {
    ModuleId::new("core").expect("fixed ID")
}

/// Bounded inventory metadata, independent of native runtime observation.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInventoryEntry {
    pub module_id: ModuleId,
    pub phase: Option<ReleasePhase>,
    pub catalogue_implemented: Option<bool>,
    pub registered: Option<bool>,
    pub membership: Option<MembershipState>,
    pub setup_state: Option<SetupState>,
    pub ownership: Option<Ownership>,
    pub desired_enabled: Option<bool>,
    pub native_state: &'static str,
}

/// Known absence and unavailable registry evidence are different states.
pub enum RegistryInventory<'a> {
    Available(&'a RegistrySnapshot),
    Absent,
    Unavailable,
}
/// Missing desired means unavailable; no native state is inferred.
pub fn list(
    catalogue: &ModuleCatalogue,
    desired: Option<&DesiredSnapshot>,
    registry_status: RegistryInventory<'_>,
) -> Vec<ModuleInventoryEntry> {
    let registry = match registry_status {
        RegistryInventory::Available(snapshot) => Some(snapshot),
        _ => None,
    };
    let registry_known = !matches!(registry_status, RegistryInventory::Unavailable);
    let mut ids: BTreeSet<_> = catalogue.entries().keys().cloned().collect();
    if let Some(registry) = registry {
        ids.extend(registry.entries.keys().cloned());
    }
    if let Some(desired) = desired {
        ids.extend(desired.features.keys().cloned());
    }
    ids.into_iter()
        .map(|id| {
            let descriptor = catalogue.describe(&id);
            let entry = registry.and_then(|registry| registry.entries.get(&id));
            ModuleInventoryEntry {
                phase: descriptor.map(|entry| entry.descriptor.phase),
                catalogue_implemented: descriptor.map(|entry| entry.implemented),
                registered: registry_known.then_some(entry.is_some()),
                membership: entry.map(|entry| entry.membership),
                setup_state: entry.map(|entry| entry.setup_state.clone()),
                ownership: entry.map(|entry| entry.ownership),
                desired_enabled: desired.map(|desired| {
                    if id.as_str() == "core" {
                        true
                    } else {
                        desired
                            .features
                            .get(&id)
                            .is_some_and(|selection| selection.enabled)
                    }
                }),
                native_state: "unverified",
                module_id: id,
            }
        })
        .collect()
}

fn finish_outcome(
    journal: &JournalStore,
    tx: &TransactionId,
    request: &LifecycleRequest,
    snapshot: &LifecycleSnapshot,
    receipts: Vec<StepReceipt>,
    outcome: Result<OperationResult>,
    ctx: &OperationContext,
) -> Result<OperationResult> {
    let mut result = match outcome {
        Ok(result) => result,
        Err(error) => {
            let mut result = result_for(snapshot, ctx);
            result.status = if matches!(
                error.code,
                ErrorCode::RecoveryRequired | ErrorCode::ResourceDrift | ErrorCode::IoUnavailable
            ) {
                OperationStatus::RecoveryRequired
            } else {
                OperationStatus::Failed
            };
            result.member_results.push(MemberResult {
                module_id: request_module(request).cloned().unwrap_or_else(core_id),
                status: result.status,
                resources: vec![],
                error: Some(error),
            });
            result
        }
    };
    if result.operation_id != ctx.operation_id {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "lifecycle effect returned another operation identity",
        ));
    }
    result.transaction_id = Some(tx.clone());
    let status = terminal_status(result.status);
    journal.finish_operation(tx, status, receipts, &result, ctx)?;
    Ok(result)
}

/// Config-independent lifecycle seam; all policy/journalling stays in this owner.
pub fn execute_runtime_request<F>(
    request: LifecycleRequest,
    snapshot: &LifecycleSnapshot,
    catalogue: &ModuleCatalogue,
    journal: &JournalStore,
    effect: F,
    ctx: &OperationContext,
) -> Result<OperationResult>
where
    F: FnOnce(&LifecyclePlan, &mut Vec<StepReceipt>) -> Result<OperationResult>,
{
    ensure_mutating(ctx, journal.control())?;
    if !matches!(
        request,
        LifecycleRequest::Down
            | LifecycleRequest::StartAccepted { .. }
            | LifecycleRequest::RollbackAccepted { .. }
            | LifecycleRequest::RestartServices { .. }
    ) {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "runtime-only entry accepts down, accepted start, exact rollback, or bounded accepted-service restart",
        ));
    }
    let prepared = PreparedOperation::default();
    let plan = plan(request.clone(), snapshot, &prepared, catalogue, ctx)?;
    if !plan.issues.is_empty() {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "runtime lifecycle plan has blockers",
        ));
    }
    if !matches!(request, LifecycleRequest::Down) && !journal.unfinished(ctx)?.is_empty() {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "unfinished operation requires reconciliation before startup",
        ));
    }
    let tx = journal.prepare_intent(Intent::Lifecycle { plan: plan.clone() }, vec![], ctx)?;
    let mut receipts = Vec::new();
    let outcome = effect(&plan, &mut receipts);
    finish_outcome(journal, &tx, &request, snapshot, receipts, outcome, ctx)
}

/// Remove an owned feature whose physical services belong to another registry
/// owner. The pure planner still validates dependents, selection and effects;
/// this seam grants no authority to stop resources or erase retained data.
pub fn execute_resource_free_remove<F>(
    request: LifecycleRequest,
    snapshot: &LifecycleSnapshot,
    prepared: &PreparedOperation,
    catalogue: &ModuleCatalogue,
    registry: &RegistryStore,
    journal: &JournalStore,
    effect: F,
    ctx: &OperationContext,
) -> Result<OperationResult>
where
    F: FnOnce(&LifecyclePlan) -> Result<OperationResult>,
{
    ensure_mutating(ctx, journal.control())?;
    let LifecycleRequest::Remove { module_id, .. } = &request else {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "feature removal entry requires Remove",
        ));
    };
    let entry = installed(snapshot, module_id, ctx)?;
    if entry.ownership != Ownership::Owned || !entry.resources.is_empty() {
        return Err(failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "module removal requires a qualified exact resource stop adapter",
        ));
    }
    let planned = plan(request.clone(), snapshot, prepared, catalogue, ctx)?;
    if !planned.issues.is_empty() || !journal.unfinished(ctx)?.is_empty() {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "unfinished operation or blocked membership plan requires reconciliation",
        ));
    }
    let fresh = registry.read(&snapshot.registry.instance_id, ctx)?;
    if fresh != snapshot.registry {
        return Err(failure(
            ctx,
            ErrorCode::PlanStale,
            "module registry changed before removal",
        ));
    }
    let tx = journal.prepare_intent(
        Intent::Lifecycle {
            plan: planned.clone(),
        },
        vec![],
        ctx,
    )?;
    let outcome = (|| {
        let mut result = effect(&planned)?;
        let current = registry.read(&snapshot.registry.instance_id, ctx)?;
        let unchanged = current
            .entries
            .get(module_id)
            .is_some_and(|current| current == entry);
        if !unchanged {
            result.status = OperationStatus::RecoveryRequired;
            result.member_results.push(MemberResult {
                module_id: module_id.clone(),
                status: OperationStatus::RecoveryRequired,
                resources: vec![],
                error: Some(failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "module ownership changed during membership apply",
                )),
            });
            return Ok(result);
        }
        let change = if result.status == OperationStatus::Succeeded && result.survivors.is_empty() {
            RegistryChange::Remove {
                module_id: module_id.clone(),
                retained_artifacts: retained_owned_artifacts(entry),
            }
        } else {
            if result.status != OperationStatus::RecoveryRequired {
                result.status = OperationStatus::Partial;
            }
            RegistryChange::CleanupRequired {
                module_id: module_id.clone(),
                resources: vec![],
            }
        };
        registry.transition(
            &snapshot.registry.instance_id,
            current.revision,
            change,
            &RegistryPreconditions::default(),
            ctx,
        )?;
        result.member_results.push(MemberResult {
            module_id: module_id.clone(),
            status: result.status,
            resources: vec![],
            error: None,
        });
        Ok(result)
    })();
    finish_outcome(journal, &tx, &request, snapshot, vec![], outcome, ctx)
}

/// Validate the supported retained-config purge without conferring storage or
/// secret authority. A config artifact is not a native resource deletion action.
pub fn validate_resource_free_config_purge(
    snapshot: &LifecycleSnapshot,
    module: &ModuleId,
    artifact: &ArtifactId,
    ctx: &OperationContext,
) -> Result<()> {
    validate_purge_participation(snapshot, module, ctx)?;
    let entry = snapshot.registry.entries.get(module).ok_or_else(|| {
        failure(
            ctx,
            ErrorCode::ModuleUnknown,
            "purge module is not registered",
        )
    })?;
    if entry.ownership != Ownership::Owned
        || entry.membership != MembershipState::RemovedRetained
        || !entry.resources.is_empty()
        || !entry.installed_artifact_versions.is_empty()
        || entry.retained_artifacts.iter().collect::<BTreeSet<_>>() != BTreeSet::from([artifact])
        || snapshot.registry.recovery_originals.contains(artifact)
        || snapshot
            .registry
            .entries
            .iter()
            .any(|(id, other)| id != module && other.retained_artifacts.contains(artifact))
    {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "purge lacks exact removed, disabled, exclusive retained-config ownership",
        ));
    }
    Ok(())
}

pub fn validate_purge_participation(
    snapshot: &LifecycleSnapshot,
    module: &ModuleId,
    ctx: &OperationContext,
) -> Result<()> {
    let desired = require_desired(snapshot, ctx)?;
    if desired
        .features
        .get(module)
        .is_some_and(|selection| selection.enabled)
    {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "enabled module participation blocks retained configuration purge",
        ));
    }
    if snapshot.runtime.trial.is_some()
        || snapshot.runtime.maintenance.is_some()
        || snapshot.runtime.lease.as_ref().is_some_and(|lease| {
            !matches!(
                lease.state,
                runtime::LeaseState::Committed
                    | runtime::LeaseState::Compensated
                    | runtime::LeaseState::StoppedWithArtifacts
            )
        })
    {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "runtime recovery or trial blocks retained configuration purge",
        ));
    }
    Ok(())
}

// Older registries retained verified setup artifacts only in checkpoints.
// Preserve those claims when removing an owned feature; external artifacts do
// not become HTON-owned through an associated lifecycle observation.
fn retained_owned_artifacts(entry: &RegistryEntry) -> Vec<ArtifactId> {
    let mut artifacts: BTreeSet<_> = entry.retained_artifacts.iter().cloned().collect();
    if entry.ownership == Ownership::Owned {
        for receipt in &entry.checkpoints {
            if receipt.status == StepStatus::Verified && receipt.error.is_none() {
                artifacts.extend(receipt.retained_artifacts.iter().cloned());
            }
        }
    }
    artifacts.into_iter().collect()
}
