//! Pure policy for assigning persistent VPN backend ports.
//!
//! Planning is pure over explicit evidence and entropy. A separate narrow writer
//! persists descriptor-allowlisted settings with protected journal/CAS recovery;
//! host probing and foreign-resource mutation remain separate owners.

use crate::{
    catalog::ModuleCatalogue,
    config::{self, LoadLimits},
    host::AllocationApproval,
    journal::{self, Intent, JournalStore, ProtectedOriginal, TerminalStatus},
};
use hton_contracts::{
    config::{
        AllocationRequest, CheckIssue, ContentRevision, DependencyGraph, NormalizedSnapshot,
        PortAllocationCandidate, PortAllocationReceipt, PortAllocationWriteState, ReadCoherence,
        ReadSet, RegistrySnapshot, ReservationEvidence, SavedAllocation, ServicePortAllocation,
        ServicePortBinding, Transport,
    },
    operations::{
        ActorSource, ArtifactId, Budget, ErrorCode, InstanceId, ModuleId, ObjectId,
        OperationContext, Result as CoreResult, ServiceId, Sha256Digest, TransactionId, UnixMillis,
    },
};
use hton_storage::SecureDir;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::{Duration, Instant},
};

pub const PORT_MIN: u16 = 5000;
pub const PORT_MAX: u16 = 6000;

/// Explicit bytes sourced from an OS CSPRNG by the caller. Never logged or persisted.
#[derive(Clone, PartialEq, Eq)]
pub struct EntropyBytes(pub Vec<u8>);
impl std::fmt::Debug for EntropyBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EntropyBytes([redacted])")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AllocationBlocker {
    InvalidBinding,
    InvalidSavedAllocation {
        service_id: ServiceId,
        port: u16,
    },
    DuplicateSavedAllocation {
        first: ServiceId,
        second: ServiceId,
        port: u16,
    },
    SavedAllocationConflict {
        service_id: ServiceId,
        port: u16,
    },
    ReservationEvidenceMissing,
    ReservationEvidenceStale,
    CandidateExpired,
    EntropyUnavailable,
    NoFreePort,
    IncoherentReadSet,
}

#[derive(Clone, Debug)]
pub struct PlannedAllocation {
    pub candidate: PortAllocationCandidate,
    pub graph: DependencyGraph,
}

#[derive(Clone, Debug)]
pub enum AllocationPlanError {
    Catalogue(Vec<CheckIssue>),
    Allocation(AllocationBlocker),
    Graph(Vec<CheckIssue>),
    Registry(Vec<CheckIssue>),
}

/// Compose catalogue-owned saved bindings, retained registry allocations,
/// deterministic planning and dynamic listener claims for one selected graph.
/// All environmental inputs remain explicit; this method performs no I/O.
#[allow(clippy::too_many_arguments)]
pub fn plan_graph(
    catalogue: &ModuleCatalogue,
    graph: &DependencyGraph,
    snapshot: &NormalizedSnapshot,
    registry: &RegistrySnapshot,
    evidence: &ReservationEvidence,
    entropy: &EntropyBytes,
    now: UnixMillis,
    expires_at: UnixMillis,
    budget: &Budget,
) -> std::result::Result<PlannedAllocation, AllocationPlanError> {
    if !graph.conflicts.is_empty() {
        return Err(AllocationPlanError::Graph(graph.conflicts.clone()));
    }
    if !registry.diagnostics.is_empty() {
        return Err(AllocationPlanError::Registry(registry.diagnostics.clone()));
    }
    let selected_bindings = catalogue
        .allocation_bindings_for(graph, snapshot)
        .map_err(AllocationPlanError::Catalogue)?;
    let selected: BTreeSet<_> = selected_bindings
        .iter()
        .map(|binding| binding.service_id.clone())
        .collect();
    let retained_services: BTreeSet<_> = registry
        .entries
        .values()
        .filter_map(|entry| {
            catalogue
                .describe(&entry.module_id)
                .and_then(|catalogue_entry| {
                    catalogue_entry
                        .descriptor
                        .port_allocation
                        .as_ref()
                        .map(|binding| binding.service_id.clone())
                })
        })
        .collect();
    let retained = catalogue
        .retained_allocations_for(&selected, &retained_services, snapshot)
        .map_err(AllocationPlanError::Catalogue)?;
    let mut read_set = snapshot.desired.read_set.clone();
    for binding in &selected_bindings {
        if !read_set.objects.contains_key(&binding.settings_document) {
            if retained_services.contains(&binding.service_id) {
                return Err(AllocationPlanError::Catalogue(vec![CheckIssue {
                    code: ErrorCode::ModuleIncomplete,
                    severity: hton_contracts::config::Severity::Error,
                    module_id: None,
                    object_id: Some(binding.settings_document.clone()),
                    path: None,
                    message: "registered VPN settings document is missing; refusing to reseed saved service state".into(),
                    repair_hint: None,
                }]));
            }
            read_set
                .absent_objects
                .insert(binding.settings_document.clone());
        }
    }
    let request = AllocationRequest {
        selected: selected_bindings,
        retained,
        read_set,
        registry_revision: registry.revision,
        now,
        expires_at,
    };
    let candidate =
        plan(&request, evidence, entropy, budget).map_err(AllocationPlanError::Allocation)?;
    let bound = catalogue.bind_allocation(graph, &candidate, snapshot);
    if !bound.conflicts.is_empty() {
        return Err(AllocationPlanError::Graph(bound.conflicts));
    }
    Ok(PlannedAllocation {
        candidate,
        graph: bound,
    })
}

