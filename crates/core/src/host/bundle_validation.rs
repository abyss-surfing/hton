//! Explicit, isolated final-bundle qualification. Native checks never imply protocol success.
use super::HostApplication;
use crate::journal::{self, Intent, TerminalStatus};
use hton_contracts::{config::*, operations::*};
use hton_host::{
    Budget, Cancellation,
    docker::ImageReference,
    validation::{Engine, Role, SandboxResource, ValidationDocker},
};
use hton_runtime_contract as wire;
use hton_storage::SecureDir;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddrV4,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

struct Cancel<'a> {
    app: &'a HostApplication,
    ctx: &'a OperationContext,
}
impl Cancellation for Cancel<'_> {
    fn cancelled(&self) -> bool {
        self.app.control.is_cancelled(&self.ctx.cancellation)
    }
}
fn fail(ctx: &OperationContext, code: ErrorCode, message: &'static str) -> Error {
    journal::failure(ctx, code, message)
}
fn budget(app: &HostApplication, ctx: &OperationContext) -> Result<Budget> {
    journal::ensure_mutating(ctx, app.control.as_ref())?;
    Budget::new(
        Instant::now()
            + Duration::from_millis(
                ctx.deadline
                    .0
                    .saturating_sub(app.control.now_ms())
                    .min(300_000),
            ),
        4 * 1024 * 1024,
    )
    .map_err(|_| {
        fail(
            ctx,
            ErrorCode::OperationDeadline,
            "validation budget invalid",
        )
    })
}
fn host<T>(value: hton_host::Result<T>, ctx: &OperationContext) -> Result<T> {
    value.map_err(|_| {
        fail(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "isolated validation step failed; protected state retained",
        )
    })
}
fn digest(bytes: &[u8]) -> Sha256Digest {
    Sha256Digest::new(wire::hash(bytes).as_str()).expect("digest conversion")
}
struct Inputs {
    candidate: wire::CandidateBundle,
    artifacts: BTreeMap<String, Vec<u8>>,
    domain: String,
    edge: SocketAddrV4,
    caddy: SocketAddrV4,
    subnet: String,
    unbound: Option<SocketAddrV4>,
    xray: Option<XrayInput>,
}
struct XrayInput {
    endpoint: SocketAddrV4,
    uuid: String,
    transport: String,
    path: String,
}
fn read_artifact(
    dir: &SecureDir,
    path: &str,
    limit: usize,
    ctx: &OperationContext,
) -> Result<Vec<u8>> {
    let mut parts = path.split('/').collect::<Vec<_>>();
    let name = parts
        .pop()
        .ok_or_else(|| fail(ctx, ErrorCode::ResourceDrift, "artifact path missing"))?;
    let mut opened = dir.try_clone().map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "bundle directory unavailable",
        )
    })?;
    for part in parts {
        opened = opened.child(part).map_err(|_| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "protected artifact parent unavailable",
            )
        })?;
    }
    opened
        .read(name, limit)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "protected artifact unreadable",
            )
        })?
        .ok_or_else(|| fail(ctx, ErrorCode::ResourceDrift, "protected artifact missing"))
}
fn inputs(app: &HostApplication, id: &BundleId, ctx: &OperationContext) -> Result<Inputs> {
    let dir = app
        .work
        .child("bundles")
        .and_then(|d| d.child(id.as_str()))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "protected bundle unavailable",
            )
        })?;
    let bytes = dir
        .read("candidate.json", wire::MAX_WIRE_BYTES)
        .map_err(|_| fail(ctx, ErrorCode::IoUnavailable, "candidate unavailable"))?
        .ok_or_else(|| fail(ctx, ErrorCode::ModuleIncomplete, "candidate missing"))?;
    let candidate: wire::CandidateBundle = wire::decode(&bytes)
        .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "candidate corrupt"))?;
    if candidate.bundle_id.as_str() != id.as_str()
        || candidate.instance_id.as_str() != app.installation.instance_id.as_str()
        || wire::candidate_digest(&candidate).ok().as_ref() != Some(&candidate.manifest_digest)
        || candidate.artifacts.as_slice().len() > 256
        || candidate.services.as_slice().is_empty()
        || candidate.services.as_slice().len() > 16
    {
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "candidate identity or bounds invalid",
        ));
    }
    let mut artifacts = BTreeMap::new();
    let mut total = 0u64;
    let mut ids = BTreeSet::new();
    for artifact in candidate.artifacts.as_slice() {
        if app.control.now_ms() >= ctx.deadline.0 || app.control.is_cancelled(&ctx.cancellation) {
            return Err(fail(
                ctx,
                ErrorCode::OperationDeadline,
                "candidate lookup budget exhausted",
            ));
        }
        total = total
            .checked_add(artifact.size_bytes)
            .ok_or_else(|| fail(ctx, ErrorCode::ResourceDrift, "artifact budget overflow"))?;
        if total > 32 * 1024 * 1024
            || artifact.size_bytes > 4 * 1024 * 1024
            || !ids.insert(artifact.artifact_id.clone())
            || !artifact
                .path
                .as_str()
                .starts_with(&format!("{}/", artifact.service_id))
            || !candidate.services.as_slice().iter().any(|s| {
                s.service_id == artifact.service_id
                    && s.artifacts.as_slice().contains(&artifact.artifact_id)
            })
        {
            return Err(fail(
                ctx,
                ErrorCode::ResourceDrift,
                "artifact inventory invalid",
            ));
        }
        let bytes = read_artifact(
            &dir,
            artifact.path.as_str(),
            artifact.size_bytes as usize,
            ctx,
        )?;
        if bytes.len() as u64 != artifact.size_bytes
            || wire::hash(&bytes) != artifact.digest
            || artifacts
                .insert(artifact.path.as_str().to_owned(), bytes)
                .is_some()
        {
            return Err(fail(
                ctx,
                ErrorCode::ResourceDrift,
                "artifact hash or path mismatch",
            ));
        }
    }
    // No native-only result may pass an unimplemented optional protocol scenario.
    let services = candidate.services.as_slice();
    let mut seen_services = BTreeSet::new();
    if !services
        .iter()
        .any(|s| s.service_id.as_str() == "haproxy" && s.engine == wire::Engine::Haproxy)
        || !services
            .iter()
            .any(|s| s.service_id.as_str() == "caddy" && s.engine == wire::Engine::Caddy)
        || services.iter().any(|s| {
            !seen_services.insert(s.service_id.clone())
                || !matches!(
                    (s.service_id.as_str(), s.engine),
                    ("haproxy", wire::Engine::Haproxy)
                        | ("caddy", wire::Engine::Caddy)
                        | ("xray", wire::Engine::Xray)
                        | ("unbound", wire::Engine::Unbound)
                )
        })
        || candidate
            .claims
            .as_slice()
            .iter()
            .any(|c| matches!(c, wire::Claim::SniRoute { .. }))
    {
        return Err(fail(
            ctx,
            ErrorCode::ModuleUnsupported,
            "selected bundle requires a protocol scenario not implemented by this runner",
        ));
    }
    let trusted_images = super::setup::locked_image_references(ctx)?;
    for service in services {
        let trusted = super::setup::locked_image_reference(service.service_id.as_str(), ctx)?;
        let repository = trusted.as_str().rsplit_once('@').expect("trusted pin").0;
        if service
            .image
            .as_str()
            .rsplit_once('@')
            .is_none_or(|(repo, _)| repo != repository)
            || !trusted_images
                .values()
                .any(|image| image.as_str() == service.image.as_str())
            || service
                .artifacts
                .as_slice()
                .iter()
                .any(|id| !ids.contains(id))
        {
            return Err(fail(
                ctx,
                ErrorCode::ResourceDrift,
                "candidate service image or artifact set is outside trusted inventory",
            ));
        }
    }
    let text = |path: &str| -> Result<&str> {
        std::str::from_utf8(artifacts.get(path).ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "required site artifact missing",
            )
        })?)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "native config encoding invalid",
            )
        })
    };
    let edge = text("haproxy/haproxy.cfg")?
        .lines()
        .find_map(|l| l.trim().strip_prefix("bind "))
        .and_then(|s| s.parse::<SocketAddrV4>().ok())
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleUnsupported,
                "edge listener mapping unavailable",
            )
        })?;
    let caddy = text("haproxy/haproxy.cfg")?
        .lines()
        .find_map(|l| l.trim().strip_prefix("server caddy "))
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<SocketAddrV4>().ok())
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleUnsupported,
                "site backend mapping unavailable",
            )
        })?;
    let domains = candidate
        .claims
        .as_slice()
        .iter()
        .filter_map(|c| match c {
            wire::Claim::Certificate {
                service, hostname, ..
            } if service.as_str() == "caddy" => Some(hostname.as_str().to_owned()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if domains.len() != 1
        || edge.port() < 1024
        || caddy.port() < 1024
        || edge.ip() == caddy.ip()
        || edge.ip().is_unspecified()
        || caddy.ip().is_unspecified()
    {
        return Err(fail(
            ctx,
            ErrorCode::ModuleUnsupported,
            "runner requires one trusted HTTPS site and private unprivileged listeners",
        ));
    }
    let domain = domains.into_iter().next().expect("one domain");
    if !text("caddy/Caddyfile")?.contains(&format!("https://{domain}:{}", caddy.port()))
        || !artifacts.contains_key("caddy/site-index.html")
    {
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "site scenario does not match final candidate",
        ));
    }
    let compose: Value = serde_json::from_str(text("haproxy/compose.json")?)
        .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "rendered topology invalid"))?;
    let subnet = compose
        .pointer("/networks/hton/ipam/config/0/subnet")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "private source subnet absent",
            )
        })?
        .to_owned();
    let (net, prefix) = subnet.split_once('/').ok_or_else(|| {
        fail(
            ctx,
            ErrorCode::ResourceDrift,
            "private source subnet invalid",
        )
    })?;
    if net.parse::<std::net::Ipv4Addr>().is_err()
        || prefix.parse::<u8>().ok().is_none_or(|p| p > 32)
    {
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "private source subnet invalid",
        ));
    }
    let unbound = if services.iter().any(|s| s.service_id.as_str() == "unbound") {
        let conf = text("unbound/unbound.conf")?;
        let address = conf
            .lines()
            .find_map(|l| l.trim().strip_prefix("interface: "))
            .and_then(|s| s.parse::<std::net::Ipv4Addr>().ok())
            .ok_or_else(|| fail(ctx, ErrorCode::ResourceDrift, "DNS listener invalid"))?;
        let port = conf
            .lines()
            .find_map(|l| l.trim().strip_prefix("port: "))
            .and_then(|s| s.parse::<u16>().ok())
            .filter(|p| *p >= 1024)
            .ok_or_else(|| fail(ctx, ErrorCode::ResourceDrift, "DNS listener port invalid"))?;
        Some(SocketAddrV4::new(address, port))
    } else {
        None
    };
    let xray = if services.iter().any(|s| s.service_id.as_str() == "xray") {
        let config: Value = serde_json::from_str(text("xray/xray.json")?)
            .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "Xray input invalid"))?;
        let inbound = config
            .get("inbounds")
            .and_then(Value::as_array)
            .filter(|a| a.len() == 1)
            .and_then(|a| a.first())
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "Xray scenario requires one native VLESS inbound",
                )
            })?;
        let field = |pointer: &str| -> Result<&str> {
            inbound
                .pointer(pointer)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    fail(
                        ctx,
                        ErrorCode::ModuleUnsupported,
                        "Xray scenario field missing",
                    )
                })
        };
        if field("/protocol")? != "vless" || field("/streamSettings/security")? != "none" {
            return Err(fail(
                ctx,
                ErrorCode::ModuleUnsupported,
                "Xray native protocol or TLS ownership unsupported",
            ));
        }
        let transport = field("/streamSettings/network")?.to_owned();
        let path = field(match transport.as_str() {
            "ws" => "/streamSettings/wsSettings/path",
            "xhttp" => "/streamSettings/xhttpSettings/path",
            _ => {
                return Err(fail(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "Xray transport scenario unsupported",
                ));
            }
        })?
        .to_owned();
        let uuid = field("/settings/clients/0/id")?.to_owned();
        if uuid.len() != 36
            || !uuid.bytes().enumerate().all(|(i, b)| {
                if [8, 13, 18, 23].contains(&i) {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit() && !b.is_ascii_uppercase()
                }
            })
            || !path.starts_with('/')
            || path.len() > 256
            || path.chars().any(char::is_control)
        {
            return Err(fail(
                ctx,
                ErrorCode::ResourceDrift,
                "Xray credential/path invalid",
            ));
        }
        let address = field("/listen")?
            .parse::<std::net::Ipv4Addr>()
            .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "Xray listener invalid"))?;
        let port = inbound
            .get("port")
            .and_then(Value::as_u64)
            .filter(|p| (1024..=65535).contains(p))
            .ok_or_else(|| fail(ctx, ErrorCode::ResourceDrift, "Xray listener port invalid"))?
            as u16;
        Some(XrayInput {
            endpoint: SocketAddrV4::new(address, port),
            uuid,
            transport,
            path,
        })
    } else {
        None
    };
    Ok(Inputs {
        candidate,
        artifacts,
        domain,
        edge,
        caddy,
        subnet,
        unbound,
        xray,
    })
}
fn receipt(
    app: &HostApplication,
    tx: &TransactionId,
    step: &str,
    r: &SandboxResource,
    ctx: &OperationContext,
) -> Result<()> {
    let key = format!(
        "{}:{}",
        if r.network { "network" } else { "container" },
        r.id
    );
    let mut fingerprints = BTreeMap::new();
    fingerprints.insert(
        ResourceId::new(format!("validation.{}", r.id)).expect("fixed bounded step"),
        digest(key.as_bytes()),
    );
    let receipt = StepReceipt {
        step_id: StepId::new(step).expect("fixed bounded step"),
        idempotency_key: ctx.actor.request_id.clone(),
        status: StepStatus::Applied,
        recorded_at: UnixMillis(app.control.now_ms()),
        resources: vec![],
        state_fingerprints: fingerprints,
        retained_artifacts: vec![],
        error: None,
    };
    app.journal.record_step(tx, &receipt, ctx)
}
fn publish(dir: &SecureDir, name: &str, bytes: &[u8], ctx: &OperationContext) -> Result<()> {
    let locked = dir
        .lock(Instant::now() + Duration::from_secs(5))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationConflict,
                "validation staging lock unavailable",
            )
        })?;
    let old = locked.read(name, 4 * 1024 * 1024).map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "validation staging unreadable",
        )
    })?;
    if old.as_deref() == Some(bytes) {
        return Ok(());
    }
    locked
        .compare_exchange(name, old.as_deref(), bytes)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "validation staging publication failed",
            )
        })
}
fn evidence(
    app: &HostApplication,
    ctx: &OperationContext,
    name: &str,
    input: &Sha256Digest,
    outcome: EvidenceOutcome,
) -> Evidence {
    Evidence {
        check_id: CheckId::new(name).expect("fixed check"),
        module_id: None,
        outcome,
        timestamp: UnixMillis(app.control.now_ms()),
        vantage: if name.starts_with("native.") {
            Vantage::NativeValidation
        } else {
            Vantage::LocalRuntime
        },
        input_fingerprint: input.clone(),
        details: SafeFields {
            operation_id: Some(ctx.operation_id.clone()),
            ..Default::default()
        },
    }
}
fn engine(value: wire::Engine, ctx: &OperationContext) -> Result<Engine> {
    match value {
        wire::Engine::Haproxy => Ok(Engine::Haproxy),
        wire::Engine::Caddy => Ok(Engine::Caddy),
        wire::Engine::Xray => Ok(Engine::Xray),
        wire::Engine::Unbound => Ok(Engine::Unbound),
        _ => Err(fail(
            ctx,
            ErrorCode::ModuleUnsupported,
            "native foreign scenario unavailable",
        )),
    }
}
fn run(
    app: &HostApplication,
    input: &Inputs,
    driver: &ValidationDocker,
    scope: &str,
    tx: &TransactionId,
    ctx: &OperationContext,
) -> Result<Vec<Evidence>> {
    let cancel = Cancel { app, ctx };
    let fingerprint = digest(input.candidate.manifest_digest.as_str().as_bytes());
    let staging = app
        .work
        .create_child("validation")
        .and_then(|d| d.create_child(scope))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "validation staging unavailable",
            )
        })?;
    for (path, bytes) in &input.artifacts {
        let (service, name) = path
            .split_once('/')
            .ok_or_else(|| fail(ctx, ErrorCode::ResourceDrift, "artifact path invalid"))?;
        if name.contains('/') {
            return Err(fail(
                ctx,
                ErrorCode::ModuleUnsupported,
                "nested artifact staging unsupported",
            ));
        }
        let dir = staging.create_child(service).map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "validation service staging unavailable",
            )
        })?;
        publish(&dir, name, bytes, ctx)?;
    }
    let mut checks = Vec::new();
    for service in input.candidate.services.as_slice() {
        let engine = engine(service.engine, ctx)?;
        let image = ImageReference::new(service.image.as_str())
            .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "candidate image pin invalid"))?;
        let r = host(
            driver.create(
                service.service_id.as_str(),
                &image,
                engine,
                Role::Native,
                None,
                None,
                budget(app, ctx)?,
                &cancel,
            ),
            ctx,
        )?;
        receipt(app, tx, &format!("native.{}", service.service_id), &r, ctx)?;
        host(driver.start(&r, true, budget(app, ctx)?, &cancel), ctx)?;
        checks.push(evidence(
            app,
            ctx,
            &format!("native.{}", service.service_id),
            &fingerprint,
            EvidenceOutcome::Pass,
        ));
    }
    let network = host(driver.network(budget(app, ctx)?, &cancel), ctx)?;
    receipt(app, tx, "network", &network, ctx)?;
    let mut servers = BTreeMap::new();
    let mut addresses = BTreeMap::new();
    for service in input.candidate.services.as_slice() {
        let engine = engine(service.engine, ctx)?;
        let image = ImageReference::new(service.image.as_str())
            .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "candidate image pin invalid"))?;
        let r = host(
            driver.create(
                service.service_id.as_str(),
                &image,
                engine,
                Role::Server,
                Some(&network),
                None,
                budget(app, ctx)?,
                &cancel,
            ),
            ctx,
        )?;
        receipt(app, tx, &format!("server.{}", service.service_id), &r, ctx)?;
        let address = host(driver.address(&r, budget(app, ctx)?, &cancel), ctx)?;
        addresses.insert(service.service_id.as_str(), address);
        servers.insert(service.service_id.as_str(), r);
    }
    let edge_ip = addresses["haproxy"];
    let caddy_ip = addresses["caddy"];
    let test_subnet = format!(
        "{}.{}.{}.0/24",
        edge_ip.octets()[0],
        edge_ip.octets()[1],
        edge_ip.octets()[2]
    );
    let fixture_port = if input.caddy.port() == 18080 {
        18081
    } else {
        18080
    };
    let mut mappings = vec![(*input.edge.ip(), edge_ip), (*input.caddy.ip(), caddy_ip)];
    if let Some(unbound) = input.unbound {
        mappings.push((*unbound.ip(), addresses["unbound"]));
    }
    if let Some(xray) = &input.xray {
        mappings.push((*xray.endpoint.ip(), addresses["xray"]));
    }
    if mappings
        .iter()
        .map(|(old, _)| old)
        .collect::<BTreeSet<_>>()
        .len()
        != mappings.len()
    {
        return Err(fail(
            ctx,
            ErrorCode::ModuleUnsupported,
            "sandbox requires distinct final native endpoint addresses",
        ));
    }
    let mut mapped_hashes = BTreeMap::new();
    for path in [
        "haproxy/haproxy.cfg",
        "caddy/Caddyfile",
        "xray/xray.json",
        "unbound/unbound.conf",
    ] {
        let Some(bytes) = input.artifacts.get(path) else {
            continue;
        };
        let source = std::str::from_utf8(bytes).map_err(|_| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "native config encoding invalid",
            )
        })?;
        let mut replaced = source.replace(&input.subnet, &test_subnet);
        for (index, (old, _)) in mappings.iter().enumerate() {
            replaced = replaced.replace(&old.to_string(), &format!("HTON_SANDBOX_ADDRESS_{index}"));
        }
        for (index, (_, new)) in mappings.iter().enumerate() {
            replaced = replaced.replace(&format!("HTON_SANDBOX_ADDRESS_{index}"), &new.to_string());
        }
        if path == "caddy/Caddyfile" && input.xray.is_some() {
            replaced.push_str(&format!("\nhttp://{caddy_ip}:{fixture_port} {{\n    bind {caddy_ip}\n    root * /etc/hton/current\n    rewrite * /site-index.html\n    file_server\n}}\n"));
        }
        if path == "unbound/unbound.conf" {
            replaced=replaced.replace("remote-control:","    local-zone: \"hton-validation.invalid.\" static\n    local-data: \"hton-validation.invalid. 60 IN A 192.0.2.1\"\nremote-control:");
        }
        if path == "xray/xray.json" {
            let mut config: Value = serde_json::from_str(&replaced)
                .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "mapped Xray config invalid"))?;
            let rules = config
                .pointer_mut("/routing/rules")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| {
                    fail(
                        ctx,
                        ErrorCode::ModuleUnsupported,
                        "Xray routing fixture cannot be scoped",
                    )
                })?;
            rules.insert(0,json!({"type":"field","inboundTag":["vless"],"ip":[format!("{caddy_ip}/32")],"port":fixture_port.to_string(),"outboundTag":"direct"}));
            replaced = serde_json::to_string_pretty(&config).map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "mapped Xray config encoding failed",
                )
            })?;
        }
        let (service, name) = path.split_once('/').expect("fixed path");
        let dir = staging
            .child(service)
            .map_err(|_| fail(ctx, ErrorCode::IoUnavailable, "validation staging missing"))?;
        publish(&dir, name, replaced.as_bytes(), ctx)?;
        mapped_hashes.insert(path.to_owned(), digest(replaced.as_bytes()));
    }
    let mut xray_client = None;
    if let Some(xray) = &input.xray {
        let server = input
            .candidate
            .services
            .as_slice()
            .iter()
            .find(|s| s.service_id.as_str() == "xray")
            .expect("selected Xray");
        let image = ImageReference::new(server.image.as_str())
            .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "Xray client pin invalid"))?;
        let dir = staging.create_child("xray-client").map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "Xray client staging unavailable",
            )
        })?;
        let proxy = host(
            driver.create(
                "xray-client",
                &image,
                Engine::Xray,
                Role::Server,
                Some(&network),
                None,
                budget(app, ctx)?,
                &cancel,
            ),
            ctx,
        )?;
        receipt(app, tx, "server.xray-client", &proxy, ctx)?;
        let address = host(driver.address(&proxy, budget(app, ctx)?, &cancel), ctx)?;
        let mut stream = json!({"network":xray.transport,"security":"tls","tlsSettings":{"serverName":input.domain,"allowInsecure":false,"alpn":if xray.transport=="ws"{vec!["http/1.1"]}else{vec!["h2","http/1.1"]}}});
        if xray.transport == "ws" {
            stream["wsSettings"] = json!({"path":xray.path,"headers":{"Host":input.domain}});
        } else {
            stream["xhttpSettings"] = json!({"path":xray.path,"host":input.domain,"mode":"auto"});
        }
        let config = json!({"log":{"access":"none","error":"","loglevel":"warning"},"inbounds":[{"tag":"test-socks","listen":address.to_string(),"port":1080,"protocol":"socks","settings":{"auth":"noauth","udp":false}}],"outbounds":[{"protocol":"vless","settings":{"vnext":[{"address":edge_ip.to_string(),"port":input.edge.port(),"users":[{"id":xray.uuid,"encryption":"none"}]}]},"streamSettings":stream}]});
        let bytes = serde_json::to_vec_pretty(&config)
            .map_err(|_| fail(ctx, ErrorCode::IoUnavailable, "Xray client encoding failed"))?;
        publish(&dir, "xray.json", &bytes, ctx)?;
        mapped_hashes.insert("xray-client/xray.json".into(), digest(&bytes));
        let native = host(
            driver.create(
                "xray-client",
                &image,
                Engine::Xray,
                Role::Native,
                None,
                None,
                budget(app, ctx)?,
                &cancel,
            ),
            ctx,
        )?;
        receipt(app, tx, "native.xray-client", &native, ctx)?;
        host(driver.start(&native, true, budget(app, ctx)?, &cancel), ctx)?;
        checks.push(evidence(
            app,
            ctx,
            "native.xray-client",
            &digest(&bytes),
            EvidenceOutcome::Pass,
        ));
        xray_client = Some((proxy, address));
    }
    let mapping_digest = digest(&serde_json::to_vec(&mapped_hashes).map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "sandbox mapping encoding failed",
        )
    })?);
    for name in ["unbound", "xray", "caddy", "haproxy"] {
        if let Some(server) = servers.get(name) {
            host(driver.start(server, false, budget(app, ctx)?, &cancel), ctx)?;
        }
    }
    if let Some((proxy, _)) = &xray_client {
        host(driver.start(proxy, false, budget(app, ctx)?, &cancel), ctx)?;
    }
    let image = super::setup::locked_image_reference("acme", ctx)?;
    let expected = &input.artifacts["caddy/site-index.html"];
    for (name, address, port) in [
        ("https.direct", caddy_ip, input.caddy.port()),
        ("https.edge", edge_ip, input.edge.port()),
    ] {
        let r = host(
            driver.create(
                "caddy",
                &image,
                Engine::Caddy,
                Role::Client,
                Some(&network),
                Some((&input.domain, port, address)),
                budget(app, ctx)?,
                &cancel,
            ),
            ctx,
        )?;
        receipt(app, tx, name, &r, ctx)?;
        let mut attempt = driver.start(&r, true, budget(app, ctx)?, &cancel);
        for _ in 0..2 {
            if attempt.is_ok() {
                break;
            }
            journal::ensure_mutating(ctx, app.control.as_ref())?;
            std::thread::sleep(Duration::from_millis(200));
            attempt = driver.start(&r, true, budget(app, ctx)?, &cancel);
        }
        let body = host(attempt, ctx)?;
        if body.strip_suffix(b"200") != Some(expected.as_slice()) {
            return Err(fail(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "isolated trusted HTTPS site body differs from final artifact",
            ));
        }
        let mapping = serde_json::to_vec(&(
            input.candidate.manifest_digest.as_str(),
            &mapping_digest,
            input.domain.as_str(),
            name,
            address,
            port,
            digest(expected),
        ))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "scenario mapping serialization failed",
            )
        })?;
        checks.push(evidence(
            app,
            ctx,
            name,
            &digest(&mapping),
            EvidenceOutcome::Pass,
        ));
    }
    if let Some((_, proxy)) = &xray_client {
        let r = host(
            driver.create(
                "caddy",
                &image,
                Engine::Caddy,
                Role::SocksRequest,
                Some(&network),
                Some((&caddy_ip.to_string(), fixture_port, *proxy)),
                budget(app, ctx)?,
                &cancel,
            ),
            ctx,
        )?;
        receipt(app, tx, "vless.roundtrip", &r, ctx)?;
        let bytes = host(driver.start(&r, true, budget(app, ctx)?, &cancel), ctx)?;
        if bytes.strip_suffix(b"200") != Some(expected.as_slice()) {
            return Err(fail(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "VLESS client did not return exact isolated fixture body",
            ));
        }
        checks.push(evidence(
            app,
            ctx,
            "vless.roundtrip",
            &mapping_digest,
            EvidenceOutcome::Pass,
        ));
    }
    if let Some(unbound) = input.unbound {
        for (name, role) in [("dns.udp", Role::DnsUdp), ("dns.tcp", Role::DnsTcp)] {
            let r = host(
                driver.create(
                    "caddy",
                    &image,
                    Engine::Caddy,
                    role,
                    Some(&network),
                    Some((
                        "hton-validation.invalid",
                        unbound.port(),
                        addresses["unbound"],
                    )),
                    budget(app, ctx)?,
                    &cancel,
                ),
                ctx,
            )?;
            receipt(app, tx, name, &r, ctx)?;
            let bytes = host(driver.start(&r, true, budget(app, ctx)?, &cancel), ctx)?;
            if bytes != b"192.0.2.1\n" {
                return Err(fail(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "isolated DNS transport returned unexpected fixture address",
                ));
            }
            checks.push(evidence(
                app,
                ctx,
                name,
                &mapping_digest,
                EvidenceOutcome::Pass,
            ));
        }
        checks.push(evidence(
            app,
            ctx,
            "dns.external-recursion",
            &fingerprint,
            EvidenceOutcome::Unknown,
        ));
    }
    let rejected = host(
        driver.create(
            "caddy",
            &image,
            Engine::Caddy,
            Role::RejectClient,
            Some(&network),
            Some((&input.domain, input.edge.port(), edge_ip)),
            budget(app, ctx)?,
            &cancel,
        ),
        ctx,
    )?;
    receipt(app, tx, "https.unknown-sni", &rejected, ctx)?;
    host(
        driver.request_rejected(&rejected, budget(app, ctx)?, &cancel),
        ctx,
    )?;
    checks.push(evidence(
        app,
        ctx,
        "https.unknown-sni",
        &fingerprint,
        EvidenceOutcome::Pass,
    ));
    checks.push(evidence(
        app,
        ctx,
        "external.reachability",
        &fingerprint,
        EvidenceOutcome::Unknown,
    ));
    Ok(checks)
}

