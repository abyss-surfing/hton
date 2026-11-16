//! Resumable setup. Acquiring a resource never implies enabling or starting it.
use crate::catalog::ModuleCatalogue;
use crate::journal::{Intent, JournalStore, TerminalStatus, ensure_mutating, failure};
use crate::registry::{RegistryPreconditions, RegistryStore};
use hton_contracts::{config::*, operations::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub struct SetupProgressEvidence {
    pub resources: Vec<ResourceIdentity>,
    pub installed_versions: BTreeMap<ArtifactId, ImageDigest>,
    pub preconditions: RegistryPreconditions,
}
/// Trusted provider: Prepared observation means proven absent/not performed;
/// Uncertain means an effect may exist and MUST NOT be replayed.
pub trait SetupBackend: ModuleSetup {
    fn observe_step(&self, step: &SetupStep, ctx: &OperationContext) -> Result<StepReceipt>;
    fn registration(
        &self,
        module: &ModuleId,
        existing: Option<&RegistryEntry>,
        ctx: &OperationContext,
    ) -> Result<RegistryEntry>;
    fn progress(
        &self,
        module: &ModuleId,
        receipt: &StepReceipt,
        ctx: &OperationContext,
    ) -> Result<SetupProgressEvidence>;
}
pub trait SetupInventory {
    fn artifacts(&self, module: &ModuleId, ctx: &OperationContext) -> Result<Vec<NativeArtifact>>;
    fn settings(&self, module: &ModuleId, ctx: &OperationContext) -> Result<ModuleSettings>;
}
#[derive(Clone, Debug)]
pub struct SetupResult {
    pub module_id: ModuleId,
    pub complete: bool,
    pub newly_registered: bool,
    pub registry_revision: StateRevision,
    pub transaction_id: Option<TransactionId>,
    pub receipts: Vec<StepReceipt>,
    pub missing_inputs: Vec<MissingInput>,
    pub operator_actions: Vec<StepId>,
    pub error: Option<Error>,
}
pub struct SetupCoordinator {
    pub catalogue: Arc<ModuleCatalogue>,
    pub registry: Arc<RegistryStore>,
    pub journal: Arc<JournalStore>,
    pub inventory: Arc<dyn SetupInventory>,
    pub providers: BTreeMap<ModuleId, Arc<dyn SetupBackend>>,
    pub instance_id: InstanceId,
}
impl SetupCoordinator {
    pub fn prepare(
        &self,
        module_id: &ModuleId,
        registry: &RegistrySnapshot,
        inventory: &[NativeArtifact],
        supplied: &ModuleSettings,
        ctx: &OperationContext,
    ) -> Result<SetupPlan> {
        let expected_format = if module_id.as_str() == "core" {
            if ctx.actor.source != ActorSource::Local
                || !ctx
                    .actor
                    .granted_scopes
                    .iter()
                    .any(|scope| scope.as_str() == "installation.bootstrap")
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "core bootstrap requires local installation authority",
                ));
            }
            FormatVersion(1)
        } else {
            let entry = self.catalogue.describe(module_id).ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleUnknown,
                    "module is not in the trusted catalogue",
                )
            })?;
            if !entry.implemented || entry.descriptor.phase != ReleasePhase::R1 {
                return Err(failure(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "module setup is outside the selected release",
                ));
            }
            entry.descriptor.config_version
        };
        let provider = self.providers.get(module_id).ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "trusted setup capability is unavailable",
            )
        })?;
        let existing = registry.entries.get(module_id);
        if existing.is_some_and(|entry| entry.membership == MembershipState::CleanupRequired) {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "module cleanup requires reconciliation",
            ));
        }
        if supplied.schema_version != expected_format {
            return Err(failure(
                ctx,
                ErrorCode::ConfigUnsupportedVersion,
                "module setup input format is unsupported",
            ));
        }
        let mut plan = provider.prepare(module_id, existing, inventory, supplied)?;
        plan.expected_registry_revision = registry.revision;
        if &plan.module_id != module_id {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "setup provider returned another module",
            ));
        }
        validate_plan(&plan, ctx)?;
        Ok(plan)
    }
    pub fn preview(&self, module_id: &ModuleId, ctx: &OperationContext) -> Result<SetupPlan> {
        let registry = self.registry.read(&self.instance_id, ctx)?;
        let artifacts = self.inventory.artifacts(module_id, ctx)?;
        let settings = self.inventory.settings(module_id, ctx)?;
        self.prepare(module_id, &registry, &artifacts, &settings, ctx)
    }
    pub fn resume(&self, plan: &SetupPlan, ctx: &OperationContext) -> Result<SetupResult> {
        ensure_mutating(ctx, self.journal.control())?;
        let mut state = self.registry.read(&self.instance_id, ctx)?;
        let current = self.prepare(
            &plan.module_id,
            &state,
            &self.inventory.artifacts(&plan.module_id, ctx)?,
            &self.inventory.settings(&plan.module_id, ctx)?,
            ctx,
        )?;
        if current != *plan {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "setup inputs changed since preview",
            ));
        }
        let provider = self.providers.get(&plan.module_id).ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "setup provider unavailable",
            )
        })?;
        let existing = state.entries.get(&plan.module_id);
        let new = existing.is_none();
        if existing.is_some_and(|entry| {
            entry.setup_state == SetupState::Complete
                && entry.membership == MembershipState::Installed
        }) {
            let done = self.observe_complete(plan, provider.as_ref(), state.revision, ctx)?;
            self.complete_prior_setup(&plan.module_id, &done.receipts, ctx)?;
            return Ok(done);
        }
        let tx = self
            .journal
            .prepare_intent(Intent::Setup { plan: plan.clone() }, vec![], ctx)?;
        if new || existing.is_some_and(|entry| entry.membership == MembershipState::RemovedRetained)
        {
            let mut entry = provider.registration(&plan.module_id, existing, ctx)?;
            if entry.module_id != plan.module_id || entry.membership != MembershipState::Installed {
                return Err(failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "setup registration identity changed",
                ));
            }
            entry.setup_state = SetupState::Incomplete {
                blocked_step: plan.steps.first().map(|s| s.step_id.clone()),
                required_operator_action: plan.required_operator_actions.first().cloned(),
            };
            let pre = provider
                .progress(&plan.module_id, &empty_receipt(ctx), ctx)?
                .preconditions;
            if entry.ownership != Ownership::Owned {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "foreign association requires a separate qualified association plan",
                ));
            }
            if entry.resources.iter().any(|resource| {
                !pre.observed
                    .get(&resource.resource_id)
                    .is_some_and(|observed| {
                        observed.identity == *resource
                            && observed.state == ObservedState::Stopped
                            && observed.valid_until.0 > self.journal.control().now_ms()
                    })
            }) {
                return Err(failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "new owned resources lack exact stopped observations",
                ));
            }
            let change = RegistryChange::Install { entry };
            state =
                self.registry
                    .transition(&self.instance_id, state.revision, change, &pre, ctx)?;
        }
        let mut receipts = Vec::new();
        let mut verified = BTreeSet::new();
        let mut pending: Vec<_> = plan.steps.iter().collect();
        while !pending.is_empty() {
            let index = pending
                .iter()
                .position(|s| s.dependencies.iter().all(|id| verified.contains(id)))
                .ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::ModuleIncomplete,
                        "setup dependency is not verified",
                    )
                })?;
            let step = pending.remove(index);
            ensure_mutating(ctx, self.journal.control())?;
            let mut bounded = ctx.clone();
            bounded.deadline = UnixMillis(
                ctx.deadline.0.min(
                    self.journal
                        .control()
                        .now_ms()
                        .saturating_add(step.timeout.0),
                ),
            );
            let mut receipt = provider.observe_step(step, &bounded)?;
            check_receipt(step, &receipt, self.journal.control().now_ms(), ctx)?;
            if receipt.status == StepStatus::Prepared {
                if let Some(approval) = &step.required_approval {
                    if !ctx.actor.local_approval_refs.contains(approval) {
                        return Err(failure(
                            ctx,
                            ErrorCode::ApprovalRequired,
                            "setup step requires a matching local approval",
                        ));
                    }
                }
                ensure_mutating(&bounded, self.journal.control())?;
                receipt = provider.execute_step(step, &bounded)?;
                check_receipt(step, &receipt, self.journal.control().now_ms(), ctx)?;
                if self.journal.control().now_ms() >= bounded.deadline.0 {
                    receipt.status = StepStatus::Uncertain;
                    receipt.error = Some(failure(
                        ctx,
                        ErrorCode::OperationDeadline,
                        "setup effect exceeded its deadline; reconcile before retry",
                    ));
                }
            }
            self.journal.record_step(&tx, &receipt, ctx)?;
            let mut progress = provider.progress(&plan.module_id, &receipt, ctx)?;
            if receipt.status != StepStatus::Verified {
                progress.resources.clear();
                progress.installed_versions.clear();
            }
            state = self.registry.transition(
                &self.instance_id,
                state.revision,
                RegistryChange::SetupProgress {
                    module_id: plan.module_id.clone(),
                    resources: progress.resources,
                    installed_versions: progress.installed_versions,
                    setup_state: SetupState::Incomplete {
                        blocked_step: Some(step.step_id.clone()),
                        required_operator_action: plan
                            .required_operator_actions
                            .iter()
                            .find(|id| !verified.contains(*id))
                            .cloned(),
                    },
                    checkpoint: Some(receipt.clone()),
                },
                &progress.preconditions,
                ctx,
            )?;
            let success = receipt.status == StepStatus::Verified && receipt.error.is_none();
            let error = receipt.error.clone();
            receipts.push(receipt);
            if !success {
                return Ok(SetupResult {
                    module_id: plan.module_id.clone(),
                    complete: false,
                    newly_registered: new,
                    registry_revision: state.revision,
                    transaction_id: Some(tx),
                    receipts,
                    missing_inputs: plan.missing_inputs.clone(),
                    operator_actions: plan.required_operator_actions.clone(),
                    error: Some(error.unwrap_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::ModuleIncomplete,
                            "setup step is incomplete or uncertain",
                        )
                    })),
                });
            }
            verified.insert(step.step_id.clone());
        }
        let unresolved: Vec<_> = plan
            .required_operator_actions
            .iter()
            .filter(|id| !verified.contains(*id))
            .cloned()
            .collect();
        let complete = plan.missing_inputs.is_empty() && unresolved.is_empty();
        if complete {
            state = self.registry.transition(
                &self.instance_id,
                state.revision,
                RegistryChange::SetupProgress {
                    module_id: plan.module_id.clone(),
                    resources: vec![],
                    installed_versions: BTreeMap::new(),
                    setup_state: SetupState::Complete,
                    checkpoint: None,
                },
                &RegistryPreconditions::default(),
                ctx,
            )?;
            self.journal
                .finish(&tx, TerminalStatus::Committed, receipts.clone(), ctx)?;
            self.complete_prior_setup(&plan.module_id, &receipts, ctx)?;
        }
        Ok(SetupResult {
            module_id: plan.module_id.clone(),
            complete,
            newly_registered: new,
            registry_revision: state.revision,
            transaction_id: Some(tx),
            receipts,
            missing_inputs: plan.missing_inputs.clone(),
            operator_actions: unresolved,
            error: None,
        })
    }
    fn observe_complete(
        &self,
        plan: &SetupPlan,
        provider: &dyn SetupBackend,
        revision: StateRevision,
        ctx: &OperationContext,
    ) -> Result<SetupResult> {
        if !plan.missing_inputs.is_empty() {
            return Err(failure(
                ctx,
                ErrorCode::ModuleIncomplete,
                "installed module settings require repair before setup can be confirmed",
            ));
        }
        let mut receipts = Vec::new();
        for step in &plan.steps {
            let receipt = provider.observe_step(step, ctx)?;
            check_receipt(step, &receipt, self.journal.control().now_ms(), ctx)?;
            if receipt.status != StepStatus::Verified || receipt.error.is_some() {
                return Err(failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "previously complete setup no longer matches its resources",
                ));
            }
            receipts.push(receipt);
        }
        Ok(SetupResult {
            module_id: plan.module_id.clone(),
            complete: true,
            newly_registered: false,
            registry_revision: revision,
            transaction_id: None,
            receipts,
            missing_inputs: vec![],
            operator_actions: vec![],
            error: None,
        })
    }
    fn complete_prior_setup(
        &self,
        module: &ModuleId,
        fresh: &[StepReceipt],
        ctx: &OperationContext,
    ) -> Result<()> {
        for previous in self.journal.unfinished(ctx)? {
            if previous.operation_id == ctx.operation_id
                || previous.actor.actor_id != ctx.actor.actor_id
            {
                continue;
            }
            let Intent::Setup { plan } = &previous.intent else {
                continue;
            };
            if &plan.module_id != module {
                continue;
            }
            let needed: BTreeSet<_> = plan.steps.iter().map(|step| step.step_id.clone()).collect();
            let receipts: Vec<_> = fresh
                .iter()
                .filter(|receipt| needed.contains(&receipt.step_id))
                .cloned()
                .collect();
            if receipts.len() != needed.len()
                || receipts
                    .iter()
                    .any(|r| r.status != StepStatus::Verified || r.error.is_some())
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "prior setup recipe differs from fresh verified progress",
                ));
            }
            let mut original_ctx = ctx.clone();
            original_ctx.operation_id = previous.operation_id.clone();
            original_ctx.actor = previous.actor.clone();
            if self
                .journal
                .terminal(&previous.transaction_id, ctx)?
                .is_some()
            {
                self.journal.resolve_operation(
                    &previous.transaction_id,
                    TerminalStatus::Committed,
                    receipts,
                    None,
                    &original_ctx,
                )?;
            } else {
                self.journal.finish(
                    &previous.transaction_id,
                    TerminalStatus::Committed,
                    receipts,
                    &original_ctx,
                )?;
            }
        }
        Ok(())
    }
}
fn empty_receipt(ctx: &OperationContext) -> StepReceipt {
    StepReceipt {
        step_id: StepId::new("registration").expect("fixed ID"),
        idempotency_key: ctx.actor.request_id.clone(),
        status: StepStatus::Prepared,
        recorded_at: UnixMillis(0),
        resources: vec![],
        state_fingerprints: BTreeMap::new(),
        retained_artifacts: vec![],
        error: None,
    }
}
pub fn validate_plan(plan: &SetupPlan, ctx: &OperationContext) -> Result<()> {
    if plan.steps.len() > 256
        || plan.checkpoints.len() > 4096
        || plan.missing_inputs.len() > 128
        || plan.required_operator_actions.len() > 128
    {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "setup plan exceeds trusted bounds",
        ));
    }
    let ids: BTreeSet<_> = plan.steps.iter().map(|s| s.step_id.clone()).collect();
    let keys: BTreeSet<_> = plan
        .steps
        .iter()
        .map(|s| s.idempotency_key.clone())
        .collect();
    if ids.len() != plan.steps.len()
        || keys.len() != plan.steps.len()
        || plan
            .required_operator_actions
            .iter()
            .any(|id| !ids.contains(id))
        || plan.steps.iter().any(|s| {
            s.timeout.0 == 0
                || s.timeout.0 > 300_000
                || s.dependencies.iter().any(|id| !ids.contains(id))
        })
    {
        return Err(failure(
            ctx,
            ErrorCode::OperationConflict,
            "setup step identities, dependencies or limits are invalid",
        ));
    }
    let mut ready = BTreeSet::new();
    while ready.len() < ids.len() {
        let before = ready.len();
        for step in &plan.steps {
            if step.dependencies.iter().all(|id| ready.contains(id)) {
                ready.insert(step.step_id.clone());
            }
        }
        if before == ready.len() {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "setup dependency cycle",
            ));
        }
    }
    Ok(())
}
fn check_receipt(
    step: &SetupStep,
    receipt: &StepReceipt,
    now: u64,
    ctx: &OperationContext,
) -> Result<()> {
    if receipt.step_id != step.step_id
        || receipt.idempotency_key != step.idempotency_key
        || receipt.recorded_at.0 > now
        || receipt.recorded_at.0 < now.saturating_sub(60_000)
        || receipt.resources.len() > 128
        || receipt.retained_artifacts.len() > 128
        || (receipt.status == StepStatus::Verified && receipt.error.is_some())
    {
        return Err(failure(
            ctx,
            ErrorCode::ResourceDrift,
            "setup receipt does not match its declared step",
        ));
    }
    Ok(())
}