/// Deterministically plan allocations from explicit settings, host evidence and
/// entropy. No network, filesystem, Docker or ambient RNG access occurs here.
pub fn plan(
    request: &AllocationRequest,
    evidence: &ReservationEvidence,
    entropy: &EntropyBytes,
    budget: &Budget,
) -> Result<PortAllocationCandidate, AllocationBlocker> {
    if request.read_set.coherence != ReadCoherence::Coherent {
        return Err(AllocationBlocker::IncoherentReadSet);
    }
    if request
        .read_set
        .absent_objects
        .iter()
        .any(|id| request.read_set.objects.contains_key(id))
    {
        return Err(AllocationBlocker::IncoherentReadSet);
    }
    if !evidence.complete {
        return Err(AllocationBlocker::ReservationEvidenceMissing);
    }
    if evidence.observed_at > request.now || evidence.valid_until <= request.now {
        return Err(AllocationBlocker::ReservationEvidenceStale);
    }
    if request.expires_at <= request.now || budget.deadline <= request.now {
        return Err(AllocationBlocker::CandidateExpired);
    }
    let candidate_expires_at = request
        .expires_at
        .min(evidence.valid_until)
        .min(budget.deadline);
    if candidate_expires_at <= request.now {
        return Err(AllocationBlocker::CandidateExpired);
    }

    let mut selected = BTreeMap::<ServiceId, &ServicePortBinding>::new();
    let mut globally_used = BTreeMap::<u16, ServiceId>::new();
    let mut retained_by_service = BTreeMap::<ServiceId, u16>::new();
    for saved in &request.retained {
        if !(PORT_MIN..=PORT_MAX).contains(&saved.port) {
            return Err(AllocationBlocker::InvalidSavedAllocation {
                service_id: saved.service_id.clone(),
                port: saved.port,
            });
        }
        if let Some(first) = globally_used.insert(saved.port, saved.service_id.clone()) {
            if first != saved.service_id {
                return Err(AllocationBlocker::DuplicateSavedAllocation {
                    first,
                    second: saved.service_id.clone(),
                    port: saved.port,
                });
            }
        }
        if let Some(previous) = retained_by_service.insert(saved.service_id.clone(), saved.port) {
            if previous != saved.port {
                return Err(AllocationBlocker::InvalidSavedAllocation {
                    service_id: saved.service_id.clone(),
                    port: saved.port,
                });
            }
        }
    }
    for binding in &request.selected {
        if binding.descriptor_revision.is_none()
            || binding.fields.is_empty()
            || binding.fields.iter().any(|f| f.path.is_empty())
            || retained_by_service.contains_key(&binding.service_id)
            || selected
                .insert(binding.service_id.clone(), binding)
                .is_some()
        {
            return Err(AllocationBlocker::InvalidBinding);
        }
        let mut paths = BTreeSet::new();
        let mut saved_values = BTreeSet::new();
        let mut saved_field_count = 0usize;
        for field in &binding.fields {
            if !paths.insert(field.path.as_str()) {
                return Err(AllocationBlocker::InvalidBinding);
            }
            if let Some(port) = field.saved_port {
                saved_field_count += 1;
                if !(PORT_MIN..=PORT_MAX).contains(&port) {
                    return Err(AllocationBlocker::InvalidSavedAllocation {
                        service_id: binding.service_id.clone(),
                        port,
                    });
                }
                saved_values.insert(port);
            }
        }
        if saved_values.len() > 1
            || (saved_field_count != 0 && saved_field_count != binding.fields.len())
        {
            return Err(AllocationBlocker::InvalidBinding);
        }
    }

    // Validate every saved value before allocating any missing one. Selected
    // assignments count toward uniqueness together with retained assignments.
    let mut used = globally_used;
    let mut allocations = BTreeMap::new();
    for (service_id, binding) in &selected {
        let mut values = binding.fields.iter().filter_map(|field| field.saved_port);
        let saved = values.next();
        if values.any(|port| Some(port) != saved) {
            return Err(AllocationBlocker::InvalidBinding);
        }
        if let Some(port) = saved {
            if let Some(first) = used.insert(port, (*service_id).clone()) {
                if first != (*service_id).clone() {
                    return Err(AllocationBlocker::DuplicateSavedAllocation {
                        first,
                        second: (*service_id).clone(),
                        port,
                    });
                }
            }
            if evidence.reservations.iter().any(|reservation| {
                reservation.port == port
                    && binding
                        .fields
                        .iter()
                        .any(|f| f.transport == reservation.transport)
                    && reservation.holder_service.as_ref() != Some(service_id)
            }) {
                return Err(AllocationBlocker::SavedAllocationConflict {
                    service_id: (*service_id).clone(),
                    port,
                });
            }
            allocations.insert((*service_id).clone(), make_allocation(binding, port));
        }
    }

    let mut entropy_offset = 0usize;
    for (service_id, binding) in &selected {
        if allocations.contains_key(service_id) {
            continue;
        }
        let occupied: BTreeSet<u16> = evidence
            .reservations
            .iter()
            .map(|reservation| reservation.port)
            .chain(used.keys().copied())
            .collect();
        let available: Vec<u16> = (PORT_MIN..=PORT_MAX)
            .filter(|port| !occupied.contains(port))
            .collect();
        if available.is_empty() {
            return Err(AllocationBlocker::NoFreePort);
        }
        let index = unbiased_index(available.len(), &entropy.0, &mut entropy_offset)?;
        let port = available[index];
        used.insert(port, (*service_id).clone());
        allocations.insert((*service_id).clone(), make_allocation(binding, port));
    }

    let reservation_digest = evidence.digest.clone();
    let mut candidate = PortAllocationCandidate {
        allocations,
        retained: request.retained.clone(),
        read_set: request.read_set.clone(),
        registry_revision: request.registry_revision,
        reservation_digest,
        created_at: request.now,
        expires_at: candidate_expires_at,
        candidate_digest: Sha256Digest::new(format!("sha256:{}", "0".repeat(64)))
            .expect("constant digest is valid"),
    };
    candidate.candidate_digest = canonical_candidate_digest(&candidate)?;
    Ok(candidate)
}

