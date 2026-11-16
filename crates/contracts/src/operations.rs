//! Identity-safe records and fixed operation vocabularies; no process or I/O logic.
use crate::config::{
    CertificateClaim, CheckIssue, DependencyGraph, HostEndpoint, InstallConfigCandidate,
    ListenerClaim, PortAllocationCandidate, ReadSet, RegistryEntry, ResourceIdentity,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    str::FromStr,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidIdentifier;
impl fmt::Display for InvalidIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid logical identifier")
    }
}
impl std::error::Error for InvalidIdentifier {}

macro_rules! identifier {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> std::result::Result<Self, InvalidIdentifier> {
                let value = value.into();
                if value.is_empty() || value.len() > 128
                    || !value.as_bytes()[0].is_ascii_alphanumeric()
                    || !value.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
                { return Err(InvalidIdentifier); }
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl TryFrom<String> for $name {
            type Error = InvalidIdentifier;
            fn try_from(value: String) -> std::result::Result<Self, Self::Error> { Self::new(value) }
        }
        impl From<$name> for String { fn from(value: $name) -> Self { value.0 } }
        impl FromStr for $name {
            type Err = InvalidIdentifier;
            fn from_str(value: &str) -> std::result::Result<Self, Self::Err> { Self::new(value) }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.debug_tuple(stringify!($name)).field(&self.0).finish() }
        }
    )+};
}
identifier!(
    InstanceId,
    ModuleId,
    ServiceId,
    ObjectId,
    OperationId,
    RequestId,
    PlanId,
    BundleId,
    DraftId,
    TransactionId,
    ResourceId,
    ApprovalId,
    ActorId,
    ScopeId,
    CheckId,
    CapabilityId,
    StepId,
    SecretId,
    SecretVersion,
    ArtifactId,
    CancellationId,
    EventSinkId,
    CredentialId,
    ProfileId,
    IdentityId,
    LeaseId
);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Digest(String);
impl Sha256Digest {
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, InvalidIdentifier> {
        let value = value.into();
        let digest = value.strip_prefix("sha256:").ok_or(InvalidIdentifier)?;
        if digest.len() != 64
            || !digest
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
impl TryFrom<String> for Sha256Digest {
    type Error = InvalidIdentifier;
    fn try_from(v: String) -> std::result::Result<Self, Self::Error> {
        Self::new(v)
    }
}
impl From<Sha256Digest> for String {
    fn from(v: Sha256Digest) -> Self {
        v.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ImageDigest(String);
impl ImageDigest {
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, InvalidIdentifier> {
        let value = value.into();
        Sha256Digest::new(value.clone())?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ImageDigest {
    type Error = InvalidIdentifier;
    fn try_from(v: String) -> std::result::Result<Self, Self::Error> {
        Self::new(v)
    }
}
impl From<ImageDigest> for String {
    fn from(v: ImageDigest) -> Self {
        v.0
    }
}

macro_rules! number { ($($name:ident),+ $(,)?) => {$ (
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct $name(pub u64);
)+}; }
number!(
    FormatVersion,
    ApiVersion,
    CatalogueVersion,
    StateRevision,
    RunEpoch,
    UnixMillis,
    DurationMillis
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    #[serde(rename = "config.syntax")]
    ConfigSyntax,
    #[serde(rename = "config.unsupportedVersion")]
    ConfigUnsupportedVersion,
    #[serde(rename = "config.semanticConflict")]
    ConfigSemanticConflict,
    #[serde(rename = "config.readConflict")]
    ConfigReadConflict,
    #[serde(rename = "plan.stale")]
    PlanStale,
    #[serde(rename = "plan.expired")]
    PlanExpired,
    #[serde(rename = "module.unknown")]
    ModuleUnknown,
    #[serde(rename = "module.incomplete")]
    ModuleIncomplete,
    #[serde(rename = "module.unsupported")]
    ModuleUnsupported,
    #[serde(rename = "resource.drift")]
    ResourceDrift,
    #[serde(rename = "approval.required")]
    ApprovalRequired,
    #[serde(rename = "runtime.stopped")]
    RuntimeStopped,
    #[serde(rename = "runtime.partial")]
    RuntimePartial,
    #[serde(rename = "recovery.required")]
    RecoveryRequired,
    #[serde(rename = "operation.cancelled")]
    OperationCancelled,
    #[serde(rename = "operation.deadline")]
    OperationDeadline,
    #[serde(rename = "operation.conflict")]
    OperationConflict,
    #[serde(rename = "operation.denied")]
    OperationDenied,
    #[serde(rename = "io.unavailable")]
    IoUnavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Retryability {
    Never,
    AfterRepair,
    AfterReconcile,
    SafeRetry,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RetainedState {
    Unchanged,
    DesiredSaved,
    OriginalsRetained,
    StoppedWithArtifacts,
    RecoveryRequired,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Error {
    pub code: ErrorCode,
    /// Approved safe prose; never attach raw command output or credentials.
    pub message: String,
    pub module_id: Option<ModuleId>,
    pub config_path: Option<ConfigPath>,
    pub resource_id: Option<ResourceId>,
    pub operation_id: OperationId,
    pub retryability: Retryability,
    pub repair_hint: Option<String>,
    pub retained_state: RetainedState,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

/// Semantic config keys, never a filesystem path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConfigPath(pub Vec<ObjectId>);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActorSource {
    Local,
    Api,
    Timer,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorContext {
    pub actor_id: ActorId,
    pub source: ActorSource,
    pub granted_scopes: BTreeSet<ScopeId>,
    pub local_approval_refs: BTreeSet<ApprovalId>,
    pub request_id: RequestId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionMode {
    ReadOnly,
    Mutating,
}
/// Trusted entry metadata, never accepted directly as an API actor assertion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationContext {
    pub operation_id: OperationId,
    pub actor: ActorContext,
    pub execution_mode: ExecutionMode,
    pub deadline: UnixMillis,
    pub cancellation: CancellationId,
    pub event_sink: EventSinkId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Budget {
    pub deadline: UnixMillis,
    pub max_concurrency: u16,
    pub max_bytes: u64,
    pub max_observations: u32,
}
impl Budget {
    pub fn bounded_default(deadline: UnixMillis) -> Self {
        Self {
            deadline,
            max_concurrency: 4,
            max_bytes: 1_048_576,
            max_observations: 128,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalEnvelope {
    pub approval_id: ApprovalId,
    pub issuer: ActorId,
    pub version: FormatVersion,
    pub listener_scopes: Vec<crate::config::ListenerClaim>,
    pub privileges: BTreeSet<crate::config::Privilege>,
    pub resources: BTreeSet<ResourceId>,
    pub evidence: Vec<crate::config::Evidence>,
    pub expires_at: Option<UnixMillis>,
}
/// Closed metadata vocabulary: credentials, URLs and backend stderr have no slot.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SafeFields {
    pub module_id: Option<ModuleId>,
    pub service_id: Option<ServiceId>,
    pub resource_id: Option<ResourceId>,
    pub operation_id: Option<OperationId>,
    pub transaction_id: Option<TransactionId>,
    pub count: Option<u64>,
    pub duration_ms: Option<u64>,
    pub error_code: Option<ErrorCode>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunIntent {
    Running,
    Stopped,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationStatus {
    Succeeded,
    Partial,
    Failed,
    RecoveryRequired,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemberResult {
    pub module_id: ModuleId,
    pub status: OperationStatus,
    pub resources: Vec<ResourceId>,
    pub error: Option<Error>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationResult {
    pub operation_id: OperationId,
    pub status: OperationStatus,
    pub transaction_id: Option<TransactionId>,
    pub desired_saved: bool,
    pub pending_bundle: Option<BundleId>,
    pub accepted_bundle: Option<BundleId>,
    pub run_intent: RunIntent,
    pub member_results: Vec<MemberResult>,
    pub survivors: Vec<ResourceId>,
    pub issues: Vec<CheckIssue>,
}

/// Exact first-run or menu selection, bound to the offer and root state that
/// produced it. It is desired-state input only and carries no foreign authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum InstallRootState {
    Present {
        revision: crate::config::ContentRevision,
    },
    Absent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallSelectionCandidate {
    pub catalogue_version: CatalogueVersion,
    pub offer_digest: Sha256Digest,
    pub selected_module_ids: BTreeSet<ModuleId>,
    pub previous_module_ids: BTreeSet<ModuleId>,
    pub root_state: InstallRootState,
    pub candidate_digest: Sha256Digest,
}

/// Fingerprint-bound native and published endpoints reviewed by install.
/// Native ports are observed and preserved; only host bindings may be remapped.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallForeignBinding {
    pub module_id: ModuleId,
    pub resources: Vec<ResourceIdentity>,
    pub host_bindings_before: Vec<HostEndpoint>,
    pub host_bindings_after: Vec<HostEndpoint>,
    pub native_bindings: Vec<HostEndpoint>,
}

/// Exact preflight basis for the user-owned host site index. Absence authorizes
/// only the fixed create-if-absent placeholder; present content binds both its
/// body digest and no-follow filesystem identity for mutating recheck.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase", deny_unknown_fields)]
pub enum InstallSiteIndexBasis {
    Absent {
        identity_digest: Sha256Digest,
    },
    Present {
        content_digest: Sha256Digest,
        identity_digest: Sha256Digest,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum InstallApprovalRequirement {
    InputChange {
        candidate_digest: Sha256Digest,
    },
    SelectionChange {
        candidate_digest: Sha256Digest,
    },
    AllocationChange {
        candidate_digest: Sha256Digest,
    },
    PublicListenerChange {
        listeners: Vec<ListenerClaim>,
    },
    ForeignRemap {
        module_id: ModuleId,
        resources: Vec<ResourceIdentity>,
        from: Vec<HostEndpoint>,
        to: Vec<HostEndpoint>,
    },
}

/// Fixed coordinator stages; callers cannot submit commands or arbitrary phase names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallEffectPhase {
    PersistInputs,
    PersistSelection,
    PersistAllocations,
    IssueOrReuseCertificate,
    SnapshotForeign,
    RemapForeign,
    ApplyIngress,
    VerifyIngress,
    Compensate,
}

/// Immutable read-only preflight result. It carries no approval capability,
/// credential bytes, Docker argv or mutable host authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallPlan {
    pub plan_id: PlanId,
    /// Stable installation identity observed or generated for virtual
    /// first-install planning; execute must revalidate it before any effect.
    pub instance_id: InstanceId,
    pub expected_inputs: ExpectedInputs,
    /// Complete fresh TCP/UDP host reservations used for both public-edge and
    /// allocated-backend conflict checks; execute must reobserve this basis.
    pub host_reservation_evidence: crate::config::ReservationEvidence,
    pub selection_candidate: Option<InstallSelectionCandidate>,
    pub input_candidate: Option<InstallConfigCandidate>,
    pub graph: DependencyGraph,
    pub public_edge: Option<HostEndpoint>,
    pub site_index: InstallSiteIndexBasis,
    pub port_allocation_candidate: Option<PortAllocationCandidate>,
    pub foreign_bindings: Vec<InstallForeignBinding>,
    pub certificate_requirements: Vec<CertificateClaim>,
    pub conflicts: Vec<CheckIssue>,
    pub approvals: Vec<InstallApprovalRequirement>,
    pub phases: Vec<InstallEffectPhase>,
    pub created_at: UnixMillis,
    pub expires_at: UnixMillis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallCertificateStatus {
    NotRequested,
    Reused,
    Issued,
    Unverified,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallPhaseResult {
    pub phase: InstallEffectPhase,
    pub status: OperationStatus,
    pub transaction_id: Option<TransactionId>,
    pub error: Option<Error>,
}

/// Install-specific completion envelope around common operation/member results.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallResult {
    pub operation: OperationResult,
    pub phases: Vec<InstallPhaseResult>,
    pub certificate_status: InstallCertificateStatus,
    pub foreign_remap_transactions: Vec<TransactionId>,
    pub external_dns_unverified: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BundleValidationStatus {
    Passed,
    Failed,
    Partial,
    RecoveryRequired,
}

/// Exact cleanup acknowledgement. Resource IDs stay private in the journal;
/// the digest binds the protected set without publishing Docker identities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleValidationCleanup {
    pub completed: bool,
    pub resources_digest: Sha256Digest,
    pub completed_at: UnixMillis,
}

/// Durable, hash-bound result of explicitly qualifying one protected candidate.
/// External network/device claims must be represented as unverified evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BundleValidationAttestation {
    pub operation_id: OperationId,
    pub bundle_id: BundleId,
    pub candidate_digest: Sha256Digest,
    pub artifact_digests: BTreeMap<ArtifactId, Sha256Digest>,
    pub image_digests: BTreeMap<ServiceId, ImageDigest>,
    pub runner_version: ObjectId,
    pub status: BundleValidationStatus,
    pub evidence: Vec<crate::config::Evidence>,
    pub cleanup: BundleValidationCleanup,
    pub validated_at: UnixMillis,
    pub expires_at: UnixMillis,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LifecycleRequest {
    Add {
        module_id: ModuleId,
        approval_refs: BTreeSet<ApprovalId>,
    },
    Enable {
        module_id: ModuleId,
        save_only: bool,
        approval_refs: BTreeSet<ApprovalId>,
    },
    Disable {
        module_id: ModuleId,
        save_only: bool,
        approval_refs: BTreeSet<ApprovalId>,
    },
    Remove {
        module_id: ModuleId,
        approval_refs: BTreeSet<ApprovalId>,
    },
    Purge {
        module_id: ModuleId,
        approval_refs: BTreeSet<ApprovalId>,
    },
    Update {
        module_id: ModuleId,
        target_digest: ImageDigest,
        approval_refs: BTreeSet<ApprovalId>,
    },
    Up,
    Down,
    StartAccepted {
        bundle_id: BundleId,
        approval_refs: BTreeSet<ApprovalId>,
    },
    RollbackAccepted {
        bundle_id: BundleId,
    },
    RestartServices {
        service_ids: Vec<ServiceId>,
        wait_seconds: u16,
        bundle_id: BundleId,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExpectedInputs {
    pub read_set: ReadSet,
    pub registry_revision: StateRevision,
    pub runtime_revision: StateRevision,
    pub run_epoch: RunEpoch,
    pub resources: BTreeMap<ResourceId, Sha256Digest>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LifecyclePlan {
    pub request: LifecycleRequest,
    pub expected_inputs: ExpectedInputs,
    pub actions: Vec<Action>,
    pub registry_changes: Vec<RegistryChange>,
    pub selection_change: Option<SelectionChange>,
    pub setup_plan: Option<SetupPlan>,
    pub effect_plan: Option<EffectPlan>,
    /// Exact managed VPN port assignment prepared with fresh evidence. It is
    /// immutable plan data; execute must validate and persist this candidate,
    /// never generate a replacement while resuming the operation.
    #[serde(default)]
    pub port_allocation_candidate: Option<PortAllocationCandidate>,
    pub issues: Vec<CheckIssue>,
    pub expected_disruption: Disruption,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionChange {
    pub module_ids: BTreeSet<ModuleId>,
    pub enabled: bool,
    pub expected_root_revision: crate::config::ContentRevision,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionKind {
    Start,
    Stop,
    Reload,
    Restart,
    Recreate,
    DisableAutostart,
    RestoreAutostart,
    SuspendJob,
    ResumeJob,
    PublishArtifacts,
    RemoveOwnedResource,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Condition {
    Autostart {
        resource_id: ResourceId,
        enabled: bool,
    },
    ResourceFingerprint {
        resource_id: ResourceId,
        fingerprint: Sha256Digest,
    },
    ResourceAbsent {
        resource_id: ResourceId,
    },
    ResourceRunning {
        resource_id: ResourceId,
        running: bool,
    },
    RunEpoch {
        epoch: RunEpoch,
    },
    Approval {
        approval_id: ApprovalId,
    },
    Evidence {
        check_id: CheckId,
        fingerprint: Sha256Digest,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Action {
    pub action_id: StepId,
    pub kind: ActionKind,
    pub resource_id: ResourceId,
    pub dependencies: BTreeSet<StepId>,
    pub preconditions: Vec<Condition>,
    pub postconditions: Vec<Condition>,
    pub timeout: DurationMillis,
    pub compensation_ref: Option<StepId>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Disruption {
    None,
    Reload,
    ConnectionsInterrupted,
    CompositionStopped,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RollbackTarget {
    Accepted { bundle_id: BundleId },
    PreInstall { originals: Vec<ArtifactId> },
    StoppedWithArtifacts,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectPlan {
    pub candidate_hash: Sha256Digest,
    pub read_set: ReadSet,
    pub catalogue_version: CatalogueVersion,
    pub descriptor_versions: BTreeMap<ModuleId, Sha256Digest>,
    pub image_versions: BTreeMap<ServiceId, ImageDigest>,
    pub secret_versions: BTreeMap<SecretId, SecretVersion>,
    pub resource_before: BTreeMap<ResourceId, Sha256Digest>,
    pub approvals: Vec<ApprovalEnvelope>,
    pub actions: Vec<Action>,
    pub compensations: Vec<Action>,
    pub expected_disruption: Disruption,
    pub expires_at: UnixMillis,
    pub rollback_target: RollbackTarget,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "change",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RegistryChange {
    RebindOwned {
        module_id: ModuleId,
        previous: ResourceIdentity,
        replacement: ResourceIdentity,
        receipt: StepReceipt,
    },
    SetupProgress {
        module_id: ModuleId,
        resources: Vec<ResourceIdentity>,
        installed_versions: BTreeMap<ArtifactId, ImageDigest>,
        setup_state: crate::config::SetupState,
        checkpoint: Option<StepReceipt>,
    },
    Adopt {
        entry: RegistryEntry,
    },
    Checkpoint {
        module_id: ModuleId,
        checkpoint: StepReceipt,
    },
    Install {
        entry: RegistryEntry,
    },
    Remove {
        module_id: ModuleId,
        retained_artifacts: Vec<ArtifactId>,
    },
    UninstallOwned {
        module_id: ModuleId,
        removed_resources: Vec<ResourceIdentity>,
        receipt: StepReceipt,
    },
    CleanupRequired {
        module_id: ModuleId,
        resources: Vec<ResourceId>,
    },
    Update {
        module_id: ModuleId,
        installed_versions: BTreeMap<ArtifactId, ImageDigest>,
        receipt: StepReceipt,
    },
    Purge {
        module_id: ModuleId,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepStatus {
    Prepared,
    Applied,
    Verified,
    Compensated,
    Failed,
    Uncertain,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StepReceipt {
    pub step_id: StepId,
    pub idempotency_key: RequestId,
    pub status: StepStatus,
    pub recorded_at: UnixMillis,
    pub resources: Vec<ResourceIdentity>,
    pub state_fingerprints: BTreeMap<ResourceId, Sha256Digest>,
    pub retained_artifacts: Vec<ArtifactId>,
    pub error: Option<Error>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SetupStepKind {
    ObtainImage {
        service_id: ServiceId,
        digest: ImageDigest,
    },
    SeedMissingConfig {
        object_id: ObjectId,
        template_id: ArtifactId,
    },
    RegisterOwnedResource {
        resource_id: ResourceId,
    },
    AssociateForeignResource {
        resource_id: ResourceId,
        approval_id: ApprovalId,
    },
    VerifyCompatibility {
        capability_id: CapabilityId,
    },
    RequestOperatorAction {
        action_id: StepId,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupStep {
    pub step_id: StepId,
    pub idempotency_key: RequestId,
    pub kind: SetupStepKind,
    pub dependencies: BTreeSet<StepId>,
    pub timeout: DurationMillis,
    pub required_approval: Option<ApprovalId>,
    pub operator_alternative: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MissingInput {
    pub object_id: ObjectId,
    pub path: ConfigPath,
    pub description: String,
    pub secret: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupPlan {
    pub module_id: ModuleId,
    pub expected_registry_revision: StateRevision,
    pub steps: Vec<SetupStep>,
    pub checkpoints: Vec<StepReceipt>,
    pub missing_inputs: Vec<MissingInput>,
    pub required_operator_actions: Vec<StepId>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Compatibility {
    Compatible,
    Incompatible,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdatePlan {
    pub module_id: ModuleId,
    pub installed_digest: ImageDigest,
    pub target_digest: ImageDigest,
    pub compatibility: Compatibility,
    pub config_format: FormatVersion,
    pub data_format: FormatVersion,
    pub irreversible_effects: Vec<StepId>,
    pub actions: Vec<Action>,
    pub retained_prior_artifacts: Vec<ArtifactId>,
    pub rollback_target: RollbackTarget,
    pub evidence: Vec<crate::config::Evidence>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretGrant {
    pub secret_id: SecretId,
    pub version: SecretVersion,
    pub consumer: ServiceId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeArtifact {
    pub artifact_id: ArtifactId,
    pub service_id: ServiceId,
    pub digest: Sha256Digest,
    pub secret_bearing: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderResult {
    pub artifacts: Vec<NativeArtifact>,
    pub listeners: Vec<crate::config::ListenerClaim>,
    pub routes: Vec<crate::config::RouteClaim>,
}

/// Ports are trusted implementations chosen in code, never names loaded from YAML.
pub trait ModuleCheck {
    fn check(
        &self,
        settings: &crate::config::ModuleSettings,
        graph: &crate::config::DependencyGraph,
        observations: &crate::config::RuntimeObservation,
    ) -> Vec<CheckIssue>;
}
pub trait ModuleObserve {
    fn observe(
        &self,
        resources: &[ResourceIdentity],
        budget: &Budget,
    ) -> Result<crate::config::RuntimeObservation>;
}
pub trait ModuleRender {
    fn render(
        &self,
        settings: &crate::config::ModuleSettings,
        consumers: &BTreeSet<ModuleId>,
        secret_grants: &[SecretGrant],
    ) -> Result<RenderResult>;
}
pub trait ModuleSetup {
    fn prepare(
        &self,
        module_id: &ModuleId,
        existing: Option<&RegistryEntry>,
        inventory: &[NativeArtifact],
        supplied: &crate::config::ModuleSettings,
    ) -> Result<SetupPlan>;
    fn execute_step(&self, step: &SetupStep, context: &OperationContext) -> Result<StepReceipt>;
}
pub trait ModuleUpdate {
    fn plan(
        &self,
        installed: &ImageDigest,
        target: &ImageDigest,
        evidence: &[crate::config::Evidence],
    ) -> Result<UpdatePlan>;
    fn execute(&self, plan: &UpdatePlan, context: &OperationContext) -> Result<Vec<StepReceipt>>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ForeignAction {
    Start,
    Stop,
    DisableAutostart,
    RestoreAutostart,
}
pub trait ForeignLifecycle {
    fn observe(
        &self,
        identity: &ResourceIdentity,
        budget: &Budget,
    ) -> Result<crate::config::ResourceObservation>;
    fn transition(
        &self,
        action: ForeignAction,
        identity: &ResourceIdentity,
        approval: &ApprovalEnvelope,
        epoch: RunEpoch,
        context: &OperationContext,
    ) -> Result<StepReceipt>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_accepts_valid_logical_ids() {
        for value in ["web", "web-1", "web_1", "web.1", "1web", "a"] {
            let id = ModuleId::new(value).unwrap_or_else(|_| panic!("should accept {value:?}"));
            assert_eq!(id.as_str(), value);
        }
    }

    #[test]
    fn identifier_rejects_invalid_logical_ids() {
        for value in [
            "",
            " ",
            " web",
            "web ",
            "web!",
            "web/slash",
            "web\\slash",
            "-web",
            ".web",
            "_web",
            "web\n",
        ] {
            assert!(ModuleId::new(value).is_err(), "should reject {value:?}");
        }
    }

    #[test]
    fn identifier_enforces_max_length_boundary() {
        assert!(ModuleId::new("x".repeat(128)).is_ok());
        assert!(ModuleId::new("x".repeat(129)).is_err());
    }

    #[test]
    fn sha256_digest_accepts_valid() {
        let hex = "a".repeat(64);
        let value = format!("sha256:{hex}");
        let digest = Sha256Digest::new(&value).unwrap();
        assert_eq!(digest.as_str(), value);
    }

    #[test]
    fn sha256_digest_rejects_invalid() {
        let hex64 = "a".repeat(64);
        assert!(Sha256Digest::new("a".repeat(64)).is_err(), "missing prefix");
        assert!(
            Sha256Digest::new(format!("sha256:{}", "a".repeat(63))).is_err(),
            "short"
        );
        assert!(
            Sha256Digest::new(format!("sha256:{}", "a".repeat(65))).is_err(),
            "long"
        );
        assert!(
            Sha256Digest::new(format!("sha256:{}", "A".repeat(64))).is_err(),
            "uppercase"
        );
        assert!(
            Sha256Digest::new(format!("sha256:{}", "g".repeat(64))).is_err(),
            "non-hex"
        );
        assert!(
            Sha256Digest::new(format!("md5:{hex64}")).is_err(),
            "wrong prefix"
        );
        assert!(Sha256Digest::new("").is_err(), "empty");
    }

    #[test]
    fn image_digest_delegates_sha256_validation() {
        let hex = "b".repeat(64);
        assert!(ImageDigest::new(format!("sha256:{hex}")).is_ok());
        assert!(ImageDigest::new(hex).is_err());
    }
}
