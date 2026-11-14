//! Fail-closed local foreign observer binding.
//!
//! The host Docker API exposes stable candidate inventories and a fixed protected
//! Amnezia Xray config reader. Those facts cover only a subset of qualification;
//! keep missing version, listener, ownership and snapshot evidence typed instead
//! of treating an image, config file or published port as proof.
use crate::{
    foreign_adapter::{ForeignObservePort, HostBinding, TransportProtocol},
    journal::{OperationControl, failure},
};
use hton_contracts::{config::*, operations::*};
use std::{
    collections::BTreeSet,
    net::Ipv4Addr,
    sync::Arc,
    time::{Duration, Instant},
};

const MAX_AMNEZIA_XRAY_CONFIG_BYTES: usize = 1024 * 1024;

/// Nonsensitive subset parsed from the pinned native Xray config. This is not
/// qualification evidence: target reachability, installed version, live socket,
/// restart owners and remap snapshot are owned by separate observers.
#[derive(Clone, PartialEq, Eq)]
struct NativeXrayConfigFacts {
    protocol: String,
    listener: Ipv4Addr,
    port: u16,
    network: String,
    server_names: BTreeSet<String>,
    target_host: String,
    target_port: u16,
    has_private_key: bool,
}

impl std::fmt::Debug for NativeXrayConfigFacts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeXrayConfigFacts([protected routing metadata])")
    }
}

impl NativeXrayConfigFacts {
    fn has_supported_reality_shape(&self) -> bool {
        matches!(self.protocol.as_str(), "vless" | "vmess")
            && !self.listener.is_loopback()
            && !self.listener.is_multicast()
            && !self.listener.is_broadcast()
            && self.port != 0
            && self.network == "tcp"
            && !self.server_names.is_empty()
            && valid_dns_name(&self.target_host)
            && self.target_port != 0
            && self.has_private_key
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeXrayConfigError {
    Empty,
    Oversized,
    InvalidJson,
    UnsupportedShape,
    InvalidListener,
    InvalidRealityTarget,
}

/// Parse only the one supported server shape and copy out a bounded allowlist.
/// The JSON tree may contain private keys and client IDs; it is never formatted,
/// serialized, returned, or included in an error.
fn parse_native_xray_config(
    bytes: &[u8],
) -> std::result::Result<NativeXrayConfigFacts, NativeXrayConfigError> {
    if bytes.is_empty() {
        return Err(NativeXrayConfigError::Empty);
    }
    if bytes.len() > MAX_AMNEZIA_XRAY_CONFIG_BYTES {
        return Err(NativeXrayConfigError::Oversized);
    }
    let root: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| NativeXrayConfigError::InvalidJson)?;
    let inbounds = root
        .as_object()
        .and_then(|value| value.get("inbounds"))
        .and_then(serde_json::Value::as_array)
        .filter(|values| values.len() == 1)
        .ok_or(NativeXrayConfigError::UnsupportedShape)?;
    let inbound = inbounds[0]
        .as_object()
        .ok_or(NativeXrayConfigError::UnsupportedShape)?;
    let protocol = bounded_string(inbound.get("protocol"), 16)
        .filter(|value| matches!(*value, "vless" | "vmess"))
        .ok_or(NativeXrayConfigError::UnsupportedShape)?;
    let port = inbound
        .get("port")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value != 0)
        .ok_or(NativeXrayConfigError::InvalidListener)?;
    let listener = inbound
        .get("listen")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| value.parse::<Ipv4Addr>().ok())
        .ok_or(NativeXrayConfigError::InvalidListener)?;
    let stream = inbound
        .get("streamSettings")
        .and_then(serde_json::Value::as_object)
        .ok_or(NativeXrayConfigError::UnsupportedShape)?;
    let network = bounded_string(stream.get("network"), 16)
        .filter(|value| *value == "tcp")
        .ok_or(NativeXrayConfigError::UnsupportedShape)?;
    if bounded_string(stream.get("security"), 16) != Some("reality") {
        return Err(NativeXrayConfigError::UnsupportedShape);
    }
    let reality = stream
        .get("realitySettings")
        .and_then(serde_json::Value::as_object)
        .ok_or(NativeXrayConfigError::UnsupportedShape)?;
    let raw_names = reality
        .get("serverNames")
        .and_then(serde_json::Value::as_array)
        .filter(|names| !names.is_empty() && names.len() <= 64)
        .ok_or(NativeXrayConfigError::UnsupportedShape)?;
    let mut server_names = BTreeSet::new();
    for value in raw_names {
        let name = value
            .as_str()
            .filter(|value| valid_dns_name(value))
            .ok_or(NativeXrayConfigError::UnsupportedShape)?;
        if !server_names.insert(name.to_owned()) {
            return Err(NativeXrayConfigError::UnsupportedShape);
        }
    }
    let destination = bounded_string(reality.get("dest"), 320)
        .ok_or(NativeXrayConfigError::InvalidRealityTarget)?;
    let (target_host, target_port) = destination
        .rsplit_once(':')
        .and_then(|(host, port)| Some((host, port.parse::<u16>().ok()?)))
        .filter(|(host, port)| valid_dns_name(host) && *port != 0)
        .ok_or(NativeXrayConfigError::InvalidRealityTarget)?;
    let has_private_key = reality
        .get("privateKey")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty() && value.len() <= 512);
    if !has_private_key {
        return Err(NativeXrayConfigError::UnsupportedShape);
    }
    Ok(NativeXrayConfigFacts {
        protocol: protocol.to_owned(),
        listener,
        port,
        network: network.to_owned(),
        server_names,
        target_host: target_host.to_owned(),
        target_port,
        has_private_key,
    })
}

