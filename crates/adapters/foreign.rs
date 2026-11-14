//! Conservative foreign-stack qualification and planning. This module performs no native effects.
use crate::journal::{OperationControl, failure};
use hton_contracts::{config::*, operations::*};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::Ipv4Addr,
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeTransport {
    Reality,
    Tls,
    Unsupported,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeEndpoint {
    pub address: Ipv4Addr,
    pub port: u16,
}
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTarget {
    pub hostname: String,
    pub port: u16,
    pub resolves_to_edge: bool,
}
impl std::fmt::Debug for NativeTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeTarget([protected routing metadata])")
    }
}
/// Safe, display-only row from the local foreign-resource inventory. It carries
/// enough metadata for explicit selection but no module identity or authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignCandidateSummary {
    pub container_id: DockerContainerId,
    pub image_digest: Sha256Digest,
    pub state: ObservedState,
    pub published_bindings: BTreeSet<HostBinding>,
    pub structural_fingerprint: Sha256Digest,
    pub observed_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignCandidateInventory {
    pub observed_at: UnixMillis,
    pub valid_until: UnixMillis,
    pub complete: bool,
    pub candidates: Vec<ForeignCandidateSummary>,
}
/// Coverage claims are emitted only by the exact read-only native observer.
/// Any false field blocks qualification; image name and open ports are not substitutes.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualificationCoverage {
    pub image_identity: bool,
    pub effective_launch: bool,
    pub restart_policy: bool,
    pub host_bindings: bool,
    pub native_listener: bool,
    pub networks: bool,
    pub mounts: bool,
    pub data_owners: bool,
    pub protocol_facts: bool,
}
impl QualificationCoverage {
    fn complete(&self) -> bool {
        self.image_identity
            && self.effective_launch
            && self.restart_policy
            && self.host_bindings
            && self.native_listener
            && self.networks
            && self.mounts
            && self.data_owners
            && self.protocol_facts
    }
}
/// Read-only proof that an implemented protected snapshot baseline applies to
/// this exact observed launch. It is not an actual snapshot or effect authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotCapabilityEvidence {
    pub source_fingerprint: Sha256Digest,
    pub kind: SnapshotCapabilityKind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SnapshotCapabilityKind {
    DockerCommitImageSaveNoMounts,
}
/// Returned by a trusted read-only observer, never deserialized from frontend JSON.
pub struct ForeignEvidence {
    pub module_id: ModuleId,
    pub container_id: DockerContainerId,
    pub structural_fingerprint: Sha256Digest,
    pub native_version: ObjectId,
    pub transport: NativeTransport,
    pub host_bindings: BTreeSet<HostBinding>,
    pub coverage: QualificationCoverage,
    pub resources: Vec<ResourceObservation>,
    pub server_names: BTreeSet<String>,
    pub target: NativeTarget,
    pub endpoint: NativeEndpoint,
    pub original_policies: BTreeMap<ResourceId, RestartPolicy>,
    pub restart_owners: BTreeSet<ResourceId>,
    pub snapshot_capability: SnapshotCapabilityEvidence,
    pub checks: Vec<Evidence>,
}
impl std::fmt::Debug for ForeignEvidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForeignEvidence")
            .field("module_id", &self.module_id)
            .field("native_version", &self.native_version)
            .field("details", &"[protected]")
            .finish()
    }
}
pub trait ForeignObservePort {
    /// Enumerate a stable, bounded, read-only candidate inventory. Implementations
    /// must not infer module ownership from image names, names, labels or ports.
    fn discover_candidates(
        &self,
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ForeignCandidateInventory> {
        let _ = budget;
        Err(failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "foreign candidate discovery observer is not bound",
        ))
    }
    /// Inspect exactly the immutable ID selected by the operator. The result must
    /// carry fresh fingerprint-bound facts; no resource is registered or changed.
    fn inspect_candidate(
        &self,
        selected_id: &DockerContainerId,
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ForeignEvidence> {
        let _ = (selected_id, budget);
        Err(failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "foreign exact-ID qualification observer is not bound",
        ))
    }
    fn observe_registered(
        &self,
        module: &ModuleId,
        exact_resources: &[ResourceIdentity],
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ForeignEvidence>;
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualifiedForeign {
    pub module_id: ModuleId,
    pub container_id: DockerContainerId,
    pub structural_fingerprint: Sha256Digest,
    pub native_version: ObjectId,
    pub transport: NativeTransport,
    pub coverage: QualificationCoverage,
    pub fingerprint: Sha256Digest,
    pub exact_resources: Vec<ResourceIdentity>,
    pub observed_at: UnixMillis,
    pub valid_until: UnixMillis,
    pub server_names: BTreeSet<String>,
    pub endpoint: NativeEndpoint,
    pub host_bindings: BTreeSet<HostBinding>,
    pub original_policies: BTreeMap<ResourceId, RestartPolicy>,
    pub snapshot_capability: SnapshotCapabilityEvidence,
}
/// Public contract name for the exact, expiring, fingerprint-bound result.
pub type ForeignQualificationEvidence = QualifiedForeign;
/// Exact published host bindings. Container protocol/port and the rest of the
/// container launch configuration are deliberately not represented as inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportProtocol {
    Tcp,
    Udp,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostBinding {
    pub protocol: TransportProtocol,
    /// Native container listening port; remapping may never alter it.
    pub container_port: u16,
    pub host: Ipv4Addr,
    pub port: u16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignRemapPlan {
    pub module_id: ModuleId,
    pub exact_resources: Vec<ResourceIdentity>,
    pub before_fingerprint: Sha256Digest,
    pub from: BTreeSet<HostBinding>,
    pub to: BTreeSet<HostBinding>,
    pub snapshot_digest: Sha256Digest,
    pub approval_id: ApprovalId,
}
/// Opaque reference to protected, durable state. It contains no config or key bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignSnapshotHandle {
    pub artifact_id: ArtifactId,
    pub exact_resources: Vec<ResourceIdentity>,
    pub source_fingerprint: Sha256Digest,
    pub digest: Sha256Digest,
    /// Digest of the complete supported create/runtime manifest, including the
    /// committed writable layer identity. Never a serialization of secret data.
    pub protected_manifest_digest: Sha256Digest,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignRemapReceipt {
    pub plan_fingerprint: Sha256Digest,
    pub exact_resources: Vec<ResourceIdentity>,
    pub prior_container_ids: BTreeSet<String>,
    pub replacement_container_ids: BTreeSet<String>,
    pub before_fingerprint: Sha256Digest,
    pub after_fingerprint: Sha256Digest,
    pub from: BTreeSet<HostBinding>,
    pub to: BTreeSet<HostBinding>,
    pub snapshot_digest: Sha256Digest,
    pub preserved_manifest_digest: Sha256Digest,
    pub changed_at: UnixMillis,
}

/// Checks the receipt boundary independently of the native executor. Unknown
/// or drifted launch state cannot be described as a successful port-only change.
pub fn validate_remap_receipt(
    plan: &ForeignRemapPlan,
    snapshot: &ForeignSnapshotHandle,
    receipt: &ForeignRemapReceipt,
) -> std::result::Result<(), Error> {
    let expected_ids: BTreeSet<_> = plan
        .exact_resources
        .iter()
        .map(|r| r.resource_id.clone())
        .collect();
    let original_container_ids: BTreeSet<_> = plan
        .exact_resources
        .iter()
        .filter_map(|resource| match &resource.locator {
            ResourceLocator::DockerContainer { container_id } => {
                Some(container_id.as_str().to_owned())
            }
            _ => None,
        })
        .collect();
    if plan.exact_resources.is_empty()
        || original_container_ids.len() != plan.exact_resources.len()
        || receipt.prior_container_ids != original_container_ids
        || receipt.replacement_container_ids.len() != original_container_ids.len()
        || !receipt
            .prior_container_ids
            .is_disjoint(&receipt.replacement_container_ids)
        || expected_ids.len() != plan.exact_resources.len()
        || plan.from.is_empty()
        || plan.to.is_empty()
        || plan.from == plan.to
        || plan.from.iter().chain(&plan.to).any(|binding| {
            binding.port == 0 || binding.container_port == 0 || binding.host.is_multicast()
        })
        || !same_binding_targets(&plan.from, &plan.to)
        || receipt.plan_fingerprint != remap_plan_fingerprint(plan)
        || snapshot.exact_resources != plan.exact_resources
        || snapshot.source_fingerprint != plan.before_fingerprint
        || snapshot.digest != plan.snapshot_digest
        || receipt.exact_resources != plan.exact_resources
        || receipt.before_fingerprint != plan.before_fingerprint
        || receipt.from != plan.from
        || receipt.to != plan.to
        || receipt.snapshot_digest != snapshot.digest
        || receipt.preserved_manifest_digest != snapshot.protected_manifest_digest
        || receipt.prior_container_ids.is_empty()
        || receipt.replacement_container_ids.is_empty()
        || receipt
            .prior_container_ids
            .iter()
            .any(|id| !valid_full_container_id(id))
        || receipt
            .replacement_container_ids
            .iter()
            .any(|id| !valid_full_container_id(id))
        || receipt.prior_container_ids == receipt.replacement_container_ids
        || receipt.after_fingerprint == receipt.before_fingerprint
        || receipt.changed_at.0 == 0
    {
        return Err(preview_error(
            ErrorCode::ResourceDrift,
            "foreign remap receipt does not prove an exact port-only replacement",
        ));
    }
    Ok(())
}
/// Require the same protocol, native port and host address multiset. Only host
/// port values may differ, so receipt validation cannot bless a native-listener
/// change or a bind-address change as a publication remap.
fn same_binding_targets(from: &BTreeSet<HostBinding>, to: &BTreeSet<HostBinding>) -> bool {
    let targets = |bindings: &BTreeSet<HostBinding>| {
        let mut targets = bindings
            .iter()
            .map(|b| (b.protocol, b.container_port, b.host))
            .collect::<Vec<_>>();
        targets.sort_unstable();
        targets
    };
    targets(from) == targets(to)
}
pub fn remap_plan_fingerprint(plan: &ForeignRemapPlan) -> Sha256Digest {
    // Fixed-order canonical input; the IDs and endpoints are bounded typed values.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(plan.module_id.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(plan.before_fingerprint.as_str().as_bytes());
    bytes.extend_from_slice(plan.snapshot_digest.as_str().as_bytes());
    for resource in &plan.exact_resources {
        bytes.extend_from_slice(resource.resource_id.as_str().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(resource.fingerprint.as_str().as_bytes());
    }
    for (tag, bindings) in [(b'F', &plan.from), (b'T', &plan.to)] {
        bytes.push(tag);
        for binding in bindings {
            bytes.push(match binding.protocol {
                TransportProtocol::Tcp => 1,
                TransportProtocol::Udp => 2,
            });
            bytes.extend_from_slice(binding.container_port.to_be_bytes().as_slice());
            bytes.extend_from_slice(binding.host.to_string().as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(binding.port.to_be_bytes().as_slice());
        }
    }
    bytes.extend_from_slice(plan.approval_id.as_str().as_bytes());
    Sha256Digest::new(format!("sha256:{:x}", Sha256::digest(bytes))).expect("canonical SHA256")
}
fn valid_full_container_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub struct ForeignAdapter {
    pub observer: Arc<dyn ForeignObservePort>,
    pub control: Arc<dyn OperationControl>,
}
impl ForeignAdapter {
    /// Read-only bounded inventory. Returned rows are display and selection
    /// evidence only and cannot be passed directly to lifecycle or allocation.
    pub fn discover_candidates(
        &self,
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ForeignCandidateInventory> {
        validate_foreign_budget(budget, self.control.as_ref(), ctx)?;
        let mut inventory = self.observer.discover_candidates(budget, ctx)?;
        let now = UnixMillis(self.control.now_ms());
        let inventory_bytes = serde_json::to_vec(&inventory).map_err(|_| {
            failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "foreign candidate inventory could not be bounded",
            )
        })?;
        if !inventory.complete
            || inventory.candidates.len() > budget.max_observations
            || inventory_bytes.len() > budget.max_bytes
            || inventory.observed_at.0 > now.0
            || inventory.observed_at.0 < now.0.saturating_sub(30_000)
            || inventory.valid_until.0 <= now.0
            || inventory.valid_until.0 > budget.deadline.0
        {
            return Err(failure(
                ctx,
                ErrorCode::IoUnavailable,
                "foreign candidate inventory is incomplete, stale or outside its budget",
            ));
        }
        let mut ids = BTreeSet::new();
        for candidate in &inventory.candidates {
            if !ids.insert(candidate.container_id.as_str().to_owned())
                || candidate.observed_at.0 > now.0
                || candidate.observed_at.0 < inventory.observed_at.0
                || candidate.observed_at.0 < now.0.saturating_sub(30_000)
                || candidate.structural_fingerprint.as_str().len() != 71
                || candidate.image_digest.as_str().len() != 71
                || candidate.published_bindings.iter().any(|binding| {
                    binding.port == 0 || binding.container_port == 0 || binding.host.is_multicast()
                })
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "foreign candidate inventory contains duplicate or invalid exact facts",
                ));
            }
        }
        inventory
            .candidates
            .sort_by(|left, right| left.container_id.as_str().cmp(right.container_id.as_str()));
        Ok(inventory)
    }

    /// Re-inspect one exact operator-selected immutable Docker ID and qualify only
    /// complete fresh native evidence. The discovery row is not accepted as input.
    pub fn qualify_candidate(
        &self,
        selected_id: &DockerContainerId,
        reserved_exact_snis: &BTreeSet<String>,
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ForeignQualificationEvidence> {
        validate_foreign_budget(budget, self.control.as_ref(), ctx)?;
        let evidence = self.observer.inspect_candidate(selected_id, budget, ctx)?;
        let evidence_bytes = serde_json::to_vec(&(
            &evidence.module_id,
            evidence.container_id.as_str(),
            &evidence.structural_fingerprint,
            &evidence.native_version,
            evidence.transport,
            &evidence.host_bindings,
            &evidence.coverage,
            &evidence.resources,
            &evidence.server_names,
            &evidence.target,
            &evidence.endpoint,
            &evidence.original_policies,
            &evidence.restart_owners,
            &evidence.snapshot_capability,
            &evidence.checks,
        ))
        .map_err(|_| {
            failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "foreign qualification evidence could not be bounded",
            )
        })?;
        if evidence_bytes.len() > budget.max_bytes
            || evidence.checks.len() > budget.max_observations
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDeadline,
                "foreign qualification evidence exceeds its observation budget",
            ));
        }
        if evidence.container_id != *selected_id
            || evidence.resources.len() != 1
            || !matches!(
                &evidence.resources[0].identity.locator,
                ResourceLocator::DockerContainer { container_id }
                    if container_id == selected_id
            )
            || evidence.resources[0].identity.fingerprint != evidence.structural_fingerprint
        {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "foreign qualification evidence does not match the selected immutable container ID",
            ));
        }
        qualify(
            &evidence,
            reserved_exact_snis,
            UnixMillis(self.control.now_ms()),
            ctx,
        )
    }

    pub fn observe(
        &self,
        module: &ModuleId,
        exact_resources: &[ResourceIdentity],
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ForeignEvidence> {
        validate_foreign_budget(budget, self.control.as_ref(), ctx)?;
        let evidence = self
            .observer
            .observe_registered(module, exact_resources, budget, ctx)?;
        let supplied: BTreeSet<_> = evidence
            .resources
            .iter()
            .map(|o| o.identity.resource_id.clone())
            .collect();
        let expected: BTreeSet<_> = exact_resources
            .iter()
            .map(|r| r.resource_id.clone())
            .collect();
        if evidence.module_id != *module
            || supplied != expected
            || expected.len() != exact_resources.len()
            || exact_resources
                .iter()
                .any(|r| !evidence.resources.iter().any(|o| o.identity == *r))
        {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "foreign native identity differs from the registered scope",
            ));
        }
        Ok(evidence)
    }
}
fn validate_foreign_budget(
    budget: &Budget,
    control: &dyn OperationControl,
    ctx: &OperationContext,
) -> Result<()> {
    if budget.max_concurrency == 0
        || budget.max_concurrency > 8
        || budget.max_bytes == 0
        || budget.max_bytes > 4 * 1024 * 1024
        || budget.max_observations == 0
        || budget.max_observations > 256
        || budget.deadline.0 <= control.now_ms()
        || control.is_cancelled(&ctx.cancellation)
    {
        return Err(failure(
            ctx,
            ErrorCode::OperationDeadline,
            "foreign observation budget rejected",
        ));
    }
    Ok(())
}
pub fn evidence_fingerprint(evidence: &ForeignEvidence) -> Result<Sha256Digest> {
    let bytes = serde_json::to_vec(&(
        &evidence.module_id,
        evidence.container_id.as_str(),
        &evidence.structural_fingerprint,
        &evidence.native_version,
        evidence.transport,
        &evidence.host_bindings,
        &evidence.coverage,
        &evidence.resources,
        &evidence.server_names,
        &evidence.target,
        &evidence.endpoint,
        &evidence.original_policies,
        &evidence.restart_owners,
        &evidence.snapshot_capability,
    ))
    .map_err(|_| {
        preview_error(
            ErrorCode::ConfigSemanticConflict,
            "foreign evidence encoding failed",
        )
    })?;
    Sha256Digest::new(format!("sha256:{:x}", Sha256::digest(&bytes))).map_err(|_| {
        preview_error(
            ErrorCode::ConfigSemanticConflict,
            "foreign evidence digest invalid",
        )
    })
}
/// D80 permits native REALITY only after exact version/routing/restart evidence.
pub fn qualify(
    evidence: &ForeignEvidence,
    reserved_exact_snis: &BTreeSet<String>,
    now: UnixMillis,
    ctx: &OperationContext,
) -> Result<QualifiedForeign> {
    if evidence.module_id.as_str() != "foreign-amnezia"
        || evidence.resources.len() != 1
        || !matches!(
            &evidence.resources[0].identity.locator,
            ResourceLocator::DockerContainer { container_id }
                if container_id == &evidence.container_id
        )
        || evidence.resources[0].identity.fingerprint != evidence.structural_fingerprint
        || evidence.transport != NativeTransport::Reality
        || !evidence.coverage.complete()
        || evidence.snapshot_capability.source_fingerprint != evidence.structural_fingerprint
        || evidence.host_bindings.is_empty()
        || evidence.host_bindings.iter().any(|binding| {
            binding.port == 0 || binding.container_port == 0 || binding.host.is_multicast()
        })
        || evidence.resources.is_empty()
        || evidence.resources.len() > 128
        || evidence.server_names.is_empty()
        || evidence.server_names.len() > 64
        || evidence.endpoint.port == 0
        || evidence.endpoint.address.is_unspecified()
        || evidence.endpoint.address.is_multicast()
        || !(evidence.endpoint.address.is_private() || evidence.endpoint.address.is_loopback())
        || evidence.target.port == 0
        || evidence.target.resolves_to_edge
        || !hostname(&evidence.target.hostname)
        || evidence
            .server_names
            .iter()
            .any(|sni| !hostname(sni) || reserved_exact_snis.contains(sni))
    {
        return Err(failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "native foreign transport or SNI/target routing is not qualified",
        ));
    }
    let ids: BTreeSet<_> = evidence
        .resources
        .iter()
        .map(|o| o.identity.resource_id.clone())
        .collect();
    if ids.len() != evidence.resources.len()
        || !evidence.restart_owners.is_subset(&ids)
        || ids
            .iter()
            .any(|id| !evidence.original_policies.contains_key(id))
        || evidence.resources.iter().any(|o| {
            o.observed_at.0 > now.0
                || o.valid_until.0 <= now.0
                || o.observed_at.0 < now.0.saturating_sub(300_000)
                || o.restart_policy.as_ref()
                    != evidence.original_policies.get(&o.identity.resource_id)
                || o.state == ObservedState::Unknown
        })
    {
        return Err(failure(
            ctx,
            ErrorCode::ResourceDrift,
            "native restart ownership or fresh resource evidence is incomplete",
        ));
    }
    let fingerprint = evidence_fingerprint(evidence).map_err(|mut error| {
        error.operation_id = ctx.operation_id.clone();
        error
    })?;
    for name in [
        "amnezia.native-version",
        "amnezia.reality-routing",
        "amnezia.restart-owners",
        "amnezia.launch-coverage",
        "amnezia.host-bindings",
        "amnezia.native-listener",
        "amnezia.network-mounts",
        "amnezia.data-owners",
        "amnezia.snapshot-capability",
    ] {
        if !evidence.checks.iter().any(|check| {
            check.check_id.as_str() == name
                && check.module_id.as_ref() == Some(&evidence.module_id)
                && check.outcome == EvidenceOutcome::Pass
                && check.input_fingerprint == fingerprint
                && check.timestamp.0 <= now.0
                && check.timestamp.0 >= now.0.saturating_sub(300_000)
        }) {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "native compatibility check is absent, stale or not bound to exact evidence",
            ));
        }
    }
    Ok(QualifiedForeign {
        module_id: evidence.module_id.clone(),
        container_id: evidence.container_id.clone(),
        structural_fingerprint: evidence.structural_fingerprint.clone(),
        native_version: evidence.native_version.clone(),
        transport: evidence.transport,
        coverage: evidence.coverage.clone(),
        fingerprint,
        exact_resources: evidence
            .resources
            .iter()
            .map(|o| o.identity.clone())
            .collect(),
        observed_at: now,
        valid_until: UnixMillis(
            evidence
                .resources
                .iter()
                .map(|o| o.valid_until.0)
                .min()
                .unwrap_or(now.0)
                .min(now.0.saturating_add(300_000)),
        ),
        server_names: evidence.server_names.clone(),
        endpoint: evidence.endpoint.clone(),
        host_bindings: evidence.host_bindings.clone(),
        original_policies: evidence.original_policies.clone(),
        snapshot_capability: evidence.snapshot_capability.clone(),
    })
}
pub fn plan_association(
    qualified: &QualifiedForeign,
    approval: &ApprovalEnvelope,
    actor: &ActorContext,
    now: UnixMillis,
    ctx: &OperationContext,
) -> Result<RegistryChange> {
    verify_approval(qualified, approval, actor, now, ctx)?;
    let resources: BTreeSet<_> = qualified
        .exact_resources
        .iter()
        .map(|r| r.resource_id.clone())
        .collect();
    Ok(RegistryChange::Adopt {
        entry: RegistryEntry {
            module_id: qualified.module_id.clone(),
            ownership: Ownership::Associated,
            membership: MembershipState::Installed,
            resources: qualified.exact_resources.clone(),
            setup_state: SetupState::Complete,
            checkpoints: vec![],
            installed_artifact_versions: BTreeMap::new(),
            original_policies: qualified.original_policies.clone(),
            adoption_scope: Some(AdoptionScope {
                approval_id: approval.approval_id.clone(),
                resources,
                lifecycle_actions: BTreeSet::from([
                    ForeignActionPermission::Start,
                    ForeignActionPermission::Stop,
                    ForeignActionPermission::DisableAutostart,
                    ForeignActionPermission::RestoreAutostart,
                ]),
                original_evidence: approval.evidence.clone(),
            }),
            retained_artifacts: vec![],
        },
    })
}
pub fn plan_remap(
    qualified: &QualifiedForeign,
    snapshot: &ForeignSnapshotHandle,
    targets: BTreeSet<HostBinding>,
    approval: &ApprovalEnvelope,
    actor: &ActorContext,
    now: UnixMillis,
    ctx: &OperationContext,
) -> Result<ForeignRemapPlan> {
    verify_approval(qualified, approval, actor, now, ctx)?;
    if snapshot.exact_resources != qualified.exact_resources
        || snapshot.source_fingerprint != qualified.structural_fingerprint
        || snapshot.source_fingerprint != qualified.snapshot_capability.source_fingerprint
    {
        return Err(failure(
            ctx,
            ErrorCode::ResourceDrift,
            "foreign remap requires actual protected snapshot of exact qualified resources",
        ));
    }
    if targets.is_empty()
        || targets == qualified.host_bindings
        || targets.iter().any(|binding| {
            binding.port == 0 || binding.container_port == 0 || binding.host.is_multicast()
        })
        || !same_binding_targets(&qualified.host_bindings, &targets)
    {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "foreign remap may change only exact published host ports",
        ));
    }
    let fingerprint = remap_fingerprint(qualified, snapshot, &targets);
    for check in ["amnezia.remap-compatible", "amnezia.remap-consent"] {
        if !approval.evidence.iter().any(|evidence| {
            evidence.check_id.as_str() == check
                && evidence.module_id.as_ref() == Some(&qualified.module_id)
                && evidence.outcome == EvidenceOutcome::Pass
                && evidence.input_fingerprint == fingerprint
                && evidence.timestamp.0 <= now.0
                && evidence.timestamp.0 >= now.0.saturating_sub(60_000)
        }) {
            return Err(failure(
                ctx,
                ErrorCode::ApprovalRequired,
                "foreign remap requires exact fresh compatibility and separate local remap consent",
            ));
        }
    }
    Ok(ForeignRemapPlan {
        module_id: qualified.module_id.clone(),
        exact_resources: qualified.exact_resources.clone(),
        before_fingerprint: qualified.structural_fingerprint.clone(),
        from: qualified.host_bindings.clone(),
        to: targets,
        snapshot_digest: snapshot.digest.clone(),
        approval_id: approval.approval_id.clone(),
    })
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignRestorePlan {
    pub module_id: ModuleId,
    pub remap_plan_fingerprint: Sha256Digest,
    pub current_resources: Vec<ResourceIdentity>,
    pub expected_current_fingerprint: Sha256Digest,
    pub original_snapshot: ArtifactId,
    pub snapshot_digest: Sha256Digest,
    pub preserved_manifest_digest: Sha256Digest,
    pub bindings: BTreeSet<HostBinding>,
    pub approval_id: ApprovalId,
}
/// Plan an exact inverse using fresh replacement identities. This executes no
/// native restoration and never treats old pre-remap IDs as current holders.
pub fn restore(
    plan: &ForeignRemapPlan,
    snapshot: &ForeignSnapshotHandle,
    receipt: &ForeignRemapReceipt,
    actual: &[ResourceObservation],
    approval: &ApprovalEnvelope,
    actor: &ActorContext,
    now: UnixMillis,
    ctx: &OperationContext,
) -> Result<ForeignRestorePlan> {
    validate_remap_receipt(plan, snapshot, receipt).map_err(|mut error| {
        error.operation_id = ctx.operation_id.clone();
        error
    })?;
    let logical_ids: BTreeSet<_> = plan
        .exact_resources
        .iter()
        .map(|resource| resource.resource_id.clone())
        .collect();
    let actual_logical_ids: BTreeSet<_> = actual
        .iter()
        .map(|resource| resource.identity.resource_id.clone())
        .collect();
    let actual_ids: BTreeSet<_> = actual
        .iter()
        .filter_map(|resource| match &resource.identity.locator {
            ResourceLocator::DockerContainer { container_id } => {
                Some(container_id.as_str().to_owned())
            }
            _ => None,
        })
        .collect();
    if actual.len() != plan.exact_resources.len()
        || actual_logical_ids != logical_ids
        || actual_ids != receipt.replacement_container_ids
        || actual.iter().any(|observation| {
            observation.identity.fingerprint != receipt.after_fingerprint
                || observation.observed_at.0 > now.0
                || observation.valid_until.0 <= now.0
                || observation.observed_at.0 < now.0.saturating_sub(30_000)
                || observation.state == ObservedState::Unknown
        })
    {
        return Err(failure(
            ctx,
            ErrorCode::ResourceDrift,
            "foreign restore requires exact fresh post-remap replacement identities",
        ));
    }
    let restore_fingerprint = restore_fingerprint(receipt, snapshot);
    if !actor.local_approval_refs.contains(&approval.approval_id)
        || approval.resources != logical_ids
        || approval.expires_at.is_none_or(|expiry| expiry.0 <= now.0)
        || !approval
            .privileges
            .contains(&Privilege::ManageForeignLifecycle)
        || !approval.evidence.iter().any(|evidence| {
            evidence.check_id.as_str() == "amnezia.restore-consent"
                && evidence.module_id.as_ref() == Some(&plan.module_id)
                && evidence.outcome == EvidenceOutcome::Pass
                && evidence.input_fingerprint == restore_fingerprint
                && evidence.timestamp.0 <= now.0
                && evidence.timestamp.0 >= now.0.saturating_sub(60_000)
        })
    {
        return Err(failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "foreign restore requires separate exact current-resource local consent",
        ));
    }
    Ok(ForeignRestorePlan {
        module_id: plan.module_id.clone(),
        remap_plan_fingerprint: receipt.plan_fingerprint.clone(),
        current_resources: actual
            .iter()
            .map(|observation| observation.identity.clone())
            .collect(),
        expected_current_fingerprint: receipt.after_fingerprint.clone(),
        original_snapshot: snapshot.artifact_id.clone(),
        snapshot_digest: snapshot.digest.clone(),
        preserved_manifest_digest: snapshot.protected_manifest_digest.clone(),
        bindings: plan.from.clone(),
        approval_id: approval.approval_id.clone(),
    })
}
pub fn restore_fingerprint(
    receipt: &ForeignRemapReceipt,
    snapshot: &ForeignSnapshotHandle,
) -> Sha256Digest {
    let bytes = serde_json::to_vec(&(
        &receipt.plan_fingerprint,
        &receipt.after_fingerprint,
        &receipt.replacement_container_ids,
        &snapshot.artifact_id,
        &snapshot.digest,
        &snapshot.protected_manifest_digest,
    ))
    .expect("fixed foreign restore evidence");
    Sha256Digest::new(format!("sha256:{:x}", Sha256::digest(bytes))).expect("canonical SHA256")
}
fn verify_approval(
    qualified: &QualifiedForeign,
    approval: &ApprovalEnvelope,
    actor: &ActorContext,
    now: UnixMillis,
    ctx: &OperationContext,
) -> Result<()> {
    let ids: BTreeSet<_> = qualified
        .exact_resources
        .iter()
        .map(|r| r.resource_id.clone())
        .collect();
    if qualified.valid_until.0 <= now.0
        || qualified.observed_at.0 > now.0
        || !actor.local_approval_refs.contains(&approval.approval_id)
        || approval.expires_at.is_none_or(|expiry| expiry.0 <= now.0)
        || approval.resources != ids
        || !approval
            .privileges
            .contains(&Privilege::ManageForeignLifecycle)
    {
        return Err(failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "foreign association requires exact unexpired local lifecycle authority",
        ));
    }
    Ok(())
}
pub fn remap_fingerprint(
    qualified: &QualifiedForeign,
    snapshot: &ForeignSnapshotHandle,
    targets: &BTreeSet<HostBinding>,
) -> Sha256Digest {
    let bytes = serde_json::to_vec(&(
        &qualified.fingerprint,
        &qualified.host_bindings,
        &snapshot.artifact_id,
        &snapshot.source_fingerprint,
        &snapshot.digest,
        &snapshot.protected_manifest_digest,
        targets,
    ))
    .expect("fixed remap evidence serialization");
    Sha256Digest::new(format!("sha256:{:x}", Sha256::digest(bytes))).expect("canonical SHA256")
}
fn hostname(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}
fn preview_error(code: ErrorCode, message: &'static str) -> Error {
    Error {
        code,
        message: message.into(),
        module_id: None,
        config_path: None,
        resource_id: None,
        operation_id: OperationId::new("foreign-preview").expect("fixed ID"),
        retryability: Retryability::AfterRepair,
        repair_hint: None,
        retained_state: RetainedState::Unchanged,
    }
}
