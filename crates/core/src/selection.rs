//! Narrow enabled-selection writer. General editing and migration belong elsewhere.
use crate::config::{self, ConfigSource, LoadLimits, SourceError};
use crate::journal::{
    Intent, JournalStore, ProtectedOriginal, TerminalStatus, Transaction, deadline,
    ensure_mutating, failure,
};
use hton_contracts::{config::*, operations::*};
use hton_storage::SecureDir;
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, PartialEq, Eq)]
pub struct SelectionCandidate {
    change: SelectionChange,
    baseline: Vec<u8>,
    replacement: Vec<u8>,
    prospective: DesiredReadResult,
}
impl std::fmt::Debug for SelectionCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectionCandidate")
            .field("change", &self.change)
            .field("bytes", &"[protected]")
            .finish_non_exhaustive()
    }
}
impl SelectionCandidate {
    pub fn change(&self) -> &SelectionChange {
        &self.change
    }
    pub fn read_result(&self) -> &DesiredReadResult {
        &self.prospective
    }
    pub fn changed(&self) -> bool {
        self.baseline != self.replacement
    }
    pub fn revision(&self) -> ContentRevision {
        config::content_revision(root_id(), &self.replacement)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionCommit {
    pub revision: ContentRevision,
    pub changed: bool,
    pub transaction_id: Option<TransactionId>,
}
fn root_id() -> ObjectId {
    ObjectId::new("root").expect("fixed logical ID")
}
struct ProspectiveRoot<'a> {
    source: &'a dyn ConfigSource,
    replacement: &'a [u8],
}
impl ConfigSource for ProspectiveRoot<'_> {
    fn list_modules(&self, limit: usize) -> std::result::Result<Vec<String>, SourceError> {
        self.source.list_modules(limit)
    }
    fn list_docker(&self, limit: usize) -> std::result::Result<Vec<String>, SourceError> {
        self.source.list_docker(limit)
    }
    fn read(
        &self,
        components: &[&str],
        limit: usize,
    ) -> std::result::Result<Option<Vec<u8>>, SourceError> {
        if components == ["hton.yaml"] {
            if self.replacement.len() > limit {
                return Err(SourceError::TooLarge);
            }
            Ok(Some(self.replacement.to_vec()))
        } else {
            self.source.read(components, limit)
        }
    }
}

/// A preview reads the complete configuration but creates no journals, locks or files.
pub fn prepare(
    source: &dyn ConfigSource,
    module_ids: BTreeSet<ModuleId>,
    enabled: bool,
    expected_root_revision: &ContentRevision,
    ctx: &OperationContext,
) -> Result<SelectionCandidate> {
    if module_ids.is_empty()
        || module_ids.len() > 64
        || module_ids.iter().any(|id| id.as_str() == "core")
    {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "selection requires known optional module IDs",
        ));
    }
    let loaded = config::load(source, &BTreeSet::from([FormatVersion(1)]));
    if !loaded.errors.is_empty()
        || loaded.snapshot.is_none()
        || loaded.read_set.coherence != ReadCoherence::Coherent
    {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "selection requires a valid coherent configuration",
        ));
    }
    let root = loaded
        .documents
        .iter()
        .find(|d| d.object_id == root_id())
        .ok_or_else(|| failure(ctx, ErrorCode::ConfigSyntax, "root document unavailable"))?;
    if &root.revision != expected_root_revision {
        return Err(failure(
            ctx,
            ErrorCode::PlanStale,
            "root selection revision changed",
        ));
    }
    let mut value = config::decode_yaml(&root.source_bytes, LoadLimits::default())
        .map_err(|_| failure(ctx, ErrorCode::ConfigSyntax, "root configuration invalid"))?;
    let object = value.as_object_mut().ok_or_else(|| {
        failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "root configuration must be a mapping",
        )
    })?;
    let features = object
        .entry("features")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "feature selection must be a mapping",
            )
        })?;
    let mut changed = false;
    for id in &module_ids {
        let existing = features
            .get(id.as_str())
            .and_then(|v| v.get("enabled"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        changed |= existing != enabled;
        features.insert(id.as_str().into(), serde_json::json!({"enabled": enabled}));
    }
    let replacement = if changed {
        serde_yaml::to_string(&value)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "selection encoding failed"))?
            .into_bytes()
    } else {
        root.source_bytes.clone()
    };
    if replacement.len() > LoadLimits::default().file_bytes {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "selection exceeds document limit",
        ));
    }
    let prospective = config::load(
        &ProspectiveRoot {
            source,
            replacement: &replacement,
        },
        &BTreeSet::from([FormatVersion(1)]),
    );
    if !prospective.errors.is_empty()
        || prospective.snapshot.is_none()
        || prospective.read_set.coherence != ReadCoherence::Coherent
    {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "prospective selection is invalid",
        ));
    }
    Ok(SelectionCandidate {
        change: SelectionChange {
            module_ids,
            enabled,
            expected_root_revision: expected_root_revision.clone(),
        },
        baseline: root.source_bytes.clone(),
        replacement,
        prospective,
    })
}

