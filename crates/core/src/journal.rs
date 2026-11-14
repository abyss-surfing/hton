//! Durable intent and observation-based recovery. This module never executes effects.
use hton_contracts::{config::*, operations::*};
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub trait OperationControl: Send + Sync {
    fn now_ms(&self) -> u64;
    fn is_cancelled(&self, cancellation: &CancellationId) -> bool;
}
/// CLI default has no cancellation registry; long-lived callers inject their own
/// OperationControl tied to their trusted cancellation source.
pub struct SystemControl;
impl OperationControl for SystemControl {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|v| v.as_millis().min(u64::MAX as u128) as u64)
            .unwrap_or(0)
    }
    fn is_cancelled(&self, _: &CancellationId) -> bool {
        false
    }
}
pub fn failure(ctx: &OperationContext, code: ErrorCode, message: &'static str) -> Error {
    Error {
        code,
        message: message.into(),
        module_id: None,
        config_path: None,
        resource_id: None,
        operation_id: ctx.operation_id.clone(),
        retryability: Retryability::AfterReconcile,
        repair_hint: None,
        retained_state: RetainedState::OriginalsRetained,
    }
}
pub fn ensure_mutating(ctx: &OperationContext, control: &dyn OperationControl) -> Result<()> {
    if ctx.execution_mode != ExecutionMode::Mutating {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "read-only operation cannot mutate storage",
        ));
    }
    if control.is_cancelled(&ctx.cancellation) {
        return Err(failure(
            ctx,
            ErrorCode::OperationCancelled,
            "operation cancelled",
        ));
    }
    if control.now_ms() >= ctx.deadline.0 {
        return Err(failure(
            ctx,
            ErrorCode::OperationDeadline,
            "operation deadline exceeded",
        ));
    }
    Ok(())
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProtectedOriginal {
    pub artifact_id: ArtifactId,
    pub digest: Sha256Digest,
    /// Defaults to true when decoding pre-presence journals; absence is distinct
    /// from an existing empty artifact.
    #[serde(default = "original_was_present")]
    pub was_present: bool,
    pub bytes: Vec<u8>,
}
fn original_was_present() -> bool {
    true
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Intent {
    PrepareBundle {
        plan: EffectPlan,
    },
    Qualification {
        bundle_id: BundleId,
        request_digest: Sha256Digest,
    },
    BundleValidation {
        bundle_id: BundleId,
        candidate_digest: Sha256Digest,
        scope: ObjectId,
    },
    Approval {
        scope_digest: Sha256Digest,
    },
    Installation {
        request_digest: Sha256Digest,
    },
    ForeignRemap {
        request_digest: Sha256Digest,
    },
    ForeignRestore {
        request_digest: Sha256Digest,
    },
    /// Immutable, digest-bound port candidate retained for exact CAS recovery.
    PortAllocation {
        candidate: PortAllocationCandidate,
        /// Installation identity that authorized this candidate; absent only
        /// on older journal records written before this binding was added.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instance_id: Option<InstanceId>,
    },
    /// Exact allowlisted root publication changes for installer CAS/recovery.
    InstallConfig {
        candidate: InstallConfigCandidate,
    },
    Effects {
        plan: EffectPlan,
    },
    Registry {
        instance_id: InstanceId,
        expected_revision: StateRevision,
        change: RegistryChange,
    },
    Lifecycle {
        plan: LifecyclePlan,
    },
    Setup {
        plan: SetupPlan,
    },
    Selection {
        expected_revision: ContentRevision,
        replacement_digest: Sha256Digest,
        module_ids: BTreeSet<ModuleId>,
        enabled: bool,
    },
    InstallationUninstall {
        registry: RegistrySnapshot,
        keep_certs: bool,
        keep_env: bool,
    },
    UninstallNative {
        resources: Vec<hton_runtime_contract::ResourceIdentity>,
    },
    ModulePurge {
        module_id: ModuleId,
        expected_registry_revision: StateRevision,
        entry_digest: Sha256Digest,
        config_revision: ContentRevision,
        approval_digest: Sha256Digest,
    },
    Update {
        plan: UpdatePlan,
    },
    Certificates {
        request_digest: Sha256Digest,
    },
    Backup {
        request_digest: Sha256Digest,
    },
    Restore {
        request_digest: Sha256Digest,
    },
    Identity {
        request_digest: Sha256Digest,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transaction {
    pub schema_version: u32,
    pub transaction_id: TransactionId,
    pub operation_id: OperationId,
    pub actor: ActorContext,
    pub prepared_at: UnixMillis,
    pub intent: Intent,
    pub originals: Vec<ProtectedOriginal>,
}
impl std::fmt::Debug for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("transaction_id", &self.transaction_id)
            .field("originals", &"[protected]")
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalStatus {
    Committed,
    RolledBack,
    RecoveryRequired,
    Partial,
    Failed,
    Deferred,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DurableRecord {
    pub schema_version: u32,
    pub transaction_id: TransactionId,
    pub operation_id: OperationId,
    pub status: TerminalStatus,
    pub completed_at: UnixMillis,
    pub receipts: Vec<StepReceipt>,
    pub operation_result: Option<OperationResult>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoredPlan {
    pub schema_version: u32,
    pub plan_id: PlanId,
    pub actor_id: ActorId,
    pub request_id: RequestId,
    pub plan: EffectPlan,
    pub staged_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryDisposition {
    NoEffectsObserved,
    EffectsObservedRequireVerification,
    RecoveryRequired,
    StoppedWithArtifacts,
}
#[derive(Clone, Debug)]
pub struct RecoveryResult {
    pub transaction_id: TransactionId,
    pub disposition: RecoveryDisposition,
    pub uncertain_resources: BTreeSet<ResourceId>,
    pub replay_permitted: bool,
}
pub struct JournalStore {
    directory: Arc<SecureDir>,
    control: Arc<dyn OperationControl>,
    log: Option<Arc<hton_logging::LogSink>>,
}
impl JournalStore {
    pub fn new(directory: Arc<SecureDir>, control: Arc<dyn OperationControl>) -> Self {
        Self {
            directory,
            control,
            log: None,
        }
    }
    pub fn with_log_sink(mut self, sink: Arc<hton_logging::LogSink>) -> Self {
        self.log = Some(sink);
        self
    }
    pub fn event(
        &self,
        component: hton_logging::Component,
        code: hton_logging::EventCode,
        ctx: &OperationContext,
    ) {
        if let Some(sink) = &self.log {
            let context = hton_logging::EventContext {
                operation_id: hton_logging::CorrelationId::new(ctx.operation_id.as_str()).ok(),
                transaction_id: None,
                module_id: None,
                ..Default::default()
            };
            let _ = sink.emit(
                component,
                context,
                hton_logging::Severity::Information,
                code,
                hton_logging::SafeFields::default(),
            );
        }
    }
    pub fn control(&self) -> &dyn OperationControl {
        self.control.as_ref()
    }
    pub fn prepare(
        &self,
        plan: &EffectPlan,
        originals: Vec<ProtectedOriginal>,
        ctx: &OperationContext,
    ) -> Result<TransactionId> {
        self.prepare_intent(Intent::Effects { plan: plan.clone() }, originals, ctx)
    }
    pub fn lookup_intent(
        &self,
        intent: &Intent,
        ctx: &OperationContext,
    ) -> Result<Option<Transaction>> {
        let id = intent_id(intent, ctx)?;
        let name = format!("intent-{}.json", id.as_str());
        let Some(bytes) = self.directory.read(&name, 8 * 1024 * 1024).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "journal intent lookup unavailable",
            )
        })?
        else {
            return Ok(None);
        };
        let tx: Transaction = serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "journal intent corrupt; original retained",
            )
        })?;
        if tx.schema_version != 1
            || tx.transaction_id != id
            || tx.operation_id != ctx.operation_id
            || tx.actor != ctx.actor
            || &tx.intent != intent
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "journal intent identity or actor mismatch",
            ));
        }
        Ok(Some(tx))
    }
    pub fn prepare_intent(
        &self,
        intent: Intent,
        originals: Vec<ProtectedOriginal>,
        ctx: &OperationContext,
    ) -> Result<TransactionId> {
        ensure_mutating(ctx, self.control())?;
        let id = intent_id(&intent, ctx)?;
        for original in &originals {
            if original.bytes.len() > 4 * 1024 * 1024
                || original.digest.as_str()
                    != format!("sha256:{:x}", Sha256::digest(&original.bytes))
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "protected original digest or size mismatch",
                ));
            }
        }
        let tx = Transaction {
            schema_version: 1,
            transaction_id: id.clone(),
            operation_id: ctx.operation_id.clone(),
            actor: ctx.actor.clone(),
            prepared_at: UnixMillis(self.control.now_ms()),
            intent,
            originals,
        };
        let name = format!("intent-{}.json", id.as_str());
        if let Some(old_bytes) = self
            .directory
            .read(&name, 8 * 1024 * 1024)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "journal read unavailable"))?
        {
            let old: Transaction = serde_json::from_slice(&old_bytes).map_err(|_| {
                failure(ctx, ErrorCode::RecoveryRequired, "journal original corrupt")
            })?;
            if old.schema_version == 1
                && old.transaction_id == id
                && old.operation_id == tx.operation_id
                && old.actor == tx.actor
                && old.intent == tx.intent
                && old.originals == tx.originals
            {
                self.create_idempotent(&name, &old, ctx)?;
                return Ok(id);
            }
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "operation identity already bound",
            ));
        }
        self.create_idempotent(&name, &tx, ctx)?;
        self.event(
            hton_logging::Component::Journal,
            hton_logging::EventCode::IntentDurable,
            ctx,
        );
        Ok(id)
    }
    pub fn read(&self, id: &TransactionId, ctx: &OperationContext) -> Result<Transaction> {
        let tx: Transaction = self.read_json(&format!("intent-{}.json", id.as_str()), ctx)?;
        if tx.schema_version != 1 || &tx.transaction_id != id {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "journal identity or version mismatch",
            ));
        }
        Ok(tx)
    }
    /// Read-only recovery inventory. Any corrupt terminal record blocks mutation
    /// rather than hiding an uncertain transaction from reconciliation.
    pub fn unfinished(&self, ctx: &OperationContext) -> Result<Vec<Transaction>> {
        let names = self.directory.list(4096).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "journal inventory unavailable",
            )
        })?;
        let mut pending = Vec::new();
        for name in names {
            let Some(raw_id) = name
                .strip_prefix("intent-")
                .and_then(|s| s.strip_suffix(".json"))
            else {
                continue;
            };
            let id = TransactionId::new(raw_id).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "journal inventory identity invalid",
                )
            })?;
            let tx = self.read(&id, ctx)?;
            match self
                .directory
                .read(&format!("terminal-{}.json", id.as_str()), 8 * 1024 * 1024)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "journal terminal unavailable",
                    )
                })? {
                None => pending.push(tx),
                Some(bytes) => {
                    let terminal: DurableRecord = serde_json::from_slice(&bytes).map_err(|_| {
                        failure(ctx, ErrorCode::RecoveryRequired, "journal terminal corrupt")
                    })?;
                    if terminal.schema_version != 1
                        || terminal.transaction_id != id
                        || terminal.operation_id != tx.operation_id
                    {
                        return Err(failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "journal terminal identity mismatch",
                        ));
                    }
                    if terminal.status == TerminalStatus::RecoveryRequired {
                        pending.push(tx);
                    }
                }
            }
        }
        Ok(pending)
    }
    pub fn record_step(
        &self,
        id: &TransactionId,
        receipt: &StepReceipt,
        ctx: &OperationContext,
    ) -> Result<()> {
        ensure_mutating(ctx, self.control())?;
        let tx = self.read(id, ctx)?;
        if !setup_receipt_matches(&tx.intent, receipt) {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "setup receipt belongs to another recipe or idempotency key",
            ));
        }
        if tx.operation_id != ctx.operation_id {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "journal operation mismatch",
            ));
        }
        if let Some(expected) = planned_steps(&tx.intent) {
            if !expected.contains(&receipt.step_id) {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "receipt step is outside journal plan",
                ));
            }
        }
        self.create_idempotent(
            &format!(
                "step-{}-{}-{:?}.json",
                id.as_str(),
                receipt.step_id.as_str(),
                receipt.status
            ),
            receipt,
            ctx,
        )
    }
    pub fn finish(
        &self,
        id: &TransactionId,
        status: TerminalStatus,
        receipts: Vec<StepReceipt>,
        ctx: &OperationContext,
    ) -> Result<DurableRecord> {
        self.finish_record(id, status, receipts, None, false, ctx)
    }
    /// Domain outcomes and receipts share one durable terminal publication.
    pub fn finish_operation(
        &self,
        id: &TransactionId,
        status: TerminalStatus,
        receipts: Vec<StepReceipt>,
        result: &OperationResult,
        ctx: &OperationContext,
    ) -> Result<DurableRecord> {
        let result = checked_operation_result(id, &status, &receipts, result, ctx)?;
        self.finish_record(id, status, receipts, Some(result), false, ctx)
    }
    /// Explicit verified reconciliation preserves the old recovery terminal first.
    pub fn resolve_operation(
        &self,
        id: &TransactionId,
        status: TerminalStatus,
        receipts: Vec<StepReceipt>,
        result: Option<&OperationResult>,
        ctx: &OperationContext,
    ) -> Result<DurableRecord> {
        if status == TerminalStatus::RecoveryRequired {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "resolution must establish a known terminal outcome",
            ));
        }
        let result = result
            .map(|value| checked_operation_result(id, &status, &receipts, value, ctx))
            .transpose()?;
        self.finish_record(id, status, receipts, result, true, ctx)
    }
    pub fn terminal(
        &self,
        id: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<Option<DurableRecord>> {
        let transaction = self.read(id, ctx)?;
        let Some(bytes) = self
            .directory
            .read(&format!("terminal-{}.json", id.as_str()), 8 * 1024 * 1024)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "journal terminal unavailable",
                )
            })?
        else {
            return Ok(None);
        };
        let record: DurableRecord = serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "journal terminal corrupt; original retained",
            )
        })?;
        if record.schema_version != 1
            || record.transaction_id != *id
            || record.operation_id != transaction.operation_id
            || record.operation_result.as_ref().is_some_and(|r| {
                r.operation_id != record.operation_id || r.transaction_id.as_ref() != Some(id)
            })
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "journal terminal identity mismatch",
            ));
        }
        Ok(Some(record))
    }
    /// Bounded read-only journal history for exact ownership/provenance checks.
    /// Every terminal is identity-validated; corrupt records fail the inventory
    /// closed instead of being skipped.
    pub fn history(
        &self,
        ctx: &OperationContext,
    ) -> Result<Vec<(Transaction, Option<DurableRecord>)>> {
        let names = self
            .directory
            .list(4096)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "journal history unavailable"))?;
        let mut records = Vec::new();
        for name in names {
            let Some(raw_id) = name
                .strip_prefix("intent-")
                .and_then(|value| value.strip_suffix(".json"))
            else {
                continue;
            };
            let id = TransactionId::new(raw_id).map_err(|_| {
                failure(ctx, ErrorCode::RecoveryRequired, "journal identity invalid")
            })?;
            let transaction = self.read(&id, ctx)?;
            let terminal = self.terminal(&id, ctx)?;
            records.push((transaction, terminal));
        }
        Ok(records)
    }
    fn finish_record(
        &self,
        id: &TransactionId,
        status: TerminalStatus,
        receipts: Vec<StepReceipt>,
        operation_result: Option<OperationResult>,
        resolving: bool,
        ctx: &OperationContext,
    ) -> Result<DurableRecord> {
        ensure_mutating(ctx, self.control())?;
        let tx = self.read(id, ctx)?;
        if receipts
            .iter()
            .any(|receipt| !setup_receipt_matches(&tx.intent, receipt))
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "setup terminal receipts differ from the approved recipe",
            ));
        }
        if tx.operation_id != ctx.operation_id {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "journal operation mismatch",
            ));
        }
        if status == TerminalStatus::Deferred
            && (!matches!(tx.intent, Intent::Effects { .. }) || operation_result.is_none())
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "deferred completion requires an explicit stopped pending effect result",
            ));
        }
        if matches!(&tx.intent, Intent::Lifecycle { .. }) && operation_result.is_none() {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "lifecycle completion requires durable operation metadata",
            ));
        }
        if matches!(status, TerminalStatus::Committed) {
            let expected = planned_steps(&tx.intent).unwrap_or_default();
            let verified = receipts
                .iter()
                .filter(|r| r.status == StepStatus::Verified)
                .map(|r| r.step_id.clone())
                .collect::<BTreeSet<_>>();
            if expected != verified
                || receipts.len() != verified.len()
                || receipts.iter().any(|r| r.status != StepStatus::Verified)
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "unverified effects cannot be committed",
                ));
            }
            if let Some(actions) = forward_actions(&tx.intent) {
                for action in actions {
                    let receipt = receipts
                        .iter()
                        .find(|r| r.step_id == action.action_id && r.status == StepStatus::Verified)
                        .ok_or_else(|| {
                            failure(ctx, ErrorCode::RecoveryRequired, "effect receipt missing")
                        })?;
                    let _resource = receipt
                        .resources
                        .iter()
                        .find(|r| r.resource_id == action.resource_id)
                        .ok_or_else(|| {
                            failure(
                                ctx,
                                ErrorCode::RecoveryRequired,
                                "effect receipt resource missing",
                            )
                        })?;
                    if action.postconditions.iter().any(|c|matches!(c,Condition::ResourceFingerprint{resource_id,fingerprint} if resource_id==&action.resource_id && receipt.state_fingerprints.get(resource_id)!=Some(fingerprint))) {return Err(failure(ctx,ErrorCode::RecoveryRequired,"verified receipt fingerprint mismatch"));}
                }
            }
        }
        if matches!(status, TerminalStatus::RolledBack)
            && receipts
                .iter()
                .any(|r| !matches!(r.status, StepStatus::Compensated | StepStatus::Prepared))
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "unreconciled effects cannot be rolled back",
            ));
        }
        let name = format!("terminal-{}.json", id.as_str());
        if let Ok(existing) = self.read_json::<DurableRecord>(&name, ctx) {
            if existing.schema_version == 1
                && existing.transaction_id == *id
                && existing.operation_id == ctx.operation_id
                && existing.status == status
                && existing.receipts == receipts
                && existing.operation_result == operation_result
            {
                self.create_idempotent(&name, &existing, ctx)?;
                return Ok(existing);
            }
            if resolving && existing.status == TerminalStatus::RecoveryRequired {
                let record = DurableRecord {
                    schema_version: 1,
                    transaction_id: id.clone(),
                    operation_id: ctx.operation_id.clone(),
                    status,
                    completed_at: UnixMillis(self.control.now_ms()),
                    receipts,
                    operation_result,
                };
                self.publish_resolution(&name, &existing, &record, ctx)?;
                return Ok(record);
            }
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "journal already has a different terminal result",
            ));
        }
        let record = DurableRecord {
            schema_version: 1,
            transaction_id: id.clone(),
            operation_id: ctx.operation_id.clone(),
            status,
            completed_at: UnixMillis(self.control.now_ms()),
            receipts,
            operation_result,
        };
        self.create_idempotent(&name, &record, ctx)?;
        self.event(
            hton_logging::Component::Journal,
            hton_logging::EventCode::OperationFinished,
            ctx,
        );
        Ok(record)
    }
    pub fn stage_plan(
        &self,
        plan_id: PlanId,
        plan: &EffectPlan,
        ctx: &OperationContext,
    ) -> Result<StoredPlan> {
        ensure_mutating(ctx, self.control())?;
        if plan.expires_at.0 <= self.control.now_ms() {
            return Err(failure(
                ctx,
                ErrorCode::PlanExpired,
                "cannot stage expired plan",
            ));
        }
        let stored = StoredPlan {
            schema_version: 1,
            plan_id: plan_id.clone(),
            actor_id: ctx.actor.actor_id.clone(),
            request_id: ctx.actor.request_id.clone(),
            plan: plan.clone(),
            staged_at: UnixMillis(self.control.now_ms()),
        };
        let name = format!("plan-{}.json", plan_id.as_str());
        if let Ok(old) = self.read_json::<StoredPlan>(&name, ctx) {
            if old.schema_version == 1
                && old.plan_id == plan_id
                && old.actor_id == stored.actor_id
                && old.request_id == stored.request_id
                && old.plan == stored.plan
            {
                self.create_idempotent(&name, &old, ctx)?;
                return Ok(old);
            }
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "plan identity already bound",
            ));
        }
        self.create_idempotent(&name, &stored, ctx)?;
        Ok(stored)
    }
    pub fn read_plan(
        &self,
        id: &PlanId,
        actor: &ActorContext,
        ctx: &OperationContext,
    ) -> Result<StoredPlan> {
        let stored = self.read_plan_metadata(id, actor, ctx)?;
        if stored.plan.expires_at.0 <= self.control.now_ms() {
            return Err(failure(ctx, ErrorCode::PlanExpired, "stored plan expired"));
        }
        Ok(stored)
    }
    pub fn read_plan_metadata(
        &self,
        id: &PlanId,
        actor: &ActorContext,
        ctx: &OperationContext,
    ) -> Result<StoredPlan> {
        let stored: StoredPlan = self.read_json(&format!("plan-{}.json", id.as_str()), ctx)?;
        if stored.schema_version != 1 || &stored.plan_id != id {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "stored plan is invalid",
            ));
        }
        if actor.actor_id != ctx.actor.actor_id || stored.actor_id != actor.actor_id {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "plan belongs to another actor",
            ));
        }
        Ok(stored)
    }
    /// Observe only. Caller must explicitly plan any compensation through its owner.
    pub fn recover(
        &self,
        tx: &Transaction,
        observations: &RuntimeObservation,
        run_intent: RunIntent,
        approvals: &BTreeSet<ApprovalId>,
    ) -> RecoveryResult {
        let mut uncertain = BTreeSet::new();
        let mut after = false;
        if let Intent::Effects { plan } = &tx.intent {
            for action in &plan.actions {
                let granted = action.preconditions.iter().all(|c| match c {
                    Condition::Approval { approval_id } => approvals.contains(approval_id),
                    _ => true,
                });
                let observed = observations.resources.get(&action.resource_id);
                match observed {
                    Some(o) if granted && o.valid_until.0 > self.control.now_ms() => {
                        let before = plan.resource_before.get(&action.resource_id)
                            == Some(&o.identity.fingerprint);
                        let target = match action.kind {
                            ActionKind::Start | ActionKind::Restart => Some(ObservedState::Running),
                            ActionKind::Stop => Some(ObservedState::Stopped),
                            _ => None,
                        };
                        if !before
                            || o.state == ObservedState::Unknown
                            || o.state == ObservedState::Failed
                            || (run_intent == RunIntent::Stopped
                                && o.state != ObservedState::Stopped)
                        {
                            uncertain.insert(action.resource_id.clone());
                        } else if target == Some(o.state) {
                            after = true;
                        }
                    }
                    _ => {
                        uncertain.insert(action.resource_id.clone());
                    }
                }
            }
        } else {
            return RecoveryResult {
                transaction_id: tx.transaction_id.clone(),
                disposition: RecoveryDisposition::RecoveryRequired,
                uncertain_resources: uncertain,
                replay_permitted: false,
            };
        }
        let disposition = if !uncertain.is_empty() {
            RecoveryDisposition::RecoveryRequired
        } else if run_intent == RunIntent::Stopped {
            RecoveryDisposition::StoppedWithArtifacts
        } else if after {
            RecoveryDisposition::EffectsObservedRequireVerification
        } else {
            RecoveryDisposition::NoEffectsObserved
        };
        RecoveryResult {
            transaction_id: tx.transaction_id.clone(),
            disposition,
            uncertain_resources: uncertain,
            replay_permitted: false,
        }
    }
    fn publish_resolution(
        &self,
        name: &str,
        previous: &DurableRecord,
        record: &DurableRecord,
        ctx: &OperationContext,
    ) -> Result<()> {
        let bytes = serde_json::to_vec(record)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "resolution encoding failed"))?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "resolution exceeds journal bound",
            ));
        }
        let lock = self
            .directory
            .lock(deadline(ctx, self.control())?)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "resolution lock unavailable"))?;
        let original = lock
            .read(name, 8 * 1024 * 1024)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "recovery terminal unavailable",
                )
            })?
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "recovery terminal disappeared",
                )
            })?;
        let parsed: DurableRecord = serde_json::from_slice(&original).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "recovery terminal corrupt",
            )
        })?;
        if &parsed != previous || parsed.status != TerminalStatus::RecoveryRequired {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "recovery terminal changed before resolution",
            ));
        }
        let retained = format!(
            "terminal-prior-{}-{:x}.json",
            previous.transaction_id,
            Sha256::digest(&original)
        );
        match lock.compare_exchange(&retained, None, &original) {
            Ok(()) => (),
            Err(hton_storage::Error::Conflict) => {
                let saved = lock.read(&retained, 8 * 1024 * 1024).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "prior terminal unavailable",
                    )
                })?;
                if saved.as_deref() != Some(original.as_slice()) {
                    return Err(failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "prior terminal fingerprint conflict",
                    ));
                }
                lock.compare_exchange(&retained, Some(&original), &original)
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "prior terminal durability uncertain",
                        )
                    })?;
            }
            Err(_) => {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "cannot durably retain recovery terminal",
                ));
            }
        }
        lock.compare_exchange(name, Some(&original), &bytes)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "resolution publication uncertain",
                )
            })
    }
    fn create_idempotent<T: Serialize + serde::de::DeserializeOwned + PartialEq>(
        &self,
        name: &str,
        value: &T,
        ctx: &OperationContext,
    ) -> Result<()> {
        let bytes = serde_json::to_vec(value).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "cannot encode journal record",
            )
        })?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(failure(
                ctx,
                ErrorCode::IoUnavailable,
                "journal record exceeds bound",
            ));
        }
        let lock = self
            .directory
            .lock(deadline(ctx, self.control())?)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "journal lock unavailable"))?;
        match lock.compare_exchange(name, None, &bytes) {
            Ok(()) => Ok(()),
            Err(hton_storage::Error::Conflict) => {
                let old: T = self.read_json(name, ctx)?;
                if &old != value {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "journal record already exists with different content",
                    ));
                }
                let actual = lock
                    .read(name, 8 * 1024 * 1024)
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "journal existing record unavailable",
                        )
                    })?
                    .ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "journal existing record disappeared",
                        )
                    })?;
                lock.compare_exchange(name, Some(&actual), &bytes)
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "journal durability uncertain; effects prohibited",
                        )
                    })
            }
            Err(_) => Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "journal publication durability uncertain; effects prohibited",
            )),
        }
    }
    fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
        ctx: &OperationContext,
    ) -> Result<T> {
        let bytes = self
            .directory
            .read(name, 8 * 1024 * 1024)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "journal record unavailable"))?
            .ok_or_else(|| failure(ctx, ErrorCode::IoUnavailable, "journal record missing"))?;
        serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "journal record corrupt; original retained",
            )
        })
    }
}

