//! Independent runtime wire data. Parsing a proposal never grants execution authority.
#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize, de};
use sha2::{Digest, Sha256};
use std::{fmt, marker::PhantomData};

pub const MAX_WIRE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ITEMS: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidValue;
impl fmt::Display for InvalidValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid runtime wire value")
    }
}
impl std::error::Error for InvalidValue {}

fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        && s != "."
        && s != ".."
}
fn digest(s: &str) -> bool {
    s.len() == 71
        && s.starts_with("sha256:")
        && s[7..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn image(s: &str) -> bool {
    s.len() <= 512
        && s.rsplit_once('@').is_some_and(|(name, pin)| {
            !name.is_empty()
                && !name.starts_with('/')
                && !name.contains("..")
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/._:-".contains(&b))
                && digest(pin)
        })
}
fn artifact_path(s: &str) -> bool {
    s.len() <= 512 && !s.is_empty() && s.split('/').all(identifier)
}
fn hostname(s: &str) -> bool {
    s.len() <= 253
        && !s.is_empty()
        && s.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

macro_rules! string_type {
    ($name:ident, $validate:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
                let value = value.into();
                if $validate(&value) {
                    Ok(Self(value))
                } else {
                    Err(InvalidValue)
                }
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}
string_type!(InstanceId, identifier);
string_type!(BundleId, identifier);
string_type!(ServiceId, identifier);
string_type!(ModuleId, identifier);
string_type!(ResourceId, identifier);
string_type!(ArtifactId, identifier);
string_type!(OperationId, identifier);
string_type!(LeaseId, identifier);
string_type!(ActionId, identifier);
string_type!(ApprovalId, identifier);
string_type!(CheckId, identifier);
string_type!(SecretId, identifier);
string_type!(SecretVersion, identifier);
string_type!(CallerId, identifier);
string_type!(Sha256Digest, digest);
string_type!(ImagePin, image);
string_type!(ArtifactPath, artifact_path);
string_type!(HostName, hostname);

/// Version is distinct from software release and cannot deserialize future formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ContractVersion(u16);
impl ContractVersion {
    pub const V1: Self = Self(1);
    pub fn get(self) -> u16 {
        self.0
    }
}
impl<'de> Deserialize<'de> for ContractVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if u16::deserialize(d)? == 1 {
            Ok(Self::V1)
        } else {
            Err(de::Error::custom("unsupported runtime contract version"))
        }
    }
}

/// Persistence format evolves independently of the executor request contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct StateSchemaVersion(u16);
impl StateSchemaVersion {
    pub const V1: Self = Self(1);
    pub fn get(self) -> u16 {
        self.0
    }
}
impl<'de> Deserialize<'de> for StateSchemaVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if u16::deserialize(d)? == 1 {
            Ok(Self::V1)
        } else {
            Err(de::Error::custom("unsupported runtime state version"))
        }
    }
}

