//! Pure native compilation and journal-bound immutable staging. No activation authority.
use crate::{
    catalog::ModuleCatalogue,
    config::{LoadLimits, canonical_secret_id, content_revision, decode_yaml, digest},
    journal::{Intent, JournalStore, ensure_mutating, failure},
    secrets::{ProtectedMaterial, SecretRef},
};
use hton_contracts::{config::*, operations::*};
use hton_runtime_contract as wire;
use hton_storage::SecureDir;
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    net::{Ipv4Addr, SocketAddrV4},
    num::{NonZeroU16, NonZeroU32},
    sync::Arc,
    time::{Duration, Instant},
};

pub struct ScopedSecret<'a> {
    pub reference: SecretRef,
    pub service_id: ServiceId,
    pub material: &'a ProtectedMaterial,
}
impl fmt::Debug for ScopedSecret<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScopedSecret")
            .field("reference", &self.reference)
            .field("service_id", &self.service_id)
            .field("material", &"[protected]")
            .finish()
    }
}
#[derive(Clone, Debug)]
pub struct CredentialBinding {
    pub module_id: ModuleId,
    pub identity_id: IdentityId,
    pub reference: SecretRef,
}
/// Constructed by certificate owner after SAN/chain/key/expiry qualification.
pub struct QualifiedCertificate {
    pub service_id: ServiceId,
    pub chain: SecretRef,
    pub key: SecretRef,
    pub names: BTreeSet<String>,
    pub valid_until: UnixMillis,
    pub qualification: Sha256Digest,
}
impl QualifiedCertificate {
    pub fn covers(&self, hostname: &str) -> bool {
        let Ok(requested) = crate::certificates::CertificateName::new(hostname) else {
            return false;
        };
        self.names
            .iter()
            .filter_map(|name| crate::certificates::CertificateName::new(name).ok())
            .any(|name| name.covers(&requested))
    }
}
impl fmt::Debug for QualifiedCertificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QualifiedCertificate")
            .field("service_id", &self.service_id)
            .field("names", &self.names)
            .field("valid_until", &self.valid_until)
            .finish_non_exhaustive()
    }
}
pub struct PreparedTrustAnchor<'a> {
    pub bytes: &'a [u8],
    pub digest: Sha256Digest,
}
impl fmt::Debug for PreparedTrustAnchor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PreparedTrustAnchor([protected input])")
    }
}
#[derive(Clone, Debug)]
pub struct ForeignBackend {
    pub endpoint: SocketAddrV4,
    pub resource_id: ResourceId,
    pub server_names: BTreeSet<String>,
    pub native_target: SocketAddrV4,
    pub qualification: Sha256Digest,
}
#[derive(Clone, Debug)]
pub struct NetworkLayout {
    pub network_id: ObjectId,
    pub subnet: Ipv4Addr,
    pub prefix: u8,
    pub gateway: Ipv4Addr,
    pub endpoints: BTreeMap<ServiceId, SocketAddrV4>,
    pub foreign_backends: BTreeMap<ServiceId, ForeignBackend>,
    pub public_addresses: BTreeSet<Ipv4Addr>,
}
#[derive(Clone, Debug)]
pub struct RuntimeServiceBinding {
    pub structural_fingerprint: Sha256Digest,
    pub resource: wire::ResourceIdentity,
    pub image: wire::ImagePin,
    pub restart_owner_resources: Vec<wire::ResourceId>,
}
/// Trusted composition supplies capabilities and observations, never frontend JSON.
pub struct RenderInputs<'a> {
    pub snapshot: &'a NormalizedSnapshot,
    pub graph: &'a DependencyGraph,
    pub catalogue: &'a ModuleCatalogue,
    pub registry: &'a RegistrySnapshot,
    pub services: &'a BTreeMap<ServiceId, RuntimeServiceBinding>,
    pub network: &'a NetworkLayout,
    pub secrets: &'a [ScopedSecret<'a>],
    pub credentials: &'a [CredentialBinding],
    pub certificate: Option<&'a QualifiedCertificate>,
    pub dns_anchor: Option<&'a PreparedTrustAnchor<'a>>,
    /// Exact bounded index bytes observed by Core's webroot owner.
    /// Renderers must not log or expose this caller-owned website content.
    pub site_body: &'a [u8],
    pub site_index_digest: Sha256Digest,
    pub now: UnixMillis,
    pub operation_id: OperationId,
}
pub struct EngineContext<'a, 'b> {
    pub inputs: &'a RenderInputs<'b>,
    pub service_id: &'a ServiceId,
    pub consumers: &'a BTreeSet<ModuleId>,
}
impl EngineContext<'_, '_> {
    pub fn error(&self, code: ErrorCode, message: &'static str) -> Error {
        error(self.inputs, code, message)
    }
    pub fn endpoint(&self, id: &str) -> Result<SocketAddrV4> {
        self.inputs
            .network
            .endpoints
            .get(&service(id))
            .copied()
            .ok_or_else(|| {
                self.error(
                    ErrorCode::ModuleIncomplete,
                    "Engine has no declared private endpoint",
                )
            })
    }
    pub fn secret(&self, reference: &SecretRef) -> Result<&[u8]> {
        let value = self
            .inputs
            .secrets
            .iter()
            .find(|secret| secret.reference == *reference && secret.service_id == *self.service_id)
            .ok_or_else(|| {
                self.error(
                    ErrorCode::OperationDenied,
                    "Engine secret consumer grant is unavailable",
                )
            })?;
        if self
            .inputs
            .snapshot
            .desired
            .read_set
            .secret_versions
            .get(&reference.secret_id)
            != Some(&reference.version)
        {
            return Err(self.error(
                ErrorCode::PlanStale,
                "Engine secret version is not in the coherent read set",
            ));
        }
        Ok(value.material.expose())
    }
    pub fn vless_path(&self) -> Result<(String, SecretRef)> {
        let settings = self
            .inputs
            .snapshot
            .desired
            .module_settings
            .get(&module("vless"))
            .ok_or_else(|| self.error(ErrorCode::ModuleIncomplete, "VLESS settings are missing"))?;
        let id = settings
            .settings
            .get("path")
            .and_then(|value| value.get("secretRef"))
            .and_then(Value::as_str)
            .and_then(canonical_secret_id)
            .ok_or_else(|| {
                self.error(
                    ErrorCode::ConfigSemanticConflict,
                    "VLESS path secret reference is invalid",
                )
            })?;
        let version = self
            .inputs
            .snapshot
            .desired
            .read_set
            .secret_versions
            .get(&id)
            .cloned()
            .ok_or_else(|| {
                self.error(
                    ErrorCode::ModuleIncomplete,
                    "VLESS path secret version is unavailable",
                )
            })?;
        let reference = SecretRef {
            secret_id: id,
            version,
        };
        let token = std::str::from_utf8(self.secret(&reference)?).map_err(|_| {
            self.error(
                ErrorCode::ConfigSemanticConflict,
                "VLESS path material has invalid encoding",
            )
        })?;
        let token = token.strip_prefix('/').unwrap_or(token);
        if token.len() < 16
            || token.len() > 128
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(self.error(
                ErrorCode::ConfigSemanticConflict,
                "VLESS path material must be an opaque URL-safe token",
            ));
        }
        let path = format!("/{token}");
        let routes: Vec<_> = self
            .inputs
            .graph
            .route_claims
            .iter()
            .filter(|claim| claim.owner.as_str() == "vless")
            .collect();
        if routes.is_empty()
            || routes.iter().any(|claim| {
                !matches!(&claim.matcher,RouteMatch::Http {path_prefix,..} if path_prefix==&path)
                    || claim.listener_service.as_str() != "caddy"
                    || claim.target_service.as_str() != "xray"
            })
        {
            return Err(self.error(
                ErrorCode::PlanStale,
                "Protected VLESS path differs from the resolved approved route claims",
            ));
        }
        Ok((path, reference))
    }
    pub fn artifact(
        &self,
        name: &str,
        bytes: Vec<u8>,
        mut secrets: Vec<SecretRef>,
    ) -> Result<RenderedArtifact> {
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(self.error(
                ErrorCode::ConfigSemanticConflict,
                "Engine artifact exceeds its compiler budget",
            ));
        }
        let name = ObjectId::new(name).map_err(|_| {
            self.error(
                ErrorCode::ConfigSemanticConflict,
                "Engine artifact name is unsafe",
            )
        })?;
        let id = ArtifactId::new(format!("{}.{}", self.service_id, name)).map_err(|_| {
            self.error(
                ErrorCode::ConfigSemanticConflict,
                "Engine artifact identity is too long",
            )
        })?;
        secrets.sort_by(|left, right| {
            (&left.secret_id, &left.version).cmp(&(&right.secret_id, &right.version))
        });
        secrets.dedup_by(|left, right| left == right);
        Ok(RenderedArtifact {
            artifact_id: id,
            service_id: self.service_id.clone(),
            name,
            bytes,
            secrets,
        })
    }
}
pub trait EngineRenderer {
    fn service_id(&self) -> ServiceId;
    fn engine(&self) -> wire::Engine;
    fn render(&self, context: &EngineContext<'_, '_>) -> Result<Vec<RenderedArtifact>>;
}
/// Bytes are deliberately not a wire/log type.
pub struct RenderedArtifact {
    pub artifact_id: ArtifactId,
    pub service_id: ServiceId,
    pub name: ObjectId,
    bytes: Vec<u8>,
    pub secrets: Vec<SecretRef>,
}
impl RenderedArtifact {
    pub fn expose(&self) -> &[u8] {
        &self.bytes
    }
    pub fn digest(&self) -> Sha256Digest {
        digest(&self.bytes)
    }
}
impl fmt::Debug for RenderedArtifact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderedArtifact")
            .field("artifact_id", &self.artifact_id)
            .field("service_id", &self.service_id)
            .field("bytes", &"[protected]")
            .finish()
    }
}
pub struct RenderedBundle {
    pub artifacts: Vec<RenderedArtifact>,
    pub services: Vec<wire::Service>,
    pub claims: Vec<wire::Claim>,
    pub secret_bindings: Vec<wire::SecretBinding>,
    pub service_artifact_digests: BTreeMap<ServiceId, Sha256Digest>,
}
impl fmt::Debug for RenderedBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderedBundle")
            .field("artifacts", &self.artifacts)
            .field("services", &self.services.len())
            .finish_non_exhaustive()
    }
}
fn module(value: &str) -> ModuleId {
    ModuleId::new(value).expect("trusted module ID")
}
fn service(value: &str) -> ServiceId {
    ServiceId::new(value).expect("trusted service ID")
}
fn object(value: &str) -> ObjectId {
    ObjectId::new(value).expect("trusted object ID")
}
fn error(inputs: &RenderInputs<'_>, code: ErrorCode, message: &'static str) -> Error {
    Error {
        code,
        message: message.into(),
        module_id: None,
        config_path: None,
        resource_id: None,
        operation_id: inputs.operation_id.clone(),
        retryability: Retryability::AfterRepair,
        repair_hint: None,
        retained_state: RetainedState::Unchanged,
    }
}
fn convert<T>(
    value: std::result::Result<T, wire::InvalidValue>,
    inputs: &RenderInputs<'_>,
) -> Result<T> {
    value.map_err(|_| {
        error(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Compiler output exceeds the runtime wire contract",
        )
    })
}
fn private(ip: Ipv4Addr) -> bool {
    ip.is_private() && !ip.is_loopback() && !ip.is_unspecified()
}
fn in_network(ip: Ipv4Addr, layout: &NetworkLayout) -> bool {
    let mask = u32::MAX
        .checked_shl((32 - layout.prefix) as u32)
        .unwrap_or(0);
    u32::from(ip) & mask == u32::from(layout.subnet) & mask
}
fn validate_inputs(inputs: &RenderInputs<'_>) -> Result<()> {
    let graph = inputs.graph;
    if crate::config::normalize(&inputs.snapshot.desired, inputs.catalogue).blocking {
        return Err(error(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Compiler received an invalid normalized configuration",
        ));
    }
    let expected_modules: BTreeSet<_> = inputs
        .snapshot
        .desired
        .features
        .iter()
        .filter(|(_, feature)| feature.enabled)
        .map(|(id, _)| id.clone())
        .chain(std::iter::once(module("core")))
        .collect();
    if expected_modules != graph.enabled_modules {
        return Err(error(
            inputs,
            ErrorCode::PlanStale,
            "Compiler graph does not match the normalized feature selection",
        ));
    }
    for (id, document) in &inputs.snapshot.desired.editable_docker {
        if !inputs.catalogue.services().contains_key(id)
            || content_revision(document.object_id.clone(), &document.source_bytes)
                != document.revision
            || inputs
                .snapshot
                .desired
                .read_set
                .objects
                .get(&document.object_id)
                .map(|dependency| &dependency.revision)
                != Some(&document.revision)
        {
            return Err(error(
                inputs,
                ErrorCode::PlanStale,
                "Docker customization has an unknown service or stale source byte identity",
            ));
        }
    }

    if !graph.conflicts.is_empty()
        || inputs.snapshot.desired.read_set.coherence != ReadCoherence::Coherent
    {
        return Err(error(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Rendering requires a coherent normalized conflict-free graph",
        ));
    }
    let layout = inputs.network;
    if !(16..=30).contains(&layout.prefix)
        || !private(layout.subnet)
        || !in_network(layout.gateway, layout)
        || !private(layout.gateway)
    {
        return Err(error(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Project network must be an explicit bounded private IPv4 subnet",
        ));
    }
    let mask = u32::MAX << (32 - layout.prefix);
    if u32::from(layout.subnet) & mask != u32::from(layout.subnet) {
        return Err(error(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Project subnet is not a canonical network address",
        ));
    }
    let mut addresses = BTreeSet::new();
    for (id, endpoint) in &layout.endpoints {
        if endpoint.port() == 0
            || !private(*endpoint.ip())
            || !in_network(*endpoint.ip(), layout)
            || *endpoint.ip() == layout.subnet
            || *endpoint.ip() == layout.gateway
            || !addresses.insert(*endpoint.ip())
            || !graph.service_consumers.contains_key(id)
        {
            return Err(error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Private service endpoints are invalid, duplicated or outside the selected network",
            ));
        }
    }
    let edge = layout.endpoints.get(&service("haproxy")).ok_or_else(|| {
        error(
            inputs,
            ErrorCode::ModuleIncomplete,
            "Edge has no private endpoint",
        )
    })?;
    for backend in layout.foreign_backends.values() {
        if backend.endpoint.port() == 0
            || backend.endpoint.ip().is_loopback()
            || backend.endpoint.ip().is_unspecified()
            || layout
                .endpoints
                .values()
                .any(|endpoint| endpoint == &backend.endpoint)
            || backend.native_target == *edge
            || layout.public_addresses.contains(backend.native_target.ip())
                && graph.listener_claims.iter().any(|claim| {
                    claim.service_id.as_str() == "haproxy"
                        && claim.port == backend.native_target.port()
                })
        {
            return Err(error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Foreign target is unroutable from project network or creates a known ingress loop",
            ));
        }
        if backend
            .server_names
            .iter()
            .any(|name| !crate::catalog::valid_hostname(name, false))
        {
            return Err(error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Foreign exact SNI names are invalid",
            ));
        }
    }
    let instance = inputs
        .snapshot
        .desired
        .platform
        .settings
        .get("instance")
        .and_then(|instance| instance.get("name"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Instance name is missing",
            )
        })?;
    if InstanceId::new(instance).is_err() || instance.len() > 64 {
        return Err(error(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Instance name must be a bounded logical identity",
        ));
    }
    if inputs.services.len() > 128 || inputs.credentials.len() > 1024 || inputs.secrets.len() > 2048
    {
        return Err(error(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Compiler input collection exceeds its bound",
        ));
    }
    Ok(())
}