fn forward_actions(intent: &Intent) -> Option<&[Action]> {
    match intent {
        Intent::Effects { plan } => Some(&plan.actions),
        Intent::Lifecycle { plan } => Some(&plan.actions),
        Intent::Update { plan } => Some(&plan.actions),
        _ => None,
    }
}
fn planned_steps(intent: &Intent) -> Option<BTreeSet<StepId>> {
    if let Some(actions) = forward_actions(intent) {
        Some(actions.iter().map(|a| a.action_id.clone()).collect())
    } else if let Intent::Setup { plan } = intent {
        Some(plan.steps.iter().map(|s| s.step_id.clone()).collect())
    } else {
        None
    }
}

pub fn deadline(ctx: &OperationContext, control: &dyn OperationControl) -> Result<Instant> {
    ensure_mutating(ctx, control)?;
    Ok(Instant::now()
        + Duration::from_millis(ctx.deadline.0.saturating_sub(control.now_ms()).min(30_000)))
}

fn intent_id(intent: &Intent, ctx: &OperationContext) -> Result<TransactionId> {
    let identity_bytes = serde_json::to_vec(&(ctx.operation_id.clone(), intent)).map_err(|_| {
        failure(
            ctx,
            ErrorCode::IoUnavailable,
            "journal identity encoding failed",
        )
    })?;
    let id =
        TransactionId::new(format!("tx-{:x}", Sha256::digest(&identity_bytes))).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "invalid transaction identity",
            )
        })?;
    Ok(id)
}

