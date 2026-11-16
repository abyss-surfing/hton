//! Exact-object local Docker observations; raw launch fields are protected.
use crate::{
    Budget, Cancellation, Error, Result,
    process::{Runner, Tool},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt, sync::Arc};

const AMNEZIA_XRAY_SERVER_CONFIG: &str = "/opt/amnezia/xray/server.json";
const MAX_NATIVE_CONFIG_BYTES: u64 = 1024 * 1024;

/// Secret-bearing bytes from one fixed native config path. They have no wire
/// serialization and Debug never exposes contents; callers should parse by borrow.
pub struct ProtectedContainerFile {
    container_id: ContainerId,
    structural_fingerprint: String,
    content_digest: String,
    bytes: Vec<u8>,
}
impl ProtectedContainerFile {
    pub fn container_id(&self) -> &ContainerId {
        &self.container_id
    }
    pub fn structural_fingerprint(&self) -> &str {
        &self.structural_fingerprint
    }
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
impl fmt::Debug for ProtectedContainerFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProtectedContainerFile")
            .field("container_id", &self.container_id)
            .field("structural_fingerprint", &self.structural_fingerprint)
            .field("content_digest", &self.content_digest)
            .field("bytes", &"[protected]")
            .finish()
    }
}

/// The only mutable listener fields admitted by the foreign remap contract.
/// Values are intentionally separate from Docker's stringly typed argv format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransportProtocol {
    Tcp,
    Udp,
}
/// Collision-only exact Docker port mapping, not a management/ownership grant.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DockerPublishedBinding {
    pub container_id: ContainerId,
    pub structural_fingerprint: String,
    pub protocol: TransportProtocol,
    pub host_ip: std::net::IpAddr,
    pub host_port: u16,
    pub container_port: u16,
    pub container_ips: BTreeMap<String, std::net::IpAddr>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PublishedPort {
    pub protocol: TransportProtocol,
    pub host_ip: std::net::Ipv4Addr,
    pub host_port: u16,
}
/// Socket present in a container's network namespace. It deliberately omits
/// PID/socket-inode ownership: namespace membership is weaker than process
/// attribution and must not be treated as proof that Xray owns the listener.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContainerNetworkSocket {
    pub protocol: TransportProtocol,
    pub address: std::net::IpAddr,
    pub port: u16,
}
impl fmt::Debug for ContainerNetworkSocket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContainerNetworkSocket")
            .field("protocol", &self.protocol)
            .field("address", &"[namespace address]")
            .field("port", &self.port)
            .finish()
    }
}
/// Exact Docker identity/fingerprint bound to a point-in-time netns socket view.
pub struct ContainerNetworkListenerObservation {
    container_id: ContainerId,
    structural_fingerprint: String,
    sockets: Vec<ContainerNetworkSocket>,
}
impl ContainerNetworkListenerObservation {
    pub fn container_id(&self) -> &ContainerId {
        &self.container_id
    }
    pub fn structural_fingerprint(&self) -> &str {
        &self.structural_fingerprint
    }
    pub fn sockets(&self) -> &[ContainerNetworkSocket] {
        &self.sockets
    }
}
impl fmt::Debug for ContainerNetworkListenerObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContainerNetworkListenerObservation")
            .field("container_id", &self.container_id)
            .field("structural_fingerprint", &self.structural_fingerprint)
            .field("socket_count", &self.sockets.len())
            .finish()
    }
}
/// Exact native process/socket attribution. Executable identity is not a
/// software-version assertion or an ownership grant.
pub struct ContainerXrayListenerObservation {
    container_id: ContainerId,
    structural_fingerprint: String,
    executable_digest: String,
}
impl ContainerXrayListenerObservation {
    pub fn container_id(&self) -> &ContainerId {
        &self.container_id
    }
    pub fn structural_fingerprint(&self) -> &str {
        &self.structural_fingerprint
    }
    pub fn executable_digest(&self) -> &str {
        &self.executable_digest
    }
}
impl fmt::Debug for ContainerXrayListenerObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ContainerXrayListenerObservation([protected process identity])")
    }
}
/// Parsed native binary-reported version. Not installer provenance or an
/// authenticated upstream release statement; no raw process output is retained.
pub struct NativeXrayVersionObservation {
    container_id: ContainerId,
    structural_fingerprint: String,
    executable_digest: String,
    version: String,
    build: String,
}
impl NativeXrayVersionObservation {
    pub fn container_id(&self) -> &ContainerId {
        &self.container_id
    }
    pub fn structural_fingerprint(&self) -> &str {
        &self.structural_fingerprint
    }
    pub fn executable_digest(&self) -> &str {
        &self.executable_digest
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn build(&self) -> &str {
        &self.build
    }
}
impl fmt::Debug for NativeXrayVersionObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NativeXrayVersionObservation([protected binary version])")
    }
}
fn parse_xray_version_statement(bytes: &[u8]) -> Result<(String, String)> {
    if bytes.is_empty() || bytes.len() > 4096 {
        return Err(Error::InvalidResponse);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidResponse)?;
    let lines: Vec<_> = text.lines().collect();
    if lines.len() != 2 || lines[1] != "A unified platform for anti-censorship." {
        return Err(Error::InvalidResponse);
    }
    let (version, rest) = lines[0]
        .strip_prefix("Xray ")
        .and_then(|line| line.split_once(" (Xray, Penetrates Everything.) "))
        .ok_or(Error::InvalidResponse)?;
    let components: Vec<_> = version.split('.').collect();
    if !(2..=3).contains(&components.len())
        || components.iter().any(|part| {
            part.is_empty() || part.len() > 3 || !part.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return Err(Error::InvalidResponse);
    }
    let (build, platform) = rest.split_once(" (").ok_or(Error::InvalidResponse)?;
    if build.is_empty()
        || build.len() > 64
        || !build
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-_".contains(&byte))
    {
        return Err(Error::InvalidResponse);
    }
    let (go, architecture) = platform
        .strip_suffix(')')
        .and_then(|value| value.split_once(" linux/"))
        .ok_or(Error::InvalidResponse)?;
    if !go.starts_with("go1.")
        || go.len() > 32
        || !go
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-".contains(&byte))
        || !matches!(
            architecture,
            "amd64"
                | "386"
                | "arm"
                | "arm64"
                | "mips"
                | "mipsle"
                | "mips64"
                | "mips64le"
                | "riscv64"
                | "loong64"
                | "ppc64"
                | "ppc64le"
                | "s390x"
        )
    {
        return Err(Error::InvalidResponse);
    }
    Ok((version.to_owned(), build.to_owned()))
}
/// Read-only proof that the implemented no-mount snapshot baseline applies.
/// It carries no stored snapshot and grants no mutation authority.
pub struct NoMountSnapshotCapability {
    container_id: ContainerId,
    structural_fingerprint: String,
    inspect_digest: String,
}
impl NoMountSnapshotCapability {
    pub fn container_id(&self) -> &ContainerId {
        &self.container_id
    }
    pub fn structural_fingerprint(&self) -> &str {
        &self.structural_fingerprint
    }
    pub fn inspect_digest(&self) -> &str {
        &self.inspect_digest
    }
}
impl fmt::Debug for NoMountSnapshotCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NoMountSnapshotCapability([protected capture coverage])")
    }
}
impl PublishedPort {
    pub fn new(
        protocol: TransportProtocol,
        host_ip: std::net::Ipv4Addr,
        host_port: u16,
    ) -> Result<Self> {
        if host_port == 0 || host_ip.is_unspecified() || host_ip.is_multicast() {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            protocol,
            host_ip,
            host_port,
        })
    }
}