pub fn preview(
    inputs: &RenderInputs<'_>,
    renderers: &BTreeMap<ServiceId, &dyn EngineRenderer>,
) -> Result<RenderedBundle> {
    validate_inputs(inputs)?;
    let mut artifacts = Vec::new();
    let mut services = Vec::new();
    let mut claims = Vec::new();
    let mut secret_bindings = Vec::new();
    let mut hashes = BTreeMap::new();
    let mut compose_services = Map::new();
    for (id, consumers) in &inputs.graph.service_consumers {
        if id.as_str() == "runtime" {
            continue;
        }
        let descriptor = inputs.catalogue.services().get(id).ok_or_else(|| {
            error(
                inputs,
                ErrorCode::ModuleUnknown,
                "Selected service is outside the trusted catalogue",
            )
        })?;
        let binding = inputs.services.get(id).ok_or_else(|| {
            error(
                inputs,
                ErrorCode::ModuleIncomplete,
                "Exact installed runtime service binding is missing",
            )
        })?;
        let entries: Vec<_> = inputs
            .registry
            .entries
            .values()
            .filter(|entry| entry.membership == MembershipState::Installed)
            .flat_map(|entry| {
                entry
                    .resources
                    .iter()
                    .map(move |resource| (entry, resource))
            })
            .filter(|(_, resource)| {
                resource.resource_id.as_str() == binding.resource.resource_id.as_str()
            })
            .collect();
        if entries.len() != 1
            || entries[0].1.service_id != *id
            || entries[0].1.fingerprint != binding.structural_fingerprint
            || (entries[0].0.ownership == Ownership::Associated)
                != (binding.resource.ownership == wire::Ownership::Associated)
            || descriptor.foreign != (binding.resource.ownership == wire::Ownership::Associated)
        {
            return Err(error(
                inputs,
                ErrorCode::ResourceDrift,
                "Runtime binding differs from exact registered ownership or identity",
            ));
        }
        let (engine, produced) = if descriptor.foreign {
            let backend = inputs.network.foreign_backends.get(id).ok_or_else(|| {
                error(
                    inputs,
                    ErrorCode::ModuleIncomplete,
                    "Foreign passthrough topology is not qualified",
                )
            })?;
            if backend.resource_id.as_str() != binding.resource.resource_id.as_str() {
                return Err(error(
                    inputs,
                    ErrorCode::ResourceDrift,
                    "Foreign endpoint belongs to another registered resource",
                ));
            }
            (wire::Engine::Amnezia, Vec::new())
        } else {
            let renderer = renderers.get(id).ok_or_else(|| {
                error(
                    inputs,
                    ErrorCode::ModuleUnsupported,
                    "Selected engine has no trusted native renderer",
                )
            })?;
            if renderer.service_id() != *id {
                return Err(error(
                    inputs,
                    ErrorCode::ConfigSemanticConflict,
                    "Native renderer identity differs from selected service",
                ));
            }
            let context = EngineContext {
                inputs,
                service_id: id,
                consumers,
            };
            (renderer.engine(), renderer.render(&context)?)
        };
        let mut ids = Vec::new();
        let mut metadata = Vec::new();
        for artifact in &produced {
            if artifact.service_id != *id {
                return Err(error(
                    inputs,
                    ErrorCode::OperationDenied,
                    "Engine attempted to write another service's artifact",
                ));
            }
            ids.push(convert(
                wire::ArtifactId::new(artifact.artifact_id.as_str()),
                inputs,
            )?);
            metadata.push((artifact.artifact_id.clone(), artifact.digest()));
            for reference in &artifact.secrets {
                if inputs
                    .snapshot
                    .desired
                    .read_set
                    .secret_versions
                    .get(&reference.secret_id)
                    != Some(&reference.version)
                    || !inputs
                        .secrets
                        .iter()
                        .any(|secret| secret.service_id == *id && secret.reference == *reference)
                {
                    return Err(error(
                        inputs,
                        ErrorCode::OperationDenied,
                        "Rendered secret is outside the exact granted version",
                    ));
                }
                secret_bindings.push(wire::SecretBinding {
                    secret_id: convert(wire::SecretId::new(reference.secret_id.as_str()), inputs)?,
                    version: convert(wire::SecretVersion::new(reference.version.as_str()), inputs)?,
                    service_id: convert(wire::ServiceId::new(id.as_str()), inputs)?,
                    artifact_id: convert(
                        wire::ArtifactId::new(artifact.artifact_id.as_str()),
                        inputs,
                    )?,
                });
            }
        }
        if !descriptor.foreign {
            if produced.is_empty() {
                return Err(error(
                    inputs,
                    ErrorCode::ModuleIncomplete,
                    "Native renderer produced no service configuration",
                ));
            }
            let spec = compose_service(inputs, id, engine, binding)?;
            let native = metadata
                .iter()
                .map(|(id, digest)| {
                    Ok((
                        convert(wire::ArtifactId::new(id.as_str()), inputs)?,
                        convert(wire::Sha256Digest::new(digest.as_str()), inputs)?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            let revision = wire::service_revision(&native, &spec).map_err(|_| {
                error(
                    inputs,
                    ErrorCode::ConfigSemanticConflict,
                    "native service revision cannot be encoded",
                )
            })?;
            hashes.insert(
                id.clone(),
                Sha256Digest::new(revision.as_str()).map_err(|_| {
                    error(
                        inputs,
                        ErrorCode::ConfigSemanticConflict,
                        "service revision cannot be encoded",
                    )
                })?,
            );
            compose_services.insert(id.as_str().into(), spec);
        }
        let owner = consumers.iter().next().ok_or_else(|| {
            error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Service has no declared consumer",
            )
        })?;
        let dependencies = descriptor
            .dependencies
            .iter()
            .filter(|id| id.as_str() != "runtime")
            .map(|id| convert(wire::ServiceId::new(id.as_str()), inputs))
            .collect::<Result<Vec<_>>>()?;
        services.push(wire::Service {
            service_id: convert(wire::ServiceId::new(id.as_str()), inputs)?,
            module_id: convert(wire::ModuleId::new(owner.as_str()), inputs)?,
            engine,
            image: binding.image.clone(),
            resource: binding.resource.clone(),
            dependencies: convert(wire::Items::new(dependencies), inputs)?,
            artifacts: convert(wire::Items::new(ids), inputs)?,
            restart_owner_resources: convert(
                wire::Items::new(binding.restart_owner_resources.clone()),
                inputs,
            )?,
        });
        artifacts.extend(produced);
    }
    // Global Compose carries metadata only; never secret material, and never grants mounts of another store.
    let instance = inputs.snapshot.desired.platform.settings["instance"]["name"]
        .as_str()
        .expect("validated instance");
    let compose = json!({"name":format!("hton-{instance}"),"services":compose_services,"networks":{"hton":{"name":format!("hton-{instance}"),"driver":"bridge","ipam":{"config":[{"subnet":format!("{}/{}",inputs.network.subnet,inputs.network.prefix),"gateway":inputs.network.gateway.to_string()}]}}}});
    let id = service("haproxy");
    let consumers = inputs.graph.service_consumers.get(&id).ok_or_else(|| {
        error(
            inputs,
            ErrorCode::ModuleIncomplete,
            "Mandatory edge consumer is absent",
        )
    })?;
    let context = EngineContext {
        inputs,
        service_id: &id,
        consumers,
    };
    let compose_artifact = context.artifact(
        "compose.json",
        serde_json::to_vec_pretty(&compose).map_err(|_| {
            error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Compose cannot be encoded",
            )
        })?,
        Vec::new(),
    )?;
    // Compose does not alter a service's native hash: its per-service spec already participates above.
    let edge_service = services
        .iter_mut()
        .find(|entry| entry.service_id.as_str() == "haproxy")
        .expect("edge service produced");
    let mut edge_artifacts = edge_service.artifacts.clone().into_vec();
    edge_artifacts.push(convert(
        wire::ArtifactId::new(compose_artifact.artifact_id.as_str()),
        inputs,
    )?);
    edge_service.artifacts = convert(wire::Items::new(edge_artifacts), inputs)?;
    artifacts.push(compose_artifact);
    for claim in &inputs.graph.listener_claims {
        if claim.namespace != ListenerNamespace::Host {
            continue;
        }
        let std::net::IpAddr::V4(address) = claim.address else {
            return Err(error(
                inputs,
                ErrorCode::ModuleUnsupported,
                "R1 public listener must be IPv4",
            ));
        };
        claims.push(wire::Claim::Listener {
            service: convert(wire::ServiceId::new(claim.service_id.as_str()), inputs)?,
            address,
            port: NonZeroU16::new(claim.port).ok_or_else(|| {
                error(
                    inputs,
                    ErrorCode::ConfigSemanticConflict,
                    "Public port cannot be zero",
                )
            })?,
            transport: match claim.transport {
                Transport::Tcp => wire::Transport::Tcp,
                Transport::Udp => wire::Transport::Udp,
            },
        });
    }
    for claim in &inputs.graph.route_claims {
        if let RouteMatch::Sni { hostname } = &claim.matcher {
            claims.push(wire::Claim::SniRoute {
                service: convert(wire::ServiceId::new(claim.target_service.as_str()), inputs)?,
                hostname: convert(wire::HostName::new(hostname), inputs)?,
            });
        }
    }
    if let Some(certificate) = inputs.certificate.filter(|_| {
        artifacts
            .iter()
            .any(|artifact| artifact.artifact_id.as_str() == "caddy.fullchain.pem")
    }) {
        for name in inputs
            .graph
            .certificate_claims
            .iter()
            .filter(|claim| claim.consumer.as_str() == "caddy" && !claim.foreign_owned)
            .flat_map(|claim| &claim.names)
        {
            claims.push(wire::Claim::Certificate {
                service: convert(
                    wire::ServiceId::new(certificate.service_id.as_str()),
                    inputs,
                )?,
                hostname: convert(wire::HostName::new(name), inputs)?,
                artifact: convert(wire::ArtifactId::new("caddy.fullchain.pem"), inputs)?,
                valid_until_ms: certificate.valid_until.0,
            });
        }
    }
    artifacts.sort_by(|a, b| a.artifact_id.cmp(&b.artifact_id));
    services.sort_by(|a, b| a.service_id.cmp(&b.service_id));
    if artifacts.len() > 256
        || artifacts
            .iter()
            .map(|artifact| artifact.bytes.len())
            .sum::<usize>()
            > 32 * 1024 * 1024
        || artifacts
            .windows(2)
            .any(|pair| pair[0].artifact_id == pair[1].artifact_id)
    {
        return Err(error(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Compiler artifact inventory is duplicate or exceeds bounds",
        ));
    }
    Ok(RenderedBundle {
        artifacts,
        services,
        claims,
        secret_bindings,
        service_artifact_digests: hashes,
    })
}

fn compose_service(
    inputs: &RenderInputs<'_>,
    id: &ServiceId,
    engine: wire::Engine,
    binding: &RuntimeServiceBinding,
) -> Result<Value> {
    let endpoint = inputs.network.endpoints.get(id).ok_or_else(|| {
        error(
            inputs,
            ErrorCode::ModuleIncomplete,
            "Container endpoint is absent",
        )
    })?;
    if endpoint.port() < 1024 {
        return Err(error(
            inputs,
            ErrorCode::OperationDenied,
            "Owned container listener requires an unprivileged private port",
        ));
    }
    let (entrypoint, arguments) = match engine {
        wire::Engine::Haproxy => (
            "haproxy",
            vec!["-W", "-db", "-f", "/etc/hton/current/haproxy.cfg"],
        ),
        wire::Engine::Caddy => (
            "caddy",
            vec![
                "run",
                "--config",
                "/etc/hton/current/Caddyfile",
                "--adapter",
                "caddyfile",
            ],
        ),
        wire::Engine::Xray => (
            "xray",
            vec!["run", "-config", "/etc/hton/current/xray.json"],
        ),
        wire::Engine::Unbound => (
            "/usr/local/unbound/unbound.d/sbin/unbound",
            vec!["-d", "-c", "/etc/hton/current/unbound.conf"],
        ),
        wire::Engine::Amnezia => {
            return Err(error(
                inputs,
                ErrorCode::OperationDenied,
                "Compiler cannot own an external Amnezia Compose service",
            ));
        }
    };
    let logging_root = inputs
        .snapshot
        .desired
        .platform
        .settings
        .get("logging")
        .and_then(|logging| logging.get("directory"))
        .and_then(Value::as_str)
        .unwrap_or("/var/logs/hton");
    if !logging_root.starts_with('/')
        || logging_root.len() > 256
        || logging_root
            .split('/')
            .skip(1)
            .any(|component| ObjectId::new(component).is_err())
    {
        return Err(error(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Configured logging root is not a safe installer-selected absolute path",
        ));
    }
    let instance = inputs.snapshot.desired.platform.settings["instance"]["name"]
        .as_str()
        .expect("validated instance");
    let mut value = json!({"image":binding.image.as_str(),"container_name":format!("hton-{instance}-{id}"),"entrypoint":[entrypoint],"command":arguments,"user":"0:0","read_only":true,"cap_drop":["ALL"],"security_opt":["no-new-privileges:true"],"restart":"no","init":true,"mem_limit":"128m","cpus":"1.0","pids_limit":64,"stop_grace_period":"30s","tmpfs":["/tmp:rw,noexec,nosuid,size=16m","/run:rw,noexec,nosuid,size=4m"],"volumes":[{"type":"bind","source":format!("/opt/hton/work/runtime/services/{id}"),"target":"/etc/hton","read_only":true}],"networks":{"hton":{"ipv4_address":endpoint.ip().to_string()}},"logging":{"driver":"none"},"labels":{"io.hton.instance":instance,"io.hton.service":id.as_str()}});
    let mut ports = Vec::new();
    for claim in inputs
        .graph
        .listener_claims
        .iter()
        .filter(|claim| claim.service_id == *id && claim.namespace == ListenerNamespace::Host)
    {
        let std::net::IpAddr::V4(address) = claim.address else {
            return Err(error(
                inputs,
                ErrorCode::ModuleUnsupported,
                "R1 host publishing requires IPv4",
            ));
        };
        ports.push(json!({"target":endpoint.port(),"published":claim.port.to_string(),"host_ip":address.to_string(),"protocol":match claim.transport {Transport::Tcp=>"tcp",Transport::Udp=>"udp"}}));
    }
    // Pinned Caddy image declares these volumes; keep them bounded and ephemeral.
    if id.as_str() == "caddy" {
        let tmpfs = value["tmpfs"]
            .as_array_mut()
            .expect("generated tmpfs array");
        tmpfs.push(json!("/data:rw,noexec,nosuid,size=16m"));
        tmpfs.push(json!("/config:rw,noexec,nosuid,size=4m"));
    }
    if !ports.is_empty() {
        value["ports"] = Value::Array(ports);
    }
    if let Some(document) = inputs.snapshot.desired.editable_docker.get(id) {
        let custom = decode_yaml(&document.source_bytes, LoadLimits::default()).map_err(|_| {
            error(
                inputs,
                ErrorCode::ConfigSyntax,
                "Editable Docker input is invalid or unsafe YAML",
            )
        })?;
        let root = custom.as_object().ok_or_else(|| {
            error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Docker customization must be a Compose services object",
            )
        })?;
        if root.len() != 1 || !root.contains_key("services") {
            return Err(error(
                inputs,
                ErrorCode::ModuleUnsupported,
                "Docker customization supports only the native services object",
            ));
        }
        let specs = root["services"].as_object().ok_or_else(|| {
            error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Compose services must be a mapping",
            )
        })?;
        if specs.len() != 1 || !specs.contains_key(id.as_str()) {
            return Err(error(
                inputs,
                ErrorCode::OperationDenied,
                "Docker customization may name only its registered service",
            ));
        }
        let fields = specs[id.as_str()].as_object().ok_or_else(|| {
            error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Compose service customization must be a mapping",
            )
        })?;
        for (key, setting) in fields {
            let accepted = match key.as_str() {
                "mem_limit" => setting
                    .as_str()
                    .and_then(|limit| limit.strip_suffix('m'))
                    .and_then(|limit| limit.parse::<u32>().ok())
                    .is_some_and(|limit| (32..=4096).contains(&limit)),
                "cpus" => setting
                    .as_str()
                    .and_then(|value| value.parse::<f64>().ok())
                    .or_else(|| setting.as_f64())
                    .is_some_and(|value| value.is_finite() && (0.05..=16.0).contains(&value)),
                "pids_limit" => setting
                    .as_u64()
                    .is_some_and(|value| (16..=512).contains(&value)),
                "labels" => setting.as_object().is_some_and(|labels| {
                    labels.len() <= 32
                        && labels.iter().all(|(key, value)| {
                            key.len() <= 128
                                && !key.starts_with("io.hton.")
                                && !key.starts_with("com.docker.")
                                && value.as_str().is_some_and(|value| value.len() <= 512)
                        })
                }),
                "logging" => setting == &json!({"driver":"none"}),
                _ => false,
            };
            if !accepted {
                return Err(error(
                    inputs,
                    ErrorCode::ModuleUnsupported,
                    "Unsupported Docker customization is retained in source but cannot activate",
                ));
            }
            if key == "labels" {
                let labels = value["labels"]
                    .as_object_mut()
                    .expect("generated labels object");
                labels.extend(setting.as_object().expect("validated labels").clone());
            } else {
                value[key] = setting.clone();
            }
        }
    }
    Ok(value)
}

