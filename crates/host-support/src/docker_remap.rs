//! Fixed no-mount snapshot-backed loopback Docker publication preparation.
use crate::{
    Budget, Cancellation, Error, Result,
    docker::{ContainerFacts, ContainerId, DockerObserver, container_structural_fingerprint},
    docker_snapshot::{ForeignSnapshot, ForeignSnapshotStore, reject_external_data},
    process::{Runner, Tool},
};
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Arc};

const LIMIT: usize = 4 * 1024 * 1024;
const ROOT: &str = "/opt/hton/work/foreign-remaps";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateIntent {
    schema_version: u32,
    source_id: String,
    source_fingerprint: String,
    snapshot_digest: String,
    candidate_name: String,
    port: u16,
    native_config_digest: String,
    originally_running: bool,
    payload: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Transition {
    intent_digest: String,
    phase: Phase,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Phase {
    DisableOriginal,
    StopOriginal,
    RenameOriginal,
    RenameCandidate,
    StartCandidate,
    Cutover,
    Accept,
    Committed,
    StopCandidate,
    ParkCandidate,
    RestoreOriginalName,
    RestoreOriginalPolicy,
    RestoreOriginalState,
    Restored,
}

/// Opaque identity, not caller-provided Docker metadata or authority.
pub struct PreparedLoopbackRemap {
    snapshot_id: String,
    source_id: ContainerId,
    candidate_id: ContainerId,
    port: u16,
}
impl PreparedLoopbackRemap {
    pub fn source_id(&self) -> &ContainerId {
        &self.source_id
    }
    pub fn candidate_id(&self) -> &ContainerId {
        &self.candidate_id
    }
    pub fn port(&self) -> u16 {
        self.port
    }
    pub fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }
}
impl std::fmt::Debug for PreparedLoopbackRemap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedLoopbackRemap")
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

pub struct LoopbackRemapStore {
    root: SecureDir,
    runner: Arc<Runner>,
}
struct TransitionContext {
    directory: SecureDir,
    intent_digest: String,
    intent: CreateIntent,
    original: Value,
    original_name: String,
    original_parked: String,
    candidate_parked: String,
    original_policy: Value,
}
impl LoopbackRemapStore {
    pub fn initialize() -> Result<Self> {
        if rustix::process::geteuid().as_raw() != 0 {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            root: SecureDir::create(Path::new(ROOT)).map_err(storage)?,
            runner: Arc::new(Runner::new()),
        })
    }

    pub fn resume(
        &self,
        snapshot: &ForeignSnapshot,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<PreparedLoopbackRemap> {
        let verified =
            ForeignSnapshotStore::open()?.verify(snapshot.snapshot_id(), budget, cancel)?;
        if verified.manifest_digest() != snapshot.manifest_digest() {
            return Err(Error::IdentityMismatch);
        }
        let dir = self.root.child(snapshot.snapshot_id()).map_err(storage)?;
        let bytes = dir
            .read_private("create-intent.json", LIMIT)
            .map_err(storage)?
            .ok_or(Error::InvalidResponse)?;
        let intent: CreateIntent =
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)?;
        let record: Value = serde_json::from_slice(
            &dir.read_private("candidate.json", 16_384)
                .map_err(storage)?
                .ok_or(Error::InvalidResponse)?,
        )
        .map_err(|_| Error::InvalidResponse)?;
        if intent.source_id != snapshot.source_container_id().as_str()
            || intent.snapshot_digest != snapshot.manifest_digest()
            || !(5000..=6000).contains(&intent.port)
            || record.get("intentDigest").and_then(Value::as_str) != Some(hash(&bytes).as_str())
        {
            return Err(Error::IdentityMismatch);
        }
        Ok(PreparedLoopbackRemap {
            snapshot_id: snapshot.snapshot_id().into(),
            source_id: ContainerId::new(intent.source_id)?,
            port: intent.port,
            candidate_id: ContainerId::new(
                record
                    .get("containerId")
                    .and_then(Value::as_str)
                    .ok_or(Error::InvalidResponse)?,
            )?,
        })
    }

    /// Resource-scoped consent and the enclosing operation journal are caller
    /// prerequisites. Every native effect below has its own private durable intent.
    pub fn cutover(
        &self,
        handle: &PreparedLoopbackRemap,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let _lock = self.root.lock(budget.deadline).map_err(storage)?;
        let context = self.load(handle, budget, cancel)?;
        let stages = [
            Phase::DisableOriginal,
            Phase::StopOriginal,
            Phase::RenameOriginal,
            Phase::RenameCandidate,
            Phase::StartCandidate,
        ];
        let phase = self.phase(&context.directory, &context.intent_digest)?;
        if matches!(
            phase,
            Some(Phase::Cutover | Phase::Accept | Phase::Committed)
        ) {
            self.check_phase(
                handle,
                &context,
                phase.expect("checked phase"),
                budget,
                cancel,
            )?;
            return Ok(());
        }
        let start = match phase {
            None => 0,
            Some(phase) => stages
                .iter()
                .position(|item| *item == phase)
                .ok_or(Error::Busy)?,
        };
        for phase in &stages[start..] {
            self.check_phase(handle, &context, *phase, budget, cancel)?;
            self.write_phase(&context, *phase, budget)?;
            match phase {
                Phase::DisableOriginal => self.command(
                    &[
                        "container",
                        "update",
                        "--restart=no",
                        handle.source_id.as_str(),
                    ],
                    budget,
                    cancel,
                )?,
                Phase::StopOriginal => self.stop(&handle.source_id, budget, cancel)?,
                Phase::RenameOriginal => {
                    self.rename(&handle.source_id, &context.original_parked, budget, cancel)?
                }
                Phase::RenameCandidate => {
                    self.rename(&handle.candidate_id, &context.original_name, budget, cancel)?
                }
                Phase::StartCandidate => {
                    if !DockerObserver::new(self.runner.clone())
                        .inspect_container(&handle.candidate_id, budget, cancel)?
                        .state
                        .running
                    {
                        self.command(
                            &["container", "start", handle.candidate_id.as_str()],
                            budget,
                            cancel,
                        )?;
                    }
                }
                _ => return Err(Error::InvalidInput),
            }
        }
        self.check_phase(handle, &context, Phase::Cutover, budget, cancel)?;
        let endpoint = std::net::SocketAddr::from(([127, 0, 0, 1], handle.port));
        std::net::TcpStream::connect_timeout(&endpoint, std::time::Duration::from_secs(2))
            .map_err(|_| Error::CommandFailed)?;
        budget.check(cancel)?;
        self.write_phase(&context, Phase::Cutover, budget)
    }

    /// Call only after the coordinator verifies exact native config/route/handshake.
    pub fn accept(
        &self,
        handle: &PreparedLoopbackRemap,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let _lock = self.root.lock(budget.deadline).map_err(storage)?;
        let context = self.load(handle, budget, cancel)?;
        let phase = self
            .phase(&context.directory, &context.intent_digest)?
            .ok_or(Error::InvalidInput)?;
        if !matches!(phase, Phase::Cutover | Phase::Accept | Phase::Committed) {
            return Err(Error::Busy);
        }
        self.check_phase(handle, &context, phase, budget, cancel)?;
        self.write_phase(&context, Phase::Accept, budget)?;
        self.restart_policy(
            &handle.candidate_id,
            &context.original_policy,
            budget,
            cancel,
        )?;
        self.check_phase(handle, &context, Phase::Committed, budget, cancel)?;
        self.write_phase(&context, Phase::Committed, budget)
    }

    /// Does not delete/recreate/overwrite the original writable layer or keys.
    pub fn restore(
        &self,
        handle: &PreparedLoopbackRemap,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let _lock = self.root.lock(budget.deadline).map_err(storage)?;
        let context = self.load(handle, budget, cancel)?;
        let stages = [
            Phase::StopCandidate,
            Phase::ParkCandidate,
            Phase::RestoreOriginalName,
            Phase::RestoreOriginalPolicy,
            Phase::RestoreOriginalState,
        ];
        let old = self.phase(&context.directory, &context.intent_digest)?;
        if old == Some(Phase::Restored) {
            return self.check_phase(handle, &context, Phase::Restored, budget, cancel);
        }
        let start = old
            .and_then(|phase| stages.iter().position(|item| *item == phase))
            .unwrap_or(0);
        for phase in &stages[start..] {
            self.check_phase(handle, &context, *phase, budget, cancel)?;
            self.write_phase(&context, *phase, budget)?;
            match phase {
                Phase::StopCandidate => {
                    self.command(
                        &[
                            "container",
                            "update",
                            "--restart=no",
                            handle.candidate_id.as_str(),
                        ],
                        budget,
                        cancel,
                    )?;
                    self.stop(&handle.candidate_id, budget, cancel)?;
                }
                Phase::ParkCandidate => self.rename(
                    &handle.candidate_id,
                    &context.candidate_parked,
                    budget,
                    cancel,
                )?,
                Phase::RestoreOriginalName => {
                    self.rename(&handle.source_id, &context.original_name, budget, cancel)?
                }
                Phase::RestoreOriginalPolicy => self.restart_policy(
                    &handle.source_id,
                    &context.original_policy,
                    budget,
                    cancel,
                )?,
                Phase::RestoreOriginalState => {
                    let running = DockerObserver::new(self.runner.clone())
                        .inspect_container(&handle.source_id, budget, cancel)?
                        .state
                        .running;
                    if context.intent.originally_running && !running {
                        self.command(
                            &["container", "start", handle.source_id.as_str()],
                            budget,
                            cancel,
                        )?;
                    } else if !context.intent.originally_running && running {
                        return Err(Error::IdentityMismatch);
                    }
                }
                _ => return Err(Error::InvalidInput),
            }
        }
        self.check_phase(handle, &context, Phase::Restored, budget, cancel)?;
        self.write_phase(&context, Phase::Restored, budget)
    }

    fn command(&self, args: &[&str], budget: Budget, cancel: &dyn Cancellation) -> Result<()> {
        let mut full = vec![
            "--host",
            "unix:///var/run/docker.sock",
            "--config",
            "/opt/hton/work/docker-client",
        ];
        full.extend_from_slice(args);
        self.runner
            .run(Tool::Docker, &full, budget, cancel)
            .map(|_| ())
    }
    fn stop(&self, id: &ContainerId, budget: Budget, cancel: &dyn Cancellation) -> Result<()> {
        if DockerObserver::new(self.runner.clone())
            .inspect_container(id, budget, cancel)?
            .state
            .running
        {
            self.command(
                &["container", "stop", "--time", "10", id.as_str()],
                budget,
                cancel,
            )?;
        }
        Ok(())
    }
    fn rename(
        &self,
        id: &ContainerId,
        name: &str,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let facts =
            DockerObserver::new(self.runner.clone()).inspect_container(id, budget, cancel)?;
        if facts.name != format!("/{name}") {
            if self.lookup(name, budget, cancel)?.is_some() {
                return Err(Error::Busy);
            }
            self.command(&["container", "rename", id.as_str(), name], budget, cancel)?;
        }
        Ok(())
    }
    fn restart_policy(
        &self,
        id: &ContainerId,
        policy: &Value,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let name = policy
            .get("Name")
            .and_then(Value::as_str)
            .ok_or(Error::InvalidResponse)?;
        let policy = match name {
            "no" | "always" | "unless-stopped" => name.into(),
            "on-failure" => format!(
                "on-failure:{}",
                policy
                    .get("MaximumRetryCount")
                    .and_then(Value::as_u64)
                    .ok_or(Error::InvalidResponse)?
            ),
            _ => return Err(Error::InvalidInput),
        };
        self.command(
            &["container", "update", "--restart", &policy, id.as_str()],
            budget,
            cancel,
        )
    }
    fn load(
        &self,
        handle: &PreparedLoopbackRemap,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<TransitionContext> {
        let snapshot = ForeignSnapshotStore::open()?.verify(&handle.snapshot_id, budget, cancel)?;
        let directory = self.root.child(&handle.snapshot_id).map_err(storage)?;
        let bytes = directory
            .read_private("create-intent.json", LIMIT)
            .map_err(storage)?
            .ok_or(Error::InvalidResponse)?;
        let intent: CreateIntent =
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)?;
        let recorded = self.resume(&snapshot, budget, cancel)?;
        if recorded.source_id != handle.source_id
            || recorded.candidate_id != handle.candidate_id
            || recorded.port != handle.port
        {
            return Err(Error::IdentityMismatch);
        }
        let original = SecureDir::open_private(Path::new("/opt/hton/work/foreign-snapshots"))
            .and_then(|root| root.child(&handle.snapshot_id))
            .and_then(|dir| dir.read_private("inspect.json", LIMIT))
            .map_err(storage)?
            .ok_or(Error::InvalidResponse)?;
        let original: Value =
            serde_json::from_slice(&original).map_err(|_| Error::InvalidResponse)?;
        let name = original
            .get("Name")
            .and_then(Value::as_str)
            .and_then(|name| name.strip_prefix('/'))
            .ok_or(Error::InvalidResponse)?;
        if name.is_empty()
            || name.len() > 128
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        {
            return Err(Error::InvalidInput);
        }
        let policy = original
            .pointer("/HostConfig/RestartPolicy")
            .cloned()
            .ok_or(Error::InvalidResponse)?;
        Ok(TransitionContext {
            directory,
            intent_digest: hash(&bytes),
            original_name: name.into(),
            original_parked: format!("hton-original-{}", &handle.snapshot_id[..32]),
            candidate_parked: format!("hton-replacement-{}", &handle.snapshot_id[..32]),
            original_policy: policy,
            original,
            intent,
        })
    }
    fn phase(&self, dir: &SecureDir, digest: &str) -> Result<Option<Phase>> {
        dir.read_private("transition.json", 16_384)
            .map_err(storage)?
            .map(|bytes| {
                let transition: Transition =
                    serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)?;
                if transition.intent_digest != digest {
                    return Err(Error::IdentityMismatch);
                }
                Ok(transition.phase)
            })
            .transpose()
    }
    fn write_phase(&self, context: &TransitionContext, phase: Phase, budget: Budget) -> Result<()> {
        let locked = context.directory.lock(budget.deadline).map_err(storage)?;
        let before = locked.read("transition.json", 16_384).map_err(storage)?;
        let after = serde_json::to_vec(&Transition {
            intent_digest: context.intent_digest.clone(),
            phase,
        })
        .map_err(|_| Error::InvalidResponse)?;
        locked
            .compare_exchange("transition.json", before.as_deref(), &after)
            .map_err(storage)
    }
    fn check_native(
        &self,
        id: &ContainerId,
        digest: &str,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let observer = DockerObserver::new(self.runner.clone());
        let facts = observer.inspect_container(id, budget, cancel)?;
        let config = observer.read_amnezia_xray_server_config(
            id,
            &container_structural_fingerprint(&facts),
            budget,
            cancel,
        )?;
        if config.content_digest() != digest {
            return Err(Error::IdentityMismatch);
        }
        Ok(())
    }
    fn check_phase(
        &self,
        handle: &PreparedLoopbackRemap,
        context: &TransitionContext,
        phase: Phase,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let source: Value =
            serde_json::from_slice(&self.inspect_raw(&handle.source_id, budget, cancel)?)
                .map_err(|_| Error::InvalidResponse)?;
        let candidate: Value =
            serde_json::from_slice(&self.inspect_raw(&handle.candidate_id, budget, cancel)?)
                .map_err(|_| Error::InvalidResponse)?;
        let disabled = json!({"Name":"no","MaximumRetryCount":0});
        let original_name = format!("/{}", context.original_name);
        let original_parked = format!("/{}", context.original_parked);
        let candidate_name = format!("/{}", context.intent.candidate_name);
        let candidate_parked = format!("/{}", context.candidate_parked);
        let sname = source
            .get("Name")
            .and_then(Value::as_str)
            .ok_or(Error::InvalidResponse)?;
        let cname = candidate
            .get("Name")
            .and_then(Value::as_str)
            .ok_or(Error::InvalidResponse)?;
        let spolicy = source
            .pointer("/HostConfig/RestartPolicy")
            .ok_or(Error::InvalidResponse)?;
        let cpolicy = candidate
            .pointer("/HostConfig/RestartPolicy")
            .ok_or(Error::InvalidResponse)?;
        let srunning = source
            .pointer("/State/Running")
            .and_then(Value::as_bool)
            .ok_or(Error::InvalidResponse)?;
        let crunning = candidate
            .pointer("/State/Running")
            .and_then(Value::as_bool)
            .ok_or(Error::InvalidResponse)?;
        for value in [&source, &candidate] {
            if value.pointer("/State/Paused").and_then(Value::as_bool) != Some(false)
                || value.pointer("/State/Restarting").and_then(Value::as_bool) != Some(false)
                || value.pointer("/State/Dead").and_then(Value::as_bool) != Some(false)
            {
                return Err(Error::Busy);
            }
        }
        // Complete captured Config and HostConfig stay exact except the one
        // explicitly admitted restart-policy field and declared replacement image.
        if source.get("Config") != context.original.get("Config")
            || source.get("Image") != context.original.get("Image")
        {
            return Err(Error::IdentityMismatch);
        }
        let mut shost = source
            .get("HostConfig")
            .cloned()
            .ok_or(Error::InvalidResponse)?;
        shost["RestartPolicy"] = context.original_policy.clone();
        if Some(&shost) != context.original.get("HostConfig") {
            return Err(Error::IdentityMismatch);
        }
        let mut chost = candidate
            .get("HostConfig")
            .cloned()
            .ok_or(Error::InvalidResponse)?;
        chost["RestartPolicy"] = disabled.clone();
        if Some(&chost) != context.intent.payload.get("HostConfig")
            || candidate.get("Image") != context.intent.payload.get("Image")
        {
            return Err(Error::IdentityMismatch);
        }
        let config = candidate
            .get("Config")
            .and_then(Value::as_object)
            .ok_or(Error::InvalidResponse)?;
        for (key, value) in context
            .intent
            .payload
            .as_object()
            .ok_or(Error::InvalidResponse)?
        {
            if key != "HostConfig" && config.get(key) != Some(value) {
                return Err(Error::IdentityMismatch);
            }
        }
        let valid = match phase {
            Phase::DisableOriginal => {
                sname == original_name
                    && (spolicy == &context.original_policy || spolicy == &disabled)
                    && srunning == context.intent.originally_running
                    && cname == candidate_name
                    && !crunning
                    && cpolicy == &disabled
            }
            Phase::StopOriginal => {
                sname == original_name
                    && spolicy == &disabled
                    && cname == candidate_name
                    && !crunning
                    && cpolicy == &disabled
            }
            Phase::RenameOriginal => {
                [original_name.as_str(), original_parked.as_str()].contains(&sname)
                    && !srunning
                    && spolicy == &disabled
                    && cname == candidate_name
                    && !crunning
                    && cpolicy == &disabled
            }
            Phase::RenameCandidate => {
                sname == original_parked
                    && !srunning
                    && spolicy == &disabled
                    && [candidate_name.as_str(), original_name.as_str()].contains(&cname)
                    && !crunning
                    && cpolicy == &disabled
            }
            Phase::StartCandidate => {
                sname == original_parked
                    && !srunning
                    && spolicy == &disabled
                    && cname == original_name
                    && cpolicy == &disabled
            }
            Phase::Cutover => {
                sname == original_parked
                    && !srunning
                    && spolicy == &disabled
                    && cname == original_name
                    && crunning
                    && cpolicy == &disabled
            }
            Phase::Accept => {
                sname == original_parked
                    && !srunning
                    && spolicy == &disabled
                    && cname == original_name
                    && crunning
                    && (cpolicy == &disabled || cpolicy == &context.original_policy)
            }
            Phase::Committed => {
                sname == original_parked
                    && !srunning
                    && spolicy == &disabled
                    && cname == original_name
                    && crunning
                    && cpolicy == &context.original_policy
            }
            Phase::StopCandidate => {
                [original_name.as_str(), original_parked.as_str()].contains(&sname)
                    && (spolicy == &disabled || spolicy == &context.original_policy)
                    && [
                        candidate_name.as_str(),
                        candidate_parked.as_str(),
                        original_name.as_str(),
                    ]
                    .contains(&cname)
                    && (cpolicy == &disabled || cpolicy == &context.original_policy)
            }
            Phase::ParkCandidate => {
                !crunning
                    && cpolicy == &disabled
                    && [
                        candidate_name.as_str(),
                        candidate_parked.as_str(),
                        original_name.as_str(),
                    ]
                    .contains(&cname)
                    && [original_name.as_str(), original_parked.as_str()].contains(&sname)
                    && (spolicy == &disabled || spolicy == &context.original_policy)
            }
            Phase::RestoreOriginalName => {
                !crunning
                    && cpolicy == &disabled
                    && cname == candidate_parked
                    && [original_name.as_str(), original_parked.as_str()].contains(&sname)
                    && (spolicy == &disabled || spolicy == &context.original_policy)
            }
            Phase::RestoreOriginalPolicy => {
                !crunning
                    && cpolicy == &disabled
                    && cname == candidate_parked
                    && sname == original_name
                    && (spolicy == &disabled || spolicy == &context.original_policy)
            }
            Phase::RestoreOriginalState => {
                !crunning
                    && cpolicy == &disabled
                    && cname == candidate_parked
                    && sname == original_name
                    && spolicy == &context.original_policy
            }
            Phase::Restored => {
                !crunning
                    && cpolicy == &disabled
                    && cname == candidate_parked
                    && sname == original_name
                    && spolicy == &context.original_policy
                    && srunning == context.intent.originally_running
            }
        };
        if !valid {
            return Err(Error::IdentityMismatch);
        }
        self.check_native(
            &handle.source_id,
            &context.intent.native_config_digest,
            budget,
            cancel,
        )?;
        self.check_native(
            &handle.candidate_id,
            &context.intent.native_config_digest,
            budget,
            cancel,
        )
    }

    pub fn prepare(
        &self,
        snapshot: &ForeignSnapshot,
        port: u16,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<PreparedLoopbackRemap> {
        budget.check(cancel)?;
        if !(5000..=6000).contains(&port) || rustix::process::geteuid().as_raw() != 0 {
            return Err(Error::InvalidInput);
        }
        let verified =
            ForeignSnapshotStore::open()?.verify(snapshot.snapshot_id(), budget, cancel)?;
        if verified.manifest_digest() != snapshot.manifest_digest()
            || verified.source_container_id() != snapshot.source_container_id()
        {
            return Err(Error::IdentityMismatch);
        }
        let _store_lock = self.root.lock(budget.deadline).map_err(storage)?;
        let directory = self
            .root
            .create_child(snapshot.snapshot_id())
            .map_err(storage)?;
        let saved = SecureDir::open_private(Path::new("/opt/hton/work/foreign-snapshots"))
            .and_then(|root| root.child(snapshot.snapshot_id()))
            .map_err(storage)?;
        let bytes = saved
            .read_private("inspect.json", LIMIT)
            .map_err(storage)?
            .ok_or(Error::InvalidResponse)?;
        let source: ContainerFacts =
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)?;
        if source.id != *snapshot.source_container_id()
            || container_structural_fingerprint(&source) != snapshot.source_fingerprint()
        {
            return Err(Error::IdentityMismatch);
        }
        let observer = DockerObserver::new(self.runner.clone());
        let current = observer.inspect_container(&source.id, budget, cancel)?;
        if container_structural_fingerprint(&current) != snapshot.source_fingerprint()
            || current.state.paused
            || current.state.restarting
            || current.state.dead
        {
            return Err(Error::IdentityMismatch);
        }
        let payload = create_payload(&bytes, &source, snapshot, port)?;
        let native = observer.read_amnezia_xray_server_config(
            &source.id,
            snapshot.source_fingerprint(),
            budget,
            cancel,
        )?;
        let name = format!("hton-remap-{}", &snapshot.snapshot_id()[..32]);
        let intent = CreateIntent {
            schema_version: 1,
            source_id: source.id.as_str().into(),
            source_fingerprint: snapshot.source_fingerprint().into(),
            snapshot_digest: snapshot.manifest_digest().into(),
            candidate_name: name.clone(),
            port,
            native_config_digest: native.content_digest().into(),
            originally_running: current.state.running,
            payload,
        };
        let encoded = serde_json::to_vec(&intent).map_err(|_| Error::InvalidResponse)?;
        let previous = directory
            .read_private("create-intent.json", LIMIT)
            .map_err(storage)?;
        if previous.as_ref().is_some_and(|old| old != &encoded) {
            return Err(Error::IdentityMismatch);
        }
        if previous.is_none() {
            // A collision before our durable intent is never adopted.
            if self.lookup(&name, budget, cancel)?.is_some() {
                return Err(Error::Busy);
            }
            let locked = directory.lock(budget.deadline).map_err(storage)?;
            locked
                .compare_exchange("create-intent.json", None, &encoded)
                .map_err(storage)?;
        }
        let recorded = directory
            .read_private("candidate.json", 16_384)
            .map_err(storage)?;
        let candidate = if let Some(bytes) = recorded.as_ref() {
            let record: Value =
                serde_json::from_slice(bytes).map_err(|_| Error::InvalidResponse)?;
            if record.get("intentDigest").and_then(Value::as_str) != Some(hash(&encoded).as_str()) {
                return Err(Error::IdentityMismatch);
            }
            ContainerId::new(
                record
                    .get("containerId")
                    .and_then(Value::as_str)
                    .ok_or(Error::InvalidResponse)?,
            )?
        } else if let Some(id) = self.lookup(&name, budget, cancel)? {
            id
        } else {
            let version = self.api("/version", None, budget, cancel)?;
            let version = version
                .get("ApiVersion")
                .and_then(Value::as_str)
                .ok_or(Error::InvalidResponse)?;
            let minor = version
                .strip_prefix("1.")
                .and_then(|value| value.parse::<u16>().ok())
                .ok_or(Error::InvalidResponse)?;
            if !(41..=99).contains(&minor) {
                return Err(Error::InvalidResponse);
            }
            let path = format!("/v1.{minor}/containers/create?name={name}");
            let response = self.api(&path, Some(&intent.payload), budget, cancel)?;
            ContainerId::new(
                response
                    .get("Id")
                    .and_then(Value::as_str)
                    .ok_or(Error::InvalidResponse)?,
            )?
        };
        let raw = self.inspect_raw(&candidate, budget, cancel)?;
        verify_candidate(&raw, &intent, snapshot.image_digest())?;
        self.check_native(&candidate, &intent.native_config_digest, budget, cancel)?;
        if recorded.is_none() {
            let record =
                serde_json::to_vec(&json!({"schemaVersion":1,"containerId":candidate.as_str(),
                "intentDigest":hash(&encoded)}))
                .map_err(|_| Error::InvalidResponse)?;
            directory
                .lock(budget.deadline)
                .map_err(storage)?
                .compare_exchange("candidate.json", None, &record)
                .map_err(storage)?;
        }
        budget.check(cancel)?;
        Ok(PreparedLoopbackRemap {
            snapshot_id: snapshot.snapshot_id().into(),
            source_id: source.id,
            candidate_id: candidate,
            port,
        })
    }

    fn api(
        &self,
        path: &str,
        payload: Option<&Value>,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Value> {
        if !path.starts_with('/') || path.contains(['\r', '\n', '#']) {
            return Err(Error::InvalidInput);
        }
        let url = format!("http://localhost{path}");
        let mut args = vec![
            "--disable",
            "--noproxy",
            "*",
            "--silent",
            "--show-error",
            "--connect-timeout",
            "2",
            "--max-time",
            "30",
            "--unix-socket",
            "/var/run/docker.sock",
            "--write-out",
            "\n%{http_code}",
        ];
        let input = payload
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|_| Error::InvalidInput)?
            .unwrap_or_default();
        if payload.is_some() {
            args.extend([
                "--request",
                "POST",
                "--header",
                "Content-Type: application/json",
                "--data-binary",
                "@-",
            ]);
        }
        args.push(&url);
        let output = self
            .runner
            .run_input(Tool::Curl, &args, &input, budget, cancel)?;
        let split = output
            .iter()
            .rposition(|byte| *byte == b'\n')
            .ok_or(Error::InvalidResponse)?;
        let status =
            std::str::from_utf8(&output[split + 1..]).map_err(|_| Error::InvalidResponse)?;
        if !matches!(status, "200" | "201") {
            return Err(Error::CommandFailed);
        }
        serde_json::from_slice(&output[..split]).map_err(|_| Error::InvalidResponse)
    }
    fn inspect_raw(
        &self,
        id: &ContainerId,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Vec<u8>> {
        self.runner.run(
            Tool::Docker,
            &[
                "--host",
                "unix:///var/run/docker.sock",
                "--config",
                "/opt/hton/work/docker-client",
                "container",
                "inspect",
                "--format",
                "{{json .}}",
                id.as_str(),
            ],
            budget,
            cancel,
        )
    }
    fn lookup(
        &self,
        name: &str,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Option<ContainerId>> {
        let ids = self.runner.run(
            Tool::Docker,
            &[
                "--host",
                "unix:///var/run/docker.sock",
                "--config",
                "/opt/hton/work/docker-client",
                "container",
                "ls",
                "--all",
                "--quiet",
                "--no-trunc",
            ],
            budget,
            cancel,
        )?;
        let ids = std::str::from_utf8(&ids)
            .map_err(|_| Error::InvalidResponse)?
            .lines()
            .collect::<Vec<_>>();
        if ids.len() > 256 {
            return Err(Error::OutputLimit);
        }
        let observer = DockerObserver::new(self.runner.clone());
        let mut matched = None;
        for id in ids {
            let id = ContainerId::new(id)?;
            let facts = observer.inspect_container(&id, budget, cancel)?;
            if facts.name == format!("/{name}") {
                if matched.replace(id).is_some() {
                    return Err(Error::IdentityMismatch);
                }
            }
        }
        Ok(matched)
    }
}

fn create_payload(
    bytes: &[u8],
    source: &ContainerFacts,
    snapshot: &ForeignSnapshot,
    port: u16,
) -> Result<Value> {
    reject_external_data(bytes, source)?;
    let raw: Value = serde_json::from_slice(bytes).map_err(|_| Error::InvalidResponse)?;
    if source.host_config.network_mode != "bridge"
        || source.network_settings.networks.len() != 1
        || !source.network_settings.networks.contains_key("bridge")
        || raw
            .pointer("/HostConfig/AutoRemove")
            .and_then(Value::as_bool)
            != Some(false)
        || raw
            .pointer("/NetworkSettings/Networks/bridge/IPAMConfig")
            .is_some_and(|value| !value.is_null())
        || raw.pointer("/HostConfig/Links").is_some_and(|value| {
            !value.is_null() && value.as_array().is_none_or(|values| !values.is_empty())
        })
    {
        return Err(Error::InvalidInput);
    }
    let original = raw
        .pointer("/HostConfig/PortBindings")
        .and_then(Value::as_object)
        .ok_or(Error::InvalidResponse)?;
    if original.len() != 2 {
        return Err(Error::InvalidInput);
    }
    for key in ["443/tcp", "443/udp"] {
        let bindings = original
            .get(key)
            .and_then(Value::as_array)
            .ok_or(Error::InvalidInput)?;
        if bindings.len() != 1
            || bindings[0].get("HostPort").and_then(Value::as_str) != Some("443")
            || !matches!(
                bindings[0].get("HostIp").and_then(Value::as_str),
                Some("" | "0.0.0.0")
            )
        {
            return Err(Error::InvalidInput);
        }
    }
    let mut payload = raw
        .get("Config")
        .and_then(Value::as_object)
        .cloned()
        .ok_or(Error::InvalidResponse)?;
    payload.insert("Image".into(), json!(snapshot.image_digest()));
    let mut host = raw
        .get("HostConfig")
        .and_then(Value::as_object)
        .cloned()
        .ok_or(Error::InvalidResponse)?;
    host.insert(
        "PortBindings".into(),
        json!({"443/tcp":[{"HostIp":"127.0.0.1","HostPort":port.to_string()}],
        "443/udp":[{"HostIp":"127.0.0.1","HostPort":port.to_string()}]}),
    );
    host.insert(
        "RestartPolicy".into(),
        json!({"Name":"no","MaximumRetryCount":0}),
    );
    payload.insert("HostConfig".into(), Value::Object(host));
    Ok(Value::Object(payload))
}
fn verify_candidate(bytes: &[u8], intent: &CreateIntent, image: &str) -> Result<()> {
    let raw: Value = serde_json::from_slice(bytes).map_err(|_| Error::InvalidResponse)?;
    if raw.get("Name").and_then(Value::as_str)
        != Some(format!("/{}", intent.candidate_name).as_str())
        || raw.get("Image").and_then(Value::as_str) != Some(image)
        || raw.pointer("/State/Running").and_then(Value::as_bool) != Some(false)
        || raw.get("HostConfig") != intent.payload.get("HostConfig")
    {
        return Err(Error::IdentityMismatch);
    }
    let expected = intent.payload.as_object().ok_or(Error::InvalidInput)?;
    let config = raw
        .get("Config")
        .and_then(Value::as_object)
        .ok_or(Error::InvalidResponse)?;
    for (key, value) in expected {
        if key != "HostConfig" && config.get(key) != Some(value) {
            return Err(Error::IdentityMismatch);
        }
    }
    Ok(())
}
fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn storage(error: hton_storage::Error) -> Error {
    match error {
        hton_storage::Error::Conflict => Error::Busy,
        hton_storage::Error::TooLarge => Error::OutputLimit,
        hton_storage::Error::Deadline => Error::Deadline,
        _ => Error::Io,
    }
}