fn bounded_string(value: Option<&serde_json::Value>, max: usize) -> Option<&str> {
    value
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= max)
}

fn valid_dns_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

struct Cancel<'a> {
    control: &'a dyn OperationControl,
    context: &'a OperationContext,
}
impl hton_host::Cancellation for Cancel<'_> {
    fn cancelled(&self) -> bool {
        self.control.is_cancelled(&self.context.cancellation)
    }
}

/// Local read-only Docker observation boundary. Native config facts are parsed
/// for diagnosis but do not alone establish qualification or mutation authority.
#[derive(Clone)]
pub struct LocalForeignObserver {
    host: hton_host::HostObserver,
    control: Arc<dyn OperationControl>,
}

impl LocalForeignObserver {
    pub fn new(control: Arc<dyn OperationControl>) -> Self {
        Self {
            host: hton_host::HostObserver::new(),
            control,
        }
    }

    fn docker_budget(&self, budget: &Budget, ctx: &OperationContext) -> Result<hton_host::Budget> {
        if self.control.is_cancelled(&ctx.cancellation) {
            return Err(failure(
                ctx,
                ErrorCode::OperationCancelled,
                "foreign Docker observation was cancelled",
            ));
        }
        let remaining_ms = ctx
            .deadline
            .0
            .min(budget.deadline.0)
            .saturating_sub(self.control.now_ms())
            .min(10_000);
        if remaining_ms == 0 || budget.max_bytes == 0 || budget.max_observations == 0 {
            return Err(failure(
                ctx,
                ErrorCode::OperationDeadline,
                "foreign Docker observation budget expired or is empty",
            ));
        }
        hton_host::Budget::new(
            Instant::now() + Duration::from_millis(remaining_ms),
            budget.max_bytes.min(4 * 1024 * 1024) as usize,
        )
        .map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationDeadline,
                "foreign Docker observation budget is invalid",
            )
        })
    }
}