fn digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hash| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// Accept the deliberately small POSIX TAR subset emitted for one `docker cp`
/// file. Extension records and every non-regular member fail closed.
fn extract_single_xray_config(archive: &[u8]) -> Result<Vec<u8>> {
    const BLOCK: usize = 512;
    if archive.len() < BLOCK * 3 || !archive.len().is_multiple_of(BLOCK) {
        return Err(Error::InvalidResponse);
    }
    let header = &archive[..BLOCK];
    if header.iter().all(|byte| *byte == 0)
        || &header[257..263] != b"ustar\0"
        || &header[263..265] != b"00"
        || !valid_tar_checksum(header)
    {
        return Err(Error::InvalidResponse);
    }
    let member_type = header[156];
    if member_type != 0 && member_type != b'0' {
        return Err(Error::InvalidResponse);
    }
    let name = tar_text(&header[..100])?;
    let prefix = tar_text(&header[345..500])?;
    let member_path = if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    };
    if !matches!(
        member_path.as_str(),
        "server.json" | "opt/amnezia/xray/server.json" | "/opt/amnezia/xray/server.json"
    ) {
        return Err(Error::InvalidResponse);
    }
    let size = tar_octal(&header[124..136])?;
    if size == 0 || size > MAX_NATIVE_CONFIG_BYTES {
        return Err(Error::OutputLimit);
    }
    let size = usize::try_from(size).map_err(|_| Error::OutputLimit)?;
    let padded_size = size
        .checked_add(BLOCK - 1)
        .map(|value| value / BLOCK * BLOCK)
        .ok_or(Error::OutputLimit)?;
    let content_end = BLOCK.checked_add(size).ok_or(Error::OutputLimit)?;
    let entry_end = BLOCK.checked_add(padded_size).ok_or(Error::OutputLimit)?;
    if entry_end
        .checked_add(BLOCK * 2)
        .is_none_or(|end| end > archive.len())
        || archive[content_end..entry_end]
            .iter()
            .any(|byte| *byte != 0)
        || archive[entry_end..].iter().any(|byte| *byte != 0)
    {
        return Err(Error::InvalidResponse);
    }
    Ok(archive[BLOCK..content_end].to_vec())
}

fn tar_text(field: &[u8]) -> Result<String> {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    if field[end..].iter().any(|byte| *byte != 0 && *byte != b' ')
        || field[..end]
            .iter()
            .any(|byte| !byte.is_ascii() || *byte == b'\\')
    {
        return Err(Error::InvalidResponse);
    }
    std::str::from_utf8(&field[..end])
        .map(str::to_owned)
        .map_err(|_| Error::InvalidResponse)
}

fn tar_octal(field: &[u8]) -> Result<u64> {
    let mut digits = Vec::new();
    let mut ended = false;
    for byte in field {
        if *byte == b' ' || *byte == 0 {
            ended = true;
            continue;
        }
        if ended || !(b'0'..=b'7').contains(byte) {
            return Err(Error::InvalidResponse);
        }
        digits.push(*byte);
    }
    if digits.is_empty() || digits.iter().any(|byte| !(b'0'..=b'7').contains(byte)) {
        return Err(Error::InvalidResponse);
    }
    digits.into_iter().try_fold(0u64, |value, digit| {
        value
            .checked_mul(8)
            .and_then(|value| value.checked_add(u64::from(digit - b'0')))
            .ok_or(Error::OutputLimit)
    })
}

fn valid_tar_checksum(header: &[u8]) -> bool {
    let Some(expected) = tar_octal(&header[148..156]).ok() else {
        return false;
    };
    let actual: u64 = header
        .iter()
        .enumerate()
        .map(|(index, byte)| {
            if (148..156).contains(&index) {
                u64::from(b' ')
            } else {
                u64::from(*byte)
            }
        })
        .sum();
    actual == expected
}