/// Narrow writer for descriptor-declared module settings. It edits only the
/// allowlisted leaf fields and uses the protected module directory's CAS lock.
pub struct PortSettingsWriter {
    modules: Arc<SecureDir>,
    catalogue: Arc<ModuleCatalogue>,
    journal: Arc<JournalStore>,
    instance_id: InstanceId,
}

struct SettingsDelta {
    document_id: ObjectId,
    filename: String,
    original: Option<Vec<u8>>,
    replacement: Vec<u8>,
}

impl PortSettingsWriter {
    pub fn new(
        config_dir: &SecureDir,
        catalogue: Arc<ModuleCatalogue>,
        journal_store: Arc<JournalStore>,
        instance_id: InstanceId,
        ctx: &OperationContext,
    ) -> CoreResult<Self> {
        let modules = config_dir.child("modules").map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "module settings inventory unavailable",
            )
        })?;
        Ok(Self {
            modules: Arc::new(modules),
            catalogue,
            journal: journal_store,
            instance_id,
        })
    }

    /// Persist only newly allocated values. Existing identical assignments are a
    /// true no-op and do not require or create an approval/journal record.
    pub fn commit(
        &self,
        candidate: &PortAllocationCandidate,
        expected_read_set: &ReadSet,
        approval: Option<&AllocationApproval>,
        ctx: &OperationContext,
    ) -> CoreResult<PortAllocationReceipt> {
        journal::ensure_mutating(ctx, self.journal.control())?;
        let now = UnixMillis(self.journal.control().now_ms());
        if candidate.created_at.0 > now.0 || candidate.expires_at.0 <= now.0 {
            return Err(journal::failure(
                ctx,
                ErrorCode::PlanExpired,
                "port allocation candidate is expired or from the future",
            ));
        }
        if candidate.read_set != *expected_read_set
            || expected_read_set.coherence != ReadCoherence::Coherent
            || !candidate_integrity(candidate)
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "port allocation candidate or read set is invalid or stale",
            ));
        }
        let exact_intent = Intent::PortAllocation {
            candidate: candidate.clone(),
            instance_id: Some(self.instance_id.clone()),
        };
        if let Some(existing) = self.journal.lookup_intent(&exact_intent, ctx)? {
            return self.recover(&existing.transaction_id, ctx);
        }
        if self.journal.unfinished(ctx)?.iter().any(|existing| {
            existing.operation_id == ctx.operation_id
                && matches!(
                    &existing.intent,
                    Intent::PortAllocation { candidate: prior, .. }
                        if prior.candidate_digest != candidate.candidate_digest
                )
        }) {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "operation already has a different unfinished port allocation candidate",
            ));
        }
        let lock = self
            .modules
            .lock(journal::deadline(ctx, self.journal.control())?)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "module settings lock unavailable",
                )
            })?;
        let (deltas, revisions) = self.inspect_candidate(candidate, &lock, ctx)?;
        if deltas.is_empty() {
            return Ok(receipt(
                None,
                candidate,
                revisions,
                PortAllocationWriteState::Committed,
            ));
        }
        let changes_saved_port = candidate.allocations.values().any(|allocation| {
            allocation
                .prior_fields
                .values()
                .any(|prior| prior.is_some_and(|prior| prior != allocation.port))
        });
        if changes_saved_port {
            let Some(approval) = approval else {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "changing a saved VPN port requires exact local candidate approval",
                ));
            };
            if !approval.covers(candidate, &self.instance_id, ctx, now) {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "allocation approval does not cover this candidate and operation",
                ));
            }
        }

        let originals = deltas
            .iter()
            .map(|delta| {
                Ok(ProtectedOriginal {
                    artifact_id: artifact_id(&delta.document_id).map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::ConfigSemanticConflict,
                            "module settings document identity invalid",
                        )
                    })?,
                    digest: config::digest(delta.original.as_deref().unwrap_or_default()),
                    was_present: delta.original.is_some(),
                    bytes: delta.original.clone().unwrap_or_default(),
                })
            })
            .collect::<CoreResult<Vec<_>>>()?;
        let transaction_id = self.journal.prepare_intent(
            Intent::PortAllocation {
                candidate: candidate.clone(),
                instance_id: Some(self.instance_id.clone()),
            },
            originals,
            ctx,
        )?;
        journal::ensure_mutating(ctx, self.journal.control())?;
        for delta in &deltas {
            if lock
                .compare_exchange(
                    &delta.filename,
                    delta.original.as_deref(),
                    &delta.replacement,
                )
                .is_err()
            {
                let _ = self.journal.finish(
                    &transaction_id,
                    TerminalStatus::RecoveryRequired,
                    vec![],
                    ctx,
                );
                return Err(journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "module settings CAS uncertain; exact allocation recovery required",
                ));
            }
        }
        self.journal
            .finish(&transaction_id, TerminalStatus::Committed, vec![], ctx)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "port allocation settings were written but journal completion is uncertain",
                )
            })?;
        let mut revisions = revisions;
        for delta in &deltas {
            revisions.insert(
                delta.document_id.clone(),
                config::content_revision(delta.document_id.clone(), &delta.replacement),
            );
        }
        Ok(receipt(
            Some(transaction_id),
            candidate,
            revisions,
            PortAllocationWriteState::Committed,
        ))
    }

    /// Reconcile the exact candidate and protected originals stored in a journal
    /// intent. Each document must be byte-for-byte expected-old or exact-new.
    pub fn recover(
        &self,
        transaction_id: &TransactionId,
        ctx: &OperationContext,
    ) -> CoreResult<PortAllocationReceipt> {
        if rustix::process::geteuid().as_raw() != 0
            || ctx.actor.source != ActorSource::Local
            || ctx.actor.actor_id.as_str() != "os.uid.0"
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "local root authority is required to recover VPN port settings",
            ));
        }
        journal::ensure_mutating(ctx, self.journal.control())?;
        let transaction = self.journal.read(transaction_id, ctx)?;
        let Intent::PortAllocation {
            candidate,
            instance_id,
        } = &transaction.intent
        else {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "journal transaction is not a VPN port allocation",
            ));
        };
        if instance_id
            .as_ref()
            .is_some_and(|instance_id| instance_id != &self.instance_id)
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "port allocation belongs to a different installation identity",
            ));
        }
        if !candidate_integrity(candidate) {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "journaled allocation candidate digest is invalid",
            ));
        }
        // A recovery command has a new CLI operation ID. Reconciliation is
        // permitted only for this exact protected transaction, so use its
        // original operation identity when publishing its terminal record.
        let mut recovery_ctx = ctx.clone();
        recovery_ctx.operation_id = transaction.operation_id.clone();
        let prior_terminal = self.journal.terminal(transaction_id, ctx)?;
        if prior_terminal.as_ref().is_some_and(|record| {
            !matches!(
                record.status,
                TerminalStatus::RecoveryRequired | TerminalStatus::Committed
            )
        }) {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "allocation journal has an incompatible terminal state",
            ));
        }
        let lock = self
            .modules
            .lock(journal::deadline(ctx, self.journal.control())?)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "module settings recovery lock unavailable",
                )
            })?;
        let expected = self.recovery_deltas(candidate, &transaction.originals, ctx)?;
        let mut revisions = BTreeMap::new();
        for delta in &expected {
            let current = lock
                .read(&delta.filename, LoadLimits::default().file_bytes)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "module settings recovery read failed",
                    )
                })?;
            if current == delta.original {
                lock.compare_exchange(
                    &delta.filename,
                    delta.original.as_deref(),
                    &delta.replacement,
                )
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "module settings recovery CAS failed; original retained",
                    )
                })?;
            } else if current.as_deref() == Some(delta.replacement.as_slice()) {
                // Re-publish the exact bytes to establish durability after an
                // earlier uncertain fsync; this cannot alter the value.
                lock.compare_exchange(
                    &delta.filename,
                    Some(delta.replacement.as_slice()),
                    &delta.replacement,
                )
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "module settings durability remains uncertain",
                    )
                })?;
            } else {
                if prior_terminal.is_none() {
                    let _ = self.journal.finish(
                        transaction_id,
                        TerminalStatus::RecoveryRequired,
                        vec![],
                        &recovery_ctx,
                    );
                }
                return Err(journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "module settings drifted from both journaled before and after values",
                ));
            }
            revisions.insert(
                delta.document_id.clone(),
                config::content_revision(delta.document_id.clone(), &delta.replacement),
            );
        }
        match prior_terminal {
            Some(record) if record.status == TerminalStatus::RecoveryRequired => {
                self.journal.resolve_operation(
                    transaction_id,
                    TerminalStatus::Committed,
                    vec![],
                    None,
                    &recovery_ctx,
                )?;
            }
            Some(record) if record.status == TerminalStatus::Committed => {}
            Some(_) => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "allocation journal has an incompatible terminal state",
                ));
            }
            None => {
                self.journal.finish(
                    transaction_id,
                    TerminalStatus::Committed,
                    vec![],
                    &recovery_ctx,
                )?;
            }
        }
        Ok(receipt(
            Some(transaction_id.clone()),
            candidate,
            revisions,
            PortAllocationWriteState::Committed,
        ))
    }

    /// Read-only provenance check used to recognize the exact module documents
    /// left by a committed first-install allocation before root config seeding.
    pub fn candidate_is_saved(
        &self,
        transaction_id: &TransactionId,
        candidate: &PortAllocationCandidate,
        ctx: &OperationContext,
    ) -> CoreResult<bool> {
        if !candidate_integrity(candidate) {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "port allocation candidate digest is invalid",
            ));
        }
        let transaction = self.journal.read(transaction_id, ctx)?;
        let Intent::PortAllocation {
            candidate: journaled,
            instance_id,
        } = &transaction.intent
        else {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "provenance transaction is not a port allocation",
            ));
        };
        if journaled != candidate || instance_id.as_ref() != Some(&self.instance_id) {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "provenance transaction does not bind the exact candidate and instance",
            ));
        }
        if !self
            .journal
            .terminal(transaction_id, ctx)?
            .is_some_and(|terminal| terminal.status == TerminalStatus::Committed)
        {
            return Ok(false);
        }
        let expected = self.recovery_deltas(candidate, &transaction.originals, ctx)?;
        let remaining = ctx
            .deadline
            .0
            .saturating_sub(self.journal.control().now_ms())
            .min(30_000);
        if remaining == 0 {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDeadline,
                "module settings provenance deadline elapsed",
            ));
        }
        let lock = self
            .modules
            .lock(Instant::now() + Duration::from_millis(remaining))
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "module settings provenance lock unavailable",
                )
            })?;
        for delta in &expected {
            let current = lock
                .read(&delta.filename, LoadLimits::default().file_bytes)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "module settings provenance read failed",
                    )
                })?;
            if current.as_deref() != Some(delta.replacement.as_slice()) {
                return Ok(false);
            }
        }
        Ok(!expected.is_empty())
    }

    fn inspect_candidate(
        &self,
        candidate: &PortAllocationCandidate,
        lock: &hton_storage::LockedDir,
        ctx: &OperationContext,
    ) -> CoreResult<(Vec<SettingsDelta>, BTreeMap<ObjectId, ContentRevision>)> {
        let mut deltas = Vec::new();
        let mut revisions = BTreeMap::new();
        let mut seen_documents = BTreeSet::new();
        for (service_id, allocation) in &candidate.allocations {
            let (descriptor, binding, filename) =
                self.resolve_binding(service_id, allocation, ctx)?;
            if !seen_documents.insert(allocation.settings_document.clone()) {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "multiple VPN allocations target one settings document",
                ));
            }
            let current = lock
                .read(&filename, LoadLimits::default().file_bytes)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "module settings document read failed",
                    )
                })?;
            self.check_revision(
                candidate,
                &allocation.settings_document,
                current.as_deref(),
                ctx,
            )?;
            let base = match current.as_deref() {
                Some(bytes) => bytes.to_vec(),
                None => default_settings_document(descriptor, ctx)?,
            };
            if let Some(bytes) = current.as_deref() {
                revisions.insert(
                    allocation.settings_document.clone(),
                    config::content_revision(allocation.settings_document.clone(), bytes),
                );
            }
            let replacement =
                replacement_for(&base, &descriptor.module_id, binding, allocation, ctx)?;
            if current.is_none() || replacement != base {
                deltas.push(SettingsDelta {
                    document_id: allocation.settings_document.clone(),
                    filename,
                    original: current,
                    replacement,
                });
            }
        }
        Ok((deltas, revisions))
    }

    fn recovery_deltas(
        &self,
        candidate: &PortAllocationCandidate,
        originals: &[ProtectedOriginal],
        ctx: &OperationContext,
    ) -> CoreResult<Vec<SettingsDelta>> {
        let original_map: BTreeMap<_, _> = originals
            .iter()
            .map(|original| (original.artifact_id.as_str().to_owned(), original))
            .collect();
        if original_map.len() != originals.len() {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "allocation journal contains duplicate protected originals",
            ));
        }
        let mut expected_artifacts = BTreeSet::new();
        let mut deltas = Vec::new();
        let mut seen_documents = BTreeSet::new();
        for (service_id, allocation) in &candidate.allocations {
            let (descriptor, binding, filename) =
                self.resolve_binding(service_id, allocation, ctx)?;
            if !seen_documents.insert(allocation.settings_document.clone()) {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "allocation candidate repeats a settings document",
                ));
            }
            let changed = allocation.prior_fields.values().any(|prior| {
                prior.is_none() || prior.is_some_and(|prior| prior != allocation.port)
            });
            let artifact = artifact_id(&allocation.settings_document).map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "allocation journal document identity is invalid",
                )
            })?;
            if !changed {
                if original_map.contains_key(artifact.as_str()) {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "allocation journal has an unexpected original for a no-op document",
                    ));
                }
                continue;
            }
            expected_artifacts.insert(artifact.as_str().to_owned());
            let original = original_map.get(artifact.as_str()).ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "allocation journal is missing a changed settings original",
                )
            })?;
            if config::digest(&original.bytes) != original.digest {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "allocation journal original digest is invalid",
                ));
            }
            let original_bytes = original.was_present.then_some(original.bytes.as_slice());
            self.check_revision(
                candidate,
                &allocation.settings_document,
                original_bytes,
                ctx,
            )?;
            let base = match original_bytes {
                Some(bytes) => bytes.to_vec(),
                None => default_settings_document(descriptor, ctx)?,
            };
            let replacement =
                replacement_for(&base, &descriptor.module_id, binding, allocation, ctx)?;
            if replacement == base {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "journaled allocation has no expected settings change",
                ));
            }
            deltas.push(SettingsDelta {
                document_id: allocation.settings_document.clone(),
                filename,
                original: original_bytes.map(|bytes| bytes.to_vec()),
                replacement,
            });
        }
        if expected_artifacts.len() != original_map.len()
            || original_map
                .keys()
                .any(|key| !expected_artifacts.contains(key))
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "allocation journal originals do not exactly match candidate changes",
            ));
        }
        Ok(deltas)
    }

    fn resolve_binding<'a>(
        &'a self,
        service_id: &ServiceId,
        allocation: &ServicePortAllocation,
        ctx: &OperationContext,
    ) -> CoreResult<(
        &'a hton_contracts::config::ModuleDescriptor,
        &'a ServicePortBinding,
        String,
    )> {
        let mut found = self.catalogue.entries().values().filter_map(|entry| {
            entry
                .descriptor
                .port_allocation
                .as_ref()
                .filter(|binding| &binding.service_id == service_id)
                .map(|binding| (&entry.descriptor, binding))
        });
        let Some((descriptor, binding)) = found.next() else {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "allocation candidate references an unknown service",
            ));
        };
        let expected_settings_document = format!("module.{}", descriptor.module_id.as_str());
        if found.next().is_some()
            || binding.settings_document != allocation.settings_document
            || binding.binding_group != allocation.binding_group
            || descriptor.descriptor_revision != allocation.descriptor_revision
            || binding.fields.len() != allocation.fields.len()
            || binding.fields.iter().any(|field| {
                !allocation.fields.iter().any(|candidate_field| {
                    candidate_field.path == field.path
                        && candidate_field.transport == field.transport
                })
            })
            || allocation.prior_fields.len() != binding.fields.len()
            || binding.fields.iter().any(|field| {
                allocation
                    .fields
                    .iter()
                    .find(|candidate_field| candidate_field.path == field.path)
                    .map_or(true, |candidate_field| {
                        allocation.prior_fields.get(&field.path)
                            != Some(&candidate_field.saved_port)
                    })
            })
            || !(PORT_MIN..=PORT_MAX).contains(&allocation.port)
            || binding.settings_document.as_str() != expected_settings_document.as_str()
            || descriptor.location.inventory_id.as_str() != "modules"
            || descriptor.location.components.len() != 1
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "allocation candidate differs from its trusted descriptor binding",
            ));
        }
        let filename = descriptor.location.components[0].as_str().to_owned();
        let expected_filename = format!("{}.yaml", descriptor.module_id.as_str());
        if filename != expected_filename {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "settings descriptor location is outside its module inventory entry",
            ));
        }
        Ok((descriptor, binding, filename))
    }

    fn check_revision(
        &self,
        candidate: &PortAllocationCandidate,
        document_id: &ObjectId,
        bytes: Option<&[u8]>,
        ctx: &OperationContext,
    ) -> CoreResult<()> {
        match bytes {
            Some(bytes) => {
                let revision = config::content_revision(document_id.clone(), bytes);
                let Some(expected) = candidate.read_set.objects.get(document_id) else {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "allocation read set omits a settings document",
                    ));
                };
                if expected.revision != revision
                    || candidate.read_set.absent_objects.contains(document_id)
                {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "settings document changed after allocation planning",
                    ));
                }
                Ok(())
            }
            None if candidate.read_set.absent_objects.contains(document_id)
                && !candidate.read_set.objects.contains_key(document_id) =>
            {
                Ok(())
            }
            None => Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "settings document absence was not bound by the allocation candidate",
            )),
        }
    }
}