fn remove_staging(
    owner: &hton_storage::LockedDir,
    scope: &str,
    ctx: &OperationContext,
) -> Result<()> {
    let dir = match owner.directory().child(scope) {
        Ok(dir) => dir,
        Err(hton_storage::Error::NotFound) => return Ok(()),
        Err(_) => {
            return Err(fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "sandbox staging unavailable for cleanup",
            ));
        }
    };
    let lock = dir
        .lock(Instant::now() + Duration::from_secs(5))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "sandbox staging cleanup lock unavailable",
            )
        })?;
    for service in lock.list(8).map_err(|_| {
        fail(
            ctx,
            ErrorCode::RecoveryRequired,
            "sandbox staging inventory failed",
        )
    })? {
        if !matches!(
            service.as_str(),
            "haproxy" | "caddy" | "xray" | "unbound" | "xray-client"
        ) {
            return Err(fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "unexpected sandbox staging resource",
            ));
        }
        let child = dir.child(&service).map_err(|_| {
            fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "sandbox service staging missing",
            )
        })?;
        {
            let files = child
                .lock(Instant::now() + Duration::from_secs(5))
                .map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "sandbox service cleanup lock unavailable",
                    )
                })?;
            for name in files.list(256).map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "sandbox artifact inventory failed",
                )
            })? {
                let bytes = files
                    .read(&name, 4 * 1024 * 1024)
                    .map_err(|_| {
                        fail(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "sandbox artifact cleanup read failed",
                        )
                    })?
                    .ok_or_else(|| {
                        fail(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "sandbox artifact changed during cleanup",
                        )
                    })?;
                files.remove(&name, &bytes).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "sandbox artifact cleanup failed",
                    )
                })?;
            }
        }
        lock.remove_empty_child(&service).map_err(|_| {
            fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "sandbox service directory cleanup failed",
            )
        })?;
    }
    drop(lock);
    owner.remove_empty_child(scope).map_err(|_| {
        fail(
            ctx,
            ErrorCode::RecoveryRequired,
            "sandbox staging directory cleanup failed",
        )
    })
}