pub struct SelectionWriter {
    directory: Arc<SecureDir>,
    journal: Arc<JournalStore>,
}
impl SelectionWriter {
    pub fn new(directory: Arc<SecureDir>, journal: Arc<JournalStore>) -> Self {
        Self { directory, journal }
    }
    pub fn commit(
        &self,
        candidate: &SelectionCandidate,
        expected_root_revision: &ContentRevision,
        ctx: &OperationContext,
    ) -> Result<SelectionCommit> {
        ensure_mutating(ctx, self.journal.control())?;
        if expected_root_revision != &candidate.change.expected_root_revision {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "selection approval revision changed",
            ));
        }
        let lock = self
            .directory
            .lock(deadline(ctx, self.journal.control())?)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "selection lock unavailable",
                )
            })?;
        let current = lock
            .read("hton.yaml", LoadLimits::default().file_bytes)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "root selection cannot be read",
                )
            })?
            .ok_or_else(|| failure(ctx, ErrorCode::ConfigSyntax, "root selection missing"))?;
        if current != candidate.baseline
            || config::content_revision(root_id(), &current) != *expected_root_revision
        {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "root bytes changed since selection preview",
            ));
        }
        if !candidate.changed() {
            return Ok(SelectionCommit {
                revision: candidate.revision(),
                changed: false,
                transaction_id: None,
            });
        }
        let tx = self.journal.prepare_intent(
            Intent::Selection {
                expected_revision: expected_root_revision.clone(),
                replacement_digest: config::digest(&candidate.replacement),
                module_ids: candidate.change.module_ids.clone(),
                enabled: candidate.change.enabled,
            },
            vec![ProtectedOriginal {
                artifact_id: ArtifactId::new("root-before-selection").expect("fixed logical ID"),
                digest: config::digest(&current),
                was_present: true,
                bytes: current.clone(),
            }],
            ctx,
        )?;
        ensure_mutating(ctx, self.journal.control())?;
        lock.compare_exchange("hton.yaml", Some(&current), &candidate.replacement)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "selection publication uncertain; original retained",
                )
            })?;
        self.journal
            .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
        Ok(SelectionCommit {
            revision: candidate.revision(),
            changed: true,
            transaction_id: Some(tx),
        })
    }
    /// Re-establish durability after an interrupted CAS only when the exact
    /// intended replacement is already present. Never merge or replay old bytes.
    pub fn reconcile(
        &self,
        transaction: &Transaction,
        ctx: &OperationContext,
    ) -> Result<SelectionCommit> {
        ensure_mutating(ctx, self.journal.control())?;
        if transaction.operation_id != ctx.operation_id || transaction.actor != ctx.actor {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "selection reconciliation authority mismatch",
            ));
        }
        let Intent::Selection {
            replacement_digest, ..
        } = &transaction.intent
        else {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "transaction is not a root selection intent",
            ));
        };
        let lock = self
            .directory
            .lock(deadline(ctx, self.journal.control())?)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "selection reconciliation lock unavailable",
                )
            })?;
        let current = lock
            .read("hton.yaml", LoadLimits::default().file_bytes)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "selection reconciliation read failed",
                )
            })?
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "selection reconciliation root missing",
                )
            })?;
        if config::digest(&current) != *replacement_digest {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "selection reconciliation found different current bytes; no overwrite performed",
            ));
        }
        lock.compare_exchange("hton.yaml", Some(&current), &current)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "selection durability remains uncertain",
                )
            })?;
        self.journal.finish(
            &transaction.transaction_id,
            TerminalStatus::Committed,
            vec![],
            ctx,
        )?;
        Ok(SelectionCommit {
            revision: config::content_revision(root_id(), &current),
            changed: false,
            transaction_id: Some(transaction.transaction_id.clone()),
        })
    }
}
