//! Protected writable-layer custody for an explicitly consented foreign snapshot.
//! This is not a recreation, remap, data-owner qualification or deletion API.
use crate::{
    Budget, Cancellation, Error, Result,
    docker::{ContainerFacts, ContainerId, DockerObserver, container_structural_fingerprint},
    process::{Runner, Tool},
};
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Arc};

const ROOT: &str = "/opt/hton/work/foreign-snapshots";
const MAX_INSPECT: usize = 4 * 1024 * 1024;
const MAX_IMAGE: u64 = 2 * 1024 * 1024 * 1024;
const PREFIX: &[&str] = &[
    "--host",
    "unix:///var/run/docker.sock",
    "--config",
    "/opt/hton/work/docker-client",
];

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreparedSnapshot {
    schema_version: u32,
    snapshot_id: String,
    source_container_id: String,
    source_fingerprint: String,
    source_inspect_digest: String,
    source_image_digest: String,
    originally_running: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompletedSnapshot {
    prepared: PreparedSnapshot,
    image_digest: String,
    archive_digest: String,
    archive_bytes: u64,
}
/// Nonsensitive handle to private rootfs/launch custody. Not caller-deserializable.
#[derive(Clone)]
pub struct ForeignSnapshot {
    snapshot_id: String,
    source_container_id: ContainerId,
    source_fingerprint: String,
    manifest_digest: String,
    image_digest: String,
    archive_digest: String,
    archive_bytes: u64,
}
impl ForeignSnapshot {
    pub fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }
    pub fn source_container_id(&self) -> &ContainerId {
        &self.source_container_id
    }
    pub fn source_fingerprint(&self) -> &str {
        &self.source_fingerprint
    }
    pub fn manifest_digest(&self) -> &str {
        &self.manifest_digest
    }
    pub fn image_digest(&self) -> &str {
        &self.image_digest
    }
    pub fn archive_digest(&self) -> &str {
        &self.archive_digest
    }
    pub fn archive_bytes(&self) -> u64 {
        self.archive_bytes
    }
}
impl std::fmt::Debug for ForeignSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ForeignSnapshot([protected recovery custody])")
    }
}
pub struct ForeignSnapshotStore {
    root: SecureDir,
    runner: Arc<Runner>,
}
impl ForeignSnapshotStore {
    pub fn open() -> Result<Self> {
        require_root()?;
        Ok(Self {
            root: SecureDir::open_private(Path::new(ROOT)).map_err(storage_error)?,
            runner: Arc::new(Runner::new()),
        })
    }
    /// Explicit store initialization, never part of read-only qualification.
    pub fn initialize() -> Result<Self> {
        require_root()?;
        Ok(Self {
            root: SecureDir::create(Path::new(ROOT)).map_err(storage_error)?,
            runner: Arc::new(Runner::new()),
        })
    }
    /// Requires the caller's exact durable intent and resource-scoped consent.
    /// Docker commit briefly pauses a running source; it never removes it.
    pub fn capture(
        &self,
        source: &ContainerId,
        expected_fingerprint: &str,
        snapshot_id: &str,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<ForeignSnapshot> {
        require_root()?;
        validate_id(snapshot_id)?;
        if !valid_digest(expected_fingerprint) {
            return Err(Error::InvalidInput);
        }
        budget.check(cancellation)?;
        let _lock = self.root.lock(budget.deadline).map_err(storage_error)?;
        let dir = self.root.create_child(snapshot_id).map_err(storage_error)?;
        if dir
            .read_private("complete.json", MAX_INSPECT)
            .map_err(storage_error)?
            .is_some()
        {
            let verified = verify_directory(&dir, snapshot_id, budget, cancellation)?;
            if verified.source_container_id() != source
                || verified.source_fingerprint() != expected_fingerprint
            {
                return Err(Error::IdentityMismatch);
            }
            return Ok(verified);
        }
        if !dir.list(8).map_err(storage_error)?.is_empty() {
            // Cancellation of the CLI does not establish daemon commit outcome.
            return Err(Error::Busy);
        }
        let observer = DockerObserver::new(self.runner.clone());
        let before = observer.inspect_container(source, budget, cancellation)?;
        if container_structural_fingerprint(&before) != expected_fingerprint
            || before.state.paused
            || before.state.restarting
            || before.state.dead
        {
            return Err(Error::IdentityMismatch);
        }
        let raw = self.inspect_document(source, budget, cancellation)?;
        let exact: ContainerFacts =
            serde_json::from_slice(&raw).map_err(|_| Error::InvalidResponse)?;
        if exact.id != *source || container_structural_fingerprint(&exact) != expected_fingerprint {
            return Err(Error::Busy);
        }
        reject_external_data(&raw, &exact)?;
        let prepared = PreparedSnapshot {
            schema_version: 1,
            snapshot_id: snapshot_id.to_owned(),
            source_container_id: source.as_str().to_owned(),
            source_fingerprint: expected_fingerprint.to_owned(),
            source_inspect_digest: hash(&raw),
            source_image_digest: exact.image.clone(),
            originally_running: exact.state.running,
        };
        {
            let locked = dir.lock(budget.deadline).map_err(storage_error)?;
            locked
                .compare_exchange("inspect.json", None, &raw)
                .map_err(storage_error)?;
            let bytes = serde_json::to_vec(&prepared).map_err(|_| Error::InvalidResponse)?;
            locked
                .compare_exchange("prepared.json", None, &bytes)
                .map_err(storage_error)?;
        }
        budget.check(cancellation)?;
        let tag = format!("hton-foreign-snapshot:{snapshot_id}");
        let filter = format!("reference={tag}");
        let mut lookup = PREFIX.to_vec();
        lookup.extend([
            "image",
            "ls",
            "--quiet",
            "--no-trunc",
            "--filter",
            filter.as_str(),
        ]);
        let existing = self
            .runner
            .run(Tool::Docker, &lookup, budget, cancellation)?;
        if existing.iter().any(|byte| !byte.is_ascii_whitespace()) {
            return Err(Error::Busy);
        }
        let mut argv = PREFIX.to_vec();
        argv.extend([
            "container",
            "commit",
            "--pause=true",
            source.as_str(),
            tag.as_str(),
        ]);
        let output = self.runner.run(Tool::Docker, &argv, budget, cancellation)?;
        let image_digest = std::str::from_utf8(&output)
            .map_err(|_| Error::InvalidResponse)?
            .trim()
            .to_owned();
        if !valid_digest(&image_digest) {
            return Err(Error::InvalidResponse);
        }
        let mut image_argv = PREFIX.to_vec();
        image_argv.extend([
            "image",
            "inspect",
            "--format",
            "{{json .}}",
            image_digest.as_str(),
        ]);
        let image = self
            .runner
            .run(Tool::Docker, &image_argv, budget, cancellation)?;
        let image_value: serde_json::Value =
            serde_json::from_slice(&image).map_err(|_| Error::InvalidResponse)?;
        if image_value.get("Id").and_then(serde_json::Value::as_str) != Some(image_digest.as_str())
        {
            return Err(Error::IdentityMismatch);
        }
        {
            let locked = dir.lock(budget.deadline).map_err(storage_error)?;
            locked
                .compare_exchange("image-inspect.json", None, &image)
                .map_err(storage_error)?;
        }
        let mut writer = dir
            .begin_stream("image.tar", MAX_IMAGE)
            .map_err(storage_error)?;
        let mut archive_hash = Sha256::new();
        let mut save = PREFIX.to_vec();
        save.extend(["image", "save", image_digest.as_str()]);
        let archive_bytes = self.runner.run_stream(
            Tool::Docker,
            &save,
            budget,
            cancellation,
            MAX_IMAGE,
            &mut |bytes| {
                budget.check(cancellation)?;
                writer.write_chunk(bytes).map_err(storage_error)?;
                archive_hash.update(bytes);
                Ok(())
            },
        )?;
        if archive_bytes < 1024 {
            return Err(Error::InvalidResponse);
        }
        let after = observer.inspect_container(source, budget, cancellation)?;
        if after.id != *source
            || container_structural_fingerprint(&after) != expected_fingerprint
            || after.state.running != before.state.running
            || after.state.paused
            || after.state.restarting
            || after.state.dead
            || after.state.pid != before.state.pid
            || after.state.started_at != before.state.started_at
        {
            return Err(Error::Busy);
        }
        if writer.finish_new("image.tar").map_err(storage_error)? != archive_bytes {
            return Err(Error::InvalidResponse);
        }
        let completed = CompletedSnapshot {
            prepared,
            image_digest,
            archive_digest: format!("sha256:{:x}", archive_hash.finalize()),
            archive_bytes,
        };
        let bytes = serde_json::to_vec(&completed).map_err(|_| Error::InvalidResponse)?;
        let locked = dir.lock(budget.deadline).map_err(storage_error)?;
        locked
            .compare_exchange("complete.json", None, &bytes)
            .map_err(storage_error)?;
        drop(locked);
        verify_directory(&dir, snapshot_id, budget, cancellation)
    }
    pub fn verify(
        &self,
        snapshot_id: &str,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<ForeignSnapshot> {
        validate_id(snapshot_id)?;
        budget.check(cancellation)?;
        let dir = self.root.child(snapshot_id).map_err(storage_error)?;
        verify_directory(&dir, snapshot_id, budget, cancellation)
    }
    fn inspect_document(
        &self,
        source: &ContainerId,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<u8>> {
        let mut argv = PREFIX.to_vec();
        argv.extend([
            "container",
            "inspect",
            "--format",
            "{{json .}}",
            source.as_str(),
        ]);
        self.runner.run(Tool::Docker, &argv, budget, cancellation)
    }
}
fn verify_directory(
    dir: &SecureDir,
    snapshot_id: &str,
    budget: Budget,
    cancellation: &dyn Cancellation,
) -> Result<ForeignSnapshot> {
    budget.check(cancellation)?;
    let manifest = dir
        .read_private("complete.json", MAX_INSPECT)
        .map_err(storage_error)?
        .ok_or(Error::Busy)?;
    let completed: CompletedSnapshot =
        serde_json::from_slice(&manifest).map_err(|_| Error::InvalidResponse)?;
    let prepared = &completed.prepared;
    let source = ContainerId::new(prepared.source_container_id.clone())?;
    if prepared.schema_version != 1
        || prepared.snapshot_id != snapshot_id
        || !valid_digest(&prepared.source_fingerprint)
        || !valid_digest(&prepared.source_image_digest)
        || !valid_digest(&prepared.source_inspect_digest)
        || !valid_digest(&completed.image_digest)
        || !valid_digest(&completed.archive_digest)
        || completed.archive_bytes < 1024
        || completed.archive_bytes > MAX_IMAGE
    {
        return Err(Error::InvalidResponse);
    }
    let inspect = dir
        .read_private("inspect.json", MAX_INSPECT)
        .map_err(storage_error)?
        .ok_or(Error::InvalidResponse)?;
    let original: ContainerFacts =
        serde_json::from_slice(&inspect).map_err(|_| Error::InvalidResponse)?;
    if hash(&inspect) != prepared.source_inspect_digest
        || original.id != source
        || container_structural_fingerprint(&original) != prepared.source_fingerprint
        || original.image != prepared.source_image_digest
        || original.state.running != prepared.originally_running
    {
        return Err(Error::IdentityMismatch);
    }
    reject_external_data(&inspect, &original)?;
    let image = dir
        .read_private("image-inspect.json", MAX_INSPECT)
        .map_err(storage_error)?
        .ok_or(Error::InvalidResponse)?;
    let image: serde_json::Value =
        serde_json::from_slice(&image).map_err(|_| Error::InvalidResponse)?;
    if image.get("Id").and_then(serde_json::Value::as_str) != Some(completed.image_digest.as_str())
    {
        return Err(Error::IdentityMismatch);
    }
    let stored_prepared = dir
        .read_private("prepared.json", MAX_INSPECT)
        .map_err(storage_error)?
        .ok_or(Error::InvalidResponse)?;
    if stored_prepared != serde_json::to_vec(prepared).map_err(|_| Error::InvalidResponse)? {
        return Err(Error::IdentityMismatch);
    }
    if dir
        .file_size_private("image.tar", MAX_IMAGE)
        .map_err(storage_error)?
        != Some(completed.archive_bytes)
    {
        return Err(Error::IdentityMismatch);
    }
    let mut digest = Sha256::new();
    let mut offset = 0u64;
    while offset < completed.archive_bytes {
        budget.check(cancellation)?;
        let bytes = dir
            .read_private_range(
                "image.tar",
                offset,
                (completed.archive_bytes - offset).min(65536) as usize,
                MAX_IMAGE,
            )
            .map_err(storage_error)?
            .ok_or(Error::InvalidResponse)?;
        if bytes.is_empty() {
            return Err(Error::InvalidResponse);
        }
        offset += bytes.len() as u64;
        digest.update(&bytes);
    }
    if format!("sha256:{:x}", digest.finalize()) != completed.archive_digest
        || dir
            .file_size_private("image.tar", MAX_IMAGE)
            .map_err(storage_error)?
            != Some(offset)
        || dir
            .read_private("complete.json", MAX_INSPECT)
            .map_err(storage_error)?
            .as_ref()
            != Some(&manifest)
    {
        return Err(Error::IdentityMismatch);
    }
    budget.check(cancellation)?;
    Ok(ForeignSnapshot {
        snapshot_id: snapshot_id.to_owned(),
        source_container_id: source,
        source_fingerprint: prepared.source_fingerprint.clone(),
        manifest_digest: hash(&manifest),
        image_digest: completed.image_digest,
        archive_digest: completed.archive_digest,
        archive_bytes: offset,
    })
}
pub(crate) fn reject_external_data(raw: &[u8], facts: &ContainerFacts) -> Result<()> {
    let document: serde_json::Value =
        serde_json::from_slice(raw).map_err(|_| Error::InvalidResponse)?;
    let empty = |value: Option<&serde_json::Value>| match value {
        None | Some(serde_json::Value::Null) => true,
        Some(serde_json::Value::Object(value)) => value.is_empty(),
        Some(serde_json::Value::Array(value)) => value.is_empty(),
        _ => false,
    };
    if !facts.mounts.is_empty()
        || !empty(document.pointer("/Config/Volumes"))
        || !empty(document.pointer("/HostConfig/Binds"))
        || !empty(document.pointer("/HostConfig/Mounts"))
        || !empty(document.pointer("/HostConfig/Tmpfs"))
        || !empty(document.pointer("/HostConfig/VolumesFrom"))
    {
        return Err(Error::InvalidInput);
    }
    Ok(())
}
fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn validate_id(value: &str) -> Result<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(Error::InvalidInput)
    }
}
fn valid_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|tail| validate_id(tail).is_ok())
}
fn require_root() -> Result<()> {
    if rustix::process::geteuid().as_raw() == 0 {
        Ok(())
    } else {
        Err(Error::InvalidInput)
    }
}
fn storage_error(error: hton_storage::Error) -> Error {
    match error {
        hton_storage::Error::Deadline => Error::Deadline,
        hton_storage::Error::Conflict => Error::Busy,
        hton_storage::Error::TooLarge => Error::OutputLimit,
        _ => Error::Io,
    }
}