fn make_allocation(binding: &ServicePortBinding, port: u16) -> ServicePortAllocation {
    ServicePortAllocation {
        settings_document: binding.settings_document.clone(),
        binding_group: binding.binding_group.clone(),
        descriptor_revision: binding
            .descriptor_revision
            .clone()
            .expect("selected binding revision validated"),
        port,
        prior_fields: binding
            .fields
            .iter()
            .map(|field| (field.path.clone(), field.saved_port))
            .collect(),
        fields: binding.fields.clone(),
    }
}

fn canonical_candidate_digest(
    candidate: &PortAllocationCandidate,
) -> Result<Sha256Digest, AllocationBlocker> {
    digest_serialized(&(
        &candidate.allocations,
        &candidate.retained,
        &candidate.read_set,
        &candidate.registry_revision,
        &candidate.reservation_digest,
        &candidate.created_at,
        &candidate.expires_at,
    ))
}

fn candidate_integrity(candidate: &PortAllocationCandidate) -> bool {
    if candidate.read_set.coherence != ReadCoherence::Coherent
        || candidate
            .read_set
            .absent_objects
            .iter()
            .any(|id| candidate.read_set.objects.contains_key(id))
        || candidate.allocations.len() > 64
        || candidate.retained.len() > 512
        || canonical_candidate_digest(candidate).ok().as_ref() != Some(&candidate.candidate_digest)
    {
        return false;
    }
    let mut services = BTreeSet::new();
    let mut ports = BTreeSet::new();
    for saved in &candidate.retained {
        if !(PORT_MIN..=PORT_MAX).contains(&saved.port)
            || !services.insert(saved.service_id.clone())
            || !ports.insert(saved.port)
        {
            return false;
        }
    }
    for (service, allocation) in &candidate.allocations {
        if !(PORT_MIN..=PORT_MAX).contains(&allocation.port)
            || !services.insert(service.clone())
            || !ports.insert(allocation.port)
            || allocation.fields.is_empty()
            || allocation.fields.len() > 16
            || allocation.prior_fields.len() != allocation.fields.len()
        {
            return false;
        }
        let mut paths = BTreeSet::new();
        let mut prior_values = BTreeSet::new();
        let mut missing = 0usize;
        for field in &allocation.fields {
            if !paths.insert(field.path.as_str())
                || allocation.prior_fields.get(&field.path) != Some(&field.saved_port)
            {
                return false;
            }
            match field.saved_port {
                Some(value) => {
                    if !(PORT_MIN..=PORT_MAX).contains(&value) {
                        return false;
                    }
                    prior_values.insert(value);
                }
                None => missing += 1,
            }
        }
        if prior_values.len() > 1 || (missing != 0 && missing != allocation.fields.len()) {
            return false;
        }
    }
    true
}