fn mount_overlaps_native_config(destination: &str) -> Result<bool> {
    if destination == "/" {
        return Ok(true);
    }
    if !destination.starts_with('/')
        || destination.ends_with('/')
        || destination.contains("//")
        || destination
            .split('/')
            .any(|component| component == "." || component == "..")
    {
        return Err(Error::InvalidResponse);
    }
    let config = AMNEZIA_XRAY_SERVER_CONFIG;
    Ok(config == destination
        || config
            .strip_prefix(destination)
            .is_some_and(|suffix| suffix.starts_with('/')))
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub struct ContainerId(String);
impl ContainerId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for ContainerId {
    type Error = Error;
    fn try_from(value: String) -> Result<Self> {
        Self::new(value)
    }
}
impl fmt::Debug for ContainerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct ImageReference(String);
impl ImageReference {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let (name, pin) = value.rsplit_once('@').ok_or(Error::InvalidInput)?;
        if value.len() > 512
            || name.is_empty()
            || !name.as_bytes()[0].is_ascii_alphanumeric()
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"./_:-".contains(&b))
            || name.contains("..")
            || name.ends_with('/')
            || !digest(pin)
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Debug for ImageReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Native launch configuration may contain secrets. Never serialize/log this record.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerFacts {
    pub id: ContainerId,
    pub image: String,
    pub name: String,
    pub path: String,
    pub args: Vec<String>,
    pub config: LaunchConfig,
    pub host_config: HostConfig,
    pub mounts: Vec<Mount>,
    pub state: ContainerState,
    pub network_settings: NetworkSettings,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LaunchConfig {
    pub image: String,
    pub user: String,
    pub env: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub working_dir: String,
    pub labels: Option<BTreeMap<String, String>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HostConfig {
    pub network_mode: String,
    pub restart_policy: RestartPolicy,
    pub readonly_rootfs: bool,
    pub privileged: bool,
    pub cap_add: Option<Vec<String>>,
    pub cap_drop: Option<Vec<String>>,
    pub security_opt: Option<Vec<String>>,
    pub binds: Option<Vec<String>>,
    pub port_bindings: Option<BTreeMap<String, Option<Vec<PortBinding>>>>,
    pub log_config: LogConfig,
    pub memory: i64,
    pub nano_cpus: i64,
    pub pids_limit: Option<i64>,
    pub init: Option<bool>,
    pub tmpfs: Option<BTreeMap<String, String>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RestartPolicy {
    pub name: String,
    pub maximum_retry_count: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LogConfig {
    pub r#type: String,
    pub config: Option<BTreeMap<String, String>>,
}
#[derive(Deserialize)]
pub struct PortBinding {
    #[serde(rename = "HostIp")]
    pub host_ip: String,
    #[serde(rename = "HostPort")]
    pub host_port: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Mount {
    pub r#type: String,
    pub source: String,
    pub destination: String,
    #[serde(rename = "RW")]
    pub writable: bool,
    pub propagation: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerState {
    pub status: String,
    pub running: bool,
    pub paused: bool,
    pub restarting: bool,
    #[serde(rename = "OOMKilled")]
    pub oom_killed: bool,
    pub dead: bool,
    pub exit_code: i32,
    pub started_at: String,
    pub pid: u32,
    pub finished_at: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkSettings {
    pub networks: BTreeMap<String, NetworkEndpoint>,
}
#[derive(Deserialize)]
pub struct NetworkEndpoint {
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
    #[serde(rename = "IPAMConfig")]
    pub ipam_config: Option<EndpointIpam>,
}
#[derive(Deserialize)]
pub struct EndpointIpam {
    #[serde(rename = "IPv4Address")]
    pub ipv4_address: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ImageConfig {
    pub env: Option<Vec<String>>,
    pub working_dir: String,
    pub labels: Option<BTreeMap<String, String>>,
}
impl fmt::Debug for ContainerFacts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContainerFacts")
            .field("id", &self.id)
            .field("launch", &"[protected]")
            .finish()
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ImageFacts {
    pub id: String,
    pub repo_digests: Option<Vec<String>>,
    pub architecture: String,
    pub os: String,
    pub config: Option<ImageConfig>,
}
impl fmt::Debug for ImageFacts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ImageFacts([protected])")
    }
}

/// Complete display-only facts for one exact local Docker container. This is
/// intentionally smaller than `ContainerFacts` and contains no launch secrets.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ForeignCandidateFacts {
    pub container_id: String,
    pub image_digest: String,
    pub running: bool,
    pub status: String,
    pub published_bindings: Vec<ForeignPublishedBinding>,
    pub structural_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ForeignPublishedBinding {
    pub protocol: TransportProtocol,
    pub container_port: u16,
    pub host_ip: std::net::Ipv4Addr,
    pub host_port: u16,
}
#[derive(Clone)]
pub struct DockerObserver {
    runner: Arc<Runner>,
}
impl DockerObserver {
    pub(crate) fn new(runner: Arc<Runner>) -> Self {
        Self { runner }
    }

    /// Return a complete stable local candidate inventory. The two full-ID
    /// sets and every projected inspect fact must agree across consecutive
    /// passes; no candidate identity is inferred from metadata.
    pub fn foreign_candidate_facts(
        &self,
        budget: Budget,
        cancellation: &dyn Cancellation,
        max_containers: usize,
    ) -> Result<Vec<ForeignCandidateFacts>> {
        if max_containers == 0 || max_containers > 256 {
            return Err(Error::InvalidInput);
        }
        let first_ids = self.container_ids(budget, cancellation)?;
        if first_ids.len() > max_containers {
            return Err(Error::OutputLimit);
        }
        let first = self.foreign_candidate_pass(&first_ids, budget, cancellation)?;
        let second_ids = self.container_ids(budget, cancellation)?;
        if second_ids.len() > max_containers || first_ids != second_ids {
            return Err(Error::Busy);
        }
        let second = self.foreign_candidate_pass(&second_ids, budget, cancellation)?;
        if first != second {
            return Err(Error::Busy);
        }
        budget.check(cancellation)?;
        Ok(second)
    }

    fn foreign_candidate_pass(
        &self,
        ids: &[ContainerId],
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<ForeignCandidateFacts>> {
        let mut candidates = Vec::with_capacity(ids.len());
        for id in ids {
            let facts = self.inspect_container(id, budget, cancellation)?;
            let mut published_bindings = Vec::new();
            if let Some(bindings) = facts.host_config.port_bindings.as_ref() {
                for (container_port, host_bindings) in bindings {
                    let (port, protocol) = container_port
                        .split_once('/')
                        .ok_or(Error::InvalidResponse)?;
                    let container_port = port
                        .parse::<u16>()
                        .ok()
                        .filter(|port| *port != 0)
                        .ok_or(Error::InvalidResponse)?;
                    let protocol = match protocol {
                        "tcp" => TransportProtocol::Tcp,
                        "udp" => TransportProtocol::Udp,
                        "sctp" => continue,
                        _ => return Err(Error::InvalidResponse),
                    };
                    let Some(host_bindings) = host_bindings else {
                        continue;
                    };
                    for binding in host_bindings {
                        let host_port = binding
                            .host_port
                            .parse::<u16>()
                            .ok()
                            .filter(|port| *port != 0)
                            .ok_or(Error::InvalidResponse)?;
                        let host_ip = if binding.host_ip.is_empty() {
                            std::net::Ipv4Addr::UNSPECIFIED
                        } else {
                            binding
                                .host_ip
                                .parse::<std::net::Ipv4Addr>()
                                .map_err(|_| Error::InvalidResponse)?
                        };
                        published_bindings.push(ForeignPublishedBinding {
                            protocol,
                            container_port,
                            host_ip,
                            host_port,
                        });
                    }
                }
            }
            published_bindings.sort();
            candidates.push(ForeignCandidateFacts {
                container_id: id.as_str().to_owned(),
                image_digest: facts.image.clone(),
                running: facts.state.running,
                status: facts.state.status.clone(),
                published_bindings,
                structural_fingerprint: container_structural_fingerprint(&facts),
            });
        }
        Ok(candidates)
    }
    pub fn inspect_container(
        &self,
        expected: &ContainerId,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<ContainerFacts> {
        let bytes = self.runner.run(
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
                expected.as_str(),
            ],
            budget,
            cancellation,
        )?;
        let facts: ContainerFacts =
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)?;
        if facts.id != *expected {
            return Err(Error::IdentityMismatch);
        }
        if !digest(&facts.image) {
            return Err(Error::InvalidResponse);
        }
        budget.check(cancellation)?;
        Ok(facts)
    }
    /// Observe TCP LISTEN and UDP bind rows in one exact running container's
    /// network namespace. The Docker init PID comes only from inspect; the
    /// container is reinspected after the bounded procfs read. This proves
    /// namespace socket presence, not owning-process identity.
    pub fn observe_container_network_listeners(
        &self,
        expected: &ContainerId,
        expected_fingerprint: &str,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<ContainerNetworkListenerObservation> {
        if expected_fingerprint.len() != 71
            || !expected_fingerprint.starts_with("sha256:")
            || !expected_fingerprint[7..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(Error::InvalidInput);
        }
        let before = self.inspect_container(expected, budget, cancellation)?;
        if container_structural_fingerprint(&before) != expected_fingerprint {
            return Err(Error::IdentityMismatch);
        }
        if !before.state.running
            || before.state.paused
            || before.state.restarting
            || before.state.dead
            || before.state.pid == 0
        {
            return Err(Error::Busy);
        }
        let pid = before.state.pid;
        let started_at = before.state.started_at.clone();
        budget.check(cancellation)?;
        let rows = crate::ports::container_network_sockets(pid)?;
        budget.check(cancellation)?;
        let after = self.inspect_container(expected, budget, cancellation)?;
        if after.id != *expected
            || !after.state.running
            || after.state.paused
            || after.state.restarting
            || after.state.dead
            || after.state.pid != pid
            || after.state.started_at != started_at
            || container_structural_fingerprint(&after) != expected_fingerprint
        {
            return Err(Error::Busy);
        }
        let mut sockets = rows
            .into_iter()
            .map(|row| ContainerNetworkSocket {
                protocol: row.protocol,
                address: row.address,
                port: row.port,
            })
            .collect::<Vec<_>>();
        sockets.sort();
        sockets.dedup();
        Ok(ContainerNetworkListenerObservation {
            container_id: expected.clone(),
            structural_fingerprint: expected_fingerprint.to_owned(),
            sockets,
        })
    }
    /// Attribute one parsed native TCP listener to the pinned Xray launch
    /// shape in the exact Docker process tree. No foreign command is executed.
    pub fn observe_amnezia_xray_listener(
        &self,
        expected: &ContainerId,
        expected_fingerprint: &str,
        address: std::net::Ipv4Addr,
        port: u16,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<ContainerXrayListenerObservation> {
        if !digest(expected_fingerprint) || port == 0 {
            return Err(Error::InvalidInput);
        }
        let before = self.inspect_container(expected, budget, cancellation)?;
        if container_structural_fingerprint(&before) != expected_fingerprint {
            return Err(Error::IdentityMismatch);
        }
        if !before.state.running
            || before.state.paused
            || before.state.restarting
            || before.state.dead
            || before.state.pid == 0
        {
            return Err(Error::Busy);
        }
        let executable_digest = crate::ports::container_xray_listener(
            before.state.pid,
            address,
            port,
            budget,
            cancellation,
        )?;
        let after = self.inspect_container(expected, budget, cancellation)?;
        if after.id != *expected
            || !after.state.running
            || after.state.paused
            || after.state.restarting
            || after.state.dead
            || after.state.pid != before.state.pid
            || after.state.started_at != before.state.started_at
            || container_structural_fingerprint(&after) != expected_fingerprint
        {
            return Err(Error::Busy);
        }
        budget.check(cancellation)?;
        Ok(ContainerXrayListenerObservation {
            container_id: expected.clone(),
            structural_fingerprint: expected_fingerprint.to_owned(),
            executable_digest,
        })
    }
    /// Fixed unprivileged version-only command, bracketed by exact native
    /// process/executable and Docker identity observations. No arbitrary exec API.
    pub fn observe_amnezia_xray_version(
        &self,
        expected: &ContainerId,
        expected_fingerprint: &str,
        address: std::net::Ipv4Addr,
        port: u16,
        expected_executable_digest: &str,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<NativeXrayVersionObservation> {
        if !digest(expected_executable_digest) {
            return Err(Error::InvalidInput);
        }
        let before = self.observe_amnezia_xray_listener(
            expected,
            expected_fingerprint,
            address,
            port,
            budget,
            cancellation,
        )?;
        if before.executable_digest() != expected_executable_digest {
            return Err(Error::IdentityMismatch);
        }
        let version_budget = Budget {
            deadline: budget.deadline,
            output_bytes: budget.output_bytes.min(4096),
        };
        let output = self.runner.run(
            Tool::Docker,
            &[
                "--host",
                "unix:///var/run/docker.sock",
                "--config",
                "/opt/hton/work/docker-client",
                "container",
                "exec",
                "--user",
                "65534:65534",
                "--workdir",
                "/",
                "--env",
                "PATH=/usr/bin:/bin",
                "--env",
                "LANG=C",
                "--env",
                "LC_ALL=C",
                expected.as_str(),
                "/usr/bin/xray",
                "version",
            ],
            version_budget,
            cancellation,
        )?;
        let (version, build) = parse_xray_version_statement(&output)?;
        let after = self.observe_amnezia_xray_listener(
            expected,
            expected_fingerprint,
            address,
            port,
            budget,
            cancellation,
        )?;
        if after.executable_digest() != expected_executable_digest {
            return Err(Error::IdentityMismatch);
        }
        budget.check(cancellation)?;
        Ok(NativeXrayVersionObservation {
            container_id: expected.clone(),
            structural_fingerprint: expected_fingerprint.to_owned(),
            executable_digest: expected_executable_digest.to_owned(),
            version,
            build,
        })
    }
    /// Qualify only the implemented snapshot data boundary, without creating
    /// a store/image/archive or pausing the foreign container.
    pub fn observe_no_mount_snapshot_capability(
        &self,
        expected: &ContainerId,
        expected_fingerprint: &str,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<NoMountSnapshotCapability> {
        if !digest(expected_fingerprint) {
            return Err(Error::InvalidInput);
        }
        let before = self.inspect_container(expected, budget, cancellation)?;
        if container_structural_fingerprint(&before) != expected_fingerprint {
            return Err(Error::IdentityMismatch);
        }
        let args = [
            "--host",
            "unix:///var/run/docker.sock",
            "--config",
            "/opt/hton/work/docker-client",
            "container",
            "inspect",
            "--format",
            "{{json .}}",
            expected.as_str(),
        ];
        let raw = self.runner.run(Tool::Docker, &args, budget, cancellation)?;
        let observed: ContainerFacts =
            serde_json::from_slice(&raw).map_err(|_| Error::InvalidResponse)?;
        if observed.id != *expected
            || container_structural_fingerprint(&observed) != expected_fingerprint
        {
            return Err(Error::IdentityMismatch);
        }
        crate::docker_snapshot::reject_external_data(&raw, &observed)?;
        let repeated = self.runner.run(Tool::Docker, &args, budget, cancellation)?;
        if raw != repeated {
            return Err(Error::Busy);
        }
        let after = self.inspect_container(expected, budget, cancellation)?;
        if after.id != *expected
            || container_structural_fingerprint(&after) != expected_fingerprint
            || after.state.pid != before.state.pid
            || after.state.started_at != before.state.started_at
            || after.state.running != before.state.running
            || after.state.paused
            || after.state.restarting
            || after.state.dead
        {
            return Err(Error::Busy);
        }
        budget.check(cancellation)?;
        Ok(NoMountSnapshotCapability {
            container_id: expected.clone(),
            structural_fingerprint: expected_fingerprint.to_owned(),
            inspect_digest: format!("sha256:{:x}", Sha256::digest(&raw)),
        })
    }
    /// Read only the pinned Amnezia Xray config path from one exact container.
    /// Inspect/copy/copy/inspect brackets the two contents; no process is run in
    /// the container and any structure/content drift blocks the observation.
    pub fn read_amnezia_xray_server_config(
        &self,
        expected: &ContainerId,
        expected_fingerprint: &str,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<ProtectedContainerFile> {
        if expected_fingerprint.len() != 71
            || !expected_fingerprint.starts_with("sha256:")
            || !expected_fingerprint[7..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(Error::InvalidInput);
        }
        let inspect_matches = |this: &Self| -> Result<()> {
            let facts = this.inspect_container(expected, budget, cancellation)?;
            if container_structural_fingerprint(&facts) != expected_fingerprint {
                return Err(Error::IdentityMismatch);
            }
            let mut overlaps_config = false;
            for mount in &facts.mounts {
                if mount_overlaps_native_config(&mount.destination)? {
                    overlaps_config = true;
                    break;
                }
            }
            if overlaps_config {
                return Err(Error::InvalidResponse);
            }
            Ok(())
        };
        let read_once = |this: &Self| -> Result<Vec<u8>> {
            let source = format!("{}:{AMNEZIA_XRAY_SERVER_CONFIG}", expected.as_str());
            let archive = this.runner.run(
                Tool::Docker,
                &[
                    "--host",
                    "unix:///var/run/docker.sock",
                    "--config",
                    "/opt/hton/work/docker-client",
                    "container",
                    "cp",
                    &source,
                    "-",
                ],
                budget,
                cancellation,
            )?;
            extract_single_xray_config(&archive)
        };

        inspect_matches(self)?;
        let first = read_once(self)?;
        inspect_matches(self)?;
        let second = read_once(self)?;
        inspect_matches(self)?;
        let first_digest = format!("sha256:{:x}", Sha256::digest(&first));
        let second_digest = format!("sha256:{:x}", Sha256::digest(&second));
        if first_digest != second_digest {
            return Err(Error::IdentityMismatch);
        }
        Ok(ProtectedContainerFile {
            container_id: expected.clone(),
            structural_fingerprint: expected_fingerprint.to_owned(),
            content_digest: first_digest,
            bytes: first,
        })
    }
    /// Collision-only inventory of all Docker-published TCP/UDP host ports.
    /// Two identical bounded snapshots are required; this does not grant resource
    /// ownership, expose launch configuration, or authorize mutation.
    pub fn published_port_reservations(
        &self,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<super::ports::HostPortReservation>> {
        Ok(self
            .published_port_bindings(budget, cancellation)?
            .into_iter()
            .map(|binding| super::ports::HostPortReservation {
                protocol: binding.protocol,
                port: binding.host_port,
                container_id: Some(binding.container_id),
                container_fingerprint: Some(binding.structural_fingerprint),
            })
            .collect())
    }
    pub(crate) fn published_port_bindings(
        &self,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<DockerPublishedBinding>> {
        let first_ids = self.container_ids(budget, cancellation)?;
        let first = self.container_bindings(&first_ids, budget, cancellation)?;
        let second_ids = self.container_ids(budget, cancellation)?;
        let second = self.container_bindings(&second_ids, budget, cancellation)?;
        if first_ids != second_ids || first != second {
            return Err(Error::Busy);
        }
        budget.check(cancellation)?;
        Ok(second.into_iter().collect())
    }
    fn container_ids(
        &self,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<ContainerId>> {
        let bytes = self.runner.run(
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
            cancellation,
        )?;
        let text = std::str::from_utf8(&bytes).map_err(|_| Error::InvalidResponse)?;
        let mut ids = Vec::new();
        for line in text.lines() {
            if ids.len() >= 256 {
                return Err(Error::OutputLimit);
            }
            if line.trim() != line || line.is_empty() {
                return Err(Error::InvalidResponse);
            }
            ids.push(ContainerId::new(line)?);
        }
        ids.sort();
        ids.dedup();
        Ok(ids)
    }
    fn container_bindings(
        &self,
        ids: &[ContainerId],
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<std::collections::BTreeSet<DockerPublishedBinding>> {
        let mut reservations = std::collections::BTreeSet::new();
        for id in ids {
            let facts = self.inspect_container(id, budget, cancellation)?;
            let structural_fingerprint = container_structural_fingerprint(&facts);
            let container_ips = facts
                .network_settings
                .networks
                .iter()
                .filter_map(|(name, network)| {
                    network.ip_address.parse().ok().map(|ip| (name.clone(), ip))
                })
                .collect::<BTreeMap<String, std::net::IpAddr>>();
            let Some(bindings) = facts.host_config.port_bindings else {
                continue;
            };
            for (container_port, host_bindings) in bindings {
                let (port, protocol) = container_port
                    .split_once('/')
                    .ok_or(Error::InvalidResponse)?;
                let _container_port = port.parse::<u16>().map_err(|_| Error::InvalidResponse)?;
                let protocol = match protocol {
                    "tcp" => TransportProtocol::Tcp,
                    "udp" => TransportProtocol::Udp,
                    // Other Docker transports cannot conflict with allocated TCP/UDP.
                    "sctp" => continue,
                    _ => return Err(Error::InvalidResponse),
                };
                let Some(host_bindings) = host_bindings else {
                    continue;
                };
                for binding in host_bindings {
                    let host_port = binding
                        .host_port
                        .parse::<u16>()
                        .ok()
                        .filter(|port| *port != 0)
                        .ok_or(Error::InvalidResponse)?;
                    let host_ip = if binding.host_ip.is_empty() {
                        std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
                    } else {
                        binding
                            .host_ip
                            .parse()
                            .map_err(|_| Error::InvalidResponse)?
                    };
                    reservations.insert(DockerPublishedBinding {
                        container_id: id.clone(),
                        structural_fingerprint: structural_fingerprint.clone(),
                        protocol,
                        host_ip,
                        host_port,
                        container_port: _container_port,
                        container_ips: container_ips.clone(),
                    });
                }
            }
        }
        Ok(reservations)
    }
    pub fn inspect_image(
        &self,
        expected: &ImageReference,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<ImageFacts> {
        let bytes = self.runner.run(
            Tool::Docker,
            &[
                "--host",
                "unix:///var/run/docker.sock",
                "--config",
                "/opt/hton/work/docker-client",
                "image",
                "inspect",
                "--format",
                "{{json .}}",
                expected.as_str(),
            ],
            budget,
            cancellation,
        )?;
        let facts: ImageFacts =
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)?;
        if !digest(&facts.id) || facts.os != "linux" {
            return Err(Error::InvalidResponse);
        }
        if !facts
            .repo_digests
            .as_ref()
            .is_some_and(|digests| digests.iter().any(|pin| pin == expected.as_str()))
        {
            return Err(Error::IdentityMismatch);
        }
        budget.check(cancellation)?;
        Ok(facts)
    }
}

/// Only trusted domain ports may use this capability after authorization/journalling.
#[derive(Clone)]
pub struct DockerControl {
    runner: Arc<Runner>,
}
#[derive(Clone, Copy, Debug)]
pub enum OwnedEngine {
    Haproxy,
    Caddy,
    Xray,
    Unbound,
}
#[derive(Clone, Copy, Debug)]
pub enum RestartMode {
    Disabled,
    Always,
    UnlessStopped,
    OnFailure,
}
fn logical(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        && !matches!(value, "." | "..")
}
impl DockerControl {
    pub(crate) fn new(runner: Arc<Runner>) -> Self {
        Self { runner }
    }
    fn command(
        &self,
        args: &[&str],
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<u8>> {
        let mut full = vec![
            "--host",
            "unix:///var/run/docker.sock",
            "--config",
            "/opt/hton/work/docker-client",
        ];
        full.extend_from_slice(args);
        self.runner.run(Tool::Docker, &full, budget, cancellation)
    }
    pub fn pull(
        &self,
        image: &ImageReference,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<ImageFacts> {
        self.command(
            &["image", "pull", "--quiet", image.as_str()],
            budget,
            cancellation,
        )?;
        DockerObserver::new(self.runner.clone()).inspect_image(image, budget, cancellation)
    }
    pub fn create_parked(
        &self,
        instance: &str,
        service: &str,
        image: &ImageReference,
        engine: OwnedEngine,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<ContainerId> {
        if !logical(instance) || !logical(service) {
            return Err(Error::InvalidInput);
        }
        DockerObserver::new(self.runner.clone()).inspect_image(image, budget, cancellation)?;
        let name = format!("hton-{instance}-{service}");
        let mount = format!(
            "type=bind,src=/opt/hton/work/runtime/services/{service},dst=/etc/hton,readonly"
        );
        let label_instance = format!("io.hton.instance={instance}");
        let label_service = format!("io.hton.service={service}");
        let (entrypoint, arguments): (&str, &[&str]) = match engine {
            OwnedEngine::Haproxy => (
                "haproxy",
                &["-W", "-db", "-f", "/etc/hton/current/haproxy.cfg"],
            ),
            OwnedEngine::Caddy => (
                "caddy",
                &[
                    "run",
                    "--config",
                    "/etc/hton/current/Caddyfile",
                    "--adapter",
                    "caddyfile",
                ],
            ),
            OwnedEngine::Xray => ("xray", &["run", "-config", "/etc/hton/current/xray.json"]),
            OwnedEngine::Unbound => (
                "/usr/local/unbound/unbound.d/sbin/unbound",
                &["-d", "-c", "/etc/hton/current/unbound.conf"],
            ),
        };
        let mut args = vec![
            "container",
            "create",
            "--pull",
            "never",
            "--name",
            &name,
            "--restart",
            "no",
            "--network",
            "none",
            "--user",
            "0:0",
            "--read-only",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges:true",
            "--init",
            "--memory",
            "128m",
            "--cpus",
            "1.0",
            "--pids-limit",
            "64",
            "--tmpfs",
            "/tmp:rw,noexec,nosuid,size=16m",
            "--tmpfs",
            "/run:rw,noexec,nosuid,size=4m",
            "--mount",
            &mount,
            "--log-driver",
            "none",
            "--label",
            &label_instance,
            "--label",
            &label_service,
            "--entrypoint",
            entrypoint,
            image.as_str(),
        ];
        if matches!(engine, OwnedEngine::Caddy) {
            let offset = args.len() - 1;
            args.splice(
                offset..offset,
                [
                    "--tmpfs",
                    "/data:rw,noexec,nosuid,size=16m",
                    "--tmpfs",
                    "/config:rw,noexec,nosuid,size=4m",
                ],
            );
        }
        args.extend_from_slice(arguments);
        let bytes = self.command(&args, budget, cancellation)?;
        let id = ContainerId::new(
            std::str::from_utf8(&bytes)
                .map_err(|_| Error::InvalidResponse)?
                .trim(),
        )?;
        let facts = DockerObserver::new(self.runner.clone()).inspect_container(
            &id,
            budget,
            cancellation,
        )?;
        if facts.state.running
            || facts.host_config.restart_policy.name != "no"
            || facts.host_config.network_mode != "none"
            || facts
                .host_config
                .port_bindings
                .as_ref()
                .is_some_and(|ports| !ports.is_empty())
        {
            return Err(Error::IdentityMismatch);
        }
        Ok(id)
    }
    pub fn start(&self, id: &ContainerId, budget: Budget, cancel: &dyn Cancellation) -> Result<()> {
        self.command(&["container", "start", id.as_str()], budget, cancel)
            .map(|_| ())
    }
    pub fn stop(&self, id: &ContainerId, budget: Budget, cancel: &dyn Cancellation) -> Result<()> {
        self.command(
            &["container", "stop", "--time", "10", id.as_str()],
            budget,
            cancel,
        )
        .map(|_| ())
    }
    pub fn restart(
        &self,
        id: &ContainerId,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        self.command(
            &["container", "restart", "--time", "10", id.as_str()],
            budget,
            cancel,
        )
        .map(|_| ())
    }
    pub fn set_restart_policy(
        &self,
        id: &ContainerId,
        policy: RestartMode,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let value = match policy {
            RestartMode::Disabled => "no",
            RestartMode::Always => "always",
            RestartMode::UnlessStopped => "unless-stopped",
            RestartMode::OnFailure => "on-failure",
        };
        self.command(
            &["container", "update", "--restart", value, id.as_str()],
            budget,
            cancel,
        )
        .map(|_| ())
    }
    pub fn reload(
        &self,
        id: &ContainerId,
        engine: OwnedEngine,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let args: Vec<&str> = match engine {
            OwnedEngine::Caddy => vec![
                "container",
                "exec",
                id.as_str(),
                "caddy",
                "reload",
                "--config",
                "/etc/hton/current/Caddyfile",
                "--adapter",
                "caddyfile",
                "--address",
                "unix//run/caddy-admin.sock",
            ],
            OwnedEngine::Haproxy => return Err(Error::InvalidInput),
            OwnedEngine::Xray | OwnedEngine::Unbound => return Err(Error::InvalidInput),
        };
        self.command(&args, budget, cancel).map(|_| ())
    }
    /// Immutable Compose has already passed independent runtime shape validation.
    /// Every component is a typed logical ID and fixed root; no caller filesystem path.
    pub fn recreate(
        &self,
        instance: &str,
        bundle: &str,
        service: &str,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ContainerId> {
        if !logical(instance) || !logical(bundle) || !logical(service) {
            return Err(Error::InvalidInput);
        }
        let project = format!("hton-{instance}");
        let compose = format!("/opt/hton/work/runtime/execution/{bundle}/compose.json");
        self.command(
            &[
                "compose",
                "--project-name",
                &project,
                "--file",
                &compose,
                "up",
                "--detach",
                "--no-deps",
                "--no-build",
                "--pull",
                "never",
                "--force-recreate",
                service,
            ],
            budget,
            cancel,
        )?;
        let bytes = self.command(
            &[
                "compose",
                "--project-name",
                &project,
                "--file",
                &compose,
                "ps",
                "--all",
                "--quiet",
                service,
            ],
            budget,
            cancel,
        )?;
        ContainerId::new(
            std::str::from_utf8(&bytes)
                .map_err(|_| Error::InvalidResponse)?
                .trim(),
        )
    }
}

/// Shared native fingerprint owner, independent of either domain DTO package.
/// Launch/env material is hashed in memory and never emitted as diagnostic fields.
pub struct NativeFingerprints {
    pub identity: String,
    pub structural: String,
    pub state: String,
    pub status: &'static str,
    pub restart_policy: &'static str,
}
fn hash_native(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}
pub fn container_identity_digest(id: &ContainerId) -> String {
    hash_native(id.as_str().as_bytes())
}
fn structural_value(facts: &ContainerFacts) -> serde_json::Value {
    let mut labels = facts.config.labels.clone().unwrap_or_default();
    labels.retain(|key, _| !key.starts_with("com.docker.compose."));
    let ports: BTreeMap<_, _> = facts
        .host_config
        .port_bindings
        .as_ref()
        .map(|ports| {
            ports
                .iter()
                .map(|(key, values)| {
                    (
                        key.clone(),
                        values
                            .as_ref()
                            .map(|values| {
                                values
                                    .iter()
                                    .map(|value| (value.host_ip.clone(), value.host_port.clone()))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let mut mounts: Vec<_> = facts
        .mounts
        .iter()
        .filter(|mount| mount.r#type != "tmpfs")
        .map(|mount| {
            (
                mount.r#type.clone(),
                mount.source.clone(),
                mount.destination.clone(),
                mount.writable,
            )
        })
        .collect();
    mounts.sort();
    serde_json::json!({"image":facts.image,"entrypoint":facts.config.entrypoint,"command":facts.config.cmd,"user":facts.config.user,"env":facts.config.env,"workingDir":facts.config.working_dir,"labels":labels,"networkMode":facts.host_config.network_mode,"networkIps":facts.network_settings.networks.iter().map(|(name,value)|(name.clone(),value.ipam_config.as_ref().and_then(|config|config.ipv4_address.as_ref()).cloned().unwrap_or_else(||value.ip_address.clone()))).collect::<BTreeMap<_,_>>(),"readOnly":facts.host_config.readonly_rootfs,"privileged":facts.host_config.privileged,"capAdd":facts.host_config.cap_add.as_ref().filter(|values|!values.is_empty()),"capDrop":facts.host_config.cap_drop,"securityOpt":facts.host_config.security_opt,"ports":ports,"logDriver":facts.host_config.log_config.r#type,"mounts":mounts,"memory":facts.host_config.memory,"nanoCpus":facts.host_config.nano_cpus,"pids":facts.host_config.pids_limit,"init":facts.host_config.init,"tmpfs":facts.host_config.tmpfs})
}
pub fn container_structural_fingerprint(facts: &ContainerFacts) -> String {
    hash_native(&serde_json::to_vec(&structural_value(facts)).expect("native facts JSON invariant"))
}
pub struct ExpectedNativeFingerprints {
    pub structural: String,
    pub state: String,
}
/// Only the independently validated Compose subset can be projected; image defaults
/// are actual cached immutable image metadata, not a guessed image environment.
pub fn expected_container_fingerprints(
    spec: &serde_json::Value,
    image: &ImageFacts,
    active_artifact_digest: Option<&str>,
    status: &str,
    restart: &str,
) -> Result<ExpectedNativeFingerprints> {
    use serde_json::{Value, json};
    let object = spec.as_object().ok_or(Error::InvalidInput)?;
    let config = image.config.as_ref().ok_or(Error::InvalidResponse)?;
    let instance = object
        .get("labels")
        .and_then(|value| value.get("io.hton.instance"))
        .and_then(Value::as_str)
        .filter(|value| logical(value))
        .ok_or(Error::InvalidInput)?;
    let network = format!("hton-{instance}");
    let ip = object
        .get("networks")
        .and_then(|value| value.get("hton"))
        .and_then(|value| value.get("ipv4_address"))
        .and_then(Value::as_str)
        .ok_or(Error::InvalidInput)?;
    ip.parse::<std::net::Ipv4Addr>()
        .map_err(|_| Error::InvalidInput)?;
    let mut labels = config.labels.clone().unwrap_or_default();
    for (key, value) in object
        .get("labels")
        .and_then(Value::as_object)
        .ok_or(Error::InvalidInput)?
    {
        labels.insert(
            key.clone(),
            value.as_str().ok_or(Error::InvalidInput)?.to_owned(),
        );
    }
    labels.retain(|key, _| !key.starts_with("com.docker.compose."));
    let mut ports = BTreeMap::<String, Vec<(String, String)>>::new();
    if let Some(bindings) = object.get("ports") {
        for binding in bindings.as_array().ok_or(Error::InvalidInput)? {
            let target = binding
                .get("target")
                .and_then(Value::as_u64)
                .ok_or(Error::InvalidInput)?;
            let proto = binding
                .get("protocol")
                .and_then(Value::as_str)
                .ok_or(Error::InvalidInput)?;
            let host = binding
                .get("host_ip")
                .and_then(Value::as_str)
                .ok_or(Error::InvalidInput)?;
            let published = binding
                .get("published")
                .and_then(Value::as_str)
                .ok_or(Error::InvalidInput)?;
            ports
                .entry(format!("{target}/{proto}"))
                .or_default()
                .push((host.to_owned(), published.to_owned()));
        }
    }
    let mut mounts = Vec::new();
    for mount in object
        .get("volumes")
        .and_then(Value::as_array)
        .ok_or(Error::InvalidInput)?
    {
        mounts.push((
            mount
                .get("type")
                .and_then(Value::as_str)
                .ok_or(Error::InvalidInput)?
                .to_owned(),
            mount
                .get("source")
                .and_then(Value::as_str)
                .ok_or(Error::InvalidInput)?
                .to_owned(),
            mount
                .get("target")
                .and_then(Value::as_str)
                .ok_or(Error::InvalidInput)?
                .to_owned(),
            !mount
                .get("read_only")
                .and_then(Value::as_bool)
                .ok_or(Error::InvalidInput)?,
        ));
    }
    mounts.sort();
    let mut tmpfs = BTreeMap::new();
    for value in object
        .get("tmpfs")
        .and_then(Value::as_array)
        .ok_or(Error::InvalidInput)?
    {
        let (path, options) = value
            .as_str()
            .and_then(|value| value.split_once(':'))
            .ok_or(Error::InvalidInput)?;
        tmpfs.insert(path.to_owned(), options.to_owned());
    }
    let memory = object
        .get("mem_limit")
        .and_then(Value::as_str)
        .and_then(|value| value.strip_suffix('m'))
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or(Error::InvalidInput)?
        .checked_mul(1024 * 1024)
        .ok_or(Error::InvalidInput)?;
    let cpus = object
        .get("cpus")
        .and_then(|value| {
            value
                .as_f64()
                .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        })
        .filter(|value| value.is_finite() && (0.05..=16.0).contains(value))
        .ok_or(Error::InvalidInput)?;
    let shape = json!({"image":image.id,"entrypoint":object.get("entrypoint"),"command":object.get("command"),"user":object.get("user"),"env":config.env,"workingDir":config.working_dir,"labels":labels,"networkMode":network,"networkIps":BTreeMap::from([(format!("hton-{instance}"),ip)]),"readOnly":object.get("read_only"),"privileged":false,"capAdd":Value::Null,"capDrop":object.get("cap_drop"),"securityOpt":object.get("security_opt"),"ports":ports,"logDriver":object.get("logging").and_then(|value|value.get("driver")),"mounts":mounts,"memory":memory,"nanoCpus":(cpus*1_000_000_000.0).round()as i64,"pids":object.get("pids_limit"),"init":object.get("init"),"tmpfs":tmpfs});
    let structural = hash_native(&serde_json::to_vec(&shape).map_err(|_| Error::InvalidInput)?);
    let state = native_state_fingerprint(
        &hash_native(&[]),
        &structural,
        status,
        restart,
        active_artifact_digest,
    )?;
    Ok(ExpectedNativeFingerprints { structural, state })
}
pub fn native_state_fingerprint(
    identity: &str,
    structural: &str,
    status: &str,
    restart: &str,
    config: Option<&str>,
) -> Result<String> {
    if !digest(identity)
        || !digest(structural)
        || config.is_some_and(|value| !digest(value))
        || !matches!(status, "running" | "stopped" | "failed" | "unknown")
        || !matches!(
            restart,
            "disabled" | "always" | "unless_stopped" | "on_failure" | "external"
        )
    {
        return Err(Error::InvalidInput);
    }
    Ok(hash_native(
        &serde_json::to_vec(&(structural, status, restart, config))
            .map_err(|_| Error::InvalidInput)?,
    ))
}
pub fn container_fingerprints(
    facts: &ContainerFacts,
    active_artifact_digest: Option<&str>,
) -> Result<NativeFingerprints> {
    let identity = container_identity_digest(&facts.id);
    let structural = container_structural_fingerprint(facts);
    let status = if facts.state.dead || facts.state.oom_killed {
        "failed"
    } else if facts.state.running && !facts.state.paused && !facts.state.restarting {
        "running"
    } else if !facts.state.running && matches!(facts.state.status.as_str(), "created" | "exited") {
        "stopped"
    } else {
        "unknown"
    };
    let restart_policy = match facts.host_config.restart_policy.name.as_str() {
        "no" => "disabled",
        "always" => "always",
        "unless-stopped" => "unless_stopped",
        "on-failure" => "on_failure",
        _ => return Err(Error::InvalidResponse),
    };
    let state = native_state_fingerprint(
        &identity,
        &structural,
        status,
        restart_policy,
        active_artifact_digest,
    )?;
    Ok(NativeFingerprints {
        identity,
        structural,
        state,
        status,
        restart_policy,
    })
}

impl DockerObserver {
    /// Known exact daemon not-found text only; all other failures remain unavailable.
    pub fn cached_image(
        &self,
        expected: &ImageReference,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Option<ImageFacts>> {
        let output = self.runner.run_capture(
            Tool::Docker,
            &[
                "--host",
                "unix:///var/run/docker.sock",
                "--config",
                "/opt/hton/work/docker-client",
                "image",
                "inspect",
                "--format",
                "{{json .}}",
                expected.as_str(),
            ],
            &[],
            budget,
            cancel,
        )?;
        if !output.success {
            let stderr = std::str::from_utf8(&output.stderr)
                .map_err(|_| Error::CommandFailed)?
                .trim();
            if output.stdout.is_empty()
                && (stderr
                    == format!(
                        "Error response from daemon: No such image: {}",
                        expected.as_str()
                    )
                    || stderr == format!("Error: No such image: {}", expected.as_str()))
            {
                return Ok(None);
            }
            return Err(Error::CommandFailed);
        }
        let facts: ImageFacts =
            serde_json::from_slice(&output.stdout).map_err(|_| Error::InvalidResponse)?;
        if !digest(&facts.id)
            || facts.os != "linux"
            || !facts
                .repo_digests
                .as_ref()
                .is_some_and(|values| values.iter().any(|value| value == expected.as_str()))
        {
            return Err(Error::IdentityMismatch);
        }
        budget.check(cancel)?;
        Ok(Some(facts))
    }
}
impl DockerControl {
    /// Recovery lookup does not adopt arbitrary discovered objects: only the exact
    /// deterministic parked instance/service launch is returned; ambiguity is drift.
    pub fn find_parked(
        &self,
        instance: &str,
        service: &str,
        image: &ImageReference,
        engine: OwnedEngine,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Option<ContainerId>> {
        if !logical(instance) || !logical(service) {
            return Err(Error::InvalidInput);
        }
        let name = format!("hton-{instance}-{service}");
        let output = self.runner.run_capture(
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
                &name,
            ],
            &[],
            budget,
            cancel,
        )?;
        if !output.success {
            let text = std::str::from_utf8(&output.stderr)
                .map_err(|_| Error::CommandFailed)?
                .trim();
            if output.stdout.is_empty()
                && (text == format!("Error: No such container: {name}")
                    || text == format!("Error response from daemon: No such container: {name}"))
            {
                return Ok(None);
            }
            return Err(Error::CommandFailed);
        }
        let facts: ContainerFacts =
            serde_json::from_slice(&output.stdout).map_err(|_| Error::InvalidResponse)?;
        let labels = facts
            .config
            .labels
            .as_ref()
            .ok_or(Error::IdentityMismatch)?;
        let expected_entry = match engine {
            OwnedEngine::Haproxy => "haproxy",
            OwnedEngine::Caddy => "caddy",
            OwnedEngine::Xray => "xray",
            OwnedEngine::Unbound => "/usr/local/unbound/unbound.d/sbin/unbound",
        };
        let expected_args: &[&str] = match engine {
            OwnedEngine::Haproxy => &["-W", "-db", "-f", "/etc/hton/current/haproxy.cfg"],
            OwnedEngine::Caddy => &[
                "run",
                "--config",
                "/etc/hton/current/Caddyfile",
                "--adapter",
                "caddyfile",
            ],
            OwnedEngine::Xray => &["run", "-config", "/etc/hton/current/xray.json"],
            OwnedEngine::Unbound => &["-d", "-c", "/etc/hton/current/unbound.conf"],
        };
        if facts.name != format!("/{name}")
            || facts.config.image != image.as_str()
            || facts.state.running
            || facts.host_config.restart_policy.name != "no"
            || facts.host_config.network_mode != "none"
            || !facts.host_config.readonly_rootfs
            || facts.host_config.privileged
            || facts.config.user != "0:0"
            || facts.host_config.log_config.r#type != "none"
            || facts
                .host_config
                .cap_add
                .as_ref()
                .is_some_and(|values| !values.is_empty())
            || facts.host_config.cap_drop.as_deref() != Some(&["ALL".to_owned()])
            || facts.host_config.security_opt.as_deref()
                != Some(&["no-new-privileges:true".to_owned()])
            || facts
                .host_config
                .port_bindings
                .as_ref()
                .is_some_and(|ports| !ports.is_empty())
            || labels.get("io.hton.instance").map(String::as_str) != Some(instance)
            || labels.get("io.hton.service").map(String::as_str) != Some(service)
            || facts
                .config
                .entrypoint
                .as_ref()
                .map(|entry| entry.iter().map(String::as_str).collect::<Vec<_>>())
                != Some(vec![expected_entry])
            || facts.args.iter().map(String::as_str).collect::<Vec<_>>() != expected_args
        {
            return Err(Error::IdentityMismatch);
        }
        let mount = facts
            .mounts
            .iter()
            .find(|mount| mount.destination == "/etc/hton")
            .ok_or(Error::IdentityMismatch)?;
        if mount.r#type != "bind"
            || mount.writable
            || mount.source != format!("/opt/hton/work/runtime/services/{service}")
            || facts
                .mounts
                .iter()
                .any(|mount| mount.destination != "/etc/hton" && mount.r#type != "tmpfs")
        {
            return Err(Error::IdentityMismatch);
        }
        let image_facts =
            DockerObserver::new(self.runner.clone()).inspect_image(image, budget, cancel)?;
        if image_facts.id != facts.image {
            return Err(Error::IdentityMismatch);
        }
        budget.check(cancel)?;
        Ok(Some(facts.id))
    }
}

impl DockerControl {
    /// Pinned native parser only, no network/public binding or candidate activation.
    pub fn validate_bundle(
        &self,
        instance: &str,
        bundle: &str,
        service: &str,
        image: &ImageReference,
        engine: OwnedEngine,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        if !logical(instance) || !logical(bundle) || !logical(service) {
            return Err(Error::InvalidInput);
        }
        DockerObserver::new(self.runner.clone()).inspect_image(image, budget, cancel)?;
        let name = format!(
            "hton-native-{}",
            &hash_native(format!("{instance}/{bundle}/{service}").as_bytes())[7..39]
        );
        let mount = format!(
            "type=bind,src=/opt/hton/work/bundles/{bundle}/{service},dst=/etc/hton/current,readonly"
        );
        let (entry, command): (&str, &[&str]) = match engine {
            OwnedEngine::Haproxy => ("haproxy", &["-c", "-f", "/etc/hton/current/haproxy.cfg"]),
            OwnedEngine::Caddy => (
                "caddy",
                &[
                    "validate",
                    "--config",
                    "/etc/hton/current/Caddyfile",
                    "--adapter",
                    "caddyfile",
                ],
            ),
            OwnedEngine::Xray => (
                "xray",
                &["run", "-test", "-config", "/etc/hton/current/xray.json"],
            ),
            OwnedEngine::Unbound => (
                "/usr/local/unbound/unbound.d/sbin/unbound-checkconf",
                &["/etc/hton/current/unbound.conf"],
            ),
        };
        let mut args = vec![
            "container",
            "create",
            "--pull",
            "never",
            "--name",
            &name,
            "--restart",
            "no",
            "--network",
            "none",
            "--user",
            "0:0",
            "--read-only",
            "--cap-drop",
            "ALL",
            "--security-opt",
            "no-new-privileges:true",
            "--memory",
            "128m",
            "--cpus",
            "1.0",
            "--pids-limit",
            "64",
            "--tmpfs",
            "/tmp:rw,noexec,nosuid,size=16m",
            "--tmpfs",
            "/run:rw,noexec,nosuid,size=4m",
            "--tmpfs",
            "/data:rw,noexec,nosuid,size=16m",
            "--tmpfs",
            "/config:rw,noexec,nosuid,size=4m",
            "--mount",
            &mount,
            "--log-driver",
            "none",
            "--entrypoint",
            entry,
            image.as_str(),
        ];
        args.extend_from_slice(command);
        let bytes = self.command(&args, budget, cancel)?;
        let id = ContainerId::new(
            std::str::from_utf8(&bytes)
                .map_err(|_| Error::InvalidResponse)?
                .trim(),
        )?;
        let attempt = (|| {
            self.command(
                &["container", "start", "--attach", id.as_str()],
                budget,
                cancel,
            )?;
            let facts =
                DockerObserver::new(self.runner.clone()).inspect_container(&id, budget, cancel)?;
            if facts.state.running || facts.state.exit_code != 0 {
                return Err(Error::CommandFailed);
            }
            Ok(())
        })();
        // Cleanup gets its own narrow bound even after the requested parser deadline;
        // exact known-owned temporary resource only, never a production/name lookup.
        let cleanup_budget = Budget::new(
            std::time::Instant::now() + std::time::Duration::from_secs(5),
            65536,
        )?;
        let cleanup_cancel = std::sync::atomic::AtomicBool::new(false);
        let cleanup = self.command(
            &["container", "rm", "--force", id.as_str()],
            cleanup_budget,
            &cleanup_cancel,
        );
        match (attempt, cleanup) {
            (Ok(()), Ok(_)) => Ok(()),
            (Err(error), _) => Err(error),
            (_, Err(error)) => Err(error),
        }
    }
    pub fn recreate_stopped(
        &self,
        instance: &str,
        bundle: &str,
        service: &str,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ContainerId> {
        if !logical(instance) || !logical(bundle) || !logical(service) {
            return Err(Error::InvalidInput);
        }
        let project = format!("hton-{instance}");
        let compose = format!("/opt/hton/work/runtime/execution/{bundle}/compose.json");
        self.command(
            &[
                "compose",
                "--project-name",
                &project,
                "--file",
                &compose,
                "up",
                "--no-start",
                "--no-deps",
                "--no-build",
                "--pull",
                "never",
                "--force-recreate",
                service,
            ],
            budget,
            cancel,
        )?;
        let bytes = self.command(
            &[
                "compose",
                "--project-name",
                &project,
                "--file",
                &compose,
                "ps",
                "--all",
                "--quiet",
                service,
            ],
            budget,
            cancel,
        )?;
        let id = ContainerId::new(
            std::str::from_utf8(&bytes)
                .map_err(|_| Error::InvalidResponse)?
                .trim(),
        )?;
        let facts =
            DockerObserver::new(self.runner.clone()).inspect_container(&id, budget, cancel)?;
        if facts.state.running {
            return Err(Error::IdentityMismatch);
        }
        Ok(id)
    }
    /// Owned uninstall only; caller must independently verify ownership and
    /// current runtime epoch. Never force removal or delete attached volumes.
    pub fn remove_stopped_owned_exact(
        &self,
        id: &ContainerId,
        expected_structural: &str,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        if !digest(expected_structural) {
            return Err(Error::InvalidInput);
        }
        let facts =
            DockerObserver::new(self.runner.clone()).inspect_container(id, budget, cancel)?;
        if facts.state.running
            || facts.state.paused
            || facts.state.restarting
            || facts.state.dead
            || facts.host_config.restart_policy.name != "no"
            || container_structural_fingerprint(&facts) != expected_structural
        {
            return Err(Error::IdentityMismatch);
        }
        budget.check(cancel)?;
        self.command(&["container", "rm", id.as_str()], budget, cancel)
            .map(|_| ())
    }
    pub fn remove_stopped_owned(
        &self,
        id: &ContainerId,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let facts =
            DockerObserver::new(self.runner.clone()).inspect_container(id, budget, cancel)?;
        if facts.state.running || facts.host_config.restart_policy.name != "no" {
            return Err(Error::IdentityMismatch);
        }
        self.command(&["container", "rm", id.as_str()], budget, cancel)
            .map(|_| ())
    }
}
