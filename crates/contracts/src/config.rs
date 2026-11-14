//! Configuration facts, declarations and observations. These records perform no I/O.
use crate::operations::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentRevision {
    pub object_id: ObjectId,
    pub digest: Sha256Digest,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadDependency {
    pub revision: ContentRevision,
    pub format_version: FormatVersion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadCoherence {
    Coherent,
    Conflict,
    Unverified,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadSet {
    pub objects: BTreeMap<ObjectId, ReadDependency>,
    /// Known descriptor document IDs proven absent by the same coherent inventory.
    /// Present object IDs must never also appear here. Defaults empty for older records.
    #[serde(default)]
    pub absent_objects: BTreeSet<ObjectId>,
    pub secret_versions: BTreeMap<SecretId, SecretVersion>,
    pub coherence: ReadCoherence,
}
/// Components resolved inside a known inventory; no user-controlled absolute paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnownLocation {
    pub inventory_id: ObjectId,
    pub components: Vec<ObjectId>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigDocument {
    pub object_id: ObjectId,
    pub location: KnownLocation,
    pub source_bytes: Vec<u8>,
    pub revision: ContentRevision,
    pub schema_version: Option<FormatVersion>,
    pub diagnostics: Vec<CheckIssue>,
}
impl std::fmt::Debug for ConfigDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigDocument")
            .field("object_id", &self.object_id)
            .field("revision", &self.revision)
            .field("source_bytes", &"[protected]")
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesiredReadResult {
    pub documents: Vec<ConfigDocument>,
    pub snapshot: Option<DesiredSnapshot>,
    pub errors: Vec<CheckIssue>,
    pub read_set: ReadSet,
}

/// Versioned module settings stay protected until validated by their module owner.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleSettings {
    pub schema_version: FormatVersion,
    pub settings: BTreeMap<String, Value>,
}
impl std::fmt::Debug for ModuleSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleSettings")
            .field("schema_version", &self.schema_version)
            .field("settings", &"[protected]")
            .finish()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeatureSelection {
    pub enabled: bool,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformSettings {
    pub schema_version: FormatVersion,
    pub settings: BTreeMap<String, Value>,
}
impl std::fmt::Debug for PlatformSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformSettings")
            .field("schema_version", &self.schema_version)
            .field("settings", &"[protected]")
            .finish()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesiredSnapshot {
    pub platform: PlatformSettings,
    pub features: BTreeMap<ModuleId, FeatureSelection>,
    pub module_settings: BTreeMap<ModuleId, ModuleSettings>,
    pub editable_docker: BTreeMap<ServiceId, ConfigDocument>,
    pub read_set: ReadSet,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValueProvenance {
    Explicit,
    Default,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvenanceEntry {
    pub object_id: ObjectId,
    pub path: ConfigPath,
    pub source: ValueProvenance,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedReference {
    pub source_object: ObjectId,
    pub source_path: ConfigPath,
    pub target_object: ObjectId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedSnapshot {
    pub desired: DesiredSnapshot,
    pub provenance: Vec<ProvenanceEntry>,
    pub supported_versions: BTreeMap<ObjectId, FormatVersion>,
    pub resolved_references: Vec<ResolvedReference>,
}

/// Explicit public host endpoint. Syntax and collision policy belong to Core.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostEndpoint {
    pub transport: Transport,
    pub address: std::net::IpAddr,
    pub port: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallValueSource {
    Existing,
    CommandLine,
    TrustedTty,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallInputProvenance {
    pub site_domain: InstallValueSource,
    pub public_edge: InstallValueSource,
    #[serde(default)]
    pub certificate_email: Option<InstallValueSource>,
}

/// Non-secret publication input only; VPN backend ports are generated separately.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallInput {
    pub site_domain: String,
    pub public_edge: HostEndpoint,
    /// Non-secret Let's Encrypt account contact; absent when a saved value is reused.
    #[serde(default)]
    pub certificate_email: Option<String>,
    pub provenance: InstallInputProvenance,
}

/// Fixed target vocabulary prevents callers from submitting arbitrary config paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallConfigField {
    SiteDomain,
    PublicEdge,
    CertificateEmail,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum InstallConfigValue {
    SiteDomain(String),
    PublicEdge(HostEndpoint),
    CertificateEmail(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallConfigChange {
    pub field: InstallConfigField,
    pub previous: Option<InstallConfigValue>,
    pub proposed: InstallConfigValue,
}

/// Immutable, digest-bound allowlisted changes and coherent source revisions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallConfigCandidate {
    pub changes: Vec<InstallConfigChange>,
    pub read_set: ReadSet,
    pub candidate_digest: Sha256Digest,
}

/// Reference to a trusted local approval record; the record itself is protected by Core.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallInputReview {
    pub candidate_digest: Sha256Digest,
    pub approval_id: ApprovalId,
    pub instance_id: InstanceId,
    pub operation_id: OperationId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallConfigWriteState {
    Prepared,
    Committed,
    Partial,
    RecoveryRequired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallConfigDocumentReceipt {
    pub object_id: ObjectId,
    pub before: Option<ContentRevision>,
    pub after: ContentRevision,
}

/// Write result contains revision metadata and never serialized configuration bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallConfigReceipt {
    pub transaction_id: TransactionId,
    pub documents: Vec<InstallConfigDocumentReceipt>,
    pub state: InstallConfigWriteState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReleasePhase {
    R1,
    R2,
    LaterManagement,
    Optional,
    FarFuture,
    LabPolicyBlocked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Transport {
    Tcp,
    Udp,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ListenerNamespace {
    Host,
    Project { network_id: ObjectId },
    Service { service_id: ServiceId },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListenerClaim {
    pub namespace: ListenerNamespace,
    pub owner: ModuleId,
    pub service_id: ServiceId,
    pub transport: Transport,
    pub address: std::net::IpAddr,
    pub port: u16,
    pub public: bool,
}
/// Descriptor-owned shape for a host listener whose port is allocated by Core.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortListenerTemplate {
    pub namespace: ListenerNamespace,
    pub exposure: ListenerExposure,
    pub addresses: Vec<std::net::IpAddr>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListenerExposure {
    Public,
    Private,
}
/// Exact setting field declared by a trusted module descriptor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AllocationField {
    pub path: String,
    pub transport: Transport,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_port: Option<u16>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServicePortBinding {
    pub allocation_id: ObjectId,
    pub service_id: ServiceId,
    pub settings_document: ObjectId,
    pub binding_group: ObjectId,
    pub listener: PortListenerTemplate,
    pub fields: Vec<AllocationField>,
    /// Injected by the trusted catalogue after descriptor validation; never user-authored.
    #[serde(skip)]
    pub descriptor_revision: Option<Sha256Digest>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedAllocation {
    pub service_id: ServiceId,
    pub settings_document: ObjectId,
    pub binding_group: ObjectId,
    pub port: u16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AllocationRequest {
    pub selected: Vec<ServicePortBinding>,
    pub retained: Vec<SavedAllocation>,
    pub read_set: ReadSet,
    pub registry_revision: StateRevision,
    pub now: UnixMillis,
    pub expires_at: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortReservation {
    pub transport: Transport,
    pub port: u16,
    pub holder_service: Option<ServiceId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReservationEvidence {
    pub observed_at: UnixMillis,
    pub valid_until: UnixMillis,
    pub digest: Sha256Digest,
    pub complete: bool,
    pub reservations: Vec<PortReservation>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServicePortAllocation {
    pub settings_document: ObjectId,
    pub binding_group: ObjectId,
    pub descriptor_revision: Sha256Digest,
    pub port: u16,
    pub prior_fields: BTreeMap<String, Option<u16>>,
    pub fields: Vec<AllocationField>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortAllocationCandidate {
    pub allocations: BTreeMap<ServiceId, ServicePortAllocation>,
    pub retained: Vec<SavedAllocation>,
    pub read_set: ReadSet,
    pub registry_revision: StateRevision,
    pub reservation_digest: Sha256Digest,
    pub created_at: UnixMillis,
    pub expires_at: UnixMillis,
    pub candidate_digest: Sha256Digest,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PortAllocationWriteState {
    Prepared,
    Committed,
    Partial,
    RecoveryRequired,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortAllocationReceipt {
    /// Absent only when every requested allocation already matches saved state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<TransactionId>,
    pub allocations: BTreeMap<ServiceId, u16>,
    pub document_revisions: BTreeMap<ObjectId, ContentRevision>,
    pub state: PortAllocationWriteState,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RouteMatch {
    Sni {
        hostname: String,
    },
    Http {
        hostname: String,
        path_prefix: String,
    },
    DefaultHttps,
}
impl std::fmt::Debug for RouteMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sni { hostname } => f.debug_struct("Sni").field("hostname", hostname).finish(),
            Self::Http { hostname, .. } => f
                .debug_struct("Http")
                .field("hostname", hostname)
                .field("path_prefix", &"[protected]")
                .finish(),
            Self::DefaultHttps => f.write_str("DefaultHttps"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RouteClaim {
    pub owner: ModuleId,
    pub listener_service: ServiceId,
    pub matcher: RouteMatch,
    pub target_service: ServiceId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CertificateClaim {
    pub owner: ModuleId,
    pub names: BTreeSet<String>,
    pub consumer: ServiceId,
    pub foreign_owned: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StateClaim {
    pub owner: ModuleId,
    pub object_id: ObjectId,
    pub exclusive: bool,
    pub secret_bearing: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Privilege {
    DockerLifecycle,
    BindPublicListener,
    HostNetwork,
    ManageSystemd,
    ManageForeignLifecycle,
    WriteProtectedState,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityBindings {
    pub check: CapabilityId,
    pub observe: Option<CapabilityId>,
    pub render: Option<CapabilityId>,
    pub setup: Option<CapabilityId>,
    pub update: Option<CapabilityId>,
    pub foreign_lifecycle: Option<CapabilityId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleDescriptor {
    pub module_id: ModuleId,
    pub phase: ReleasePhase,
    pub location: KnownLocation,
    pub config_schema: ObjectId,
    pub config_version: FormatVersion,
    pub descriptor_revision: Sha256Digest,
    pub defaults: ModuleSettings,
    pub dependencies: BTreeSet<ModuleId>,
    pub services: BTreeSet<ServiceId>,
    pub listener_claims: Vec<ListenerClaim>,
    #[serde(default)]
    pub port_allocation: Option<ServicePortBinding>,
    pub route_claims: Vec<RouteClaim>,
    pub certificate_claims: Vec<CertificateClaim>,
    pub privileges: BTreeSet<Privilege>,
    pub state: Vec<StateClaim>,
    pub capabilities: CapabilityBindings,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Ownership {
    Owned,
    Associated,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ResourceLocator {
    DockerContainer { container_id: DockerContainerId },
    SystemdUnit { unit_id: ObjectId },
    ProtectedArtifact { artifact_id: ArtifactId },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceIdentity {
    pub resource_id: ResourceId,
    pub service_id: ServiceId,
    pub locator: ResourceLocator,
    pub fingerprint: Sha256Digest,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SetupState {
    Incomplete {
        blocked_step: Option<StepId>,
        required_operator_action: Option<StepId>,
    },
    Complete,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RestartPolicy {
    DockerNo,
    DockerAlways,
    DockerUnlessStopped,
    DockerOnFailure { maximum_retry_count: u32 },
    SystemdDisabled,
    SystemdEnabled,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdoptionScope {
    pub approval_id: ApprovalId,
    pub resources: BTreeSet<ResourceId>,
    pub lifecycle_actions: BTreeSet<ForeignActionPermission>,
    pub original_evidence: Vec<Evidence>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ForeignActionPermission {
    Start,
    Stop,
    DisableAutostart,
    RestoreAutostart,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MembershipState {
    Installed,
    RemovedRetained,
    CleanupRequired,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegistryEntry {
    pub module_id: ModuleId,
    pub ownership: Ownership,
    pub membership: MembershipState,
    pub resources: Vec<ResourceIdentity>,
    pub setup_state: SetupState,
    pub checkpoints: Vec<StepReceipt>,
    pub installed_artifact_versions: BTreeMap<ArtifactId, ImageDigest>,
    pub original_policies: BTreeMap<ResourceId, RestartPolicy>,
    pub adoption_scope: Option<AdoptionScope>,
    pub retained_artifacts: Vec<ArtifactId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegistrySnapshot {
    pub instance_id: InstanceId,
    pub revision: StateRevision,
    pub entries: BTreeMap<ModuleId, RegistryEntry>,
    pub diagnostics: Vec<CheckIssue>,
    pub recovery_originals: Vec<ArtifactId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyGraph {
    pub enabled_modules: BTreeSet<ModuleId>,
    pub service_consumers: BTreeMap<ServiceId, BTreeSet<ModuleId>>,
    pub topological_order: Vec<ModuleId>,
    pub listener_claims: Vec<ListenerClaim>,
    #[serde(default)]
    pub allocation_bindings: Vec<ServicePortBinding>,
    pub route_claims: Vec<RouteClaim>,
    pub certificate_claims: Vec<CertificateClaim>,
    pub state_claims: Vec<StateClaim>,
    pub conflicts: Vec<CheckIssue>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObservedState {
    Running,
    Stopped,
    Failed,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceObservation {
    pub identity: ResourceIdentity,
    pub state: ObservedState,
    pub observed_at: UnixMillis,
    pub valid_until: UnixMillis,
    pub restart_policy: Option<RestartPolicy>,
    pub evidence: Vec<Evidence>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeObservation {
    pub resources: BTreeMap<ResourceId, ResourceObservation>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceOutcome {
    Pass,
    Fail,
    Unknown,
    Skipped,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Vantage {
    LocalConfiguration,
    LocalRuntime,
    ExternalProbe,
    NativeValidation,
    Supplied,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evidence {
    pub check_id: CheckId,
    pub module_id: Option<ModuleId>,
    pub outcome: EvidenceOutcome,
    pub timestamp: UnixMillis,
    pub vantage: Vantage,
    pub input_fingerprint: Sha256Digest,
    pub details: SafeFields,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
    Information,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckIssue {
    pub code: ErrorCode,
    pub severity: Severity,
    pub module_id: Option<ModuleId>,
    pub object_id: Option<ObjectId>,
    pub path: Option<ConfigPath>,
    pub message: String,
    pub repair_hint: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckReport {
    pub issues: Vec<CheckIssue>,
    pub evidence: Vec<Evidence>,
    pub blocking: bool,
    pub normalized_snapshot: Option<NormalizedSnapshot>,
}

/// Full Docker identity, distinct from a human container name or logical resource ID.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DockerContainerId(String);
impl DockerContainerId {
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, InvalidIdentifier> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(InvalidIdentifier);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for DockerContainerId {
    type Error = InvalidIdentifier;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<DockerContainerId> for String {
    fn from(value: DockerContainerId) -> Self {
        value.0
    }
}