pub fn execute(app: &HostApplication, args: &[String], ctx: &OperationContext) -> Result<Value> {
    if args.len() != 1 {
        return Err(fail(
            ctx,
            ErrorCode::OperationDenied,
            "bundle.validate requires one exact BundleId",
        ));
    }
    let id = BundleId::new(&args[0])
        .map_err(|_| fail(ctx, ErrorCode::OperationDenied, "invalid bundle identity"))?;
    let input = inputs(app, &id, ctx)?;
    let raw_scope = digest(
        format!(
            "{}/{}/{}",
            app.installation.instance_id, ctx.operation_id, input.candidate.manifest_digest
        )
        .as_bytes(),
    );
    let scope = &raw_scope.as_str()[7..39];
    let driver = ValidationDocker::new(scope)
        .map_err(|_| fail(ctx, ErrorCode::OperationDenied, "invalid validation scope"))?;
    let cancel = Cancel { app, ctx };
    // Image lookup is read-only and does not pull or create Docker resources.
    for service in input.candidate.services.as_slice() {
        let image = ImageReference::new(service.image.as_str())
            .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "image pin invalid"))?;
        let remaining = ctx
            .deadline
            .0
            .saturating_sub(app.control.now_ms())
            .min(300_000);
        let read_budget = Budget::new(
            Instant::now() + Duration::from_millis(remaining),
            4 * 1024 * 1024,
        )
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationDeadline,
                "validation lookup budget invalid",
            )
        })?;
        host(driver.check_image(&image, read_budget, &cancel), ctx)?;
    }
    let client_image = super::setup::locked_image_reference("acme", ctx)?;
    let lookup_budget = Budget::new(
        Instant::now()
            + Duration::from_millis(
                ctx.deadline
                    .0
                    .saturating_sub(app.control.now_ms())
                    .min(300_000),
            ),
        4 * 1024 * 1024,
    )
    .map_err(|_| {
        fail(
            ctx,
            ErrorCode::OperationDeadline,
            "client lookup budget invalid",
        )
    })?;
    host(
        driver.check_image(&client_image, lookup_budget, &cancel),
        ctx,
    )?;
    let mut scenarios = vec![
        "native.haproxy",
        "native.caddy",
        "https.direct",
        "https.edge",
        "https.unknown-sni",
    ];
    if input.unbound.is_some() {
        scenarios.extend(["native.unbound", "dns.udp", "dns.tcp"]);
    }
    if input.xray.is_some() {
        scenarios.extend(["native.xray", "native.xray-client", "vless.roundtrip"]);
    }
    if ctx.execution_mode != ExecutionMode::Mutating {
        return Ok(
            json!({"status":"planned","operation":"bundle.validate","bundleId":id,"candidateDigest":input.candidate.manifest_digest,"scenarios":scenarios,"writes":false,"externalReachability":"unverified"}),
        );
    }
    // Serialize explicit validation/recovery without holding the configuration lock.
    let owner = app.work.create_child("validation").map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "validation owner unavailable",
        )
    })?;
    let owner_lock = owner
        .lock(journal::deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationConflict,
                "another validation is active",
            )
        })?;
    let intent = Intent::BundleValidation {
        bundle_id: id.clone(),
        candidate_digest: Sha256Digest::new(input.candidate.manifest_digest.as_str())
            .expect("wire digest"),
        scope: ObjectId::new(scope).expect("bounded scope"),
    };
    if let Some(previous) = app.journal.lookup_intent(&intent, ctx)? {
        if app
            .journal
            .terminal(&previous.transaction_id, ctx)?
            .is_some()
        {
            return Err(fail(
                ctx,
                ErrorCode::OperationConflict,
                "validation operation already completed; retain its result or use a new operation identity",
            ));
        }
    }
    // Recover only this owner's exact persisted scopes, including create-before-receipt crashes.
    for pending in app.journal.unfinished(ctx)? {
        if let Intent::BundleValidation {
            scope: old_scope,
            bundle_id: old_bundle,
            candidate_digest: old_digest,
        } = &pending.intent
        {
            let old = ValidationDocker::new(old_scope.as_str()).map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "validation journal scope invalid",
                )
            })?;
            let cleanup_budget =
                Budget::new(Instant::now() + Duration::from_secs(30), 4 * 1024 * 1024).map_err(
                    |_| fail(ctx, ErrorCode::RecoveryRequired, "cleanup budget invalid"),
                )?;
            old.cleanup(cleanup_budget, &AtomicBool::new(false))
                .map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "previous sandbox cleanup incomplete",
                    )
                })?;
            remove_staging(&owner_lock, old_scope.as_str(), ctx)?;
            let attestations = app
                .work
                .child("bundles")
                .and_then(|d| d.child(old_bundle.as_str()))
                .and_then(|d| d.child("validation"));
            match attestations {
                Err(hton_storage::Error::NotFound) => {}
                Err(_) => {
                    return Err(fail(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "unfinished validation attestation store unavailable",
                    ));
                }
                Ok(dir) => {
                    let lock = dir
                        .lock(Instant::now() + Duration::from_secs(5))
                        .map_err(|_| {
                            fail(
                                ctx,
                                ErrorCode::RecoveryRequired,
                                "unfinished attestation lock unavailable",
                            )
                        })?;
                    let name = format!("{}.json", old_scope.as_str());
                    if let Some(bytes) = lock.read(&name, 4 * 1024 * 1024).map_err(|_| {
                        fail(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "unfinished attestation unreadable",
                        )
                    })? {
                        let attestation: BundleValidationAttestation =
                            serde_json::from_slice(&bytes).map_err(|_| {
                                fail(
                                    ctx,
                                    ErrorCode::RecoveryRequired,
                                    "unfinished attestation corrupt",
                                )
                            })?;
                        if attestation.operation_id != pending.operation_id
                            || attestation.bundle_id != *old_bundle
                            || attestation.candidate_digest != *old_digest
                        {
                            return Err(fail(
                                ctx,
                                ErrorCode::RecoveryRequired,
                                "unfinished attestation identity drifted",
                            ));
                        }
                        lock.remove(&name, &bytes).map_err(|_| {
                            fail(
                                ctx,
                                ErrorCode::RecoveryRequired,
                                "unfinished attestation cleanup failed",
                            )
                        })?;
                    }
                }
            }
            // Recovery is an exact original operation identity; no new effects or attestation.
            let mut recovered = ctx.clone();
            recovered.operation_id = pending.operation_id.clone();
            recovered.actor = pending.actor.clone();
            app.journal.finish(
                &pending.transaction_id,
                TerminalStatus::Failed,
                vec![],
                &recovered,
            )?;
            if pending.operation_id == ctx.operation_id {
                return Err(fail(
                    ctx,
                    ErrorCode::OperationConflict,
                    "interrupted validation reconciled; use a new operation identity",
                ));
            }
        }
    }
    let tx = app.journal.prepare_intent(intent, vec![], ctx)?;
    let outcome = run(app, &input, &driver, scope, &tx, ctx);
    // Cleanup has its own narrow uncancelled recovery budget; it never runs production names.
    let cleanup_budget = Budget::new(Instant::now() + Duration::from_secs(30), 4 * 1024 * 1024)
        .map_err(|_| fail(ctx, ErrorCode::RecoveryRequired, "cleanup budget invalid"))?;
    let removed = driver
        .cleanup(cleanup_budget, &AtomicBool::new(false))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "sandbox cleanup incomplete; retry bundle.validate to reconcile",
            )
        })?;
    remove_staging(&owner_lock, scope, ctx)?;
    let checks = match outcome {
        Ok(checks) => checks,
        Err(error) => {
            app.journal
                .finish(&tx, TerminalStatus::Failed, vec![], ctx)?;
            return Err(error);
        }
    };
    // Source artifacts must still be exact after the entire qualification.
    let final_input = inputs(app, &id, ctx)?;
    if final_input.candidate != input.candidate || final_input.artifacts != input.artifacts {
        app.journal
            .finish(&tx, TerminalStatus::Failed, vec![], ctx)?;
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "candidate changed during isolated qualification",
        ));
    }
    let mut resources = removed
        .iter()
        .map(|r| format!("{}:{}", r.network, r.id))
        .collect::<Vec<_>>();
    resources.sort();
    let now = UnixMillis(app.control.now_ms());
    let mut image_digests = input
        .candidate
        .services
        .as_slice()
        .iter()
        .map(|s| {
            (
                ServiceId::new(s.service_id.as_str()).expect("wire ID"),
                ImageDigest::new(s.image.as_str().rsplit_once('@').expect("wire image").1)
                    .expect("wire digest"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    image_digests.insert(
        ServiceId::new("validation-client").expect("fixed ID"),
        ImageDigest::new(
            client_image
                .as_str()
                .rsplit_once('@')
                .expect("pinned client image")
                .1,
        )
        .expect("client digest"),
    );
    let attestation = BundleValidationAttestation {
        operation_id: ctx.operation_id.clone(),
        bundle_id: id.clone(),
        candidate_digest: Sha256Digest::new(input.candidate.manifest_digest.as_str())
            .expect("wire digest"),
        artifact_digests: input
            .candidate
            .artifacts
            .as_slice()
            .iter()
            .map(|a| {
                (
                    ArtifactId::new(a.artifact_id.as_str()).expect("wire ID"),
                    Sha256Digest::new(a.digest.as_str()).expect("wire digest"),
                )
            })
            .collect(),
        image_digests,
        runner_version: ObjectId::new("isolated-r1-v1").expect("fixed ID"),
        status: BundleValidationStatus::Passed,
        evidence: checks,
        cleanup: BundleValidationCleanup {
            completed: true,
            resources_digest: digest(&serde_json::to_vec(&resources).map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "cleanup receipt encoding failed",
                )
            })?),
            completed_at: now,
        },
        validated_at: now,
        expires_at: UnixMillis(now.0.saturating_add(300_000)),
    };
    let bytes = serde_json::to_vec_pretty(&attestation).map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "attestation serialization failed",
        )
    })?;
    let directory = app
        .work
        .child("bundles")
        .and_then(|d| d.child(id.as_str()))
        .and_then(|d| d.create_child("validation"))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "attestation store unavailable",
            )
        })?;
    publish(&directory, &format!("{scope}.json"), &bytes, ctx)?;
    app.journal
        .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
    Ok(
        json!({"status":"succeeded","operation":"bundle.validate","attestation":attestation,"externalReachability":"unverified"}),
    )
}