fn artifact_id(
    document_id: &ObjectId,
) -> std::result::Result<ArtifactId, hton_contracts::operations::InvalidIdentifier> {
    let digest = config::digest(document_id.as_str().as_bytes());
    ArtifactId::new(format!("port-settings-{}", &digest.as_str()[7..39]))
}

fn receipt(
    transaction_id: Option<TransactionId>,
    candidate: &PortAllocationCandidate,
    document_revisions: BTreeMap<ObjectId, ContentRevision>,
    state: PortAllocationWriteState,
) -> PortAllocationReceipt {
    PortAllocationReceipt {
        transaction_id,
        allocations: candidate
            .allocations
            .iter()
            .map(|(service, allocation)| (service.clone(), allocation.port))
            .collect(),
        document_revisions,
        state,
    }
}

fn replacement_for(
    original: &[u8],
    module_id: &ModuleId,
    binding: &ServicePortBinding,
    allocation: &ServicePortAllocation,
    ctx: &OperationContext,
) -> CoreResult<Vec<u8>> {
    let mut root = config::decode_yaml(original, LoadLimits::default()).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "module settings YAML is invalid; original retained",
        )
    })?;
    if root.get("moduleId").and_then(Value::as_str) != Some(module_id.as_str()) {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "module settings identity does not match its descriptor",
        ));
    }
    let mut changed = false;
    for declared in &binding.fields {
        let candidate_field = allocation
            .fields
            .iter()
            .find(|field| field.path == declared.path && field.transport == declared.transport)
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "allocation field is outside its descriptor mapping",
                )
            })?;
        let prior = allocation
            .prior_fields
            .get(&declared.path)
            .copied()
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "allocation prior-value map omits a descriptor field",
                )
            })?;
        let actual = setting_port(&root, &declared.path).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "module port setting has an invalid type or value",
            )
        })?;
        if prior != actual || candidate_field.saved_port != actual {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "module port setting differs from the allocation read set",
            ));
        }
        match actual {
            Some(saved) if saved == allocation.port => {}
            Some(_) => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "saved VPN port cannot be silently replaced",
                ));
            }
            None => {
                set_setting_port(&mut root, &declared.path, allocation.port).map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::ConfigSemanticConflict,
                        "descriptor port field cannot be safely updated",
                    )
                })?;
                changed = true;
            }
        }
    }
    if !changed {
        return Ok(original.to_vec());
    }
    let replacement = serde_yaml::to_string(&root)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "module settings encoding failed",
            )
        })?
        .into_bytes();
    if replacement.len() > LoadLimits::default().file_bytes {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "updated module settings exceed document limits",
        ));
    }
    Ok(replacement)
}