/// Bounded while decoding, not merely checked after allocating an arbitrary vector.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Items<T>(Vec<T>);
impl<T> Items<T> {
    pub fn new(items: Vec<T>) -> Result<Self, InvalidValue> {
        if items.len() <= MAX_ITEMS {
            Ok(Self(items))
        } else {
            Err(InvalidValue)
        }
    }
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }
}
impl<T> Default for Items<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Items<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Visitor<T> {
            type Value = Items<T>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("at most 4096 records")
            }
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut result = Vec::new();
                while let Some(value) = seq.next_element()? {
                    if result.len() == MAX_ITEMS {
                        return Err(de::Error::custom("runtime collection limit exceeded"));
                    }
                    result.push(value);
                }
                Ok(Items(result))
            }
        }
        d.deserialize_seq(Visitor(PhantomData))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireError {
    InputTooLarge,
    InvalidEncoding,
}
/// Bounds input bytes before JSON decoding; this is not a total heap-allocation bound.
/// Callers must also bound transport reads and validate their operation budget.
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, WireError> {
    if bytes.len() > MAX_WIRE_BYTES {
        return Err(WireError::InputTooLarge);
    }
    serde_json::from_slice(bytes).map_err(|_| WireError::InvalidEncoding)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunIntent {
    Running,
    Stopped,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    Owned,
    Associated,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Container,
    ComposeProject,
    SystemdUnit,
    Listener,
    ServiceArtifacts,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceIdentity {
    pub resource_id: ResourceId,
    pub kind: ResourceKind,
    pub identity_digest: Sha256Digest,
    pub fingerprint: Sha256Digest,
    pub ownership: Ownership,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Artifact {
    pub artifact_id: ArtifactId,
    pub path: ArtifactPath,
    pub digest: Sha256Digest,
    pub size_bytes: u64,
    pub service_id: ServiceId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecretBinding {
    pub secret_id: SecretId,
    pub version: SecretVersion,
    pub service_id: ServiceId,
    pub artifact_id: ArtifactId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    Haproxy,
    Caddy,
    Xray,
    Amnezia,
    Unbound,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Service {
    pub service_id: ServiceId,
    pub module_id: ModuleId,
    pub engine: Engine,
    pub image: ImagePin,
    pub resource: ResourceIdentity,
    pub dependencies: Items<ServiceId>,
    pub artifacts: Items<ArtifactId>,
    pub restart_owner_resources: Items<ResourceId>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Tcp,
    Udp,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Claim {
    Listener {
        service: ServiceId,
        address: std::net::Ipv4Addr,
        port: std::num::NonZeroU16,
        transport: Transport,
    },
    SniRoute {
        service: ServiceId,
        hostname: HostName,
    },
    Certificate {
        service: ServiceId,
        hostname: HostName,
        artifact: ArtifactId,
        valid_until_ms: u64,
    },
    State {
        service: ServiceId,
        resource: ResourceId,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOutcome {
    Pass,
    Fail,
    Unknown,
    Skipped,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeKind {
    NativeConfig,
    RouteIdentity,
    AuthenticationAllowed,
    AuthenticationDenied,
    ResourceIdentity,
    CertificateCoverage,
    StagingClient,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vantage {
    LocalNative,
    LocalRuntime,
    IsolatedStaging,
    ApprovedExternal,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evidence {
    pub check_id: CheckId,
    pub service_id: ServiceId,
    pub kind: ProbeKind,
    pub vantage: Vantage,
    pub outcome: EvidenceOutcome,
    pub timestamp_ms: u64,
    pub bundle_digest: Sha256Digest,
    pub input_digest: Sha256Digest,
    pub observed_identity: Option<Sha256Digest>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StagingAttestation {
    pub bundle_digest: Sha256Digest,
    pub mapping_digest: Sha256Digest,
    pub image_and_secret_digest: Sha256Digest,
    pub evidence: Items<Evidence>,
    pub timestamp_ms: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyFences {
    pub credential_revision: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeActionKind {
    Start,
    Stop,
    Reload,
    Restart,
    Recreate,
    EnableAutostart,
    DisableAutostart,
    ActivateArtifacts,
    RestoreArtifacts,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FixedAction {
    pub action_id: ActionId,
    pub kind: NativeActionKind,
    pub resource_id: ResourceId,
    pub service_id: ServiceId,
    pub dependencies: Items<ActionId>,
    pub before: Option<Sha256Digest>,
    pub after: Option<Sha256Digest>,
    pub timeout_ms: std::num::NonZeroU32,
    pub compensation: Option<ActionId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetiredService {
    pub service_id: ServiceId,
    pub module_id: ModuleId,
    pub resource: ResourceIdentity,
    pub restart_owner_resources: Items<ResourceId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CandidateBundle {
    pub bundle_id: BundleId,
    pub instance_id: InstanceId,
    pub contract_version: ContractVersion,
    pub manifest_digest: Sha256Digest,
    pub artifacts: Items<Artifact>,
    pub services: Items<Service>,
    pub retired_services: Items<RetiredService>,
    pub claims: Items<Claim>,
    pub secrets: Items<SecretBinding>,
    pub actions: Items<FixedAction>,
    pub fences: PolicyFences,
    pub native_evidence: Items<Evidence>,
    pub staging: Option<StagingAttestation>,
}

/// This is untrusted evidence. Only runtime admission can grant authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalEnvelope {
    pub approval_id: ApprovalId,
    pub issuer: CallerId,
    pub version: ContractVersion,
    pub scope_digest: Sha256Digest,
    pub resources: Items<ResourceId>,
    pub permitted_actions: Items<NativeActionKind>,
    pub expires_ms: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RollbackTarget {
    Accepted {
        bundle_id: BundleId,
        manifest_digest: Sha256Digest,
    },
    PreInstall {
        closed_listeners: Items<ResourceId>,
        preserved_resources: Items<ResourceIdentity>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FallbackCompatibility {
    pub target: RollbackTarget,
    pub fences: PolicyFences,
    pub certificate_valid_until_ms: Option<u64>,
    pub qualification_digest: Sha256Digest,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdmissionProof {
    pub contract_version: ContractVersion,
    pub expected_run_epoch: u64,
    pub expected_state_revision: u64,
    pub expected_accepted: Option<BundleId>,
    pub actual_resources: Items<ResourceIdentity>,
    pub approvals: Items<ApprovalEnvelope>,
    pub fallback: FallbackCompatibility,
    pub caller: CallerId,
    pub protected_artifact_digest: Sha256Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    Running,
    Stopped,
    Failed,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    Disabled,
    Always,
    UnlessStopped,
    OnFailure,
    External,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceReceipt {
    pub resource: ResourceIdentity,
    pub status: ResourceStatus,
    pub restart_policy: RestartPolicy,
    pub effective_bundle: Option<BundleId>,
    pub timestamp_ms: u64,
    pub run_epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseState {
    Armed,
    Activating,
    Verifying,
    Committed,
    Compensating,
    /// Owner-requested restore of the exact previous-compatible bundle.
    RollingBack,
    Compensated,
    StoppedWithArtifacts,
    RecoveryRequired,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lease {
    pub started_from_stopped: bool,
    pub lease_id: LeaseId,
    pub candidate: BundleId,
    pub fallback: FallbackCompatibility,
    pub run_epoch: u64,
    pub deadline_ms: u64,
    pub state: LeaseState,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MaintenanceReservation {
    pub reservation_id: LeaseId,
    pub operation_id: OperationId,
    pub run_epoch: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeState {
    pub schema_version: StateSchemaVersion,
    pub instance_id: InstanceId,
    pub revision: u64,
    pub run_intent: RunIntent,
    pub run_epoch: u64,
    pub accepted: Option<BundleId>,
    pub previous_compatible: Option<BundleId>,
    pub trial: Option<BundleId>,
    pub lease: Option<Lease>,
    pub maintenance: Option<MaintenanceReservation>,
    pub fences: PolicyFences,
    pub resources: Items<ResourceReceipt>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum StateTransition {
    StartTrial {
        lease: Lease,
    },
    RestoreStopped {
        reservation_id: LeaseId,
        accepted: Option<BundleId>,
        previous_compatible: Option<BundleId>,
        credential_revision: u64,
    },
    ReserveMaintenance {
        reservation_id: LeaseId,
    },
    ReleaseMaintenance {
        reservation_id: LeaseId,
    },
    StopIntent,
    StartIntent,
    ArmTrial {
        lease: Lease,
    },
    SetLeaseState {
        lease_id: LeaseId,
        state: LeaseState,
    },
    BeginRollback {
        lease_id: LeaseId,
        target_bundle_id: BundleId,
    },
    CommitTrial {
        lease_id: LeaseId,
    },
    FinishCompensation {
        lease_id: LeaseId,
        restored: RollbackTarget,
    },
    RecordReceipts {
        receipts: Items<ResourceReceipt>,
    },
    RecordOwnedRemovals {
        resources: Items<ResourceIdentity>,
    },
    AdvanceFence {
        credential_revision: u64,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StateWrite {
    pub instance_id: InstanceId,
    pub expected_revision: u64,
    pub expected_run_epoch: u64,
    pub transition: StateTransition,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Budget {
    pub deadline_ms: u64,
    pub max_concurrency: std::num::NonZeroU16,
    pub max_bytes: std::num::NonZeroU32,
    pub max_observations: std::num::NonZeroU16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationContext {
    pub operation_id: OperationId,
    pub caller: CallerId,
    pub budget: Budget,
    pub cancellation_id: Option<OperationId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ExecutorAction {
    StartCandidate {
        bundle_id: BundleId,
        proof: AdmissionProof,
    },
    ReserveMaintenance {
        reservation_id: LeaseId,
    },
    ReleaseMaintenance {
        reservation_id: LeaseId,
    },
    RestoreStopped {
        reservation_id: LeaseId,
        accepted: Option<BundleId>,
        previous_compatible: Option<BundleId>,
        credential_revision: u64,
    },
    AdvanceFence {
        credential_revision: u64,
    },
    Activate {
        bundle_id: BundleId,
        proof: AdmissionProof,
    },
    StartAccepted {
        bundle_id: BundleId,
    },
    /// Restore only the previous-compatible target already bound by the
    /// committed lease and its protected fallback authority.
    RollbackAccepted {
        bundle_id: BundleId,
    },
    /// Restart selected services from the currently admitted accepted bundle.
    /// Core resolves omitted CLI selection to the saved enabled optional/data
    /// service set. Empty selections are rejected by Runtime.
    RestartAccepted {
        bundle_id: BundleId,
        expected_run_epoch: u64,
        service_ids: Items<ServiceId>,
    },
    StopAll,
    UninstallOwned {
        resources: Items<ResourceIdentity>,
    },
    CommitTrial {
        lease_id: LeaseId,
    },
    Recover {
        lease_id: LeaseId,
    },
    Observe,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecuteRequest {
    pub contract_version: ContractVersion,
    pub instance_id: InstanceId,
    pub expected_run_epoch: u64,
    pub action: ExecutorAction,
    pub context: OperationContext,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    UnsupportedVersion,
    InvalidManifest,
    StaleInput,
    ResourceDrift,
    ApprovalRequired,
    NativeEvidenceMissing,
    StagingEvidenceInvalid,
    MissingArtifact,
    MissingImage,
    SecretUnavailable,
    FenceConflict,
    FallbackIncompatible,
    LeaseConflict,
    LeaseExpired,
    Stopped,
    Partial,
    DeadlineExceeded,
    Cancelled,
    RecoveryRequired,
    Io,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairHint {
    RetryObservation,
    RepairConfiguration,
    SupplyLocalApproval,
    RestoreArtifacts,
    ReconcileResources,
    InspectLocalJournal,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeError {
    pub code: ErrorCode,
    pub operation_id: OperationId,
    pub resource_id: Option<ResourceId>,
    pub service_id: Option<ServiceId>,
    pub retryable: bool,
    pub repair_hint: RepairHint,
    pub retained_bundle: Option<BundleId>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultStatus {
    Succeeded,
    Partial,
    Failed,
    RecoveryRequired,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ActionOutcome {
    Completed {
        receipt: ResourceReceipt,
    },
    Unchanged {
        receipt: ResourceReceipt,
    },
    Removed {
        resource: ResourceIdentity,
        timestamp_ms: u64,
    },
    Failed {
        error: RuntimeError,
    },
    Uncertain {
        error: RuntimeError,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionReceipt {
    pub action_id: ActionId,
    pub outcome: ActionOutcome,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResult {
    pub contract_version: ContractVersion,
    pub operation_id: OperationId,
    pub status: ResultStatus,
    pub run_intent: RunIntent,
    pub run_epoch: u64,
    pub state_revision: u64,
    pub accepted: Option<BundleId>,
    pub receipts: Items<ActionReceipt>,
    pub survivors: Items<ResourceId>,
    pub errors: Items<RuntimeError>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeRequest {
    pub bundle_id: BundleId,
    pub manifest_digest: Sha256Digest,
    pub services: Items<ServiceId>,
    pub required_checks: Items<ProbeKind>,
    pub budget: Budget,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeReport {
    pub bundle_id: BundleId,
    pub manifest_digest: Sha256Digest,
    pub evidence: Items<Evidence>,
    pub blocking: bool,
}

/// Canonical serialization helpers shared by compiler and independent admission.
pub fn hash(bytes: &[u8]) -> Sha256Digest {
    // Lowercase hexadecimal and SHA-256 length are guaranteed by the hasher.
    Sha256Digest::new(format!("sha256:{:x}", Sha256::digest(bytes)))
        .expect("SHA-256 encoding invariant")
}
pub fn candidate_digest(bundle: &CandidateBundle) -> Result<Sha256Digest, serde_json::Error> {
    let mut value = serde_json::to_value(bundle)?;
    if let Some(map) = value.as_object_mut() {
        map.remove("manifestDigest");
        map.remove("nativeEvidence");
        map.remove("staging");
    }
    fn canonical(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let sorted: std::collections::BTreeMap<_, _> =
                    map.into_iter().map(|(k, v)| (k, canonical(v))).collect();
                serde_json::Value::Object(sorted.into_iter().collect())
            }
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.into_iter().map(canonical).collect())
            }
            other => other,
        }
    }
    Ok(hash(&serde_json::to_vec(&canonical(value))?))
}

/// Protected installed host identity; never a discovery/adoption instruction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum NativeLocator {
    Container { container_id: String },
    Systemd { unit_name: String },
    Artifacts { service_id: ServiceId },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorizedResource {
    pub identity: ResourceIdentity,
    pub service_id: ServiceId,
    pub locator: NativeLocator,
    pub structural_fingerprint: Sha256Digest,
    pub dependencies: Items<ResourceId>,
    pub enabled_restart_policy: RestartPolicy,
}
/// Persisted by trusted journalled preparation; still independently checked at admission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeAuthority {
    pub schema_version: StateSchemaVersion,
    pub instance_id: InstanceId,
    pub revision: u64,
    pub run_epoch: u64,
    pub bundle_id: BundleId,
    pub bundle_digest: Sha256Digest,
    pub resources: Items<AuthorizedResource>,
    pub approvals: Items<ApprovalEnvelope>,
    pub native_evidence: Items<Evidence>,
    pub staging: Option<StagingAttestation>,
    pub fallbacks: Items<FallbackCompatibility>,
    pub secrets: Items<SecretBinding>,
    pub claims: Items<Claim>,
}

/// Canonical uninitialized, stopped record; construction performs no persistence.
pub fn initial_state(instance: InstanceId) -> RuntimeState {
    RuntimeState {
        schema_version: StateSchemaVersion::V1,
        instance_id: instance,
        revision: 0,
        run_intent: RunIntent::Stopped,
        run_epoch: 0,
        accepted: None,
        previous_compatible: None,
        trial: None,
        lease: None,
        maintenance: None,
        fences: PolicyFences {
            credential_revision: 0,
        },
        resources: Items::default(),
    }
}

/// Exact current installed projection for shutdown before any accepted bundle exists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShutdownAuthorityRecord {
    pub schema_version: StateSchemaVersion,
    pub instance_id: InstanceId,
    pub registry_revision: u64,
    pub resources: Items<AuthorizedResource>,
}
/// Verified native recreation; its original is retained for declared compensation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeBindingRecord {
    pub schema_version: StateSchemaVersion,
    pub instance_id: InstanceId,
    pub resource: AuthorizedResource,
    pub bundle_id: Option<BundleId>,
    pub original: AuthorizedResource,
    pub original_image: Option<ImagePin>,
    pub original_bundle: Option<BundleId>,
}

/// Canonical effective per-service revision; global Compose is launch metadata,
/// and each service revision binds only its own exact launch specification.
pub fn service_revision(
    native: &[(ArtifactId, Sha256Digest)],
    launch: &serde_json::Value,
) -> Result<Sha256Digest, serde_json::Error> {
    let mut native = native.to_vec();
    native.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(hash(&serde_json::to_vec(&(&native, launch))?))
}

/// Single deterministic private R1 network policy, shared by compiler/admission.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivateNetworkLayout {
    pub network_id: String,
    pub subnet: std::net::Ipv4Addr,
    pub prefix: u8,
    pub gateway: std::net::Ipv4Addr,
    pub endpoints: std::collections::BTreeMap<ServiceId, std::net::SocketAddrV4>,
}
pub fn private_network(instance: &InstanceId) -> PrivateNetworkLayout {
    let octet = Sha256::digest(instance.as_str().as_bytes())[0];
    let endpoints = [
        ("haproxy", 10, 4443),
        ("caddy", 11, 8443),
        ("xray", 12, 10000),
        ("unbound", 13, 1053),
    ]
    .into_iter()
    .map(|(name, host, port)| {
        (
            ServiceId::new(name).expect("fixed service ID"),
            std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(172, 30, octet, host), port),
        )
    })
    .collect();
    PrivateNetworkLayout {
        network_id: "vpn-net".into(),
        subnet: std::net::Ipv4Addr::new(172, 30, octet, 0),
        prefix: 24,
        gateway: std::net::Ipv4Addr::new(172, 30, octet, 1),
        endpoints,
    }
}

/// Acknowleged native generation metadata; loaded=false never proves active config.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActiveServiceArtifacts {
    pub bundle_id: BundleId,
    pub service_id: ServiceId,
    pub digest: Sha256Digest,
    pub loaded: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_path_accepts_relative_identifier_segments() {
        for path in ["file.tar", "bundles/xray.json", "a-b/c_d/e.f"] {
            assert!(ArtifactPath::new(path).is_ok(), "should accept {path:?}");
        }
    }

    #[test]
    fn artifact_path_rejects_traversal_and_absolute() {
        for path in [
            "",
            "/etc/passwd",
            "a//b",
            "a/",
            "/a",
            "a/../b",
            "../etc",
            "a/../../b",
            "..",
            "a b/c",
            "a/b?c",
        ] {
            assert!(ArtifactPath::new(path).is_err(), "should reject {path:?}");
        }
    }

    #[test]
    fn hostname_requires_lowercase_labels() {
        for name in ["example.com", "a-b.example.com", "localhost"] {
            assert!(HostName::new(name).is_ok(), "should accept {name:?}");
        }
        for name in [
            "Example.com",
            "EXAMPLE.com",
            "-example.com",
            "example-.com",
            ".example.com",
            "a..b",
        ] {
            assert!(HostName::new(name).is_err(), "should reject {name:?}");
        }
    }

    #[test]
    fn sha256_digest_requires_64_lowercase_hex() {
        let hex64 = "a".repeat(64);
        assert!(Sha256Digest::new(format!("sha256:{hex64}")).is_ok());
        assert!(Sha256Digest::new(format!("sha256:{}", "A".repeat(64))).is_err()); // uppercase
        assert!(Sha256Digest::new(format!("sha256:{}", "a".repeat(63))).is_err()); // short
        assert!(Sha256Digest::new(hex64).is_err()); // missing prefix
    }

    #[test]
    fn hash_matches_known_sha256() {
        assert_eq!(
            hash(b"abc").as_str(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