pub struct CompileInputs<'a> {
    pub render: &'a RenderInputs<'a>,
    pub plan: &'a EffectPlan,
    pub transaction_id: &'a TransactionId,
    pub bundle_id: BundleId,
    pub instance_id: InstanceId,
    pub credential_revision: u64,
}
pub struct BundleStaging {
    directory: Arc<SecureDir>,
}
impl BundleStaging {
    pub fn new(directory: Arc<SecureDir>) -> Self {
        Self { directory }
    }
}
pub struct CompiledBundle {
    pub candidate: wire::CandidateBundle,
    pub service_artifact_digests: BTreeMap<ServiceId, Sha256Digest>,
    pub directory: Arc<SecureDir>,
}
impl fmt::Debug for CompiledBundle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledBundle")
            .field("bundle_id", &self.candidate.bundle_id)
            .field("manifest_digest", &self.candidate.manifest_digest)
            .finish_non_exhaustive()
    }
}

pub fn compile(
    inputs: &CompileInputs<'_>,
    renderers: &BTreeMap<ServiceId, &dyn EngineRenderer>,
    staging: &BundleStaging,
    journal: &JournalStore,
    context: &OperationContext,
) -> Result<CompiledBundle> {
    ensure_mutating(context, journal.control())?;
    let transaction = journal.read(inputs.transaction_id, context)?;
    if transaction.operation_id != context.operation_id
        || transaction.actor != context.actor
        || !matches!(&transaction.intent,Intent::Effects {plan}|Intent::PrepareBundle {plan} if plan==inputs.plan)
    {
        return Err(failure(
            context,
            ErrorCode::OperationDenied,
            "Compilation has no exact durable effect intent",
        ));
    }
    if inputs.plan.expires_at.0 <= journal.control().now_ms()
        || inputs.plan.catalogue_version != inputs.render.catalogue.version()
        || inputs.plan.secret_versions != inputs.render.snapshot.desired.read_set.secret_versions
    {
        return Err(failure(
            context,
            ErrorCode::PlanStale,
            "Compilation versions differ from the approved effect plan",
        ));
    }
    for (id, dependency) in &inputs.render.snapshot.desired.read_set.objects {
        if inputs.plan.read_set.objects.get(id) != Some(dependency) {
            return Err(failure(
                context,
                ErrorCode::PlanStale,
                "Compilation configuration differs from the approved read set",
            ));
        }
    }
    for id in &inputs.render.graph.enabled_modules {
        if let Some(entry) = inputs.render.catalogue.describe(id) {
            if inputs.plan.descriptor_versions.get(id)
                != Some(&entry.descriptor.descriptor_revision)
            {
                return Err(failure(
                    context,
                    ErrorCode::PlanStale,
                    "Compilation descriptor revision differs from the plan",
                ));
            }
        }
    }
    if inputs.instance_id != inputs.render.registry.instance_id
        || inputs
            .render
            .snapshot
            .desired
            .platform
            .settings
            .get("instance")
            .and_then(|instance| instance.get("name"))
            .and_then(Value::as_str)
            != Some(inputs.instance_id.as_str())
    {
        return Err(failure(
            context,
            ErrorCode::ResourceDrift,
            "Compilation instance identity differs from desired and registered instance",
        ));
    }
    for (name, value) in [
        ("state.registry", serde_json::to_vec(inputs.render.registry)),
        (
            "state.catalogue",
            serde_json::to_vec(&inputs.render.catalogue.fingerprint()),
        ),
    ] {
        let bytes = value.map_err(|_| {
            failure(
                context,
                ErrorCode::ConfigSemanticConflict,
                "Compilation state identity cannot be encoded",
            )
        })?;
        let expected = content_revision(object(name), &bytes);
        if inputs
            .plan
            .read_set
            .objects
            .get(&object(name))
            .map(|dependency| &dependency.revision)
            != Some(&expected)
        {
            return Err(failure(
                context,
                ErrorCode::PlanStale,
                "Compilation registry or catalogue changed after review",
            ));
        }
    }
    let rendered = preview(inputs.render, renderers)?;
    let encoded = serde_json::to_vec(&rendered.service_artifact_digests).map_err(|_| {
        failure(
            context,
            ErrorCode::ConfigSemanticConflict,
            "Service artifact digest inventory cannot be encoded",
        )
    })?;
    let target = content_revision(object("plan.targetArtifacts"), &encoded);
    if inputs
        .plan
        .read_set
        .objects
        .get(&object("plan.targetArtifacts"))
        .map(|dependency| &dependency.revision)
        != Some(&target)
    {
        return Err(failure(
            context,
            ErrorCode::PlanStale,
            "Rendered service artifacts differ from the exact approved target",
        ));
    }
    for service in &rendered.services {
        if service.resource.ownership == wire::Ownership::Owned {
            let digest = service
                .image
                .as_str()
                .rsplit_once('@')
                .map(|(_, digest)| digest)
                .ok_or_else(|| {
                    failure(
                        context,
                        ErrorCode::ConfigSemanticConflict,
                        "Owned image is not pinned",
                    )
                })?;
            let id = ServiceId::new(service.service_id.as_str()).map_err(|_| {
                failure(
                    context,
                    ErrorCode::ConfigSemanticConflict,
                    "Service identity conversion failed",
                )
            })?;
            if inputs.plan.image_versions.get(&id).map(ImageDigest::as_str) != Some(digest) {
                return Err(failure(
                    context,
                    ErrorCode::PlanStale,
                    "Rendered image differs from approved digest",
                ));
            }
        }
        let id = ResourceId::new(service.resource.resource_id.as_str()).map_err(|_| {
            failure(
                context,
                ErrorCode::ConfigSemanticConflict,
                "Resource identity conversion failed",
            )
        })?;
        if inputs
            .plan
            .resource_before
            .get(&id)
            .map(Sha256Digest::as_str)
            != inputs.render.services.iter().find_map(|(id, binding)| {
                (id.as_str() == service.service_id.as_str())
                    .then_some(binding.structural_fingerprint.as_str())
            })
        {
            return Err(failure(
                context,
                ErrorCode::ResourceDrift,
                "Rendered resource differs from approved native fingerprint",
            ));
        }
    }
    let mut artifacts = Vec::new();
    for artifact in &rendered.artifacts {
        artifacts.push(wire::Artifact {
            artifact_id: convert(
                wire::ArtifactId::new(artifact.artifact_id.as_str()),
                inputs.render,
            )?,
            path: convert(
                wire::ArtifactPath::new(format!("{}/{}", artifact.service_id, artifact.name)),
                inputs.render,
            )?,
            digest: convert(
                wire::Sha256Digest::new(artifact.digest().as_str()),
                inputs.render,
            )?,
            size_bytes: artifact.bytes.len() as u64,
            service_id: convert(
                wire::ServiceId::new(artifact.service_id.as_str()),
                inputs.render,
            )?,
        });
    }
    let mut retired = BTreeMap::<ServiceId, wire::RetiredService>::new();
    for action in &inputs.plan.actions {
        let (entry, resource) = inputs
            .render
            .registry
            .entries
            .values()
            .flat_map(|entry| {
                entry
                    .resources
                    .iter()
                    .map(move |resource| (entry, resource))
            })
            .find(|(_, resource)| resource.resource_id == action.resource_id)
            .ok_or_else(|| {
                failure(
                    context,
                    ErrorCode::ResourceDrift,
                    "Retirement target is not registered",
                )
            })?;
        if inputs
            .render
            .graph
            .service_consumers
            .contains_key(&resource.service_id)
        {
            continue;
        }
        if entry.ownership != Ownership::Owned
            || !matches!(action.kind, ActionKind::Stop | ActionKind::DisableAutostart)
        {
            return Err(failure(
                context,
                ErrorCode::OperationDenied,
                "Retirement may only deactivate owned registered resources",
            ));
        }
        let binding = inputs
            .render
            .services
            .get(&resource.service_id)
            .ok_or_else(|| {
                failure(
                    context,
                    ErrorCode::ModuleIncomplete,
                    "Retired service exact runtime binding is unavailable",
                )
            })?;
        if binding.resource.resource_id.as_str() != resource.resource_id.as_str()
            || binding.resource.ownership != wire::Ownership::Owned
            || binding.structural_fingerprint != resource.fingerprint
            || inputs.plan.resource_before.get(&resource.resource_id) != Some(&resource.fingerprint)
        {
            return Err(failure(
                context,
                ErrorCode::ResourceDrift,
                "Retirement runtime binding differs from the approved exact resource",
            ));
        }
        retired.insert(
            resource.service_id.clone(),
            wire::RetiredService {
                service_id: convert(
                    wire::ServiceId::new(resource.service_id.as_str()),
                    inputs.render,
                )?,
                module_id: convert(wire::ModuleId::new(entry.module_id.as_str()), inputs.render)?,
                resource: binding.resource.clone(),
                restart_owner_resources: convert(
                    wire::Items::new(binding.restart_owner_resources.clone()),
                    inputs.render,
                )?,
            },
        );
    }
    let actions = map_actions(inputs.plan, inputs.render)?;
    let mut seen_native = BTreeSet::new();
    for action in &inputs.plan.actions {
        if seen_native.insert(action.resource_id.clone()) {
            let observed = inputs
                .render
                .services
                .values()
                .find(|binding| {
                    binding.resource.resource_id.as_str() == action.resource_id.as_str()
                })
                .ok_or_else(|| {
                    failure(
                        context,
                        ErrorCode::ResourceDrift,
                        "native action has no observed runtime binding",
                    )
                })?;
            if !action.preconditions.iter().any(|condition| matches!(condition, Condition::ResourceFingerprint { resource_id, fingerprint } if resource_id == &action.resource_id && fingerprint.as_str() == observed.resource.fingerprint.as_str())) {
                return Err(failure(context, ErrorCode::PlanStale, "first native transition differs from its observed runtime state"));
            }
        }
    }

    let mut candidate = wire::CandidateBundle {
        bundle_id: convert(
            wire::BundleId::new(inputs.bundle_id.as_str()),
            inputs.render,
        )?,
        instance_id: convert(
            wire::InstanceId::new(inputs.instance_id.as_str()),
            inputs.render,
        )?,
        contract_version: wire::ContractVersion::V1,
        manifest_digest: convert(wire::Sha256Digest::new(digest(b"").as_str()), inputs.render)?,
        artifacts: convert(wire::Items::new(artifacts), inputs.render)?,
        services: convert(wire::Items::new(rendered.services), inputs.render)?,
        retired_services: convert(
            wire::Items::new(retired.into_values().collect()),
            inputs.render,
        )?,
        claims: convert(wire::Items::new(rendered.claims), inputs.render)?,
        secrets: convert(wire::Items::new(rendered.secret_bindings), inputs.render)?,
        actions: convert(wire::Items::new(actions), inputs.render)?,
        fences: wire::PolicyFences {
            credential_revision: inputs.credential_revision,
        },
        native_evidence: wire::Items::default(),
        staging: None,
    };
    candidate.manifest_digest = wire::candidate_digest(&candidate).map_err(|_| {
        failure(
            context,
            ErrorCode::ConfigSemanticConflict,
            "Candidate digest encoding failed",
        )
    })?;
    ensure_mutating(context, journal.control())?;
    let directory = Arc::new(
        staging
            .directory
            .create_child(inputs.bundle_id.as_str())
            .map_err(|_| {
                failure(
                    context,
                    ErrorCode::IoUnavailable,
                    "Protected bundle staging unavailable",
                )
            })?,
    );
    for artifact in &rendered.artifacts {
        ensure_mutating(context, journal.control())?;
        let dir = directory
            .create_child(artifact.service_id.as_str())
            .map_err(|_| {
                failure(
                    context,
                    ErrorCode::IoUnavailable,
                    "Protected service staging unavailable",
                )
            })?;
        publish_exact(
            &dir,
            artifact.name.as_str(),
            &artifact.bytes,
            journal,
            context,
        )?;
    }
    let metadata = serde_json::to_vec_pretty(&candidate).map_err(|_| {
        failure(
            context,
            ErrorCode::ConfigSemanticConflict,
            "Candidate metadata encoding failed",
        )
    })?;
    publish_exact(&directory, "candidate.json", &metadata, journal, context)?;
    Ok(CompiledBundle {
        candidate,
        service_artifact_digests: rendered.service_artifact_digests,
        directory,
    })
}
fn publish_exact(
    directory: &SecureDir,
    name: &str,
    bytes: &[u8],
    journal: &JournalStore,
    context: &OperationContext,
) -> Result<()> {
    ensure_mutating(context, journal.control())?;
    let remaining = context
        .deadline
        .0
        .saturating_sub(journal.control().now_ms());
    let locked = directory
        .lock(Instant::now() + Duration::from_millis(remaining.min(300_000)))
        .map_err(|_| {
            failure(
                context,
                ErrorCode::IoUnavailable,
                "Immutable artifact publication lock unavailable",
            )
        })?;
    match locked.read(name, 64 * 1024 * 1024).map_err(|_| {
        failure(
            context,
            ErrorCode::IoUnavailable,
            "Immutable artifact read failed",
        )
    })? {
        Some(existing) if existing == bytes => locked
            .compare_exchange(name, Some(&existing), bytes)
            .map_err(|_| {
                failure(
                    context,
                    ErrorCode::RecoveryRequired,
                    "existing staged artifact durability could not be confirmed",
                )
            }),
        Some(_) => Err(failure(
            context,
            ErrorCode::ResourceDrift,
            "Immutable artifact name already contains different bytes",
        )),
        None => locked.compare_exchange(name, None, bytes).map_err(|_| {
            failure(
                context,
                ErrorCode::IoUnavailable,
                "Immutable artifact publication failed",
            )
        }),
    }
}
/// Native validator supplies evidence. This operation does not claim runtime admission.
pub fn publish_qualified(
    compiled: &CompiledBundle,
    candidate: &wire::CandidateBundle,
    journal: &JournalStore,
    context: &OperationContext,
) -> Result<()> {
    ensure_mutating(context, journal.control())?;
    if wire::candidate_digest(candidate).ok().as_ref() != Some(&compiled.candidate.manifest_digest)
        || candidate.manifest_digest != compiled.candidate.manifest_digest
        || candidate.bundle_id != compiled.candidate.bundle_id
    {
        return Err(failure(
            context,
            ErrorCode::PlanStale,
            "Qualification changed the exact compiled bundle",
        ));
    }
    for service in candidate.services.as_slice() {
        if !candidate.native_evidence.as_slice().iter().any(|evidence| {
            evidence.service_id == service.service_id
                && evidence.kind == wire::ProbeKind::NativeConfig
                && evidence.outcome == wire::EvidenceOutcome::Pass
                && evidence.bundle_digest == candidate.manifest_digest
                && evidence.timestamp_ms <= journal.control().now_ms()
        }) {
            return Err(failure(
                context,
                ErrorCode::ModuleIncomplete,
                "Final bundle lacks qualified native evidence for a selected service",
            ));
        }
    }
    let metadata = serde_json::to_vec_pretty(candidate).map_err(|_| {
        failure(
            context,
            ErrorCode::ConfigSemanticConflict,
            "Qualified manifest encoding failed",
        )
    })?;
    publish_exact(
        &compiled.directory,
        "manifest.json",
        &metadata,
        journal,
        context,
    )
}
fn map_actions(plan: &EffectPlan, inputs: &RenderInputs<'_>) -> Result<Vec<wire::FixedAction>> {
    let mut result = Vec::new();
    let mut ids = BTreeSet::new();
    let inverse_ids: BTreeSet<_> = plan
        .compensations
        .iter()
        .map(|action| action.action_id.clone())
        .collect();
    for action in plan.actions.iter().chain(&plan.compensations) {
        if !ids.insert(action.action_id.clone()) {
            return Err(error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Forward and compensation action IDs overlap",
            ));
        }
        let identity = inputs
            .registry
            .entries
            .values()
            .flat_map(|entry| &entry.resources)
            .find(|resource| resource.resource_id == action.resource_id)
            .ok_or_else(|| {
                error(
                    inputs,
                    ErrorCode::ResourceDrift,
                    "Action resource is not in the registered inventory",
                )
            })?;
        let inverse = inverse_ids.contains(&action.action_id);
        let kind = match action.kind {
            ActionKind::Start => wire::NativeActionKind::Start,
            ActionKind::Stop => wire::NativeActionKind::Stop,
            ActionKind::Reload => wire::NativeActionKind::Reload,
            ActionKind::Restart => wire::NativeActionKind::Restart,
            ActionKind::Recreate => wire::NativeActionKind::Recreate,
            ActionKind::DisableAutostart => wire::NativeActionKind::DisableAutostart,
            ActionKind::RestoreAutostart => wire::NativeActionKind::EnableAutostart,
            ActionKind::PublishArtifacts if inverse => wire::NativeActionKind::RestoreArtifacts,
            ActionKind::PublishArtifacts => wire::NativeActionKind::ActivateArtifacts,
            _ => {
                return Err(error(
                    inputs,
                    ErrorCode::ModuleUnsupported,
                    "Effect kind has no admitted fixed runtime equivalent",
                ));
            }
        };
        let fingerprint = |conditions: &[Condition]| -> Result<wire::Sha256Digest> {
            let matches: Vec<_> = conditions
                .iter()
                .filter_map(|condition| match condition {
                    Condition::ResourceFingerprint {
                        resource_id,
                        fingerprint,
                    } if *resource_id == action.resource_id => Some(fingerprint),
                    _ => None,
                })
                .collect();
            if matches.len() != 1 {
                return Err(error(
                    inputs,
                    ErrorCode::ResourceDrift,
                    "Action lacks a unique exact native transition fingerprint",
                ));
            }
            convert(wire::Sha256Digest::new(matches[0].as_str()), inputs)
        };
        if !inverse
            && action
                .compensation_ref
                .as_ref()
                .is_none_or(|id| !inverse_ids.contains(id))
        {
            return Err(error(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Forward action lacks its reviewed inverse body",
            ));
        }
        result.push(wire::FixedAction {
            action_id: convert(wire::ActionId::new(action.action_id.as_str()), inputs)?,
            kind,
            resource_id: convert(wire::ResourceId::new(action.resource_id.as_str()), inputs)?,
            service_id: convert(wire::ServiceId::new(identity.service_id.as_str()), inputs)?,
            dependencies: convert(
                wire::Items::new(
                    action
                        .dependencies
                        .iter()
                        .map(|id| convert(wire::ActionId::new(id.as_str()), inputs))
                        .collect::<Result<Vec<_>>>()?,
                ),
                inputs,
            )?,
            before: Some(fingerprint(&action.preconditions)?),
            after: Some(fingerprint(&action.postconditions)?),
            timeout_ms: NonZeroU32::new(u32::try_from(action.timeout.0).map_err(|_| {
                error(
                    inputs,
                    ErrorCode::ConfigSemanticConflict,
                    "Action timeout exceeds runtime bound",
                )
            })?)
            .ok_or_else(|| {
                error(
                    inputs,
                    ErrorCode::ConfigSemanticConflict,
                    "Action timeout must be positive",
                )
            })?,
            compensation: action
                .compensation_ref
                .as_ref()
                .map(|id| convert(wire::ActionId::new(id.as_str()), inputs))
                .transpose()?,
        });
    }
    Ok(result)
}