fn default_settings_document(
    descriptor: &hton_contracts::config::ModuleDescriptor,
    ctx: &OperationContext,
) -> CoreResult<Vec<u8>> {
    if descriptor.defaults.schema_version != descriptor.config_version {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "module descriptor defaults have an unsupported schema version",
        ));
    }
    let root = Value::Object(serde_json::Map::from_iter([
        (
            "schemaVersion".into(),
            Value::from(descriptor.defaults.schema_version.0),
        ),
        (
            "moduleId".into(),
            Value::String(descriptor.module_id.as_str().to_owned()),
        ),
        (
            "settings".into(),
            Value::Object(descriptor.defaults.settings.clone().into_iter().collect()),
        ),
    ]));
    let bytes = serde_yaml::to_string(&root)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "module descriptor defaults encoding failed",
            )
        })?
        .into_bytes();
    if bytes.len() > LoadLimits::default().file_bytes {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "module descriptor defaults exceed document limits",
        ));
    }
    Ok(bytes)
}

fn setting_port(root: &Value, path: &str) -> std::result::Result<Option<u16>, ()> {
    let segments: Vec<_> = path.split('.').collect();
    if segments.len() < 3
        || segments.len() > 16
        || segments[0] != "settings"
        || segments.iter().any(|part| part.is_empty())
    {
        return Err(());
    }
    let mut current = root;
    for segment in segments {
        if !current.is_object() {
            return Err(());
        }
        let Some(next) = current.get(segment) else {
            return Ok(None);
        };
        current = next;
    }
    let value = current.as_u64().ok_or(())?;
    if !(1..=u16::MAX as u64).contains(&value) {
        return Err(());
    }
    Ok(Some(value as u16))
}

