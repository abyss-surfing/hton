//! One exact-plan apply coordinator; shell/API do not duplicate its safety rules.
use crate::{
    journal::{
        Intent, JournalStore, ProtectedOriginal, TerminalStatus, Transaction, ensure_mutating,
        failure,
    },
    lock,
    render::CompiledBundle,
};
use hton_contracts::{config::*, operations::*};
use hton_runtime_contract as wire;
use hton_storage::SecureDir;
use std::collections::BTreeSet;

pub struct FreshApplyInputs {
    pub plan: EffectPlan,
    pub expected_inputs: ExpectedInputs,
    pub instance_id: InstanceId,
    pub run_intent: RunIntent,
    pub accepted_bundle: Option<BundleId>,
    pub originals: Vec<ProtectedOriginal>,
}
/// Recompose via existing config/catalog/plan owners, including ALL five synthetic
/// records. Native observations and approval provenance are refreshed explicitly.
pub trait ApplyRecheck: Send + Sync {
    fn refresh(&self, plan: &EffectPlan, ctx: &OperationContext) -> Result<FreshApplyInputs>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualificationMode {
    NativeOnly,
    NativeAndStaging,
}
pub struct QualifiedApply {
    pub candidate: wire::CandidateBundle,
    pub proof: wire::AdmissionProof,
    pub manifest_digest: wire::Sha256Digest,
}
pub trait ApplyPreparation: Send + Sync {
    /// Protected PlanId-associated option, never smuggled by apply payload.
    fn mode(&self, plan: &PlanId, ctx: &OperationContext) -> Result<QualificationMode>;
    /// Uses render::compile; journal intent already exists before scoped staging.
    /// No undeclared secret allocation or alteration of approved plan/inverses.
    fn compile(
        &self,
        plan: &EffectPlan,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<CompiledBundle>;
    /// Pinned native validators always; staging only when requested. Publish through
    /// render::publish_qualified after evidence is bound to immutable artifact hashes.
    fn qualify(
        &self,
        bundle: &CompiledBundle,
        mode: QualificationMode,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<QualifiedApply>;
    fn save_pending(
        &self,
        bundle: &QualifiedApply,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<()>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeFinality {
    Final,
    TrialPending,
}
pub struct RuntimeApplyResult {
    pub result: OperationResult,
    pub receipts: Vec<StepReceipt>,
    pub finality: RuntimeFinality,
}
pub trait ApplyRuntime: Send + Sync {
    /// Independent runtime rechecks admission proof/epoch and returns verified
    /// mapped native receipts. It must not call compiler, pull images or load YAML.
    fn activate(
        &self,
        bundle: &QualifiedApply,
        ctx: &OperationContext,
    ) -> Result<RuntimeApplyResult>;
}
/// Core-side bridge from independently verified native outcomes to RegistryStore.
/// It must rebind exact owned recreations and read back the resulting registry;
/// no new native effects or discovery/adoption are permitted here.
pub trait AcceptedBindingPort: Send + Sync {
    fn reconcile(
        &self,
        plan: &EffectPlan,
        applied: &RuntimeApplyResult,
        ctx: &OperationContext,
    ) -> Result<()>;
}
pub enum Reconciliation {
    Resolved {
        status: TerminalStatus,
        receipts: Vec<StepReceipt>,
        result: Option<OperationResult>,
    },
    /// Only a live enclosing coordinator in this same trusted operation may qualify.
    ActiveParent,
    NeedsRecovery,
}
pub trait ApplyReconciliation: Send + Sync {
    fn reconcile(
        &self,
        transaction: &Transaction,
        ctx: &OperationContext,
    ) -> Result<Reconciliation>;
}
pub trait EffectApplyPort: Send + Sync {
    fn stage_and_apply(
        &self,
        effect: &EffectPlan,
        ctx: &OperationContext,
    ) -> Result<OperationResult>;
}
pub trait ApplyOperation: Send + Sync {
    fn apply(
        &self,
        plan: &PlanId,
        expected: &ExpectedInputs,
        ctx: &OperationContext,
    ) -> Result<OperationResult>;
}

pub struct ApplyCoordinator<'a> {
    pub journal: &'a JournalStore,
    pub config_lock: &'a SecureDir,
    pub recheck: &'a dyn ApplyRecheck,
    pub preparation: &'a dyn ApplyPreparation,
    pub runtime: &'a dyn ApplyRuntime,
    pub bindings: &'a dyn AcceptedBindingPort,
    pub reconciliation: &'a dyn ApplyReconciliation,
}
impl ApplyCoordinator<'_> {
    fn reconcile_pending(
        &self,
        except: Option<&TransactionId>,
        ctx: &OperationContext,
    ) -> Result<()> {
        for tx in self.journal.unfinished(ctx)? {
            if except == Some(&tx.transaction_id) {
                continue;
            }
            match self.reconciliation.reconcile(&tx, ctx)? {
                Reconciliation::ActiveParent if tx.operation_id == ctx.operation_id => (),
                Reconciliation::Resolved {
                    status,
                    receipts,
                    result,
                } => {
                    let mut recovered = ctx.clone();
                    recovered.operation_id = tx.operation_id.clone();
                    recovered.actor = tx.actor.clone();
                    if let Some(result) = result {
                        self.journal.resolve_operation(
                            &tx.transaction_id,
                            status,
                            receipts,
                            Some(&result),
                            &recovered,
                        )?;
                    } else {
                        self.journal.resolve_operation(
                            &tx.transaction_id,
                            status,
                            receipts,
                            None,
                            &recovered,
                        )?;
                    }
                }
                _ => {
                    return Err(failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "unfinished operation requires reconciliation before apply",
                    ));
                }
            }
        }
        Ok(())
    }
    fn cached(&self, intent: &Intent, ctx: &OperationContext) -> Result<Option<OperationResult>> {
        let Some(tx) = self.journal.lookup_intent(intent, ctx)? else {
            return Ok(None);
        };
        if let Some(record) = self.journal.terminal(&tx.transaction_id, ctx)? {
            if let Some(result) = record.operation_result {
                return Ok(Some(result));
            }
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "apply terminal lacks durable result metadata",
            ));
        }
        match self.reconciliation.reconcile(&tx, ctx)? {
            Reconciliation::Resolved {
                status,
                receipts,
                result: Some(result),
            } => {
                self.journal.resolve_operation(
                    &tx.transaction_id,
                    status,
                    receipts,
                    Some(&result),
                    ctx,
                )?;
                Ok(Some(result))
            }
            _ => Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "interrupted apply cannot be replayed without reconciliation",
            )),
        }
    }
    fn verify_fresh(
        &self,
        stored: &EffectPlan,
        fresh: &FreshApplyInputs,
        expected: Option<&ExpectedInputs>,
        ctx: &OperationContext,
    ) -> Result<()> {
        if &fresh.plan != stored
            || fresh.plan.read_set.coherence != ReadCoherence::Coherent
            || fresh.expected_inputs.read_set != stored.read_set
            || fresh.expected_inputs.resources != stored.resource_before
            || expected.is_some_and(|e| e != &fresh.expected_inputs)
        {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "staged plan or expected inputs changed",
            ));
        }
        let required = [
            "state.registry",
            "state.runtime",
            "state.catalogue",
            "plan.targetArtifacts",
            "plan.priorArtifacts",
        ];
        if required.iter().any(|name| {
            !stored
                .read_set
                .objects
                .contains_key(&ObjectId::new(*name).expect("fixed read-set identity"))
        }) {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "plan lacks complete synthetic input binding",
            ));
        }
        if stored.expires_at.0 <= self.journal.control().now_ms() {
            return Err(failure(ctx, ErrorCode::PlanExpired, "staged plan expired"));
        }
        let approved: BTreeSet<_> = stored.approvals.iter().map(|a| &a.approval_id).collect();
        if !approved
            .iter()
            .all(|id| ctx.actor.local_approval_refs.contains(*id))
        {
            return Err(failure(
                ctx,
                ErrorCode::ApprovalRequired,
                "apply lacks original exact approval references",
            ));
        }
        Ok(())
    }
    fn validate_qualified(
        &self,
        compiled: &CompiledBundle,
        qualified: &QualifiedApply,
        mode: QualificationMode,
        fresh: &FreshApplyInputs,
        ctx: &OperationContext,
    ) -> Result<()> {
        let published = compiled
            .directory
            .read("manifest.json", wire::MAX_WIRE_BYTES)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "qualified manifest is unavailable",
                )
            })?
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "qualified manifest has not been published",
                )
            })?;
        let published_candidate: wire::CandidateBundle =
            wire::decode(&published).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "published native manifest is invalid",
                )
            })?;
        if published_candidate != qualified.candidate
            || crate::config::digest(&published).as_str() != qualified.manifest_digest.as_str()
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "protected manifest differs from qualified admission material",
            ));
        }
        let original = wire::candidate_digest(&compiled.candidate).map_err(|_| {
            failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "compiled candidate cannot be fingerprinted",
            )
        })?;
        let actual = wire::candidate_digest(&qualified.candidate).map_err(|_| {
            failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "qualified candidate cannot be fingerprinted",
            )
        })?;
        if original != actual
            || qualified.candidate.manifest_digest != actual
            || qualified.proof.protected_artifact_digest != qualified.manifest_digest
            || qualified.proof.expected_state_revision != fresh.expected_inputs.runtime_revision.0
            || qualified.proof.expected_run_epoch != fresh.expected_inputs.run_epoch.0
            || qualified
                .proof
                .expected_accepted
                .as_ref()
                .map(|id| id.as_str())
                != fresh.accepted_bundle.as_ref().map(|id| id.as_str())
        {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "native qualification changed approved bundle or admission inputs",
            ));
        }
        for service in qualified.candidate.services.as_slice() {
            if !qualified
                .candidate
                .native_evidence
                .as_slice()
                .iter()
                .any(|e| {
                    e.service_id == service.service_id
                        && e.kind == wire::ProbeKind::NativeConfig
                        && e.outcome == wire::EvidenceOutcome::Pass
                        && e.bundle_digest == actual
                        && e.timestamp_ms <= self.journal.control().now_ms()
                })
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "required native validation evidence is missing",
                ));
            }
        }
        if mode == QualificationMode::NativeAndStaging
            && !qualified.candidate.staging.as_ref().is_some_and(|a| {
                a.bundle_digest == actual
                    && !a.evidence.as_slice().is_empty()
                    && a.evidence
                        .as_slice()
                        .iter()
                        .all(|e| e.outcome == wire::EvidenceOutcome::Pass)
            })
        {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "requested staging qualification is missing or failed",
            ));
        }
        Ok(())
    }
}
impl ApplyOperation for ApplyCoordinator<'_> {
    fn apply(
        &self,
        plan_id: &PlanId,
        expected: &ExpectedInputs,
        ctx: &OperationContext,
    ) -> Result<OperationResult> {
        ensure_mutating(ctx, self.journal.control())?;
        let stored = self.journal.read_plan_metadata(plan_id, &ctx.actor, ctx)?;
        let intent = Intent::Effects {
            plan: stored.plan.clone(),
        };
        if let Some(result) = self.cached(&intent, ctx)? {
            return Ok(result);
        }
        self.reconcile_pending(None, ctx)?;
        let initial = self.recheck.refresh(&stored.plan, ctx)?;
        let guard = lock::acquire(
            self.config_lock,
            &initial.instance_id,
            ctx,
            self.journal.control(),
        )?;
        let fresh = self.recheck.refresh(&stored.plan, ctx)?;
        self.verify_fresh(&stored.plan, &fresh, Some(expected), ctx)?;
        let tx = self
            .journal
            .prepare(&stored.plan, fresh.originals.clone(), ctx)?;
        let compiled = self.preparation.compile(&stored.plan, &tx, ctx)?;
        let mode = self.preparation.mode(plan_id, ctx)?;
        let qualified = self.preparation.qualify(&compiled, mode, &tx, ctx)?;
        ensure_mutating(ctx, self.journal.control())?;
        self.validate_qualified(&compiled, &qualified, mode, &fresh, ctx)?;
        // Compilation/qualification must not overwrite any desired or baseline input.
        let rechecked = self.recheck.refresh(&stored.plan, ctx)?;
        self.verify_fresh(&stored.plan, &rechecked, Some(expected), ctx)?;
        drop(guard); // Independent guardian must never wait for this Core lock.
        if fresh.run_intent == RunIntent::Stopped {
            self.preparation.save_pending(&qualified, &tx, ctx)?;
            let result = OperationResult {
                operation_id: ctx.operation_id.clone(),
                status: OperationStatus::Succeeded,
                transaction_id: Some(tx.clone()),
                desired_saved: true,
                pending_bundle: Some(
                    BundleId::new(qualified.candidate.bundle_id.as_str()).map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::OperationConflict,
                            "runtime bundle identity conversion failed",
                        )
                    })?,
                ),
                accepted_bundle: fresh.accepted_bundle,
                run_intent: RunIntent::Stopped,
                member_results: vec![],
                survivors: vec![],
                issues: vec![],
            };
            self.journal
                .finish_operation(&tx, TerminalStatus::Deferred, vec![], &result, ctx)?;
            return Ok(result);
        }
        let applied = self.runtime.activate(&qualified, ctx)?;
        if applied.result.operation_id != ctx.operation_id
            || applied.result.status == OperationStatus::Succeeded
                && (applied.result.accepted_bundle.as_ref().map(|b| b.as_str())
                    != Some(qualified.candidate.bundle_id.as_str())
                    || applied.result.run_intent != RunIntent::Running)
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "runtime result does not prove candidate acceptance",
            ));
        }
        if applied.finality == RuntimeFinality::Final {
            self.bindings.reconcile(&stored.plan, &applied, ctx)?;
        }
        let mut result = applied.result;
        result.transaction_id = Some(tx.clone());
        if applied.finality == RuntimeFinality::TrialPending {
            result.status = OperationStatus::RecoveryRequired;
            result.issues.push(CheckIssue {
                code: ErrorCode::RecoveryRequired,
                severity: Severity::Error,
                module_id: None,
                object_id: None,
                path: None,
                message: "runtime trial remains pending independent guardian reconciliation".into(),
                repair_hint: None,
            });
        }
        let status = match result.status {
            OperationStatus::Succeeded => TerminalStatus::Committed,
            OperationStatus::Partial => TerminalStatus::Partial,
            OperationStatus::Failed => TerminalStatus::Failed,
            OperationStatus::RecoveryRequired => TerminalStatus::RecoveryRequired,
        };
        self.journal
            .finish_operation(&tx, status, applied.receipts, &result, ctx)?;
        Ok(result)
    }
}
impl EffectApplyPort for ApplyCoordinator<'_> {
    fn stage_and_apply(
        &self,
        effect: &EffectPlan,
        ctx: &OperationContext,
    ) -> Result<OperationResult> {
        ensure_mutating(ctx, self.journal.control())?;
        if let Some(result) = self.cached(
            &Intent::Effects {
                plan: effect.clone(),
            },
            ctx,
        )? {
            return Ok(result);
        }
        let fresh = self.recheck.refresh(effect, ctx)?;
        self.verify_fresh(effect, &fresh, None, ctx)?;
        let bytes = serde_json::to_vec(&(ctx.operation_id.clone(), effect)).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "staged plan identity encoding failed",
            )
        })?;
        let id = PlanId::new(format!(
            "apply.{}",
            &crate::config::digest(&bytes).as_str()[7..]
        ))
        .map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "staged plan identity invalid",
            )
        })?;
        self.journal.stage_plan(id.clone(), effect, ctx)?;
        self.apply(&id, &fresh.expected_inputs, ctx)
    }
}