fn checked_operation_result(
    id: &TransactionId,
    status: &TerminalStatus,
    receipts: &[StepReceipt],
    result: &OperationResult,
    ctx: &OperationContext,
) -> Result<OperationResult> {
    let matching_status = matches!(
        (status, result.status),
        (TerminalStatus::Committed, OperationStatus::Succeeded)
            | (TerminalStatus::Partial, OperationStatus::Partial)
            | (TerminalStatus::Failed, OperationStatus::Failed)
            | (TerminalStatus::Deferred, OperationStatus::Succeeded)
            | (
                TerminalStatus::RecoveryRequired,
                OperationStatus::RecoveryRequired
            )
            | (
                TerminalStatus::RolledBack,
                OperationStatus::Failed | OperationStatus::Partial
            )
    );
    if result.operation_id != ctx.operation_id
        || result
            .transaction_id
            .as_ref()
            .is_some_and(|value| value != id)
        || !matching_status
        || (*status == TerminalStatus::Deferred
            && (result.run_intent != RunIntent::Stopped
                || result.pending_bundle.is_none()
                || !receipts.is_empty()
                || !result.member_results.is_empty()))
        || (result.status == OperationStatus::Succeeded
            && (!result.survivors.is_empty()
                || result
                    .member_results
                    .iter()
                    .any(|m| m.status != OperationStatus::Succeeded || m.error.is_some())
                || result.issues.iter().any(|i| i.severity == Severity::Error)))
        || (result.status != OperationStatus::RecoveryRequired
            && result
                .member_results
                .iter()
                .any(|m| m.status == OperationStatus::RecoveryRequired))
    {
        return Err(failure(
            ctx,
            ErrorCode::OperationConflict,
            "operation result does not match its terminal status",
        ));
    }
    let mut result = result.clone();
    result.transaction_id = Some(id.clone());
    Ok(result)
}

fn setup_receipt_matches(intent: &Intent, receipt: &StepReceipt) -> bool {
    match intent {
        Intent::Setup { plan } => plan.steps.iter().any(|step| {
            step.step_id == receipt.step_id && step.idempotency_key == receipt.idempotency_key
        }),
        _ => true,
    }
}
