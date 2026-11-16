//! Installation facts and explicit association. Desired participation lives elsewhere.
use crate::journal::{
    Intent, JournalStore, ProtectedOriginal, TerminalStatus, deadline, ensure_mutating, failure,
};
use hton_contracts::{config::*, operations::*};
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryDocument {
    pub schema_version: u32,
    pub snapshot: RegistrySnapshot,
}
impl Serialize for RegistryDocument {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let mut value = serde_json::to_value(&self.snapshot).map_err(serde::ser::Error::custom)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| serde::ser::Error::custom("invalid registry record"))?;
        object.insert(
            "schemaVersion".into(),
            serde_json::Value::from(self.schema_version),
        );
        value.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for RegistryDocument {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let mut value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| serde::de::Error::custom("invalid registry record"))?;
        let version = object
            .remove("schemaVersion")
            .and_then(|v| v.as_u64())
            .filter(|v| *v == 1)
            .ok_or_else(|| serde::de::Error::custom("unsupported registry version"))?;
        let snapshot = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self {
            schema_version: version as u32,
            snapshot,
        })
    }
}
#[derive(Clone, Debug, Default)]
pub struct RegistryPreconditions {
    pub observed: BTreeMap<ResourceId, ResourceObservation>,
    pub no_shared_consumers: BTreeSet<ResourceId>,
    pub recovery_released: BTreeSet<ModuleId>,
}
pub struct RegistryStore {
    directory: Arc<SecureDir>,
    journal: Arc<JournalStore>,
}

/// Pure retained-state projection; only the verified transition grants authority.
pub(crate) fn uninstalled_owned_projection(original: &RegistryEntry) -> RegistryEntry {
    let mut entry = original.clone();
    for checkpoint in &original.checkpoints {
        retain_verified_artifacts(&mut entry, checkpoint);
    }
    for artifact in original.installed_artifact_versions.keys() {
        if !entry.retained_artifacts.contains(artifact) {
            entry.retained_artifacts.push(artifact.clone());
        }
    }
    entry.resources.clear();
    entry.installed_artifact_versions.clear();
    entry.checkpoints.clear();
    entry.original_policies.clear();
    entry.adoption_scope = None;
    entry.membership = MembershipState::RemovedRetained;
    entry.setup_state = SetupState::Incomplete {
        blocked_step: None,
        required_operator_action: None,
    };
    entry
}

