//! Logical encrypted archives; restore never executes archive paths or running intent.
use crate::{
    journal::{Intent, JournalStore, ProtectedOriginal, TerminalStatus, ensure_mutating, failure},
    lock,
};
use hton_contracts::{config::*, operations::*};
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArchiveKind {
    Configuration,
    FullDurable,
}
/// No cache, socket, host identity key, code, arbitrary path or executable entry exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArchiveCategory {
    Configuration,
    SecretVersion,
    Registry,
    Journal,
    AcceptedBundle,
    PolicyFence,
    Certificate,
    AcmeState,
    RequiredImage,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArchiveProtection {
    Private0600,
    Metadata0644,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArchiveScope {
    Config,
    Work,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveLocation {
    pub scope: ArchiveScope,
    pub components: Vec<String>,
}
impl ArchiveLocation {
    pub fn valid_for(&self, category: ArchiveCategory) -> bool {
        if self.components.is_empty()
            || self.components.len() > 16
            || self.components.iter().any(|part| {
                part.is_empty()
                    || part.len() > 255
                    || part == "."
                    || part == ".."
                    || !part
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            })
        {
            return false;
        }
        let first = self.components[0].as_str();
        let last = self.components.last().map(String::as_str).unwrap_or("");
        match (self.scope, category) {
            (ArchiveScope::Config, ArchiveCategory::Configuration) => {
                (self.components.len() == 1
                    && matches!(first, "hton.yaml" | "backup.yaml" | "hton.deprecated.yml"))
                    || (self.components.len() == 2
                        && matches!(first, "modules" | "docker")
                        && (last.ends_with(".yaml") || last.ends_with(".yml")))
            }
            (ArchiveScope::Config, ArchiveCategory::SecretVersion) => {
                self.components.len() == 2
                    && first == "secrets"
                    && last.ends_with(".json")
                    && (last.starts_with("secret-")
                        || last.starts_with("current-")
                        || last.starts_with("request-"))
            }
            (ArchiveScope::Work, ArchiveCategory::Registry) => {
                self.components.len() == 1 && first == "registry.json"
            }
            (ArchiveScope::Work, ArchiveCategory::Journal) => {
                self.components.len() == 2 && first == "journal" && last.ends_with(".json")
            }
            (ArchiveScope::Work, ArchiveCategory::AcceptedBundle) => {
                self.components.len() >= 3 && first == "bundles"
            }
            (ArchiveScope::Work, ArchiveCategory::PolicyFence) => {
                (self.components.len() == 2 && first == "runtime" && last == "state.json")
                    || (self.components.len() == 2
                        && first == "identities"
                        && last == "bindings.json")
            }
            (ArchiveScope::Work, ArchiveCategory::Certificate) => {
                self.components.len() == 2 && first == "certificates" && last.ends_with(".json")
            }
            (ArchiveScope::Work, ArchiveCategory::AcmeState) => {
                self.components.len() >= 2
                    && first == "acme"
                    && !last.ends_with(".sh")
                    && !last.ends_with(".env")
            }
            (ArchiveScope::Work, ArchiveCategory::RequiredImage) => {
                self.components.len() == 2 && first == "images" && last.ends_with(".tar")
            }
            _ => false,
        }
    }
    pub fn object_id(&self) -> std::result::Result<ObjectId, InvalidIdentifier> {
        let bytes = serde_json::to_vec(self).map_err(|_| InvalidIdentifier)?;
        ObjectId::new(format!(
            "archive.{}",
            &crate::config::digest(&bytes).as_str()[7..]
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveEntry {
    pub object_id: ObjectId,
    pub location: ArchiveLocation,
    pub category: ArchiveCategory,
    pub digest: Sha256Digest,
    pub size: u64,
    pub protection: ArchiveProtection,
    pub format_version: FormatVersion,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArchiveManifest {
    pub schema_version: FormatVersion,
    pub instance_id: InstanceId,
    pub kind: ArchiveKind,
    pub read_set: ReadSet,
    pub entries: Vec<ArchiveEntry>,
    pub credential_fence: u64,
    pub created_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncryptedArchiveMetadata {
    pub archive_id: ObjectId,
    pub manifest_digest: Sha256Digest,
    pub ciphertext_digest: Sha256Digest,
    pub encrypted: bool,
    pub size: u64,
    pub created_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupRequest {
    pub archive_id: ObjectId,
    pub kind: ArchiveKind,
    pub recipients: BTreeSet<ObjectId>,
    pub keep: u16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestorePlan {
    pub archive_id: ObjectId,
    pub manifest_digest: Sha256Digest,
    pub manifest: ArchiveManifest,
    pub expected_objects: BTreeMap<ObjectId, Option<Sha256Digest>>,
    pub approval_id: ApprovalId,
}
#[derive(Clone, Debug)]
pub struct RestoreResult {
    pub transaction_id: TransactionId,
    pub restored: Vec<ObjectId>,
    pub run_intent: RunIntent,
    pub requires_explicit_up: bool,
}
#[derive(Clone, Debug)]
pub struct RestoreFence {
    pub maintenance_id: hton_runtime_contract::LeaseId,
    pub stopped_epoch: RunEpoch,
    pub credential_fence: u64,
}

/// Owns a coherent immutable inventory and only its fixed logical object mapping.
/// read_chunk is bounded/streamable; no whole-archive allocation on a small VPS.
pub trait BackupInventory: Send + Sync {
    fn snapshot(&self, kind: ArchiveKind, ctx: &OperationContext) -> Result<ArchiveManifest>;
    fn current(
        &self,
        objects: &[ObjectId],
        ctx: &OperationContext,
    ) -> Result<BTreeMap<ObjectId, Option<Sha256Digest>>>;
    fn read_chunk(
        &self,
        manifest: &ArchiveManifest,
        entry: &ArchiveEntry,
        offset: u64,
        max_bytes: usize,
        ctx: &OperationContext,
    ) -> Result<Vec<u8>>;
    fn validate_restore(&self, manifest: &ArchiveManifest, ctx: &OperationContext) -> Result<()>;
    fn originals(
        &self,
        plan: &RestorePlan,
        ctx: &OperationContext,
    ) -> Result<Vec<ProtectedOriginal>>;
    fn restore_object(
        &self,
        plan: &RestorePlan,
        entry: &ArchiveEntry,
        crypto: &dyn ArchiveCrypto,
        guard: &RestoreFence,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<Sha256Digest>;
}
/// Qualified age/authenticated archive binding. Encrypt streams and hashes every
/// inventory entry, stores a protected completed archive atomically, and retains
/// transaction checkpoints. Decrypt rejects duplicates/links/traversal/devices,
/// unknown logical entries and size/hash mismatches before any restore publication.
/// Recipient/decryption identities resolve from trusted handles outside the archive.
pub trait ArchiveCrypto: Send + Sync {
    fn encrypt(
        &self,
        request: &BackupRequest,
        manifest: &ArchiveManifest,
        inventory: &dyn BackupInventory,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<EncryptedArchiveMetadata>;
    fn inspect(
        &self,
        id: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<(EncryptedArchiveMetadata, ArchiveManifest)>;
    fn read_chunk(
        &self,
        id: &ObjectId,
        entry: &ArchiveEntry,
        offset: u64,
        max_bytes: usize,
        ctx: &OperationContext,
    ) -> Result<Vec<u8>>;
    fn completed(
        &self,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<Option<EncryptedArchiveMetadata>>;
    fn retain(&self, keep: u16, protected: &ObjectId, ctx: &OperationContext) -> Result<()>;
}
pub trait RestoreRuntime: Send + Sync {
    /// Verify full shutdown, then atomically reserve durable runtime maintenance
    /// by CAS before restoration writes. A racing up refuses reservation without
    /// writing restore data; every startup/promotion respects the reservation.
    fn stop_for_restore(&self, ctx: &OperationContext) -> Result<(OperationResult, RestoreFence)>;
    /// Verify durable committed restore terminal and exact original reservation
    /// before releasing it. Idempotent metadata retry never performs restore writes.
    fn release_completed(
        &self,
        plan: &RestorePlan,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<()>;
    fn verify_approval(&self, plan: &RestorePlan, ctx: &OperationContext) -> Result<bool>;
    /// Publish accepted/restored inventory references while preserving CURRENT stopped
    /// epoch, monotonic fences and disabled jobs. Never deserialize archived run intent.
    fn publish_restored(
        &self,
        plan: &RestorePlan,
        guard: &RestoreFence,
        ctx: &OperationContext,
    ) -> Result<()>;
    fn verify_stopped(&self, guard: &RestoreFence, ctx: &OperationContext) -> Result<bool>;
}
pub struct BackupCoordinator<'a> {
    pub journal: &'a JournalStore,
    pub config_lock: &'a SecureDir,
    pub instance_id: InstanceId,
    pub inventory: &'a dyn BackupInventory,
    pub crypto: &'a dyn ArchiveCrypto,
    pub runtime: &'a dyn RestoreRuntime,
}
impl BackupCoordinator<'_> {
    pub fn backup(
        &self,
        request: &BackupRequest,
        ctx: &OperationContext,
    ) -> Result<EncryptedArchiveMetadata> {
        ensure_mutating(ctx, self.journal.control())?;
        if request.recipients.is_empty()
            || request.recipients.len() > 32
            || request.keep == 0
            || request.keep > 100
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "archive recipient or retention bounds invalid",
            ));
        }
        let digest = crate::config::digest(&serde_json::to_vec(request).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "backup request encoding failed",
            )
        })?);
        let intent = Intent::Backup {
            request_digest: digest,
        };
        if let Some(tx) = self.journal.lookup_intent(&intent, ctx)? {
            let record = self
                .journal
                .terminal(&tx.transaction_id, ctx)?
                .ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "interrupted backup requires reconciliation",
                    )
                })?;
            if record.status != TerminalStatus::Committed {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "backup requires reconciliation",
                ));
            }
            return self
                .crypto
                .completed(&tx.transaction_id, ctx)?
                .ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "completed encrypted archive is missing",
                    )
                });
        }
        let tx = self.journal.prepare_intent(intent, vec![], ctx)?;
        if let Some(record) = self.journal.terminal(&tx, ctx)? {
            if record.status != TerminalStatus::Committed {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "backup requires reconciliation",
                ));
            }
            return self.crypto.completed(&tx, ctx)?.ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "completed encrypted archive is missing",
                )
            });
        }
        let guard = lock::acquire(
            self.config_lock,
            &self.instance_id,
            ctx,
            self.journal.control(),
        )?;
        let snapshot = self.inventory.snapshot(request.kind, ctx)?;
        validate_manifest(&snapshot, ctx)?;
        if snapshot.instance_id != self.instance_id || snapshot.kind != request.kind {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "backup inventory scope mismatch",
            ));
        }
        let encrypted = self
            .crypto
            .encrypt(request, &snapshot, self.inventory, &tx, ctx)?;
        let digest = crate::config::digest(&serde_json::to_vec(&snapshot).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "archive manifest encoding failed",
            )
        })?);
        if !encrypted.encrypted
            || encrypted.archive_id != request.archive_id
            || encrypted.manifest_digest != digest
            || encrypted.size == 0
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "archive encryption or manifest binding is unverified",
            ));
        }
        ensure_mutating(ctx, self.journal.control())?;
        self.journal
            .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
        self.crypto
            .retain(request.keep, &encrypted.archive_id, ctx)?;
        drop(guard);
        Ok(encrypted)
    }
    pub fn plan_restore(
        &self,
        archive_id: ObjectId,
        approval_id: ApprovalId,
        ctx: &OperationContext,
    ) -> Result<RestorePlan> {
        let (metadata, manifest) = self.crypto.inspect(&archive_id, ctx)?;
        validate_manifest(&manifest, ctx)?;
        let digest = crate::config::digest(&serde_json::to_vec(&manifest).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "restore manifest encoding failed",
            )
        })?);
        if !metadata.encrypted
            || metadata.archive_id != archive_id
            || metadata.manifest_digest != digest
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "restore archive authentication failed",
            ));
        }
        self.inventory.validate_restore(&manifest, ctx)?;
        let objects = manifest
            .entries
            .iter()
            .map(|e| e.object_id.clone())
            .collect::<Vec<_>>();
        Ok(RestorePlan {
            archive_id,
            manifest_digest: digest,
            manifest,
            expected_objects: self.inventory.current(&objects, ctx)?,
            approval_id,
        })
    }
    pub fn restore(&self, plan: &RestorePlan, ctx: &OperationContext) -> Result<RestoreResult> {
        ensure_mutating(ctx, self.journal.control())?;
        if !ctx.actor.local_approval_refs.contains(&plan.approval_id)
            || !self.runtime.verify_approval(plan, ctx)?
        {
            return Err(failure(
                ctx,
                ErrorCode::ApprovalRequired,
                "restore requires exact local archive approval",
            ));
        }
        let request_digest = crate::config::digest(&serde_json::to_vec(plan).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "restore plan encoding failed",
            )
        })?);
        let intent = Intent::Restore { request_digest };
        if let Some(tx) = self.journal.lookup_intent(&intent, ctx)? {
            if self
                .journal
                .terminal(&tx.transaction_id, ctx)?
                .is_some_and(|record| record.status == TerminalStatus::Committed)
            {
                self.runtime
                    .release_completed(plan, &tx.transaction_id, ctx)?;
                return Ok(RestoreResult {
                    transaction_id: tx.transaction_id,
                    restored: plan
                        .manifest
                        .entries
                        .iter()
                        .map(|e| e.object_id.clone())
                        .collect(),
                    run_intent: RunIntent::Stopped,
                    requires_explicit_up: true,
                });
            }
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "interrupted restore requires explicit reconciliation",
            ));
        }
        let fresh = self.plan_restore(plan.archive_id.clone(), plan.approval_id.clone(), ctx)?;
        if &fresh != plan {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "restore inventory changed since preview",
            ));
        }
        let tx = self
            .journal
            .prepare_intent(intent, self.inventory.originals(plan, ctx)?, ctx)?;
        // Host stop is independent of the preparation lock and precedes ALL writes.
        let (stopped, fence) = self.runtime.stop_for_restore(ctx)?;
        if stopped.status != OperationStatus::Succeeded
            || stopped.run_intent != RunIntent::Stopped
            || !stopped.survivors.is_empty()
            || plan.manifest.credential_fence < fence.credential_fence
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "restore requires verified shutdown and compatible credential fences",
            ));
        }
        let lock = lock::acquire(
            self.config_lock,
            &self.instance_id,
            ctx,
            self.journal.control(),
        )?;
        let current = self.inventory.current(
            &plan
                .manifest
                .entries
                .iter()
                .map(|e| e.object_id.clone())
                .collect::<Vec<_>>(),
            ctx,
        )?;
        if current != plan.expected_objects {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "restore objects changed before publication",
            ));
        }
        let mut restored = vec![];
        for entry in &plan.manifest.entries {
            ensure_mutating(ctx, self.journal.control())?;
            if !self.runtime.verify_stopped(&fence, ctx)? {
                return Err(failure(
                    ctx,
                    ErrorCode::RuntimeStopped,
                    "restore stop fence changed",
                ));
            }
            if self
                .inventory
                .restore_object(plan, entry, self.crypto, &fence, &tx, ctx)?
                != entry.digest
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "restored object fingerprint differs from authenticated archive",
                ));
            }
            restored.push(entry.object_id.clone());
        }
        self.runtime.publish_restored(plan, &fence, ctx)?;
        drop(lock);
        if !self.runtime.verify_stopped(&fence, ctx)? {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "restore cannot confirm stopped state",
            ));
        }
        self.journal
            .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
        self.runtime.release_completed(plan, &tx, ctx)?;
        Ok(RestoreResult {
            transaction_id: tx,
            restored,
            run_intent: RunIntent::Stopped,
            requires_explicit_up: true,
        })
    }
}
fn validate_manifest(manifest: &ArchiveManifest, ctx: &OperationContext) -> Result<()> {
    if manifest.schema_version != FormatVersion(1)
        || manifest.read_set.coherence != ReadCoherence::Coherent
        || manifest.entries.is_empty()
        || manifest.entries.len() > 4096
    {
        return Err(failure(
            ctx,
            ErrorCode::ConfigUnsupportedVersion,
            "archive schema or coherent inventory is invalid",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut total = 0u64;
    for e in &manifest.entries {
        total = total
            .checked_add(e.size)
            .ok_or_else(|| failure(ctx, ErrorCode::OperationDenied, "archive size overflow"))?;
        if !e.location.valid_for(e.category)
            || e.location.object_id().ok().as_ref() != Some(&e.object_id)
            || !ids.insert(e.object_id.clone())
            || e.size > 8 * 1024 * 1024 * 1024
            || total > 32 * 1024 * 1024 * 1024
            || e.format_version.0 == 0
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "archive object or size bounds invalid",
            ));
        }
        if e.category != ArchiveCategory::Configuration
            && e.protection != ArchiveProtection::Private0600
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "private durable archive data has unsafe permissions",
            ));
        }
        if manifest.kind == ArchiveKind::Configuration
            && !matches!(
                e.category,
                ArchiveCategory::Configuration
                    | ArchiveCategory::SecretVersion
                    | ArchiveCategory::PolicyFence
            )
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "configuration archive contains working-state objects",
            ));
        }
    }
    Ok(())
}