impl ForeignObservePort for LocalForeignObserver {
    fn discover_candidates(
        &self,
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ForeignCandidateInventory> {
        let native_budget = self.docker_budget(budget, ctx)?;
        let cancellation = Cancel {
            control: self.control.as_ref(),
            context: ctx,
        };
        let facts = self
            .host
            .docker()
            .foreign_candidate_facts(
                native_budget,
                &cancellation,
                budget.max_observations.min(256) as usize,
            )
            .map_err(|error| {
                let (code, message) = match error {
                    hton_host::Error::Deadline => (
                        ErrorCode::OperationDeadline,
                        "foreign Docker inventory deadline expired",
                    ),
                    hton_host::Error::Cancelled => (
                        ErrorCode::OperationCancelled,
                        "foreign Docker inventory was cancelled",
                    ),
                    hton_host::Error::Busy => (
                        ErrorCode::OperationConflict,
                        "Docker inventory changed during observation",
                    ),
                    hton_host::Error::OutputLimit => (
                        ErrorCode::ModuleIncomplete,
                        "Docker inventory exceeded its observation bound",
                    ),
                    _ => (
                        ErrorCode::IoUnavailable,
                        "complete Docker candidate inventory is unavailable",
                    ),
                };
                failure(ctx, code, message)
            })?;
        let observed_at = UnixMillis(self.control.now_ms());
        let valid_until = UnixMillis(observed_at.0.saturating_add(30_000));
        let mut candidates = Vec::with_capacity(facts.len());
        for fact in facts {
            let container_id = DockerContainerId::new(fact.container_id).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "Docker returned an invalid immutable container ID",
                )
            })?;
            let image_digest = ImageDigest::new(fact.image_digest).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "Docker returned an invalid image digest",
                )
            })?;
            let structural_fingerprint =
                Sha256Digest::new(fact.structural_fingerprint).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "Docker returned an invalid structural fingerprint",
                    )
                })?;
            let state = if fact.running {
                ObservedState::Running
            } else if matches!(fact.status.as_str(), "created" | "exited") {
                ObservedState::Stopped
            } else {
                ObservedState::Unknown
            };
            let published_bindings = fact
                .published_bindings
                .into_iter()
                .map(|binding| HostBinding {
                    protocol: match binding.protocol {
                        hton_host::docker::TransportProtocol::Tcp => TransportProtocol::Tcp,
                        hton_host::docker::TransportProtocol::Udp => TransportProtocol::Udp,
                    },
                    container_port: binding.container_port,
                    host: binding.host_ip,
                    port: binding.host_port,
                })
                .collect();
            candidates.push(ForeignCandidateSummary {
                container_id,
                image_digest,
                state,
                published_bindings,
                structural_fingerprint,
                observed_at,
            });
        }
        Ok(ForeignCandidateInventory {
            observed_at,
            valid_until,
            complete: true,
            candidates,
        })
    }

    fn inspect_candidate(
        &self,
        selected_id: &DockerContainerId,
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ForeignEvidence> {
        let native_budget = self.docker_budget(budget, ctx)?;
        let cancellation = Cancel {
            control: self.control.as_ref(),
            context: ctx,
        };
        let max_containers = budget.max_observations.min(256) as usize;
        let candidates = self
            .host
            .docker()
            .foreign_candidate_facts(native_budget, &cancellation, max_containers)
            .map_err(|error| map_observation_error(error, ctx))?;
        let exact = candidates
            .iter()
            .find(|candidate| candidate.container_id == selected_id.as_str())
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "selected Docker ID is absent from the fresh exact inventory",
                )
            })?;
        let container_id =
            hton_host::docker::ContainerId::new(selected_id.as_str()).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "selected Docker ID is invalid",
                )
            })?;
        let protected = self
            .host
            .docker()
            .read_amnezia_xray_server_config(
                &container_id,
                &exact.structural_fingerprint,
                native_budget,
                &cancellation,
            )
            .map_err(|error| match error {
                hton_host::Error::Deadline => failure(
                    ctx,
                    ErrorCode::OperationDeadline,
                    "native Xray config observation deadline expired",
                ),
                hton_host::Error::Cancelled => failure(
                    ctx,
                    ErrorCode::OperationCancelled,
                    "native Xray config observation was cancelled",
                ),
                hton_host::Error::IdentityMismatch | hton_host::Error::Busy => failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "selected Docker resource or Xray config changed during observation",
                ),
                _ => failure(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "fixed native Xray config is unavailable or outside the supported file shape",
                ),
            })?;
        let parsed = parse_native_xray_config(protected.bytes()).map_err(|_| {
            failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "native Xray config does not match the supported bounded REALITY shape",
            )
        })?;
        if !parsed.has_supported_reality_shape() {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "native Xray config does not match the supported bounded REALITY shape",
            ));
        }
        if !exact.running {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "selected Amnezia container is not running; a published mapping cannot prove a live native listener",
            ));
        }
        let tcp_host_ports = exact
            .published_bindings
            .iter()
            .filter(|binding| {
                binding.protocol == hton_host::docker::TransportProtocol::Tcp
                    && binding.container_port == parsed.port
            })
            .map(|binding| binding.host_port)
            .collect::<BTreeSet<_>>();
        if tcp_host_ports.len() != 1 {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "native REALITY TCP listener does not map to exactly one published host port",
            ));
        }
        let namespace = self
            .host
            .docker()
            .observe_container_network_listeners(
                &container_id,
                &exact.structural_fingerprint,
                native_budget,
                &cancellation,
            )
            .map_err(|error| match error {
                hton_host::Error::Deadline => failure(
                    ctx,
                    ErrorCode::OperationDeadline,
                    "container network listener observation deadline expired",
                ),
                hton_host::Error::Cancelled => failure(
                    ctx,
                    ErrorCode::OperationCancelled,
                    "container network listener observation was cancelled",
                ),
                hton_host::Error::IdentityMismatch | hton_host::Error::Busy => failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "selected Amnezia container changed during network listener observation",
                ),
                _ => failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "container network namespace listener evidence is unavailable",
                ),
            })?;
        if namespace.container_id() != &container_id
            || namespace.structural_fingerprint() != exact.structural_fingerprint.as_str()
        {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "container namespace listener evidence differs from the selected immutable resource",
            ));
        }
        let native_port_present = namespace.sockets().iter().any(|socket| {
            socket.protocol == hton_host::docker::TransportProtocol::Tcp
                && socket.port == parsed.port
                && socket.address == std::net::IpAddr::V4(parsed.listener)
        });
        if !native_port_present {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "native REALITY config port has no matching TCP listener in the container network namespace",
            ));
        }
        let process = self
            .host
            .docker()
            .observe_amnezia_xray_listener(
                &container_id,
                &exact.structural_fingerprint,
                parsed.listener,
                parsed.port,
                native_budget,
                &cancellation,
            )
            .map_err(|error| match error {
                hton_host::Error::Deadline => failure(
                    ctx,
                    ErrorCode::OperationDeadline,
                    "native Xray process attribution deadline expired",
                ),
                hton_host::Error::Cancelled => failure(
                    ctx,
                    ErrorCode::OperationCancelled,
                    "native Xray process attribution was cancelled",
                ),
                hton_host::Error::Busy | hton_host::Error::IdentityMismatch => failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "native Xray process or listener identity is not stable or supported",
                ),
                _ => failure(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "exact native Xray executable/config/socket attribution is unavailable",
                ),
            })?;
        if process.container_id() != &container_id
            || process.structural_fingerprint() != exact.structural_fingerprint.as_str()
        {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "native Xray process evidence differs from selected Docker identity",
            ));
        }
        let version = self
            .host
            .docker()
            .observe_amnezia_xray_version(
                &container_id,
                &exact.structural_fingerprint,
                parsed.listener,
                parsed.port,
                process.executable_digest(),
                native_budget,
                &cancellation,
            )
            .map_err(|error| match error {
                hton_host::Error::Deadline => failure(
                    ctx,
                    ErrorCode::OperationDeadline,
                    "native Xray version observation deadline expired",
                ),
                hton_host::Error::Cancelled => failure(
                    ctx,
                    ErrorCode::OperationCancelled,
                    "native Xray version observation was cancelled",
                ),
                hton_host::Error::Busy | hton_host::Error::IdentityMismatch => failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "native Xray identity changed during version observation",
                ),
                _ => failure(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "fixed native Xray version observation is unavailable or unsupported",
                ),
            })?;
        if version.container_id() != &container_id
            || version.structural_fingerprint() != exact.structural_fingerprint.as_str()
            || version.executable_digest() != process.executable_digest()
        {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "native Xray version evidence differs from selected executable identity",
            ));
        }
        let snapshot_capability = self
            .host
            .docker()
            .observe_no_mount_snapshot_capability(
                &container_id,
                &exact.structural_fingerprint,
                native_budget,
                &cancellation,
            )
            .map_err(|error| map_observation_error(error, ctx))?;
        if snapshot_capability.container_id() != &container_id
            || snapshot_capability.structural_fingerprint() != exact.structural_fingerprint.as_str()
        {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "snapshot capability differs from exact native resource identity",
            ));
        }
        // Qualification observes supported capture capability, never creates a
        // snapshot. Actual custody requires a later consented journaled action.
        Err(failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "native config, binary-reported Xray version, executable identity and process-attributed TCP listener were observed, and no-mount protected snapshot capability applies, but external restart owners, resolved edge-loop exclusion and exact recreation compatibility remain unverified",
        ))
    }

    fn observe_registered(
        &self,
        _module: &ModuleId,
        _exact_resources: &[ResourceIdentity],
        _budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ForeignEvidence> {
        Err(failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "native foreign-resource observation facts are unavailable",
        ))
    }
}

fn map_observation_error(error: hton_host::Error, ctx: &OperationContext) -> Error {
    let (code, message) = match error {
        hton_host::Error::Deadline => (
            ErrorCode::OperationDeadline,
            "foreign Docker observation deadline expired",
        ),
        hton_host::Error::Cancelled => (
            ErrorCode::OperationCancelled,
            "foreign Docker observation was cancelled",
        ),
        hton_host::Error::Busy => (
            ErrorCode::OperationConflict,
            "Docker inventory changed during observation",
        ),
        hton_host::Error::OutputLimit => (
            ErrorCode::ModuleIncomplete,
            "Docker inventory exceeded its observation bound",
        ),
        _ => (
            ErrorCode::IoUnavailable,
            "complete Docker candidate inventory is unavailable",
        ),
    };
    failure(ctx, code, message)
}