impl RegistryStore {
    pub fn new(directory: Arc<SecureDir>, journal: Arc<JournalStore>) -> Self {
        Self { directory, journal }
    }
    pub(crate) fn recover_uninstall(
        &self,
        id: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<()> {
        ensure_mutating(ctx, self.journal.control())?;
        let transaction = self.journal.read(id, ctx)?;
        let Intent::Registry {
            instance_id,
            expected_revision,
            change,
        } = &transaction.intent
        else {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "not an uninstall registry intent",
            ));
        };
        let RegistryChange::UninstallOwned { receipt, .. } = change else {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "registry intent is outside uninstall recovery",
            ));
        };
        if transaction.originals.len() != 1 || transaction.operation_id != ctx.operation_id {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "uninstall registry original or operation differs",
            ));
        }
        let original = &transaction.originals[0];
        if !original.was_present || crate::config::digest(&original.bytes) != original.digest {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "uninstall registry original is corrupt",
            ));
        }
        let before = decode(&original.bytes, instance_id, ctx)?;
        if before.revision != *expected_revision {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "uninstall registry revision is invalid",
            ));
        }
        let mut after = before.clone();
        apply_change(
            &mut after,
            change,
            &RegistryPreconditions::default(),
            ctx,
            receipt.recorded_at.0,
        )?;
        after.revision = StateRevision(expected_revision.0.checked_add(1).ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "uninstall registry revision exhausted",
            )
        })?);
        let lock = self
            .directory
            .lock(deadline(ctx, self.journal.control())?)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "uninstall registry recovery lock unavailable",
                )
            })?;
        let bytes = lock
            .read("registry.json", 4 * 1024 * 1024)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "uninstall registry unavailable",
                )
            })?
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "uninstall registry disappeared",
                )
            })?;
        let current = decode(&bytes, instance_id, ctx)?;
        let status = if current == after {
            TerminalStatus::Committed
        } else if current == before {
            TerminalStatus::RolledBack
        } else {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "uninstall registry differs from exact original and committed states",
            ));
        };
        drop(lock);
        match self.journal.terminal(id, ctx)? {
            Some(record) if record.status == TerminalStatus::RecoveryRequired => {
                self.journal
                    .resolve_operation(id, status, vec![], None, ctx)?;
            }
            Some(_) => {}
            None => {
                self.journal.finish(id, status, vec![], ctx)?;
            }
        }
        Ok(())
    }
    /// Existing inventory only: no journal construction or expected-instance authority.
    pub fn inspect(
        directory: &SecureDir,
        ctx: &OperationContext,
    ) -> Result<Option<RegistrySnapshot>> {
        if ctx.execution_mode != ExecutionMode::ReadOnly {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "inventory requires read-only context",
            ));
        }
        directory
            .read("registry.json", 4 * 1024 * 1024)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "registry unavailable"))?
            .map(|bytes| decode_record(&bytes, None, ctx))
            .transpose()
    }
    pub fn read(&self, instance: &InstanceId, ctx: &OperationContext) -> Result<RegistrySnapshot> {
        let bytes = self
            .directory
            .read("registry.json", 4 * 1024 * 1024)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "registry unavailable"))?;
        match bytes {
            Some(b) => decode(&b, instance, ctx),
            None => Ok(RegistrySnapshot {
                instance_id: instance.clone(),
                revision: StateRevision(0),
                entries: BTreeMap::new(),
                diagnostics: vec![],
                recovery_originals: vec![],
            }),
        }
    }
    pub fn transition(
        &self,
        instance: &InstanceId,
        expected: StateRevision,
        change: RegistryChange,
        pre: &RegistryPreconditions,
        ctx: &OperationContext,
    ) -> Result<RegistrySnapshot> {
        ensure_mutating(ctx, self.journal.control())?;
        let lock = self
            .directory
            .lock(deadline(ctx, self.journal.control())?)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "registry lock unavailable"))?;
        let original = lock
            .read("registry.json", 4 * 1024 * 1024)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "registry unavailable"))?;
        let mut state = match &original {
            Some(b) => decode(b, instance, ctx)?,
            None => RegistrySnapshot {
                instance_id: instance.clone(),
                revision: StateRevision(0),
                entries: BTreeMap::new(),
                diagnostics: vec![],
                recovery_originals: vec![],
            },
        };
        if state.revision != expected {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "registry revision changed",
            ));
        }
        apply_change(
            &mut state,
            &change,
            pre,
            ctx,
            self.journal.control().now_ms(),
        )?;
        state.revision = StateRevision(state.revision.0.checked_add(1).ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "registry revision exhausted",
            )
        })?);
        validate(&state, ctx)?;
        let bytes = serde_json::to_vec(&RegistryDocument {
            schema_version: 1,
            snapshot: state.clone(),
        })
        .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "registry encoding failed"))?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(failure(
                ctx,
                ErrorCode::IoUnavailable,
                "registry exceeds bound",
            ));
        }
        let originals = if let Some(ref old) = original {
            vec![ProtectedOriginal {
                artifact_id: ArtifactId::new(format!("registry-{}", expected.0)).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "invalid original identity",
                    )
                })?,
                digest: Sha256Digest::new(format!("sha256:{:x}", Sha256::digest(old))).map_err(
                    |_| failure(ctx, ErrorCode::RecoveryRequired, "original digest failed"),
                )?,
                was_present: true,
                bytes: old.clone(),
            }]
        } else {
            vec![]
        };
        let tx = self.journal.prepare_intent(
            Intent::Registry {
                instance_id: instance.clone(),
                expected_revision: expected,
                change,
            },
            originals,
            ctx,
        )?;
        ensure_mutating(ctx, self.journal.control())?;
        lock.compare_exchange("registry.json", original.as_deref(), &bytes)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "registry publication uncertain; reconcile journal",
                )
            })?;
        self.journal
            .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
        self.journal.event(
            hton_logging::Component::Registry,
            hton_logging::EventCode::StatePublished,
            ctx,
        );
        Ok(state)
    }
}
fn decode(bytes: &[u8], instance: &InstanceId, ctx: &OperationContext) -> Result<RegistrySnapshot> {
    decode_record(bytes, Some(instance), ctx)
}
fn decode_record(
    bytes: &[u8],
    instance: Option<&InstanceId>,
    ctx: &OperationContext,
) -> Result<RegistrySnapshot> {
    let doc: RegistryDocument = serde_json::from_slice(bytes).map_err(|_| {
        failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "registry corrupt; original retained",
        )
    })?;
    if doc.schema_version != 1 || instance.is_some_and(|id| &doc.snapshot.instance_id != id) {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "registry version or instance mismatch",
        ));
    }
    validate(&doc.snapshot, ctx)?;
    Ok(doc.snapshot)
}
fn validate(state: &RegistrySnapshot, ctx: &OperationContext) -> Result<()> {
    if state.entries.len() > 256 {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "registry member bound exceeded",
        ));
    }
    let mut resources = BTreeSet::new();
    for (id, entry) in &state.entries {
        if id != &entry.module_id || entry.resources.len() > 128 || entry.checkpoints.len() > 4096 {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "registry entry invariant violated",
            ));
        }
        for r in &entry.resources {
            if !resources.insert(r.resource_id.clone()) {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "resource belongs to multiple registry entries",
                ));
            }
        }
        if entry.ownership == Ownership::Associated {
            let scope = entry.adoption_scope.as_ref().ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "associated resource lacks approval scope",
                )
            })?;
            let ids: BTreeSet<_> = entry
                .resources
                .iter()
                .map(|r| r.resource_id.clone())
                .collect();
            if scope.resources != ids
                || ![
                    ForeignActionPermission::Start,
                    ForeignActionPermission::Stop,
                    ForeignActionPermission::DisableAutostart,
                    ForeignActionPermission::RestoreAutostart,
                ]
                .iter()
                .all(|a| scope.lifecycle_actions.contains(a))
                || ids.iter().any(|r| !entry.original_policies.contains_key(r))
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "associated lifecycle authority or original policies incomplete",
                ));
            }
        } else if entry.adoption_scope.is_some() {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "owned entry cannot contain foreign adoption scope",
            ));
        }
    }
    Ok(())
}
fn require_stopped(
    entry: &RegistryEntry,
    pre: &RegistryPreconditions,
    ctx: &OperationContext,
    now: u64,
) -> Result<()> {
    for resource in &entry.resources {
        let obs = pre.observed.get(&resource.resource_id).ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::ResourceDrift,
                "resource stop evidence missing",
            )
        })?;
        if obs.identity != *resource
            || obs.valid_until.0 <= now
            || obs.state != ObservedState::Stopped
        {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "resource identity or stopped evidence changed",
            ));
        }
    }
    Ok(())
}
fn apply_change(
    state: &mut RegistrySnapshot,
    change: &RegistryChange,
    pre: &RegistryPreconditions,
    ctx: &OperationContext,
    now: u64,
) -> Result<()> {
    match change {
        RegistryChange::RebindOwned {
            module_id,
            previous,
            replacement,
            receipt,
        } => {
            let entry = state
                .entries
                .get_mut(module_id)
                .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "module not registered"))?;
            if entry.ownership != Ownership::Owned
                || entry.membership != MembershipState::Installed
                || entry.setup_state != SetupState::Complete
                || previous.resource_id != replacement.resource_id
                || previous.service_id != replacement.service_id
                || std::mem::discriminant(&previous.locator)
                    != std::mem::discriminant(&replacement.locator)
                || receipt.status != StepStatus::Verified
                || receipt.error.is_some()
                || receipt.recorded_at.0 > now
                || !receipt.resources.contains(replacement)
                || !receipt
                    .state_fingerprints
                    .contains_key(&replacement.resource_id)
                || !pre
                    .observed
                    .get(&replacement.resource_id)
                    .is_some_and(|observed| {
                        observed.identity == *replacement
                            && observed.observed_at.0 <= now
                            && observed.valid_until.0 > now
                            && !matches!(
                                observed.state,
                                ObservedState::Unknown | ObservedState::Failed
                            )
                    })
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "owned replacement lacks matching verified identity and fresh observation",
                ));
            }
            let old = entry
                .resources
                .iter_mut()
                .find(|r| r.resource_id == previous.resource_id)
                .ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "previous owned resource is not registered",
                    )
                })?;
            if old != previous {
                return Err(failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "owned replacement baseline changed",
                ));
            }
            *old = replacement.clone();
        }
        RegistryChange::Adopt { entry } => {
            let scope = entry.adoption_scope.as_ref().ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "explicit adoption approval required",
                )
            })?;
            if entry.ownership != Ownership::Associated
                || !ctx.actor.local_approval_refs.contains(&scope.approval_id)
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "foreign association requires matching local approval",
                ));
            }
            if state.entries.contains_key(&entry.module_id) {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "module already registered",
                ));
            }
            for r in &entry.resources {
                if !pre.observed.get(&r.resource_id).is_some_and(|o| {
                    o.identity == *r
                        && o.valid_until.0 > now
                        && o.restart_policy.as_ref() == entry.original_policies.get(&r.resource_id)
                }) {
                    return Err(failure(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "adoption resource evidence changed",
                    ));
                }
            }
            state.entries.insert(entry.module_id.clone(), entry.clone());
        }
        RegistryChange::Install { entry } => {
            if entry.ownership != Ownership::Owned {
                return Err(failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "foreign resources require adoption",
                ));
            }
            if let Some(old) = state.entries.get(&entry.module_id) {
                if old.resources != entry.resources
                    || old.ownership != entry.ownership
                    || old.installed_artifact_versions != entry.installed_artifact_versions
                    || old.original_policies != entry.original_policies
                    || !old
                        .checkpoints
                        .iter()
                        .all(|r| entry.checkpoints.contains(r))
                    || matches!(old.membership, MembershipState::CleanupRequired)
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "install cannot replace existing resource ownership or versions",
                    ));
                }
            }
            state.entries.insert(entry.module_id.clone(), entry.clone());
        }
        RegistryChange::SetupProgress {
            module_id,
            resources,
            installed_versions,
            setup_state,
            checkpoint,
        } => {
            let entry = state
                .entries
                .get_mut(module_id)
                .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "module not registered"))?;
            if entry.ownership != Ownership::Owned
                || entry.membership != MembershipState::Installed
                || !matches!(entry.setup_state, SetupState::Incomplete { .. })
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "setup progress requires an incomplete owned installation",
                ));
            }
            let verified = checkpoint.as_ref().filter(|receipt| {
                receipt.status == StepStatus::Verified && receipt.error.is_none()
            });
            for resource in resources {
                if let Some(existing) = entry
                    .resources
                    .iter()
                    .find(|r| r.resource_id == resource.resource_id)
                {
                    if existing != resource {
                        return Err(failure(
                            ctx,
                            ErrorCode::ResourceDrift,
                            "setup cannot replace existing resource identity",
                        ));
                    }
                    continue;
                }
                if !verified.is_some_and(|receipt| receipt.resources.contains(resource))
                    || !pre
                        .observed
                        .get(&resource.resource_id)
                        .is_some_and(|observation| {
                            observation.identity == *resource
                                && observation.state == ObservedState::Stopped
                                && observation.observed_at.0 <= now
                                && observation.valid_until.0 > now
                        })
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "new setup resource requires verified exact stopped evidence",
                    ));
                }
                entry.resources.push(resource.clone());
            }
            for (artifact, version) in installed_versions {
                if let Some(existing) = entry.installed_artifact_versions.get(artifact) {
                    if existing != version {
                        return Err(failure(
                            ctx,
                            ErrorCode::OperationConflict,
                            "setup cannot replace installed versions; use update",
                        ));
                    }
                } else {
                    if !verified
                        .is_some_and(|receipt| receipt.retained_artifacts.contains(artifact))
                    {
                        return Err(failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "new setup version requires a verified retained artifact receipt",
                        ));
                    }
                    entry
                        .installed_artifact_versions
                        .insert(artifact.clone(), version.clone());
                }
            }
            if let Some(receipt) = checkpoint {
                append_checkpoint(entry, receipt, ctx, now)?;
            }
            if *setup_state == SetupState::Complete {
                let mut latest = BTreeMap::new();
                for receipt in &entry.checkpoints {
                    latest.insert((&receipt.step_id, &receipt.idempotency_key), receipt);
                }
                if latest.values().any(|receipt| {
                    receipt.status != StepStatus::Verified || receipt.error.is_some()
                }) {
                    return Err(failure(
                        ctx,
                        ErrorCode::ModuleIncomplete,
                        "setup has unresolved checkpoint outcomes",
                    ));
                }
            }
            entry.setup_state = setup_state.clone();
        }
        RegistryChange::Checkpoint {
            module_id,
            checkpoint,
        } => {
            let entry = state
                .entries
                .get_mut(module_id)
                .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "module not registered"))?;
            append_checkpoint(entry, checkpoint, ctx, now)?;
        }
        RegistryChange::Remove {
            module_id,
            retained_artifacts,
        } => {
            let entry = state
                .entries
                .get_mut(module_id)
                .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "module not registered"))?;
            require_stopped(entry, pre, ctx, now)?;
            entry.membership = MembershipState::RemovedRetained;
            for artifact in retained_artifacts {
                if !entry.retained_artifacts.contains(artifact) {
                    entry.retained_artifacts.push(artifact.clone());
                }
            }
        }
        RegistryChange::UninstallOwned {
            module_id,
            removed_resources,
            receipt,
        } => {
            let entry = state.entries.get_mut(module_id).ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleUnknown,
                    "uninstall module is not registered",
                )
            })?;
            let expected: BTreeSet<_> = entry
                .resources
                .iter()
                .map(|resource| &resource.resource_id)
                .collect();
            let supplied: BTreeSet<_> = removed_resources
                .iter()
                .map(|resource| &resource.resource_id)
                .collect();
            if entry.ownership != Ownership::Owned
                || expected != supplied
                || supplied.len() != removed_resources.len()
                || removed_resources
                    .iter()
                    .any(|resource| !entry.resources.contains(resource))
                || receipt.step_id.as_str() != "uninstall-owned"
                || receipt.status != StepStatus::Verified
                || receipt.error.is_some()
                || receipt.resources != *removed_resources
                || receipt.recorded_at.0 > now
                || now.saturating_sub(receipt.recorded_at.0) > 60_000
                || receipt.state_fingerprints.len() != removed_resources.len()
                || removed_resources.iter().any(|resource| {
                    let expected =
                        serde_json::to_vec(&("uninstall-owned-v1", resource)).map(|bytes| {
                            Sha256Digest::new(format!("sha256:{:x}", Sha256::digest(bytes)))
                                .expect("SHA256 encoding")
                        });
                    expected.ok().as_ref() != receipt.state_fingerprints.get(&resource.resource_id)
                })
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "uninstall lacks exact verified owned resource-removal receipt",
                ));
            }
            *entry = uninstalled_owned_projection(entry);
        }
        RegistryChange::CleanupRequired {
            module_id,
            resources,
        } => {
            let entry = state
                .entries
                .get_mut(module_id)
                .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "module not registered"))?;
            if resources
                .iter()
                .any(|id| !entry.resources.iter().any(|r| &r.resource_id == id))
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "unknown cleanup resource",
                ));
            }
            entry.membership = MembershipState::CleanupRequired;
        }
        RegistryChange::Update {
            module_id,
            installed_versions,
            receipt,
        } => {
            let entry = state
                .entries
                .get_mut(module_id)
                .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "module not registered"))?;
            if receipt.status != StepStatus::Verified {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "unverified update cannot replace installed versions",
                ));
            }
            entry.installed_artifact_versions = installed_versions.clone();
            entry.checkpoints.push(receipt.clone());
        }
        RegistryChange::Purge { module_id } => {
            let entry = state
                .entries
                .get(module_id)
                .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "module not registered"))?;
            if entry.ownership != Ownership::Owned
                || entry.membership != MembershipState::RemovedRetained
                || !pre.recovery_released.contains(module_id)
                || !entry
                    .resources
                    .iter()
                    .all(|r| pre.no_shared_consumers.contains(&r.resource_id))
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "purge requires removed owned resources without shared or recovery consumers",
                ));
            }
            require_stopped(entry, pre, ctx, now)?;
            state.entries.remove(module_id);
        }
    }
    Ok(())
}