fn set_setting_port(root: &mut Value, path: &str, port: u16) -> std::result::Result<(), ()> {
    let segments: Vec<_> = path.split('.').collect();
    if segments.len() < 3
        || segments.len() > 16
        || segments[0] != "settings"
        || segments.iter().any(|part| part.is_empty())
    {
        return Err(());
    }
    let mut current = root.as_object_mut().ok_or(())?;
    if !current.contains_key("settings") {
        current.insert("settings".into(), Value::Object(Default::default()));
    }
    let mut value = current.get_mut("settings").ok_or(())?;
    if !value.is_object() {
        return Err(());
    }
    for segment in &segments[1..segments.len() - 1] {
        let object = value.as_object_mut().ok_or(())?;
        if !object.contains_key(*segment) {
            object.insert((*segment).into(), Value::Object(Default::default()));
        }
        value = object.get_mut(*segment).ok_or(())?;
        if !value.is_object() {
            return Err(());
        }
    }
    let leaf = segments.last().ok_or(())?;
    value
        .as_object_mut()
        .ok_or(())?
        .insert((*leaf).into(), Value::from(port));
    Ok(())
}

fn unbiased_index(
    bound: usize,
    entropy: &[u8],
    offset: &mut usize,
) -> Result<usize, AllocationBlocker> {
    if bound == 0 {
        return Err(AllocationBlocker::NoFreePort);
    }
    let bound = bound as u64;
    // Largest multiple of `bound` representable as a count of u64 values.
    let limit = u64::MAX - (u64::MAX % bound);
    loop {
        let end = offset
            .checked_add(8)
            .ok_or(AllocationBlocker::EntropyUnavailable)?;
        let chunk = entropy
            .get(*offset..end)
            .ok_or(AllocationBlocker::EntropyUnavailable)?;
        *offset = end;
        let value = u64::from_le_bytes(chunk.try_into().expect("8-byte slice"));
        if value < limit {
            return Ok((value % bound) as usize);
        }
    }
}

fn digest_serialized<T: Serialize>(value: &T) -> Result<Sha256Digest, AllocationBlocker> {
    let bytes = serde_json::to_vec(value).map_err(|_| AllocationBlocker::InvalidBinding)?;
    let digest = Sha256::digest(bytes);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Sha256Digest::new(format!("sha256:{hex}")).map_err(|_| AllocationBlocker::InvalidBinding)
}
