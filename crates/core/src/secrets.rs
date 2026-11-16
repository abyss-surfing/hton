//! Protected immutable secret versions; deliberate disclosure is separate from metadata.
use crate::journal::{OperationControl, deadline, ensure_mutating, failure};
use hton_contracts::operations::*;
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fmt, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretRef {
    pub secret_id: SecretId,
    pub version: SecretVersion,
}
impl SecretRef {
    pub fn from_yaml(
        reference: &str,
        version: SecretVersion,
    ) -> std::result::Result<Self, InvalidIdentifier> {
        if reference.len() > 128 {
            return Err(InvalidIdentifier);
        }
        let parts: Vec<_> = reference.split('/').collect();
        if parts.len() < 2
            || parts.iter().any(|p| {
                p.is_empty()
                    || !p.as_bytes()[0].is_ascii_lowercase()
                    || !p
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
        {
            return Err(InvalidIdentifier);
        }
        Ok(Self {
            secret_id: SecretId::new(parts.join("."))?,
            version,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretKind {
    RandomToken,
    UuidV4,
    Opaque,
}
#[derive(Clone, Debug)]
pub struct ConsumerGrant {
    pub consumer: ServiceId,
    pub scope: ScopeId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ReplacementPolicy {
    Create,
    Reuse {
        reference: SecretRef,
    },
    Rotate {
        previous: SecretRef,
        confirmed: bool,
    },
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SecretRecord {
    schema_version: u32,
    reference: SecretRef,
    scope: ScopeId,
    kind: SecretKind,
    consumers: BTreeSet<ServiceId>,
    created_at: UnixMillis,
    material: Vec<u8>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GenerationIntent {
    schema_version: u32,
    request_id: RequestId,
    actor_id: ActorId,
    reference: SecretRef,
    scope: ScopeId,
    kind: SecretKind,
    consumers: BTreeSet<ServiceId>,
    policy: ReplacementPolicy,
    material_digest: Option<Sha256Digest>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CurrentBinding {
    schema_version: u32,
    reference: SecretRef,
}
pub struct ProtectedMaterial(Vec<u8>);
impl ProtectedMaterial {
    pub fn from_input(bytes: Vec<u8>) -> std::result::Result<Self, InvalidIdentifier> {
        if bytes.is_empty() || bytes.len() > 262_144 {
            return Err(InvalidIdentifier);
        }
        Ok(Self(bytes))
    }
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Debug for ProtectedMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProtectedMaterial([redacted])")
    }
}
pub struct DisclosureHandle(ProtectedMaterial);
impl DisclosureHandle {
    pub fn disclose(self) -> ProtectedMaterial {
        self.0
    }
}
impl fmt::Debug for DisclosureHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DisclosureHandle([redacted])")
    }
}
#[derive(Debug)]
pub struct GeneratedSecret {
    pub reference: SecretRef,
    pub disclosure: Option<DisclosureHandle>,
    pub created: bool,
}
pub struct SecretStore {
    directory: Arc<SecureDir>,
    control: Arc<dyn OperationControl>,
}
impl SecretStore {
    pub fn new(directory: Arc<SecureDir>, control: Arc<dyn OperationControl>) -> Self {
        Self { directory, control }
    }
    pub fn validate_archive_record(name: &str, bytes: &[u8], ctx: &OperationContext) -> Result<()> {
        let invalid = || {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "archived secret record is invalid or mismatched",
            )
        };
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(invalid());
        }
        if name.starts_with("secret-") {
            let record: SecretRecord = serde_json::from_slice(bytes).map_err(|_| invalid())?;
            if record.schema_version != 1
                || filename(&record.reference) != name
                || record.material.is_empty()
                || record.material.len() > 262144
                || record.consumers.is_empty()
                || record.consumers.len() > 64
            {
                return Err(invalid());
            }
        } else if name.starts_with("current-") {
            let binding: CurrentBinding = serde_json::from_slice(bytes).map_err(|_| invalid())?;
            if binding.schema_version != 1
                || format!("current-{}.json", binding.reference.secret_id) != name
            {
                return Err(invalid());
            }
        } else if name.starts_with("request-") {
            let intent: GenerationIntent = serde_json::from_slice(bytes).map_err(|_| invalid())?;
            let key = serde_json::to_vec(&(
                intent.request_id.clone(),
                intent.reference.secret_id.clone(),
            ))
            .map_err(|_| invalid())?;
            if intent.schema_version != 1
                || format!("request-{:x}.json", Sha256::digest(&key)) != name
                || intent.consumers.is_empty()
                || intent.consumers.len() > 64
                || (intent.kind == SecretKind::Opaque) != intent.material_digest.is_some()
            {
                return Err(invalid());
            }
            if let ReplacementPolicy::Rotate { previous, .. }
            | ReplacementPolicy::Reuse {
                reference: previous,
            } = &intent.policy
            {
                if previous.secret_id != intent.reference.secret_id {
                    return Err(invalid());
                }
            }
        } else {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn generate(
        &self,
        id: SecretId,
        scope: ScopeId,
        kind: SecretKind,
        consumers: BTreeSet<ServiceId>,
        policy: ReplacementPolicy,
        ctx: &OperationContext,
    ) -> Result<GeneratedSecret> {
        if kind == SecretKind::Opaque {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "opaque material must be explicitly imported",
            ));
        }
        self.store(id, scope, kind, consumers, policy, None, ctx)
    }
    pub fn import(
        &self,
        id: SecretId,
        scope: ScopeId,
        consumers: BTreeSet<ServiceId>,
        policy: ReplacementPolicy,
        material: ProtectedMaterial,
        ctx: &OperationContext,
    ) -> Result<GeneratedSecret> {
        self.store(
            id,
            scope,
            SecretKind::Opaque,
            consumers,
            policy,
            Some(material),
            ctx,
        )
    }
    fn store(
        &self,
        id: SecretId,
        scope: ScopeId,
        kind: SecretKind,
        consumers: BTreeSet<ServiceId>,
        policy: ReplacementPolicy,
        supplied: Option<ProtectedMaterial>,
        ctx: &OperationContext,
    ) -> Result<GeneratedSecret> {
        ensure_mutating(ctx, self.control.as_ref())?;
        let material_digest = supplied
            .as_ref()
            .map(|material| crate::config::digest(material.expose()));
        if consumers.is_empty() || consumers.len() > 64 {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "secret requires bounded consumer grant",
            ));
        }
        match &policy {
            ReplacementPolicy::Reuse { reference } => {
                let old = self.read_record(reference, ctx)?;
                if reference.secret_id != id
                    || old.scope != scope
                    || old.kind != kind
                    || old.consumers != consumers
                    || supplied
                        .as_ref()
                        .is_some_and(|material| material.expose() != old.material)
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "secret reuse scope mismatch",
                    ));
                }
                return Ok(GeneratedSecret {
                    reference: reference.clone(),
                    disclosure: None,
                    created: false,
                });
            }
            ReplacementPolicy::Rotate {
                previous,
                confirmed,
            } => {
                if !confirmed {
                    return Err(failure(
                        ctx,
                        ErrorCode::ApprovalRequired,
                        "secret replacement requires explicit confirmation",
                    ));
                }
                let old = self.read_record(previous, ctx)?;
                if previous.secret_id != id || old.scope != scope || old.kind != kind {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "secret replacement scope mismatch",
                    ));
                }
            }
            ReplacementPolicy::Create => {}
        }
        let lock = self
            .directory
            .lock(deadline(ctx, self.control.as_ref())?)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "secret store lock unavailable",
                )
            })?;
        let request_key =
            serde_json::to_vec(&(ctx.actor.request_id.clone(), id.clone())).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "secret request identity encoding failed",
                )
            })?;
        let request_name = format!("request-{:x}.json", Sha256::digest(&request_key));
        if let Some(bytes) = lock.read(&request_name, 16 * 1024).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "secret request metadata unavailable",
            )
        })? {
            let previous: GenerationIntent = serde_json::from_slice(&bytes).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "secret generation intent corrupt",
                )
            })?;
            if previous.schema_version != 1
                || previous.actor_id != ctx.actor.actor_id
                || previous.reference.secret_id != id
                || previous.scope != scope
                || previous.kind != kind
                || previous.consumers != consumers
                || previous.policy != policy
                || previous.material_digest != material_digest
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "secret request identity reused with changed inputs",
                ));
            }
            self.read_record(&previous.reference, ctx).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "secret generation interrupted; reconcile retained intent",
                )
            })?;
            let retained = lock
                .read(&filename(&previous.reference), 2 * 1024 * 1024)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "retained secret version unavailable",
                    )
                })?
                .ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "retained secret version disappeared",
                    )
                })?;
            lock.compare_exchange(&filename(&previous.reference), Some(&retained), &retained)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "retained secret durability uncertain",
                    )
                })?;
            self.select_current(&lock, &previous.reference, &previous.policy, ctx)?;
            return Ok(GeneratedSecret {
                reference: previous.reference,
                disclosure: None,
                created: false,
            });
        }
        let selected = self.current_version(&id, ctx)?;
        match &policy {
            ReplacementPolicy::Create if selected.is_some() => {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "logical secret already has a selected version",
                ));
            }
            ReplacementPolicy::Rotate { previous, .. }
                if selected.as_ref() != Some(&previous.version) =>
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "secret rotation baseline is no longer selected",
                ));
            }
            _ => (),
        }
        if matches!(policy, ReplacementPolicy::Create) {
            let prefix = format!("secret-{}-{}-", id.as_str().len(), id.as_str());
            if lock
                .list(4096)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "secret inventory unavailable",
                    )
                })?
                .iter()
                .any(|n| n.starts_with(&prefix))
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "secret exists; specify reuse or explicit replacement",
                ));
            }
        }
        let mut version = [0u8; 16];
        getrandom::fill(&mut version).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "cryptographic randomness unavailable",
            )
        })?;
        let version = SecretVersion::new(hex(&version)).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "invalid generated secret version",
            )
        })?;
        let reference = SecretRef {
            secret_id: id,
            version,
        };
        let material = if let Some(material) = supplied {
            material.0
        } else {
            let mut raw = vec![0u8; if kind == SecretKind::UuidV4 { 16 } else { 32 }];
            getrandom::fill(&mut raw).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "cryptographic randomness unavailable",
                )
            })?;
            if kind == SecretKind::UuidV4 {
                raw[6] = (raw[6] & 15) | 64;
                raw[8] = (raw[8] & 63) | 128;
            }
            let material = if kind == SecretKind::UuidV4 {
                format!(
                    "{}-{}-{}-{}-{}",
                    hex(&raw[0..4]),
                    hex(&raw[4..6]),
                    hex(&raw[6..8]),
                    hex(&raw[8..10]),
                    hex(&raw[10..16])
                )
                .into_bytes()
            } else {
                hex(&raw).into_bytes()
            };
            material
        };
        let intent = GenerationIntent {
            schema_version: 1,
            request_id: ctx.actor.request_id.clone(),
            actor_id: ctx.actor.actor_id.clone(),
            reference: reference.clone(),
            scope: scope.clone(),
            kind,
            consumers: consumers.clone(),
            policy: policy.clone(),
            material_digest,
        };
        let intent_bytes = serde_json::to_vec(&intent).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "secret intent serialization failed",
            )
        })?;
        lock.compare_exchange(&request_name, None, &intent_bytes)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "secret generation intent publication failed",
                )
            })?;
        let record = SecretRecord {
            schema_version: 1,
            reference: reference.clone(),
            scope,
            kind,
            consumers,
            created_at: UnixMillis(self.control.now_ms()),
            material: material.clone(),
        };
        let bytes = serde_json::to_vec(&record)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "secret serialization failed"))?;
        ensure_mutating(ctx, self.control.as_ref())?;
        lock.compare_exchange(&filename(&reference), None, &bytes)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "secret publication failed; inspect inventory before retry",
                )
            })?;
        self.select_current(&lock, &reference, &policy, ctx)?;
        Ok(GeneratedSecret {
            reference,
            disclosure: Some(DisclosureHandle(ProtectedMaterial(material))),
            created: true,
        })
    }
    pub fn current_version(
        &self,
        id: &SecretId,
        ctx: &OperationContext,
    ) -> Result<Option<SecretVersion>> {
        let Some(bytes) = self
            .directory
            .read(&format!("current-{}.json", id), 16 * 1024)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "secret version metadata unavailable",
                )
            })?
        else {
            return Ok(None);
        };
        let binding: CurrentBinding = serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "secret version metadata corrupt",
            )
        })?;
        if binding.schema_version != 1 || &binding.reference.secret_id != id {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "secret version metadata identity mismatch",
            ));
        }
        self.read_record(&binding.reference, ctx)?;
        Ok(Some(binding.reference.version))
    }
    fn select_current(
        &self,
        lock: &hton_storage::LockedDir,
        reference: &SecretRef,
        policy: &ReplacementPolicy,
        ctx: &OperationContext,
    ) -> Result<()> {
        let name = format!("current-{}.json", reference.secret_id);
        let old = lock.read(&name, 16 * 1024).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "selected secret version unavailable",
            )
        })?;
        let current: Option<CurrentBinding> = old
            .as_ref()
            .map(|bytes| serde_json::from_slice(bytes))
            .transpose()
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "selected secret metadata corrupt",
                )
            })?;
        if current.as_ref().is_some_and(|binding| {
            binding.schema_version != 1 || binding.reference.secret_id != reference.secret_id
        }) {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "selected secret identity mismatch",
            ));
        }
        let existing = current.as_ref().map(|binding| &binding.reference);
        let baseline = match policy {
            ReplacementPolicy::Create => None,
            ReplacementPolicy::Rotate { previous, .. } => Some(previous),
            ReplacementPolicy::Reuse { reference } => Some(reference),
        };
        if existing != Some(reference) && existing != baseline {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "a different secret version superseded this request",
            ));
        }
        let selected = serde_json::to_vec(&CurrentBinding {
            schema_version: 1,
            reference: reference.clone(),
        })
        .map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "secret metadata encoding failed",
            )
        })?;
        lock.compare_exchange(&name, old.as_deref(), &selected)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "secret selection durability uncertain",
                )
            })
    }
    pub fn resolve(
        &self,
        reference: &SecretRef,
        grant: &ConsumerGrant,
        ctx: &OperationContext,
    ) -> Result<ProtectedMaterial> {
        let record = self.read_record(reference, ctx)?;
        if record.scope != grant.scope || !record.consumers.contains(&grant.consumer) {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "secret consumer grant denied",
            ));
        }
        Ok(ProtectedMaterial(record.material))
    }
    fn read_record(&self, reference: &SecretRef, ctx: &OperationContext) -> Result<SecretRecord> {
        let bytes = self
            .directory
            .read(&filename(reference), 2 * 1024 * 1024)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "secret version unavailable"))?
            .ok_or_else(|| failure(ctx, ErrorCode::IoUnavailable, "secret version missing"))?;
        let record: SecretRecord = serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "secret record corrupt; retained for recovery",
            )
        })?;
        if record.schema_version != 1
            || &record.reference != reference
            || record.material.is_empty()
            || record.material.len() > 262_144
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "secret record invariant invalid",
            ));
        }
        Ok(record)
    }
}
fn filename(reference: &SecretRef) -> String {
    format!(
        "secret-{}-{}-{}.json",
        reference.secret_id.as_str().len(),
        reference.secret_id.as_str(),
        reference.version.as_str()
    )
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