fn append_checkpoint(
    entry: &mut RegistryEntry,
    receipt: &StepReceipt,
    ctx: &OperationContext,
    now: u64,
) -> Result<()> {
    if entry.checkpoints.iter().any(|old| old == receipt) {
        retain_verified_artifacts(entry, receipt);
        return Ok(());
    }
    if let Some(old) = entry.checkpoints.iter().rev().find(|old| {
        old.step_id == receipt.step_id
            && old.idempotency_key == receipt.idempotency_key
            && old.status == receipt.status
    }) {
        let refresh = receipt.status == StepStatus::Verified
            && receipt.error.is_none()
            && old.error.is_none()
            && receipt.resources == old.resources
            && receipt.state_fingerprints == old.state_fingerprints
            && receipt.retained_artifacts == old.retained_artifacts
            && receipt.recorded_at.0 >= old.recorded_at.0
            && receipt.recorded_at.0 <= now
            && now.saturating_sub(receipt.recorded_at.0) <= 60_000;
        if !refresh {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "checkpoint identity conflict",
            ));
        }
    }
    if entry.checkpoints.len() >= 4096 {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "checkpoint history bound exceeded",
        ));
    }
    retain_verified_artifacts(entry, receipt);
    entry.checkpoints.push(receipt.clone());
    Ok(())
}

fn retain_verified_artifacts(entry: &mut RegistryEntry, receipt: &StepReceipt) {
    if entry.ownership == Ownership::Owned
        && receipt.status == StepStatus::Verified
        && receipt.error.is_none()
    {
        for artifact in &receipt.retained_artifacts {
            if !entry.retained_artifacts.contains(artifact) {
                entry.retained_artifacts.push(artifact.clone());
            }
        }
    }
}
