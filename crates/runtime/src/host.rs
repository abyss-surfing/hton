//! Protected host composition, independent of management/compiler/YAML.
mod collector;
use crate::{
    Cancellation, Clock,
    actions::{
        EffectGate, EffectGuard, NativeAction, NativeRequest, ShutdownAuthority, ShutdownResource,
    },
    error,
    guardian::GuardianSupervisor,
    manifest::{AdmissionPorts, AdmittedBundle},
    probes::ProbePort,
    state::{RuntimeStateStore, StateRepository},
};
use hton_host::{
    HostObserver,
    docker::{ContainerFacts, ContainerId, OwnedEngine, RestartMode},
    systemd::{UnitAction, UnitFacts, UnitName},
};
use hton_runtime_contract::*;
use hton_storage::{LockedDir, SecureDir};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    path::Path,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub struct SystemClock;
impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|v| v.as_millis().min(u64::MAX as u128) as u64)
            .unwrap_or(0)
    }
}
pub struct LocalCancellation;
impl Cancellation for LocalCancellation {
    fn cancelled(&self, _: &OperationContext) -> bool {
        false
    }
}
struct HostCancel;
impl hton_host::Cancellation for HostCancel {
    fn cancelled(&self) -> bool {
        false
    }
}

pub struct HostRuntime {
    state: SecureDir,
    instance: InstanceId,
    validation_bundle: std::sync::Mutex<Option<BundleId>>,
    execution_gate: std::sync::Mutex<()>,
    bundles: SecureDir,
    authorities: SecureDir,
    effects: SecureDir,
    pub store: RuntimeStateStore,
    observer: HostObserver,
    clock: Arc<SystemClock>,
}
// RAII effect lock: the field is held (never read) and released on drop.
#[allow(dead_code)]
struct HostEffectGuard(LockedDir);
impl EffectGuard for HostEffectGuard {}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActiveArtifacts {
    bundle_id: BundleId,
    service_id: ServiceId,
    digest: Sha256Digest,
    loaded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeRemovalRecord {
    instance_id: InstanceId,
    resource: ResourceIdentity,
    authorized: AuthorizedResource,
    run_epoch: u64,
}

fn host_error(value: hton_host::Error, ctx: &OperationContext) -> RuntimeError {
    error(
        match value {
            hton_host::Error::Deadline => ErrorCode::DeadlineExceeded,
            hton_host::Error::Cancelled => ErrorCode::Cancelled,
            hton_host::Error::IdentityMismatch => ErrorCode::ResourceDrift,
            _ => ErrorCode::Io,
        },
        ctx,
    )
}
impl HostRuntime {
    pub fn open(
        state: &Path,
        bundles: &Path,
        authorities: &Path,
        instance: InstanceId,
    ) -> Result<Self, ErrorCode> {
        if state != Path::new("/opt/hton/work/runtime")
            || bundles != Path::new("/opt/hton/work/bundles")
            || authorities != Path::new("/opt/hton/work/runtime/authorities")
        {
            return Err(ErrorCode::InvalidManifest);
        }
        let state = SecureDir::open_private(state).map_err(|_| ErrorCode::Io)?;
        let clock = Arc::new(SystemClock);
        let store = RuntimeStateStore::new(
            state.try_clone().map_err(|_| ErrorCode::Io)?,
            instance.clone(),
            clock.clone(),
        );
        Ok(Self {
            instance,
            validation_bundle: std::sync::Mutex::new(None),
            execution_gate: std::sync::Mutex::new(()),
            effects: state.child("effects").map_err(|_| ErrorCode::Io)?,
            state,
            bundles: SecureDir::open_private(bundles).map_err(|_| ErrorCode::Io)?,
            authorities: SecureDir::open_private(authorities).map_err(|_| ErrorCode::Io)?,
            store,
            observer: HostObserver::new(),
            clock,
        })
    }
    fn budget(&self, ctx: &OperationContext) -> Result<hton_host::Budget, RuntimeError> {
        let remaining = ctx
            .budget
            .deadline_ms
            .saturating_sub(self.clock.now_ms())
            .min(120_000);
        if remaining == 0 {
            return Err(error(ErrorCode::DeadlineExceeded, ctx));
        }
        hton_host::Budget::new(
            Instant::now() + Duration::from_millis(remaining),
            (ctx.budget.max_bytes.get() as usize).min(4 * 1024 * 1024),
        )
        .map_err(|e| host_error(e, ctx))
    }
    fn read_authority(
        &self,
        bundle: &BundleId,
        ctx: &OperationContext,
    ) -> Result<RuntimeAuthority, RuntimeError> {
        let name = format!("{bundle}.json");
        let bytes = self
            .authorities
            .read(&name, MAX_WIRE_BYTES)
            .map_err(|_| error(ErrorCode::Io, ctx))?
            .ok_or_else(|| error(ErrorCode::ApprovalRequired, ctx))?;
        let record: RuntimeAuthority =
            decode(&bytes).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        if record.bundle_id != *bundle {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        validate_authority(&record, ctx)?;
        Ok(record)
    }
    fn current(&self, ctx: &OperationContext) -> Result<RuntimeAuthority, RuntimeError> {
        if let Some(bundle) = self
            .validation_bundle
            .lock()
            .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?
            .clone()
        {
            return self.read_authority(&bundle, ctx);
        }
        let state = self.store.read(&self.instance, ctx)?;
        if let Some(bundle) = state.trial.as_ref().or(state.accepted.as_ref()) {
            return self.read_authority(bundle, ctx);
        }
        let bytes = self
            .state
            .read("authority.json", MAX_WIRE_BYTES)
            .map_err(|_| error(ErrorCode::Io, ctx))?
            .ok_or_else(|| error(ErrorCode::ApprovalRequired, ctx))?;
        let record: RuntimeAuthority =
            decode(&bytes).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        validate_authority(&record, ctx)?;
        Ok(record)
    }
    fn shutdown(&self, ctx: &OperationContext) -> Result<ShutdownAuthorityRecord, RuntimeError> {
        let bytes = self
            .state
            .read("shutdown.json", MAX_WIRE_BYTES)
            .map_err(|_| error(ErrorCode::Io, ctx))?
            .ok_or_else(|| error(ErrorCode::ApprovalRequired, ctx))?;
        let record: ShutdownAuthorityRecord =
            decode(&bytes).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        if record.instance_id != self.instance {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        validate_resources(record.resources.as_slice(), ctx)?;
        Ok(record)
    }
    fn binding(
        &self,
        id: &ResourceId,
        ctx: &OperationContext,
    ) -> Result<Option<NativeBindingRecord>, RuntimeError> {
        let directory = match self.state.child("native-bindings") {
            Ok(dir) => dir,
            Err(hton_storage::Error::NotFound) => return Ok(None),
            Err(_) => return Err(error(ErrorCode::Io, ctx)),
        };
        let name = format!("{id}.json");
        directory
            .read(&name, MAX_WIRE_BYTES)
            .map_err(|_| error(ErrorCode::Io, ctx))?
            .map(|bytes| decode(&bytes).map_err(|_| error(ErrorCode::RecoveryRequired, ctx)))
            .transpose()
    }
    fn write_binding(
        &self,
        binding: &NativeBindingRecord,
        ctx: &OperationContext,
    ) -> Result<(), RuntimeError> {
        let directory = self
            .state
            .create_child("native-bindings")
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let name = format!("{}.json", binding.resource.identity.resource_id);
        let bytes =
            serde_json::to_vec(binding).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        let locked = directory
            .lock(Instant::now() + Duration::from_secs(1))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let old = locked
            .read(&name, MAX_WIRE_BYTES)
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        locked
            .compare_exchange(&name, old.as_deref(), &bytes)
            .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))
    }
    fn authorized(
        &self,
        id: &ResourceId,
        ctx: &OperationContext,
    ) -> Result<AuthorizedResource, RuntimeError> {
        let base = self
            .shutdown(ctx)?
            .resources
            .as_slice()
            .iter()
            .find(|r| &r.identity.resource_id == id)
            .cloned()
            .ok_or_else(|| error(ErrorCode::ApprovalRequired, ctx))?;
        let Some(binding) = self.binding(id, ctx)? else {
            return Ok(base);
        };
        if binding.instance_id != self.instance
            || binding.resource.identity.resource_id != base.identity.resource_id
            || binding.resource.identity.kind != base.identity.kind
            || binding.resource.identity.ownership != Ownership::Owned
            || base.identity.ownership != Ownership::Owned
            || binding.resource.service_id != base.service_id
            || (base.identity.identity_digest != binding.original.identity.identity_digest
                && base.identity.identity_digest != binding.resource.identity.identity_digest)
        {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        validate_resources(std::slice::from_ref(&binding.resource), ctx).or_else(|_| {
            // Dependencies are validated against the full protected shutdown graph.
            let mut standalone = binding.resource.clone();
            standalone.dependencies = Items::default();
            validate_resources(&[standalone], ctx)
        })?;
        Ok(binding.resource)
    }
    fn active_artifacts(
        &self,
        service: &ServiceId,
        ctx: &OperationContext,
    ) -> Result<Option<ActiveArtifacts>, RuntimeError> {
        let services = match self.state.child("services") {
            Ok(dir) => dir,
            Err(hton_storage::Error::NotFound) => return Ok(None),
            Err(_) => return Err(error(ErrorCode::Io, ctx)),
        };
        let directory = match services.child(service.as_str()) {
            Ok(dir) => dir,
            Err(hton_storage::Error::NotFound) => return Ok(None),
            Err(_) => return Err(error(ErrorCode::Io, ctx)),
        };
        let Some(bytes) = directory
            .read("active.json", 64 * 1024)
            .map_err(|_| error(ErrorCode::Io, ctx))?
        else {
            return Ok(None);
        };
        let active: ActiveArtifacts =
            decode(&bytes).map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?;
        if &active.service_id != service {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        let candidate: CandidateBundle = decode(&self.manifest_bytes(&active.bundle_id, ctx)?)
            .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?;
        let launch = self.candidate_launch(&candidate, service, ctx)?;
        if service_artifact_digest(&candidate, service, &launch)
            .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?
            != active.digest
        {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        let current = directory
            .child("current")
            .map_err(|_| error(ErrorCode::MissingArtifact, ctx))?;
        for artifact in candidate
            .artifacts
            .as_slice()
            .iter()
            .filter(|a| &a.service_id == service && a.path.as_str() != "haproxy/compose.json")
        {
            let (_, name) = artifact_parts(artifact, ctx)?;
            let bytes = current
                .read(name, artifact.size_bytes as usize)
                .map_err(|_| error(ErrorCode::MissingArtifact, ctx))?
                .ok_or_else(|| error(ErrorCode::MissingArtifact, ctx))?;
            if bytes.len() as u64 != artifact.size_bytes
                || crate::manifest::hash(&bytes) != artifact.digest
            {
                return Err(error(ErrorCode::ResourceDrift, ctx));
            }
        }
        Ok(Some(active))
    }
    fn observe_authorized(
        &self,
        resource: &AuthorizedResource,
        ctx: &OperationContext,
    ) -> Result<ResourceReceipt, RuntimeError> {
        let active = self.active_artifacts(&resource.service_id, ctx)?;
        let (identity, status, policy, structural) = match &resource.locator {
            NativeLocator::Container { container_id } => {
                let id = ContainerId::new(container_id).map_err(|e| host_error(e, ctx))?;
                let facts = self
                    .observer
                    .docker()
                    .inspect_container(&id, self.budget(ctx)?, &HostCancel)
                    .map_err(|e| host_error(e, ctx))?;
                (
                    crate::manifest::hash(container_id.as_bytes()),
                    container_status(&facts),
                    docker_policy(&facts.host_config.restart_policy.name)
                        .ok_or_else(|| error(ErrorCode::ResourceDrift, ctx))?,
                    container_structural_fingerprint(&facts),
                )
            }
            NativeLocator::Systemd { unit_name } => {
                let name = UnitName::new(unit_name).map_err(|e| host_error(e, ctx))?;
                let facts = self
                    .observer
                    .systemd()
                    .inspect(&name, self.budget(ctx)?, &HostCancel)
                    .map_err(|e| host_error(e, ctx))?;
                let status = match facts.active_state.as_str() {
                    "active" => ResourceStatus::Running,
                    "inactive" => ResourceStatus::Stopped,
                    "failed" => ResourceStatus::Failed,
                    _ => ResourceStatus::Unknown,
                };
                let policy = match facts.unit_file_state.as_str() {
                    "disabled" | "masked" | "masked-runtime" => RestartPolicy::Disabled,
                    "enabled" | "enabled-runtime" => RestartPolicy::Always,
                    _ => RestartPolicy::External,
                };
                (
                    crate::manifest::hash(unit_name.as_bytes()),
                    status,
                    policy,
                    unit_structural_fingerprint(&facts),
                )
            }
            NativeLocator::Artifacts { .. } => {
                let digest = active
                    .as_ref()
                    .map(|a| a.digest.clone())
                    .unwrap_or_else(|| crate::manifest::hash(&[]));
                (
                    resource.identity.identity_digest.clone(),
                    ResourceStatus::Stopped,
                    RestartPolicy::Disabled,
                    digest,
                )
            }
        };
        if identity != resource.identity.identity_digest
            || structural != resource.structural_fingerprint
        {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        if status == ResourceStatus::Running && active.as_ref().is_some_and(|active| !active.loaded)
        {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        let config = active.as_ref().map(|a| &a.digest);
        let fingerprint = native_state_fingerprint(&identity, &structural, status, policy, config);
        Ok(ResourceReceipt {
            resource: ResourceIdentity {
                fingerprint,
                ..resource.identity.clone()
            },
            status,
            restart_policy: policy,
            effective_bundle: active.map(|a| a.bundle_id),
            timestamp_ms: self.clock.now_ms(),
            run_epoch: self.store.read(&self.instance, ctx)?.run_epoch,
        })
    }
}

/// Canonical structural facts exclude mutable status/restart/current artifact selection.
pub fn container_structural_fingerprint(facts: &ContainerFacts) -> Sha256Digest {
    Sha256Digest::new(hton_host::docker::container_structural_fingerprint(facts))
        .expect("host SHA256 invariant")
}
pub fn unit_structural_fingerprint(facts: &UnitFacts) -> Sha256Digest {
    Sha256Digest::new(&facts.structural_fingerprint).expect("host SHA256 invariant")
}
pub fn native_state_fingerprint(
    identity: &Sha256Digest,
    structural: &Sha256Digest,
    status: ResourceStatus,
    restart: RestartPolicy,
    config: Option<&Sha256Digest>,
) -> Sha256Digest {
    let status = match status {
        ResourceStatus::Running => "running",
        ResourceStatus::Stopped => "stopped",
        ResourceStatus::Failed => "failed",
        ResourceStatus::Unknown => "unknown",
    };
    let restart = match restart {
        RestartPolicy::Disabled => "disabled",
        RestartPolicy::Always => "always",
        RestartPolicy::UnlessStopped => "unless_stopped",
        RestartPolicy::OnFailure => "on_failure",
        RestartPolicy::External => "external",
    };
    Sha256Digest::new(
        hton_host::docker::native_state_fingerprint(
            identity.as_str(),
            structural.as_str(),
            status,
            restart,
            config.map(Sha256Digest::as_str),
        )
        .expect("native fingerprint inputs validated"),
    )
    .expect("host SHA256 invariant")
}
fn container_status(facts: &ContainerFacts) -> ResourceStatus {
    if facts.state.dead || facts.state.oom_killed {
        ResourceStatus::Failed
    } else if facts.state.running && !facts.state.paused && !facts.state.restarting {
        ResourceStatus::Running
    } else if !facts.state.running && matches!(facts.state.status.as_str(), "created" | "exited") {
        ResourceStatus::Stopped
    } else {
        ResourceStatus::Unknown
    }
}
fn docker_policy(value: &str) -> Option<RestartPolicy> {
    Some(match value {
        "no" => RestartPolicy::Disabled,
        "always" => RestartPolicy::Always,
        "unless-stopped" => RestartPolicy::UnlessStopped,
        "on-failure" => RestartPolicy::OnFailure,
        _ => return None,
    })
}
fn validate_authority(
    record: &RuntimeAuthority,
    ctx: &OperationContext,
) -> Result<(), RuntimeError> {
    validate_resources(record.resources.as_slice(), ctx)
}
fn validate_resources(
    entries: &[AuthorizedResource],
    ctx: &OperationContext,
) -> Result<(), RuntimeError> {
    let mut resources = BTreeSet::new();
    if entries.len() > 1024 {
        return Err(error(ErrorCode::InvalidManifest, ctx));
    }
    for resource in entries {
        if !resources.insert(&resource.identity.resource_id) {
            return Err(error(ErrorCode::InvalidManifest, ctx));
        }
        match &resource.locator {
            NativeLocator::Container { container_id } => {
                ContainerId::new(container_id)
                    .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
                if resource.identity.kind != ResourceKind::Container
                    || crate::manifest::hash(container_id.as_bytes())
                        != resource.identity.identity_digest
                {
                    return Err(error(ErrorCode::ResourceDrift, ctx));
                }
            }
            NativeLocator::Systemd { unit_name } => {
                UnitName::new(unit_name).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
                if resource.identity.kind != ResourceKind::SystemdUnit
                    || crate::manifest::hash(unit_name.as_bytes())
                        != resource.identity.identity_digest
                {
                    return Err(error(ErrorCode::ResourceDrift, ctx));
                }
            }
            NativeLocator::Artifacts { service_id } => {
                if service_id != &resource.service_id
                    || resource.identity.kind != ResourceKind::ServiceArtifacts
                    || resource.identity.ownership != Ownership::Owned
                {
                    return Err(error(ErrorCode::ResourceDrift, ctx));
                }
            }
        }
        if resource.enabled_restart_policy == RestartPolicy::External {
            return Err(error(ErrorCode::InvalidManifest, ctx));
        }
    }
    if entries.iter().any(|r| {
        r.dependencies
            .as_slice()
            .iter()
            .any(|dependency| !resources.contains(dependency))
    }) {
        return Err(error(ErrorCode::InvalidManifest, ctx));
    }
    Ok(())
}
impl EffectGate for HostRuntime {
    fn acquire(&self, ctx: &OperationContext) -> Result<Box<dyn EffectGuard + '_>, RuntimeError> {
        let remaining = ctx
            .budget
            .deadline_ms
            .saturating_sub(self.clock.now_ms())
            .min(30_000);
        if remaining == 0 {
            return Err(error(ErrorCode::DeadlineExceeded, ctx));
        }
        Ok(Box::new(HostEffectGuard(
            self.effects
                .lock(Instant::now() + Duration::from_millis(remaining))
                .map_err(|_| error(ErrorCode::Io, ctx))?,
        )))
    }
}
impl ShutdownAuthority for HostRuntime {
    fn resources(
        &self,
        instance: &InstanceId,
        ctx: &OperationContext,
    ) -> Result<Vec<ShutdownResource>, RuntimeError> {
        let authority = self.shutdown(ctx)?;
        if &authority.instance_id != instance {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        Ok(authority
            .resources
            .as_slice()
            .iter()
            .filter(|r| !matches!(r.locator, NativeLocator::Artifacts { .. }))
            .map(|resource| ShutdownResource {
                resource: resource.identity.clone(),
                service_id: resource.service_id.clone(),
                dependencies: resource.dependencies.as_slice().to_vec(),
            })
            .collect())
    }
}
impl NativeAction for HostRuntime {
    fn remove_stopped_owned(
        &self,
        resource: &ResourceIdentity,
        ctx: &OperationContext,
    ) -> Result<(), RuntimeError> {
        let authorized = self.authorized(&resource.resource_id, ctx)?;
        if resource.ownership != Ownership::Owned
            || resource.kind != ResourceKind::Container
            || authorized.identity.ownership != Ownership::Owned
            || authorized.identity.kind != ResourceKind::Container
            || authorized.identity.identity_digest != resource.identity_digest
        {
            return Err(error(ErrorCode::ApprovalRequired, ctx));
        }
        let NativeLocator::Container { container_id } = &authorized.locator else {
            return Err(error(ErrorCode::InvalidManifest, ctx));
        };
        let id = ContainerId::new(container_id).map_err(|e| host_error(e, ctx))?;
        let state = self.store.read(&self.instance, ctx)?;
        if state.run_intent != RunIntent::Stopped {
            return Err(error(ErrorCode::Stopped, ctx));
        }
        let name = format!("remove-{}.json", &resource.identity_digest.as_str()[7..]);
        let locked = self
            .effects
            .lock(
                Instant::now()
                    + Duration::from_millis(
                        ctx.budget
                            .deadline_ms
                            .saturating_sub(self.clock.now_ms())
                            .min(30_000),
                    ),
            )
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let stored = locked
            .read(&name, MAX_WIRE_BYTES)
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let expected = NativeRemovalRecord {
            instance_id: self.instance.clone(),
            resource: resource.clone(),
            authorized: authorized.clone(),
            run_epoch: state.run_epoch,
        };
        let has_intent = if let Some(bytes) = &stored {
            let record: NativeRemovalRecord =
                decode(bytes).map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?;
            if record != expected {
                return Err(error(ErrorCode::ResourceDrift, ctx));
            }
            true
        } else {
            false
        };
        let present = self
            .observer
            .docker()
            .foreign_candidate_facts(self.budget(ctx)?, &HostCancel, 256)
            .map_err(|e| host_error(e, ctx))?
            .iter()
            .any(|facts| facts.container_id == id.as_str());
        if !present {
            return if has_intent {
                Ok(())
            } else {
                Err(error(ErrorCode::ResourceDrift, ctx))
            };
        }
        let before = self.observe_authorized(&authorized, ctx)?;
        if before.resource != *resource
            || before.status != ResourceStatus::Stopped
            || before.restart_policy != RestartPolicy::Disabled
        {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        if !has_intent {
            let bytes = serde_json::to_vec(&expected).map_err(|_| error(ErrorCode::Io, ctx))?;
            locked
                .compare_exchange(&name, None, &bytes)
                .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?;
        }
        let collector = UnitName::new(format!("hton-native-log@{}.service", id.as_str()))
            .map_err(|e| host_error(e, ctx))?;
        let collector_loaded = self
            .observer
            .systemd_control()
            .stop_if_loaded(&collector, self.budget(ctx)?, &HostCancel)
            .map_err(|e| host_error(e, ctx))?;
        if collector_loaded {
            let stopped = self
                .observer
                .systemd()
                .inspect(&collector, self.budget(ctx)?, &HostCancel)
                .map_err(|e| host_error(e, ctx))?;
            if stopped.active_state != "inactive" || stopped.main_pid.is_some_and(|pid| pid != 0) {
                return Err(error(ErrorCode::ResourceDrift, ctx));
            }
        }
        let attempt = self.observer.docker_control().remove_stopped_owned_exact(
            &id,
            authorized.structural_fingerprint.as_str(),
            self.budget(ctx)?,
            &HostCancel,
        );
        let still_present = self
            .observer
            .docker()
            .foreign_candidate_facts(self.budget(ctx)?, &HostCancel, 256)
            .map_err(|e| host_error(e, ctx))?
            .iter()
            .any(|facts| facts.container_id == id.as_str());
        if still_present {
            return Err(attempt
                .err()
                .map(|e| host_error(e, ctx))
                .unwrap_or_else(|| error(ErrorCode::ResourceDrift, ctx)));
        }
        Ok(())
    }
    fn observe(
        &self,
        resource: &ResourceIdentity,
        ctx: &OperationContext,
    ) -> Result<ResourceReceipt, RuntimeError> {
        let authorized = self.authorized(&resource.resource_id, ctx)?;
        if resource.kind != authorized.identity.kind
            || resource.ownership != authorized.identity.ownership
            || resource.identity_digest != authorized.identity.identity_digest
        {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        self.observe_authorized(&authorized, ctx)
    }
    fn matches_admitted(
        &self,
        bundle: &AdmittedBundle,
        resource: &ResourceIdentity,
        observed: &ResourceReceipt,
        ctx: &OperationContext,
    ) -> Result<bool, RuntimeError> {
        let authority = self.read_authority(&bundle.candidate().bundle_id, ctx)?;
        if authority.bundle_digest != bundle.candidate().manifest_digest {
            return Ok(false);
        }
        let Some(authorized) = authority
            .resources
            .as_slice()
            .iter()
            .find(|r| r.identity.resource_id == resource.resource_id)
        else {
            return Ok(false);
        };
        let current = self.authorized(&authorized.identity.resource_id, ctx)?;
        let fresh = self.observe_authorized(&current, ctx)?;
        if resource.ownership == Ownership::Associated {
            return Ok(fresh.resource == observed.resource);
        }
        let active = self.active_artifacts(&authorized.service_id, ctx)?;
        let target = self.admitted_launch(bundle, &authorized.service_id, ctx)?;
        let revision = service_artifact_digest(bundle.candidate(), &authorized.service_id, &target)
            .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        Ok(fresh.resource == observed.resource
            && active
                .as_ref()
                .is_some_and(|active| active.loaded && active.digest == revision)
            && self.service_files_match(bundle, &authorized.service_id, ctx)?)
    }
    fn run(
        &self,
        request: &NativeRequest<'_>,
        ctx: &OperationContext,
    ) -> Result<ResourceReceipt, RuntimeError> {
        let mut authorized = self.authorized(&request.resource().resource_id, ctx)?;
        if authorized.identity.identity_digest != request.resource().identity_digest {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        let kind = request.action().kind;
        if authorized.identity.ownership == Ownership::Associated
            && !matches!(
                kind,
                NativeActionKind::Start
                    | NativeActionKind::Stop
                    | NativeActionKind::DisableAutostart
                    | NativeActionKind::EnableAutostart
            )
        {
            return Err(error(ErrorCode::ApprovalRequired, ctx));
        }
        let mut acknowledge = None;
        if let NativeLocator::Container { container_id } = authorized.locator.clone() {
            let mut id = ContainerId::new(container_id).map_err(|e| host_error(e, ctx))?;
            let driver = self.observer.docker_control();
            let before = self
                .observer
                .docker()
                .inspect_container(&id, self.budget(ctx)?, &HostCancel)
                .map_err(|e| host_error(e, ctx))?;
            if kind == NativeActionKind::Stop
                && self.restore_parked(request, &authorized, &id, ctx)?
            {
                let mut receipt = self.observe_authorized(
                    &self.authorized(&authorized.identity.resource_id, ctx)?,
                    ctx,
                )?;
                receipt.run_epoch = request.run_epoch();
                return Ok(receipt);
            }
            if authorized.identity.ownership == Ownership::Owned
                && matches!(
                    kind,
                    NativeActionKind::Start
                        | NativeActionKind::Restart
                        | NativeActionKind::Recreate
                        | NativeActionKind::Reload
                        | NativeActionKind::ActivateArtifacts
                        | NativeActionKind::RestoreArtifacts
                )
            {
                let bundle = request
                    .bundle()
                    .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?;
                let service = bundle
                    .candidate()
                    .services
                    .as_slice()
                    .iter()
                    .find(|service| service.service_id == authorized.service_id)
                    .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?;
                if kind == NativeActionKind::Reload && service.engine != Engine::Caddy {
                    return Err(error(ErrorCode::NativeEvidenceMissing, ctx));
                }
                if matches!(
                    kind,
                    NativeActionKind::ActivateArtifacts | NativeActionKind::RestoreArtifacts
                ) && before.state.running
                {
                    if self
                        .active_artifacts(&service.service_id, ctx)?
                        .is_some_and(|active| {
                            active.loaded && active.bundle_id == bundle.candidate().bundle_id
                        })
                        && self.service_files_match(bundle, &service.service_id, ctx)?
                    {
                        let mut receipt = self.observe_authorized(&authorized, ctx)?;
                        receipt.run_epoch = request.run_epoch();
                        return Ok(receipt);
                    }
                    return Err(error(ErrorCode::ResourceDrift, ctx));
                }
                let spec = self.compose_service(bundle, &service.service_id, ctx)?;
                let image = self
                    .observer
                    .docker()
                    .inspect_image(
                        &hton_host::docker::ImageReference::new(service.image.as_str())
                            .map_err(|e| host_error(e, ctx))?,
                        self.budget(ctx)?,
                        &HostCancel,
                    )
                    .map_err(|e| host_error(e, ctx))?;
                let target = hton_host::docker::expected_container_fingerprints(
                    &spec,
                    &image,
                    Some(
                        service_artifact_digest(bundle.candidate(), &service.service_id, &spec)
                            .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?
                            .as_str(),
                    ),
                    "stopped",
                    "disabled",
                )
                .map_err(|e| host_error(e, ctx))?;
                let changed = target.structural != authorized.structural_fingerprint.as_str();
                if changed && !matches!(kind, NativeActionKind::Start | NativeActionKind::Recreate)
                {
                    return Err(error(ErrorCode::ResourceDrift, ctx));
                }
                if changed
                    && (request.action().before.is_none() || request.action().after.is_none())
                {
                    return Err(error(ErrorCode::ApprovalRequired, ctx));
                }
                let original_active = self.active_artifacts(&service.service_id, ctx)?;
                if kind != NativeActionKind::Restart {
                    self.install_artifacts(bundle, &service.service_id, ctx)?;
                }
                if changed || kind == NativeActionKind::Recreate {
                    driver
                        .set_restart_policy(
                            &id,
                            RestartMode::Disabled,
                            self.budget(ctx)?,
                            &HostCancel,
                        )
                        .map_err(|e| host_error(e, ctx))?;
                    driver
                        .stop(&id, self.budget(ctx)?, &HostCancel)
                        .map_err(|e| host_error(e, ctx))?;
                    let old = self.binding(&authorized.identity.resource_id, ctx)?;
                    id = driver
                        .recreate_stopped(
                            self.instance.as_str(),
                            bundle.candidate().bundle_id.as_str(),
                            service.service_id.as_str(),
                            self.budget(ctx)?,
                            &HostCancel,
                        )
                        .map_err(|e| host_error(e, ctx))?;
                    let actual = self
                        .observer
                        .docker()
                        .inspect_container(&id, self.budget(ctx)?, &HostCancel)
                        .map_err(|e| host_error(e, ctx))?;
                    if actual.state.running
                        || container_structural_fingerprint(&actual).as_str() != target.structural
                    {
                        return Err(error(ErrorCode::ResourceDrift, ctx));
                    }
                    let original = old
                        .as_ref()
                        .map(|binding| binding.original.clone())
                        .unwrap_or_else(|| authorized.clone());
                    let original_image = match old.as_ref() {
                        Some(binding) => binding.original_image.clone(),
                        None => Some(
                            ImagePin::new(&before.config.image)
                                .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
                        ),
                    };
                    let original_bundle = match old.as_ref() {
                        Some(binding) => binding.original_bundle.clone(),
                        None => original_active.map(|active| active.bundle_id),
                    };
                    authorized.locator = NativeLocator::Container {
                        container_id: id.as_str().to_owned(),
                    };
                    authorized.identity.identity_digest =
                        crate::manifest::hash(id.as_str().as_bytes());
                    authorized.structural_fingerprint = Sha256Digest::new(target.structural)
                        .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
                    let record = NativeBindingRecord {
                        schema_version: StateSchemaVersion::V1,
                        instance_id: self.instance.clone(),
                        resource: authorized.clone(),
                        bundle_id: Some(bundle.candidate().bundle_id.clone()),
                        original,
                        original_image,
                        original_bundle,
                    };
                    self.write_binding(&record, ctx)?;
                }
                acknowledge = Some(bundle);
            }
            match kind {
                NativeActionKind::Start | NativeActionKind::Recreate => {
                    self.ensure_collector(&id, ctx)?;
                    driver
                        .start(&id, self.budget(ctx)?, &HostCancel)
                        .map_err(|e| host_error(e, ctx))?;
                }
                NativeActionKind::Restart => {
                    driver
                        .stop(&id, self.budget(ctx)?, &HostCancel)
                        .map_err(|e| host_error(e, ctx))?;
                    self.ensure_collector(&id, ctx)?;
                    driver
                        .start(&id, self.budget(ctx)?, &HostCancel)
                        .map_err(|e| host_error(e, ctx))?;
                }
                NativeActionKind::Stop => driver
                    .stop(&id, self.budget(ctx)?, &HostCancel)
                    .map_err(|e| host_error(e, ctx))?,
                NativeActionKind::DisableAutostart => driver
                    .set_restart_policy(&id, RestartMode::Disabled, self.budget(ctx)?, &HostCancel)
                    .map_err(|e| host_error(e, ctx))?,
                NativeActionKind::EnableAutostart => driver
                    .set_restart_policy(
                        &id,
                        restart_mode(authorized.enabled_restart_policy)
                            .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?,
                        self.budget(ctx)?,
                        &HostCancel,
                    )
                    .map_err(|e| host_error(e, ctx))?,
                NativeActionKind::Reload => driver
                    .reload(&id, OwnedEngine::Caddy, self.budget(ctx)?, &HostCancel)
                    .map_err(|e| host_error(e, ctx))?,
                NativeActionKind::ActivateArtifacts | NativeActionKind::RestoreArtifacts => (),
            }
            if let Some(bundle) = acknowledge {
                let actual = self
                    .observer
                    .docker()
                    .inspect_container(&id, self.budget(ctx)?, &HostCancel)
                    .map_err(|e| host_error(e, ctx))?;
                let started = matches!(
                    kind,
                    NativeActionKind::Start
                        | NativeActionKind::Restart
                        | NativeActionKind::Recreate
                );
                if started
                    && (!actual.state.running
                        || actual.state.pid == 0
                        || (actual.id == before.id
                            && actual.state.started_at == before.state.started_at))
                {
                    return Err(error(ErrorCode::ResourceDrift, ctx));
                }
                if container_structural_fingerprint(&actual) != authorized.structural_fingerprint {
                    return Err(error(ErrorCode::ResourceDrift, ctx));
                }
                self.acknowledge_artifacts(
                    bundle,
                    &authorized.service_id,
                    actual.state.running,
                    ctx,
                )?;
            }
        } else if let NativeLocator::Systemd { unit_name } = &authorized.locator {
            let action = match kind {
                NativeActionKind::Start => UnitAction::Start,
                NativeActionKind::Stop => UnitAction::Stop,
                NativeActionKind::Restart => UnitAction::Restart,
                NativeActionKind::EnableAutostart => UnitAction::Enable,
                NativeActionKind::DisableAutostart => UnitAction::Disable,
                _ => return Err(error(ErrorCode::InvalidManifest, ctx)),
            };
            self.observer
                .systemd_control()
                .apply(
                    &UnitName::new(unit_name).map_err(|e| host_error(e, ctx))?,
                    action,
                    self.budget(ctx)?,
                    &HostCancel,
                )
                .map_err(|e| host_error(e, ctx))?;
        } else {
            if !matches!(
                kind,
                NativeActionKind::ActivateArtifacts | NativeActionKind::RestoreArtifacts
            ) {
                return Err(error(ErrorCode::InvalidManifest, ctx));
            }
            let bundle = request
                .bundle()
                .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?;
            self.install_artifacts(bundle, &authorized.service_id, ctx)?;
            self.acknowledge_artifacts(bundle, &authorized.service_id, false, ctx)?;
        }
        let mut receipt = self.observe_authorized(&authorized, ctx)?;
        receipt.run_epoch = request.run_epoch();
        Ok(receipt)
    }
}
fn restart_mode(policy: RestartPolicy) -> Option<RestartMode> {
    Some(match policy {
        RestartPolicy::Disabled => RestartMode::Disabled,
        RestartPolicy::Always => RestartMode::Always,
        RestartPolicy::UnlessStopped => RestartMode::UnlessStopped,
        RestartPolicy::OnFailure => RestartMode::OnFailure,
        RestartPolicy::External => return None,
    })
}
fn engine(value: Engine) -> Option<OwnedEngine> {
    Some(match value {
        Engine::Haproxy => OwnedEngine::Haproxy,
        Engine::Caddy => OwnedEngine::Caddy,
        Engine::Xray => OwnedEngine::Xray,
        Engine::Unbound => OwnedEngine::Unbound,
        Engine::Amnezia => return None,
    })
}

fn artifact_parts<'a>(
    artifact: &'a Artifact,
    ctx: &OperationContext,
) -> Result<(&'a str, &'a str), RuntimeError> {
    let (service, name) = artifact
        .path
        .as_str()
        .split_once('/')
        .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?;
    if service != artifact.service_id.as_str() || name.contains('/') {
        return Err(error(ErrorCode::InvalidManifest, ctx));
    }
    Ok((service, name))
}
pub fn service_artifact_digest(
    candidate: &CandidateBundle,
    service: &ServiceId,
    launch: &Value,
) -> Result<Sha256Digest, serde_json::Error> {
    let entries: Vec<_> = candidate
        .artifacts
        .as_slice()
        .iter()
        .filter(|a| &a.service_id == service && a.path.as_str() != "haproxy/compose.json")
        .map(|a| (a.artifact_id.clone(), a.digest.clone()))
        .collect();
    hton_runtime_contract::service_revision(&entries, launch)
}

impl HostRuntime {
    fn service_files_match(
        &self,
        bundle: &AdmittedBundle,
        service: &ServiceId,
        ctx: &OperationContext,
    ) -> Result<bool, RuntimeError> {
        let directory = self
            .state
            .child("services")
            .and_then(|dir| dir.child(service.as_str()))
            .and_then(|dir| dir.child("current"))
            .map_err(|_| error(ErrorCode::MissingArtifact, ctx))?;
        for artifact in bundle
            .candidate()
            .artifacts
            .as_slice()
            .iter()
            .filter(|a| &a.service_id == service && a.path.as_str() != "haproxy/compose.json")
        {
            let (_, name) = artifact_parts(artifact, ctx)?;
            let bytes = directory
                .read(name, artifact.size_bytes as usize)
                .map_err(|_| error(ErrorCode::MissingArtifact, ctx))?
                .ok_or_else(|| error(ErrorCode::MissingArtifact, ctx))?;
            if bytes.len() as u64 != artifact.size_bytes
                || crate::manifest::hash(&bytes) != artifact.digest
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn acknowledge_artifacts(
        &self,
        bundle: &AdmittedBundle,
        service: &ServiceId,
        loaded: bool,
        ctx: &OperationContext,
    ) -> Result<(), RuntimeError> {
        let directory = self
            .state
            .child("services")
            .and_then(|dir| dir.child(service.as_str()))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let metadata = ActiveArtifacts {
            bundle_id: bundle.candidate().bundle_id.clone(),
            service_id: service.clone(),
            digest: service_artifact_digest(
                bundle.candidate(),
                service,
                &self.admitted_launch(bundle, service, ctx)?,
            )
            .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
            loaded,
        };
        let bytes =
            serde_json::to_vec(&metadata).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        let locked = directory
            .lock(Instant::now() + Duration::from_secs(1))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let old = locked
            .read("active.json", 64 * 1024)
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        locked
            .compare_exchange("active.json", old.as_deref(), &bytes)
            .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))
    }
    fn install_artifacts(
        &self,
        bundle: &AdmittedBundle,
        service: &ServiceId,
        ctx: &OperationContext,
    ) -> Result<(), RuntimeError> {
        let authority = self.read_authority(&bundle.candidate().bundle_id, ctx)?;
        if authority.bundle_digest != bundle.candidate().manifest_digest {
            return Err(error(ErrorCode::InvalidManifest, ctx));
        }
        let dir = self
            .state
            .create_child("services")
            .and_then(|dir| dir.create_child(service.as_str()))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let current = dir
            .create_child("current")
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let deadline = Instant::now()
            + Duration::from_millis(
                ctx.budget
                    .deadline_ms
                    .saturating_sub(self.clock.now_ms())
                    .min(30_000),
            );
        let locked = current
            .lock(deadline)
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        for artifact in bundle
            .candidate()
            .artifacts
            .as_slice()
            .iter()
            .filter(|a| &a.service_id == service && a.path.as_str() != "haproxy/compose.json")
        {
            let (_, name) = artifact_parts(artifact, ctx)?;
            let bytes = bundle
                .artifact_bytes(&artifact.artifact_id)
                .ok_or_else(|| error(ErrorCode::MissingArtifact, ctx))?;
            let old = locked
                .read(name, crate::manifest::MAX_ARTIFACT_BYTES)
                .map_err(|_| error(ErrorCode::Io, ctx))?;
            locked
                .compare_exchange(name, old.as_deref(), bytes)
                .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?;
        }
        drop(locked);
        let metadata = ActiveArtifacts {
            bundle_id: bundle.candidate().bundle_id.clone(),
            service_id: service.clone(),
            digest: service_artifact_digest(
                bundle.candidate(),
                service,
                &self.admitted_launch(bundle, service, ctx)?,
            )
            .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
            loaded: false,
        };
        let bytes =
            serde_json::to_vec(&metadata).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        let locked = dir.lock(deadline).map_err(|_| error(ErrorCode::Io, ctx))?;
        let old = locked
            .read("active.json", 64 * 1024)
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        locked
            .compare_exchange("active.json", old.as_deref(), &bytes)
            .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))
    }
}
impl AdmissionPorts for HostRuntime {
    fn manifest_bytes(
        &self,
        bundle: &BundleId,
        ctx: &OperationContext,
    ) -> Result<Vec<u8>, RuntimeError> {
        self.bundles
            .child(bundle.as_str())
            .and_then(|dir| dir.read("manifest.json", MAX_WIRE_BYTES))
            .map_err(|_| error(ErrorCode::MissingArtifact, ctx))?
            .ok_or_else(|| error(ErrorCode::MissingArtifact, ctx))
    }
    fn artifact_bytes(
        &self,
        bundle: &BundleId,
        artifact: &Artifact,
        limit: usize,
        ctx: &OperationContext,
    ) -> Result<Vec<u8>, RuntimeError> {
        let (service, name) = artifact_parts(artifact, ctx)?;
        self.bundles
            .child(bundle.as_str())
            .and_then(|dir| dir.child(service))
            .and_then(|dir| dir.read(name, limit))
            .map_err(|_| error(ErrorCode::MissingArtifact, ctx))?
            .ok_or_else(|| error(ErrorCode::MissingArtifact, ctx))
    }
    fn observe(
        &self,
        expected: &ResourceIdentity,
        ctx: &OperationContext,
    ) -> Result<ResourceIdentity, RuntimeError> {
        Ok(NativeAction::observe(self, expected, ctx)?.resource)
    }
    fn image_cached(&self, image: &ImagePin, ctx: &OperationContext) -> Result<bool, RuntimeError> {
        let reference = hton_host::docker::ImageReference::new(image.as_str())
            .map_err(|e| host_error(e, ctx))?;
        Ok(self
            .observer
            .docker()
            .cached_image(&reference, self.budget(ctx)?, &HostCancel)
            .map_err(|e| host_error(e, ctx))?
            .is_some_and(|facts| {
                facts.os == "linux"
                    && facts.architecture
                        == match std::env::consts::ARCH {
                            "x86_64" => "amd64",
                            "aarch64" => "arm64",
                            _ => "unsupported",
                        }
            }))
    }
    fn secret_available(
        &self,
        binding: &SecretBinding,
        ctx: &OperationContext,
    ) -> Result<bool, RuntimeError> {
        Ok(self.current(ctx)?.secrets.as_slice().contains(binding))
    }
    fn approval_valid(
        &self,
        approval: &ApprovalEnvelope,
        caller: &CallerId,
        digest: &Sha256Digest,
        ctx: &OperationContext,
    ) -> Result<bool, RuntimeError> {
        let authority = self.current(ctx)?;
        Ok(&ctx.caller == caller
            && approval.issuer == *caller
            && approval.scope_digest == *digest
            && authority.bundle_digest == *digest
            && authority.approvals.as_slice().contains(approval)
            && approval.expires_ms > self.clock.now_ms())
    }
    fn native_evidence_valid(
        &self,
        bundle: &CandidateBundle,
        service: &Service,
        evidence: &Evidence,
        ctx: &OperationContext,
    ) -> Result<bool, RuntimeError> {
        let authority = self.read_authority(&bundle.bundle_id, ctx)?;
        Ok(authority.instance_id == bundle.instance_id
            && authority.bundle_digest == bundle.manifest_digest
            && authority.native_evidence.as_slice().contains(evidence)
            && evidence.vantage == Vantage::LocalNative
            && evidence.kind == ProbeKind::NativeConfig
            && evidence.service_id == service.service_id
            && evidence.input_digest
                == crate::manifest::hash(
                    &serde_json::to_vec(service)
                        .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
                ))
    }
    fn fallback_valid(
        &self,
        fallback: &FallbackCompatibility,
        ctx: &OperationContext,
    ) -> Result<bool, RuntimeError> {
        let authority = self.current(ctx)?;
        if !authority.fallbacks.as_slice().contains(fallback)
            || fallback
                .certificate_valid_until_ms
                .is_some_and(|until| until <= self.clock.now_ms())
        {
            return Ok(false);
        }
        match &fallback.target {
            RollbackTarget::Accepted {
                bundle_id,
                manifest_digest,
            } => {
                let candidate: CandidateBundle = decode(&self.manifest_bytes(bundle_id, ctx)?)
                    .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
                Ok(candidate.manifest_digest == *manifest_digest
                    && crate::manifest::candidate_digest(&candidate)
                        .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?
                        == *manifest_digest
                    && candidate.fences.credential_revision >= fallback.fences.credential_revision)
            }
            RollbackTarget::PreInstall {
                closed_listeners,
                preserved_resources,
            } => Ok(closed_listeners.as_slice().iter().all(|id| {
                authority.resources.as_slice().iter().any(|r| {
                    &r.identity.resource_id == id && r.identity.ownership == Ownership::Owned
                })
            }) && preserved_resources.as_slice().iter().all(|r| {
                authority
                    .resources
                    .as_slice()
                    .iter()
                    .any(|known| known.identity == *r)
            })),
        }
    }
    fn staging_valid(
        &self,
        bundle: &CandidateBundle,
        attestation: &StagingAttestation,
        ctx: &OperationContext,
    ) -> Result<bool, RuntimeError> {
        let authority = self.read_authority(&bundle.bundle_id, ctx)?;
        Ok(authority.bundle_digest == bundle.manifest_digest
            && authority.staging.as_ref() == Some(attestation))
    }
    fn claims_valid(
        &self,
        bundle: &CandidateBundle,
        ctx: &OperationContext,
    ) -> Result<bool, RuntimeError> {
        let authority = self.read_authority(&bundle.bundle_id, ctx)?;
        if authority.instance_id != bundle.instance_id
            || authority.bundle_digest != bundle.manifest_digest
            || authority.claims != bundle.claims
        {
            return Ok(false);
        }
        let Some(artifact) = bundle
            .artifacts
            .as_slice()
            .iter()
            .find(|a| a.path.as_str() == "haproxy/compose.json")
        else {
            return Ok(false);
        };
        let bytes = self.artifact_bytes(
            &bundle.bundle_id,
            artifact,
            artifact.size_bytes as usize,
            ctx,
        )?;
        if crate::manifest::hash(&bytes) != artifact.digest {
            return Ok(false);
        }
        validate_compose(bundle, &bytes).map_err(|_| error(ErrorCode::InvalidManifest, ctx))
    }
}

/// Independent parser for the emitted Compose subset; unrecognized authority is refused.
pub fn validate_compose(bundle: &CandidateBundle, bytes: &[u8]) -> Result<bool, ()> {
    let root: Value = serde_json::from_slice(bytes).map_err(|_| ())?;
    let root = root.as_object().ok_or(())?;
    if root
        .keys()
        .any(|key| !matches!(key.as_str(), "name" | "services" | "networks"))
        || root.get("name").and_then(Value::as_str)
            != Some(format!("hton-{}", bundle.instance_id).as_str())
    {
        return Ok(false);
    }
    let services = root.get("services").and_then(Value::as_object).ok_or(())?;
    let expected: Vec<_> = bundle
        .services
        .as_slice()
        .iter()
        .filter(|service| service.resource.ownership == Ownership::Owned)
        .collect();
    if services.len() != expected.len() {
        return Ok(false);
    }
    let network = root.get("networks").and_then(Value::as_object).ok_or(())?;
    if network.len() != 1
        || network
            .get("hton")
            .and_then(|value| value.get("driver"))
            .and_then(Value::as_str)
            != Some("bridge")
    {
        return Ok(false);
    }
    let net = network["hton"].as_object().ok_or(())?;
    if net
        .keys()
        .any(|key| !matches!(key.as_str(), "name" | "driver" | "ipam"))
        || net.get("name").and_then(Value::as_str)
            != Some(format!("hton-{}", bundle.instance_id).as_str())
    {
        return Ok(false);
    }
    let mut published = BTreeSet::new();
    for service in expected {
        let spec = services
            .get(service.service_id.as_str())
            .and_then(Value::as_object)
            .ok_or(())?;
        if spec.keys().any(|key| {
            !matches!(
                key.as_str(),
                "image"
                    | "container_name"
                    | "entrypoint"
                    | "command"
                    | "user"
                    | "read_only"
                    | "cap_drop"
                    | "security_opt"
                    | "restart"
                    | "init"
                    | "mem_limit"
                    | "cpus"
                    | "pids_limit"
                    | "stop_grace_period"
                    | "tmpfs"
                    | "volumes"
                    | "networks"
                    | "logging"
                    | "labels"
                    | "ports"
            )
        }) {
            return Ok(false);
        }
        if spec.get("image").and_then(Value::as_str) != Some(service.image.as_str())
            || spec.get("container_name").and_then(Value::as_str)
                != Some(format!("hton-{}-{}", bundle.instance_id, service.service_id).as_str())
            || spec.get("user") != Some(&json!("0:0"))
            || spec.get("read_only") != Some(&json!(true))
            || spec.get("cap_drop") != Some(&json!(["ALL"]))
            || spec.get("security_opt") != Some(&json!(["no-new-privileges:true"]))
            || spec.get("logging") != Some(&json!({"driver":"none"}))
            || spec.get("restart") != Some(&json!("no"))
            || spec.get("init") != Some(&json!(true))
        {
            return Ok(false);
        }
        let (entry, command) = match service.engine {
            Engine::Haproxy => (
                "haproxy",
                json!(["-W", "-db", "-f", "/etc/hton/current/haproxy.cfg"]),
            ),
            Engine::Caddy => (
                "caddy",
                json!([
                    "run",
                    "--config",
                    "/etc/hton/current/Caddyfile",
                    "--adapter",
                    "caddyfile"
                ]),
            ),
            Engine::Xray => (
                "xray",
                json!(["run", "-config", "/etc/hton/current/xray.json"]),
            ),
            Engine::Unbound => (
                "/usr/local/unbound/unbound.d/sbin/unbound",
                json!(["-d", "-c", "/etc/hton/current/unbound.conf"]),
            ),
            Engine::Amnezia => return Ok(false),
        };
        if spec.get("entrypoint") != Some(&json!([entry])) || spec.get("command") != Some(&command)
        {
            return Ok(false);
        }
        let expected_mount = json!([{"type":"bind","source":format!("/opt/hton/work/runtime/services/{}",service.service_id),"target":"/etc/hton","read_only":true}]);
        if spec.get("volumes") != Some(&expected_mount)
            || spec.get("tmpfs")
                != Some(&json!([
                    "/tmp:rw,noexec,nosuid,size=16m",
                    "/run:rw,noexec,nosuid,size=4m"
                ]))
            || spec.get("stop_grace_period") != Some(&json!("30s"))
        {
            return Ok(false);
        }
        let memory = spec
            .get("mem_limit")
            .and_then(Value::as_str)
            .and_then(|v| v.strip_suffix('m'))
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or(())?;
        let cpus = spec
            .get("cpus")
            .and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
            })
            .ok_or(())?;
        if !(32..=4096).contains(&memory)
            || !cpus.is_finite()
            || !(0.05..=16.0).contains(&cpus)
            || !spec
                .get("pids_limit")
                .and_then(Value::as_u64)
                .is_some_and(|v| (16..=512).contains(&v))
        {
            return Ok(false);
        }
        let labels = spec.get("labels").and_then(Value::as_object).ok_or(())?;
        if labels.get("io.hton.instance").and_then(Value::as_str)
            != Some(bundle.instance_id.as_str())
            || labels.get("io.hton.service").and_then(Value::as_str)
                != Some(service.service_id.as_str())
            || labels.len() > 34
            || labels.iter().any(|(key, value)| {
                key.len() > 128
                    || value.as_str().is_none_or(|v| v.len() > 512)
                    || key.starts_with("com.docker.")
            })
        {
            return Ok(false);
        }
        let nets = spec.get("networks").and_then(Value::as_object).ok_or(())?;
        if nets.len() != 1
            || nets
                .get("hton")
                .and_then(Value::as_object)
                .is_none_or(|map| map.len() != 1)
            || nets
                .get("hton")
                .and_then(|v| v.get("ipv4_address"))
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<std::net::Ipv4Addr>().ok())
                .is_none()
        {
            return Ok(false);
        }
        if let Some(ports) = spec.get("ports") {
            for port in ports.as_array().ok_or(())? {
                let p = port.as_object().ok_or(())?;
                if p.len() != 4
                    || p.keys().any(|k| {
                        !matches!(k.as_str(), "target" | "published" | "host_ip" | "protocol")
                    })
                {
                    return Ok(false);
                }
                let address = p
                    .get("host_ip")
                    .and_then(Value::as_str)
                    .and_then(|v| v.parse::<std::net::Ipv4Addr>().ok())
                    .ok_or(())?;
                let number = p
                    .get("published")
                    .and_then(Value::as_str)
                    .and_then(|v| v.parse::<u16>().ok())
                    .filter(|v| *v > 0)
                    .ok_or(())?;
                let target = p
                    .get("target")
                    .and_then(Value::as_u64)
                    .filter(|v| (1024..=65535).contains(v))
                    .ok_or(())?;
                let transport = match p.get("protocol").and_then(Value::as_str) {
                    Some("tcp") => Transport::Tcp,
                    Some("udp") => Transport::Udp,
                    _ => return Ok(false),
                };
                if !bundle.claims.as_slice().iter().any(|claim|matches!(claim,Claim::Listener{service:id,address:ip,port,transport:kind}if id==&service.service_id&&ip==&address&&port.get()==number&&kind==&transport)){return Ok(false);}
                if !published.insert((address, number, transport as u8, target)) {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GuardianAck {
    instance_id: InstanceId,
    lease_id: LeaseId,
    run_epoch: u64,
    pid: u32,
    timestamp_ms: u64,
}
impl GuardianSupervisor for HostRuntime {
    fn ensure_monitoring(
        &self,
        instance: &InstanceId,
        lease: &Lease,
        ctx: &OperationContext,
    ) -> Result<(), RuntimeError> {
        let unit = UnitName::new("hton-guardian.service").expect("fixed unit");
        self.observer
            .systemd_control()
            .apply(&unit, UnitAction::Start, self.budget(ctx)?, &HostCancel)
            .map_err(|e| host_error(e, ctx))?;
        loop {
            if self.clock.now_ms() >= ctx.budget.deadline_ms {
                return Err(error(ErrorCode::DeadlineExceeded, ctx));
            }
            if let Some(bytes) = self
                .state
                .read("guardian.json", 64 * 1024)
                .map_err(|_| error(ErrorCode::Io, ctx))?
            {
                if let Ok(ack) = decode::<GuardianAck>(&bytes) {
                    let observed = self
                        .observer
                        .systemd()
                        .inspect(&unit, self.budget(ctx)?, &HostCancel)
                        .map_err(|e| host_error(e, ctx))?;
                    if ack.instance_id == *instance
                        && ack.lease_id == lease.lease_id
                        && ack.run_epoch == lease.run_epoch
                        && ack.timestamp_ms <= self.clock.now_ms()
                        && ack.timestamp_ms >= self.clock.now_ms().saturating_sub(5000)
                        && observed.active_state == "active"
                        && observed.main_pid == Some(ack.pid)
                        && ack.pid > 1
                    {
                        return Ok(());
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
impl ProbePort for HostRuntime {
    fn probe(
        &self,
        bundle: &AdmittedBundle,
        service: &Service,
        kind: ProbeKind,
        ctx: &OperationContext,
    ) -> Result<Evidence, RuntimeError> {
        if kind != ProbeKind::ResourceIdentity {
            return Err(error(ErrorCode::NativeEvidenceMissing, ctx));
        }
        let observed = NativeAction::observe(self, &service.resource, ctx)?;
        let matches = self.matches_admitted(bundle, &service.resource, &observed, ctx)?;
        Ok(Evidence {
            check_id: CheckId::new(format!("resource-{}", service.service_id))
                .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
            service_id: service.service_id.clone(),
            kind,
            vantage: Vantage::LocalRuntime,
            outcome: if matches && observed.status == ResourceStatus::Running {
                EvidenceOutcome::Pass
            } else {
                EvidenceOutcome::Fail
            },
            timestamp_ms: self.clock.now_ms(),
            bundle_digest: bundle.candidate().manifest_digest.clone(),
            input_digest: crate::manifest::hash(
                &serde_json::to_vec(service).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
            ),
            observed_identity: Some(observed.resource.identity_digest),
        })
    }
}
impl HostRuntime {
    fn execute_inner(
        &self,
        request: &ExecuteRequest,
        logs: &hton_logging::LogSink,
    ) -> Result<RuntimeResult, RuntimeError> {
        let cancellation = LocalCancellation;
        let actions = crate::actions::ActionExecutor {
            store: &self.store,
            gate: self,
            native: self,
            clock: self.clock.as_ref(),
            cancellation: &cancellation,
            logs,
        };
        let guardian = crate::guardian::RuntimeGuardian {
            store: &self.store,
            admission: self,
            actions: &actions,
            probes: self,
            clock: self.clock.as_ref(),
            cancellation: &cancellation,
            supervisor: self,
        };
        crate::RuntimeExecutor {
            store: &self.store,
            admission: self,
            actions: &actions,
            guardian: &guardian,
            shutdown: self,
            probes: self,
            clock: self.clock.as_ref(),
            cancellation: &cancellation,
        }
        .execute(request)
    }

    /// Installed-host candidate start: same admit → arm → apply → verify flow as
    /// `Activate`, issued through the independent host path (already select-validated
    /// by `execute`). Delegates to the single executor so business logic is not duplicated.
    fn start_candidate(
        &self,
        request: &ExecuteRequest,
        logs: &hton_logging::LogSink,
    ) -> Result<RuntimeResult, RuntimeError> {
        let ExecutorAction::StartCandidate { bundle_id, proof } = &request.action else {
            return Err(error(ErrorCode::InvalidManifest, &request.context));
        };
        let mut translated = request.clone();
        translated.action = ExecutorAction::Activate {
            bundle_id: bundle_id.clone(),
            proof: proof.clone(),
        };
        self.execute_inner(&translated, logs)
    }

    pub fn execute(
        &self,
        request: &ExecuteRequest,
        logs: &hton_logging::LogSink,
    ) -> Result<RuntimeResult, RuntimeError> {
        let ctx = &request.context;
        let caller = CallerId::new(format!("os.uid.{}", rustix::process::geteuid().as_raw()))
            .expect("OS caller ID");
        if request.context.caller != caller
            || rustix::process::geteuid().as_raw() != 0
            || request.context.cancellation_id.is_some()
        {
            return Err(error(ErrorCode::ApprovalRequired, ctx));
        }
        if self.instance != request.instance_id {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        crate::check_budget(ctx, self.clock.as_ref(), &LocalCancellation)?;
        let _execution = loop {
            match self.execution_gate.try_lock() {
                Ok(guard) => break guard,
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    return Err(error(ErrorCode::RecoveryRequired, ctx));
                }
                Err(std::sync::TryLockError::WouldBlock) => {
                    if self.clock.now_ms() >= ctx.budget.deadline_ms {
                        return Err(error(ErrorCode::DeadlineExceeded, ctx));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        };
        let selected = match &request.action {
            ExecutorAction::Activate { bundle_id, .. }
            | ExecutorAction::StartCandidate { bundle_id, .. }
            | ExecutorAction::StartAccepted { bundle_id }
            | ExecutorAction::RestartAccepted { bundle_id, .. } => Some(bundle_id.clone()),
            _ => None,
        };
        let _selection = self.select_validation(selected, ctx)?;
        match &request.action {
            ExecutorAction::Observe => self.observe_accepted(request),
            ExecutorAction::ReserveMaintenance { .. }
            | ExecutorAction::ReleaseMaintenance { .. }
            | ExecutorAction::RestoreStopped { .. }
            | ExecutorAction::AdvanceFence { .. } => self.metadata(request),
            ExecutorAction::StartCandidate { .. } => self.start_candidate(request, logs),
            _ => self.execute_inner(request, logs),
        }
    }
    pub fn boot(
        &self,
        ctx: &OperationContext,
        logs: &hton_logging::LogSink,
    ) -> Result<Option<RuntimeResult>, RuntimeError> {
        let state = self.store.read(&self.instance, ctx)?;
        if state.run_intent == RunIntent::Stopped {
            return Ok(None);
        }
        let bundle = state
            .accepted
            .ok_or_else(|| error(ErrorCode::MissingArtifact, ctx))?;
        let request = ExecuteRequest {
            contract_version: ContractVersion::V1,
            instance_id: self.instance.clone(),
            expected_run_epoch: state.run_epoch,
            action: ExecutorAction::StartAccepted { bundle_id: bundle },
            context: ctx.clone(),
        };
        self.execute(&request, logs).map(Some)
    }
    pub fn monitor_once(
        &self,
        ctx: &OperationContext,
        logs: &hton_logging::LogSink,
    ) -> Result<(), RuntimeError> {
        let state = self.store.read(&self.instance, ctx)?;
        let Some(lease) = state.lease.filter(|lease| {
            matches!(
                lease.state,
                LeaseState::Armed
                    | LeaseState::Activating
                    | LeaseState::Verifying
                    | LeaseState::Compensating
                    | LeaseState::RollingBack
                    | LeaseState::RecoveryRequired
            )
        }) else {
            return Ok(());
        };
        let ack = GuardianAck {
            instance_id: self.instance.clone(),
            lease_id: lease.lease_id.clone(),
            run_epoch: lease.run_epoch,
            pid: std::process::id(),
            timestamp_ms: self.clock.now_ms(),
        };
        let bytes = serde_json::to_vec(&ack).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        let locked = self
            .state
            .lock(Instant::now() + Duration::from_secs(1))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let old = locked
            .read("guardian.json", 64 * 1024)
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        locked
            .compare_exchange("guardian.json", old.as_deref(), &bytes)
            .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?;
        drop(locked);
        let cancellation = LocalCancellation;
        let actions = crate::actions::ActionExecutor {
            store: &self.store,
            gate: self,
            native: self,
            clock: self.clock.as_ref(),
            cancellation: &cancellation,
            logs,
        };
        let guardian = crate::guardian::RuntimeGuardian {
            store: &self.store,
            admission: self,
            actions: &actions,
            probes: self,
            clock: self.clock.as_ref(),
            cancellation: &cancellation,
            supervisor: self,
        };
        guardian
            .monitor(&self.instance, &lease.lease_id, ctx)
            .map(|_| ())
    }
    pub fn logging(&self) -> Result<hton_logging::LogSink, ErrorCode> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Config {
            directory: String,
            max_bytes: u64,
            retention_days: u64,
            level: String,
        }
        let bytes = self
            .state
            .read("logging.json", 64 * 1024)
            .map_err(|_| ErrorCode::Io)?
            .ok_or(ErrorCode::Io)?;
        let config: Config =
            serde_json::from_slice(&bytes).map_err(|_| ErrorCode::InvalidManifest)?;

        if !Path::new(&config.directory).is_absolute()
            || !(64 * 1024..=4 * 1024 * 1024 * 1024).contains(&config.max_bytes)
            || !(1..=7).contains(&config.retention_days)
        {
            return Err(ErrorCode::InvalidManifest);
        }
        let directory =
            SecureDir::open_private(Path::new(&config.directory)).map_err(|_| ErrorCode::Io)?;
        let level = match config.level.as_str() {
            "error" => hton_logging::Severity::Error,
            "warning" | "warn" => hton_logging::Severity::Warning,
            "info" => hton_logging::Severity::Information,
            "debug" => hton_logging::Severity::Debug,
            "trace" => hton_logging::Severity::Trace,
            _ => return Err(ErrorCode::InvalidManifest),
        };
        let policy =
            hton_logging::LoggingPolicy::bounded(level, config.max_bytes, config.retention_days)
                .map_err(|_| ErrorCode::InvalidManifest)?;
        hton_logging::LogSink::configure(policy, directory).map_err(|_| ErrorCode::InvalidManifest)
    }
}

impl HostRuntime {
    fn candidate_launch(
        &self,
        candidate: &CandidateBundle,
        service: &ServiceId,
        ctx: &OperationContext,
    ) -> Result<Value, RuntimeError> {
        let artifact = candidate
            .artifacts
            .as_slice()
            .iter()
            .find(|artifact| artifact.path.as_str() == "haproxy/compose.json")
            .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?;
        let bytes = self.artifact_bytes(&candidate.bundle_id, artifact, MAX_WIRE_BYTES, ctx)?;
        if crate::manifest::hash(&bytes) != artifact.digest
            || !validate_compose(candidate, &bytes)
                .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?
        {
            return Err(error(ErrorCode::InvalidManifest, ctx));
        }
        launch_from_bytes(&bytes, service, ctx)
    }
    fn admitted_launch(
        &self,
        bundle: &AdmittedBundle,
        service: &ServiceId,
        ctx: &OperationContext,
    ) -> Result<Value, RuntimeError> {
        let artifact = bundle
            .candidate()
            .artifacts
            .as_slice()
            .iter()
            .find(|artifact| artifact.path.as_str() == "haproxy/compose.json")
            .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?;
        let bytes = bundle
            .artifact_bytes(&artifact.artifact_id)
            .ok_or_else(|| error(ErrorCode::MissingArtifact, ctx))?;
        if !validate_compose(bundle.candidate(), bytes)
            .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?
        {
            return Err(error(ErrorCode::InvalidManifest, ctx));
        }
        launch_from_bytes(bytes, service, ctx)
    }
    fn compose_service(
        &self,
        bundle: &AdmittedBundle,
        service: &ServiceId,
        ctx: &OperationContext,
    ) -> Result<Value, RuntimeError> {
        let artifact = bundle
            .candidate()
            .artifacts
            .as_slice()
            .iter()
            .find(|artifact| artifact.path.as_str() == "haproxy/compose.json")
            .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?;
        let bytes = bundle
            .artifact_bytes(&artifact.artifact_id)
            .ok_or_else(|| error(ErrorCode::MissingArtifact, ctx))?;
        if !validate_compose(bundle.candidate(), bytes)
            .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?
        {
            return Err(error(ErrorCode::InvalidManifest, ctx));
        }
        let value: Value =
            serde_json::from_slice(bytes).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        let spec = value
            .get("services")
            .and_then(|services| services.get(service.as_str()))
            .cloned()
            .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?;
        // Execute retained admitted bytes from the runtime-owned tree, never a
        // potentially replaced compiler-owned Compose path after admission.
        let directory = self
            .state
            .create_child("execution")
            .and_then(|dir| dir.create_child(bundle.candidate().bundle_id.as_str()))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let locked = directory
            .lock(Instant::now() + Duration::from_secs(1))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let old = locked
            .read("compose.json", MAX_WIRE_BYTES)
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        if old.as_deref().is_some_and(|old| old != bytes) {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        locked
            .compare_exchange("compose.json", old.as_deref(), bytes)
            .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?;
        Ok(spec)
    }
    fn restore_parked(
        &self,
        request: &NativeRequest<'_>,
        resource: &AuthorizedResource,
        current: &ContainerId,
        ctx: &OperationContext,
    ) -> Result<bool, RuntimeError> {
        let Some(after) = request.action().after.as_ref() else {
            return Ok(false);
        };
        let Some(mut binding) = self.binding(&resource.identity.resource_id, ctx)? else {
            return Ok(false);
        };
        if binding.original_bundle.is_some() {
            return Ok(false);
        }
        let expected = native_state_fingerprint(
            &binding.original.identity.identity_digest,
            &binding.original.structural_fingerprint,
            ResourceStatus::Stopped,
            RestartPolicy::Disabled,
            None,
        );
        if after != &expected {
            return Ok(false);
        }
        let state = self.store.read(&self.instance, ctx)?;
        let Some(lease) = state.lease.as_ref() else {
            return Err(error(ErrorCode::ApprovalRequired, ctx));
        };
        if !matches!(&lease.fallback.target,RollbackTarget::PreInstall{closed_listeners,..}if closed_listeners.as_slice().contains(&resource.identity.resource_id))
            || resource.identity.ownership != Ownership::Owned
        {
            return Err(error(ErrorCode::ApprovalRequired, ctx));
        }
        let bundle = request
            .bundle()
            .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?;
        let service = bundle
            .candidate()
            .services
            .as_slice()
            .iter()
            .find(|service| service.service_id == resource.service_id)
            .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?;
        let image = binding
            .original_image
            .as_ref()
            .ok_or_else(|| error(ErrorCode::RecoveryRequired, ctx))?;
        let image = hton_host::docker::ImageReference::new(image.as_str())
            .map_err(|e| host_error(e, ctx))?;
        let driver = self.observer.docker_control();
        driver
            .set_restart_policy(
                current,
                RestartMode::Disabled,
                self.budget(ctx)?,
                &HostCancel,
            )
            .map_err(|e| host_error(e, ctx))?;
        driver
            .stop(current, self.budget(ctx)?, &HostCancel)
            .map_err(|e| host_error(e, ctx))?;
        driver
            .remove_stopped_owned(current, self.budget(ctx)?, &HostCancel)
            .map_err(|e| host_error(e, ctx))?;
        let id = driver
            .create_parked(
                self.instance.as_str(),
                resource.service_id.as_str(),
                &image,
                engine(service.engine).ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))?,
                self.budget(ctx)?,
                &HostCancel,
            )
            .map_err(|e| host_error(e, ctx))?;
        let actual = self
            .observer
            .docker()
            .inspect_container(&id, self.budget(ctx)?, &HostCancel)
            .map_err(|e| host_error(e, ctx))?;
        if actual.state.running
            || container_structural_fingerprint(&actual) != binding.original.structural_fingerprint
        {
            return Err(error(ErrorCode::ResourceDrift, ctx));
        }
        binding.resource = binding.original.clone();
        binding.resource.locator = NativeLocator::Container {
            container_id: id.as_str().to_owned(),
        };
        binding.resource.identity.identity_digest = crate::manifest::hash(id.as_str().as_bytes());
        binding.bundle_id = None;
        self.write_binding(&binding, ctx)?;
        let directory = self
            .state
            .child("services")
            .and_then(|dir| dir.child(resource.service_id.as_str()))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let locked = directory
            .lock(Instant::now() + Duration::from_secs(1))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        if let Some(bytes) = locked
            .read("active.json", 64 * 1024)
            .map_err(|_| error(ErrorCode::Io, ctx))?
        {
            locked
                .remove("active.json", &bytes)
                .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?;
        }
        Ok(true)
    }
    fn observe_accepted(&self, request: &ExecuteRequest) -> Result<RuntimeResult, RuntimeError> {
        let ctx = &request.context;
        let state = self.store.read(&request.instance_id, ctx)?;
        if state.run_epoch != request.expected_run_epoch {
            return Err(error(ErrorCode::StaleInput, ctx));
        }
        let accepted = state
            .accepted
            .as_ref()
            .ok_or_else(|| error(ErrorCode::MissingArtifact, ctx))?;
        let bundle =
            crate::manifest::admit_accepted(accepted, &state, self, self.clock.as_ref(), ctx)?;
        let mut receipts = Vec::new();
        let mut errors = Vec::new();
        let mut survivors = Vec::new();
        for service in bundle.candidate().services.as_slice() {
            let resource = self.authorized(&service.resource.resource_id, ctx)?;
            let attempt = self.observe_authorized(&resource, ctx).and_then(|receipt| {
                if receipt.status != ResourceStatus::Running
                    || !self.matches_admitted(&bundle, &receipt.resource, &receipt, ctx)?
                {
                    return Err(error(ErrorCode::ResourceDrift, ctx));
                }
                Ok(receipt)
            });
            let outcome = match attempt {
                Ok(receipt) => ActionOutcome::Completed { receipt },
                Err(error) => {
                    survivors.push(service.resource.resource_id.clone());
                    errors.push(error.clone());
                    ActionOutcome::Failed { error }
                }
            };
            receipts.push(ActionReceipt {
                action_id: ActionId::new(format!("observe-{}", service.service_id))
                    .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
                outcome,
            });
        }
        Ok(RuntimeResult {
            contract_version: ContractVersion::V1,
            operation_id: ctx.operation_id.clone(),
            status: if errors.is_empty() {
                ResultStatus::Succeeded
            } else {
                ResultStatus::Failed
            },
            run_intent: state.run_intent,
            run_epoch: state.run_epoch,
            state_revision: state.revision,
            accepted: state.accepted,
            receipts: Items::new(receipts).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
            survivors: Items::new(survivors).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
            errors: Items::new(errors).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
        })
    }
}

fn launch_from_bytes(
    bytes: &[u8],
    service: &ServiceId,
    ctx: &OperationContext,
) -> Result<Value, RuntimeError> {
    let root: Value =
        serde_json::from_slice(bytes).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
    root.get("services")
        .and_then(|value| value.get(service.as_str()))
        .cloned()
        .ok_or_else(|| error(ErrorCode::InvalidManifest, ctx))
}

struct ValidationSelection<'a> {
    selected: &'a std::sync::Mutex<Option<BundleId>>,
    previous: Option<BundleId>,
}
impl Drop for ValidationSelection<'_> {
    fn drop(&mut self) {
        if let Ok(mut selected) = self.selected.lock() {
            *selected = self.previous.take();
        }
    }
}
impl HostRuntime {
    fn select_validation(
        &self,
        bundle: Option<BundleId>,
        ctx: &OperationContext,
    ) -> Result<ValidationSelection<'_>, RuntimeError> {
        let mut selected = self
            .validation_bundle
            .lock()
            .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?;
        let previous = std::mem::replace(&mut *selected, bundle);
        Ok(ValidationSelection {
            selected: &self.validation_bundle,
            previous,
        })
    }
    fn stopped_resources(
        &self,
        ctx: &OperationContext,
    ) -> Result<Vec<ResourceReceipt>, RuntimeError> {
        let authority = self.shutdown(ctx)?;
        let mut receipts = Vec::new();
        for resource in authority.resources.as_slice() {
            let current = self.authorized(&resource.identity.resource_id, ctx)?;
            let receipt = self.observe_authorized(&current, ctx)?;
            if receipt.status != ResourceStatus::Stopped
                || receipt.restart_policy != RestartPolicy::Disabled
            {
                return Err(error(ErrorCode::Partial, ctx));
            }
            receipts.push(receipt);
        }
        Ok(receipts)
    }
    fn qualify_pointer(
        &self,
        bundle: &BundleId,
        floor: u64,
        current: &RuntimeState,
        ctx: &OperationContext,
    ) -> Result<(), RuntimeError> {
        let _selection = self.select_validation(Some(bundle.clone()), ctx)?;
        let mut prospective = current.clone();
        prospective.run_intent = RunIntent::Running;
        prospective.maintenance = None;
        prospective.accepted = Some(bundle.clone());
        prospective.fences.credential_revision = floor;
        let _admitted =
            crate::manifest::admit_accepted(bundle, &prospective, self, self.clock.as_ref(), ctx)?;
        Ok(())
    }
    fn metadata(&self, request: &ExecuteRequest) -> Result<RuntimeResult, RuntimeError> {
        let ctx = &request.context;
        let _effect = EffectGate::acquire(self, ctx)?;
        let mut current = self.store.read(&request.instance_id, ctx)?;
        if current.run_epoch != request.expected_run_epoch {
            return Err(error(ErrorCode::StaleInput, ctx));
        }
        let transition = match &request.action {
            ExecutorAction::ReserveMaintenance { reservation_id } => {
                if current.run_intent != RunIntent::Stopped {
                    return Err(error(ErrorCode::Stopped, ctx));
                }
                let receipts = self.stopped_resources(ctx)?;
                current = crate::state::write_transition(
                    &self.store,
                    &current,
                    StateTransition::RecordReceipts {
                        receipts: Items::new(receipts)
                            .map_err(|_| error(ErrorCode::InvalidManifest, ctx))?,
                    },
                    ctx,
                )?;
                StateTransition::ReserveMaintenance {
                    reservation_id: reservation_id.clone(),
                }
            }
            ExecutorAction::ReleaseMaintenance { reservation_id } => {
                self.stopped_resources(ctx)?;
                StateTransition::ReleaseMaintenance {
                    reservation_id: reservation_id.clone(),
                }
            }
            ExecutorAction::RestoreStopped {
                reservation_id,
                accepted,
                previous_compatible,
                credential_revision,
            } => {
                self.stopped_resources(ctx)?;
                let reservation = current
                    .maintenance
                    .as_ref()
                    .ok_or_else(|| error(ErrorCode::LeaseConflict, ctx))?;
                if reservation.reservation_id != *reservation_id
                    || reservation.operation_id != ctx.operation_id
                    || reservation.run_epoch != current.run_epoch
                {
                    return Err(error(ErrorCode::LeaseConflict, ctx));
                }
                if *credential_revision < current.fences.credential_revision {
                    return Err(error(ErrorCode::FenceConflict, ctx));
                }
                if let Some(bundle) = accepted {
                    self.qualify_pointer(bundle, *credential_revision, &current, ctx)?;
                }
                if let Some(bundle) = previous_compatible {
                    self.qualify_pointer(bundle, *credential_revision, &current, ctx)?;
                }
                StateTransition::RestoreStopped {
                    reservation_id: reservation_id.clone(),
                    accepted: accepted.clone(),
                    previous_compatible: previous_compatible.clone(),
                    credential_revision: *credential_revision,
                }
            }
            ExecutorAction::AdvanceFence {
                credential_revision,
            } => {
                if let Some(bundle) = &current.accepted {
                    self.qualify_pointer(bundle, *credential_revision, &current, ctx)?;
                } else {
                    self.stopped_resources(ctx)?;
                }
                if let Some(bundle) = &current.previous_compatible {
                    self.qualify_pointer(bundle, *credential_revision, &current, ctx)?;
                }
                StateTransition::AdvanceFence {
                    credential_revision: *credential_revision,
                }
            }
            _ => return Err(error(ErrorCode::InvalidManifest, ctx)),
        };
        let result = crate::state::write_transition(&self.store, &current, transition, ctx)?;
        Ok(RuntimeResult {
            contract_version: ContractVersion::V1,
            operation_id: ctx.operation_id.clone(),
            status: ResultStatus::Succeeded,
            run_intent: result.run_intent,
            run_epoch: result.run_epoch,
            state_revision: result.revision,
            accepted: result.accepted,
            receipts: Items::default(),
            survivors: Items::default(),
            errors: Items::default(),
        })
    }
}
