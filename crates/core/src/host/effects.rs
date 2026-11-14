//! Concrete immutable preparation/apply composition over existing domain owners.
use super::{HostApplication, approvals, observations};
use crate::{
    apply::*,
    catalog::{ModuleCatalogue, ResolutionEvidence},
    config::{self, SecretAvailability, ValidationPorts},
    identities::IdentityBinding,
    journal::{self, Intent, TerminalStatus, Transaction, deadline},
    plan::{self, NativeTransitionBinding, PlanInputs},
    registry::RegistryPreconditions,
    render::{self, *},
    secrets::{ConsumerGrant, ProtectedMaterial, SecretRef},
};
use hton_contracts::{config::*, operations::*};
use hton_host::{
    Budget as HostBudget, HostObserver,
    docker::{
        ContainerId, ImageFacts, ImageReference, OwnedEngine, container_fingerprints,
        expected_container_fingerprints, native_state_fingerprint,
    },
};
use hton_runtime_contract as wire;
use hton_storage::SecureDir;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
struct ImageLock {
    images: BTreeMap<String, ImageMetadata>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageMetadata {
    reference: String,
    manifest_digest: ImageDigest,
}
struct OwnedSecret {
    reference: SecretRef,
    service: ServiceId,
    material: ProtectedMaterial,
}
struct AvailableSecrets(BTreeMap<SecretId, SecretVersion>);
impl SecretAvailability for AvailableSecrets {
    fn version(&self, id: &SecretId) -> Option<SecretVersion> {
        self.0.get(id).cloned()
    }
}
struct Cancel<'a> {
    app: &'a HostApplication,
    ctx: &'a OperationContext,
}
impl hton_host::Cancellation for Cancel<'_> {
    fn cancelled(&self) -> bool {
        self.app.control.is_cancelled(&self.ctx.cancellation)
    }
}
fn id(value: &str) -> ObjectId {
    ObjectId::new(value).expect("fixed ID")
}
fn service(value: &str) -> ServiceId {
    ServiceId::new(value).expect("fixed ID")
}
fn module(value: &str) -> ModuleId {
    ModuleId::new(value).expect("fixed ID")
}
fn fail(ctx: &OperationContext, code: ErrorCode, message: &'static str) -> Error {
    journal::failure(ctx, code, message)
}
fn budget(app: &HostApplication, ctx: &OperationContext) -> Result<HostBudget> {
    let remaining = ctx
        .deadline
        .0
        .saturating_sub(app.control.now_ms())
        .min(300_000);
    if remaining == 0 || app.control.is_cancelled(&ctx.cancellation) {
        return Err(fail(
            ctx,
            ErrorCode::OperationDeadline,
            "native preparation budget exhausted",
        ));
    }
    HostBudget::new(
        Instant::now() + Duration::from_millis(remaining),
        4 * 1024 * 1024,
    )
    .map_err(|_| {
        fail(
            ctx,
            ErrorCode::OperationDeadline,
            "native preparation budget invalid",
        )
    })
}
fn wire_digest(value: &Sha256Digest, ctx: &OperationContext) -> Result<wire::Sha256Digest> {
    wire::Sha256Digest::new(value.as_str()).map_err(|_| {
        fail(
            ctx,
            ErrorCode::ConfigSyntax,
            "native digest conversion invalid",
        )
    })
}
fn digest(value: &wire::Sha256Digest, ctx: &OperationContext) -> Result<Sha256Digest> {
    Sha256Digest::new(value.as_str()).map_err(|_| {
        fail(
            ctx,
            ErrorCode::ResourceDrift,
            "native digest conversion invalid",
        )
    })
}
fn items<T>(values: Vec<T>, ctx: &OperationContext) -> Result<wire::Items<T>> {
    wire::Items::new(values).map_err(|_| {
        fail(
            ctx,
            ErrorCode::OperationDenied,
            "runtime inventory exceeds bounded contract",
        )
    })
}
fn engine(value: wire::Engine, ctx: &OperationContext) -> Result<OwnedEngine> {
    match value {
        wire::Engine::Haproxy => Ok(OwnedEngine::Haproxy),
        wire::Engine::Caddy => Ok(OwnedEngine::Caddy),
        wire::Engine::Xray => Ok(OwnedEngine::Xray),
        wire::Engine::Unbound => Ok(OwnedEngine::Unbound),
        _ => Err(fail(
            ctx,
            ErrorCode::ModuleUnsupported,
            "no owned validator for foreign engine",
        )),
    }
}
fn renderers() -> BTreeMap<ServiceId, &'static dyn EngineRenderer> {
    static H: crate::haproxy_adapter::HaproxyRenderer = crate::haproxy_adapter::HaproxyRenderer;
    static C: crate::caddy_adapter::CaddyRenderer = crate::caddy_adapter::CaddyRenderer;
    static X: crate::xray_adapter::XrayRenderer = crate::xray_adapter::XrayRenderer;
    static U: crate::unbound_adapter::UnboundRenderer = crate::unbound_adapter::UnboundRenderer;
    BTreeMap::from([
        (service("haproxy"), &H as &dyn EngineRenderer),
        (service("caddy"), &C as &dyn EngineRenderer),
        (service("xray"), &X as &dyn EngineRenderer),
        (service("unbound"), &U as &dyn EngineRenderer),
    ])
}
struct VlessCheck;
impl ModuleCheck for VlessCheck {
    fn check(
        &self,
        settings: &ModuleSettings,
        graph: &DependencyGraph,
        observed: &RuntimeObservation,
    ) -> Vec<CheckIssue> {
        let mut issues = vec![];
        if !matches!(
            settings.settings.get("transport").and_then(Value::as_str),
            Some("ws" | "xhttp")
        ) || !["xray", "unbound"].iter().all(|name| {
            graph
                .service_consumers
                .get(&service(name))
                .is_some_and(|consumers| consumers.contains(&module("vless")))
        }) {
            issues.push(config::issue(
                ErrorCode::ConfigSemanticConflict,
                None,
                Some(module("vless")),
                "VLESS transport and shared engine closure are inconsistent",
            ));
        }
        for name in ["xray", "unbound"] {
            if !observed.resources.values().any(|resource| {
                resource.identity.service_id.as_str() == name
                    && !matches!(
                        resource.state,
                        ObservedState::Unknown | ObservedState::Failed
                    )
            }) {
                issues.push(config::issue(
                    ErrorCode::ModuleIncomplete,
                    None,
                    Some(module("vless")),
                    "VLESS requires qualified installed Xray and Unbound observations",
                ));
            }
        }
        issues
    }
}
struct Prepared {
    now: UnixMillis,
    context: OperationContext,
    catalogue: ModuleCatalogue,
    snapshot: NormalizedSnapshot,
    validation: CheckReport,
    graph: DependencyGraph,
    registry: RegistrySnapshot,
    runtime: wire::RuntimeState,
    observed: observations::ObservedResources,
    services: BTreeMap<ServiceId, RuntimeServiceBinding>,
    network: NetworkLayout,
    secrets: Vec<OwnedSecret>,
    credentials: Vec<CredentialBinding>,
    certificate: Option<QualifiedCertificate>,
    site_index: super::webroot::SiteIndex,
    anchor: Option<Vec<u8>>,
    rendered: Option<RenderedBundle>,
    images: BTreeMap<ServiceId, ImageFacts>,
    prior: BTreeMap<ServiceId, Sha256Digest>,
    accepted: Option<wire::CandidateBundle>,
    approvals: Vec<ApprovalEnvelope>,
    verified_ids: BTreeSet<ApprovalId>,
    credential_revision: u64,
}
impl Prepared {
    fn with_render<T>(&self, operation: impl FnOnce(&RenderInputs<'_>) -> Result<T>) -> Result<T> {
        let scoped: Vec<_> = self
            .secrets
            .iter()
            .map(|secret| ScopedSecret {
                reference: secret.reference.clone(),
                service_id: secret.service.clone(),
                material: &secret.material,
            })
            .collect();
        let anchor = self.anchor.as_ref().map(|bytes| PreparedTrustAnchor {
            bytes,
            digest: config::digest(bytes),
        });
        operation(&RenderInputs {
            snapshot: &self.snapshot,
            graph: &self.graph,
            catalogue: &self.catalogue,
            registry: &self.registry,
            services: &self.services,
            network: &self.network,
            secrets: &scoped,
            credentials: &self.credentials,
            certificate: self.certificate.as_ref(),
            dns_anchor: anchor.as_ref(),
            site_body: self.site_index.bytes(),
            site_index_digest: self.site_index.content_digest().clone(),
            now: self.now,
            operation_id: self.context.operation_id.clone(),
        })
    }
    fn expected(&self, plan: &EffectPlan) -> ExpectedInputs {
        ExpectedInputs {
            read_set: plan.read_set.clone(),
            registry_revision: self.registry.revision,
            runtime_revision: StateRevision(self.runtime.revision),
            run_epoch: RunEpoch(self.runtime.run_epoch),
            resources: plan.resource_before.clone(),
        }
    }
}
fn references(value: &Value, found: &mut BTreeSet<SecretId>, ctx: &OperationContext) -> Result<()> {
    match value {
        Value::Object(map) => {
            if let Some(reference) = map.get("secretRef") {
                let id = reference
                    .as_str()
                    .and_then(config::canonical_secret_id)
                    .ok_or_else(|| {
                        fail(
                            ctx,
                            ErrorCode::ConfigSyntax,
                            "configured secret reference invalid",
                        )
                    })?;
                found.insert(id);
            }
            for child in map.values() {
                references(child, found, ctx)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                references(child, found, ctx)?;
            }
        }
        _ => (),
    }
    Ok(())
}
fn add_secret(
    app: &HostApplication,
    secrets: &mut Vec<OwnedSecret>,
    snapshot: &mut NormalizedSnapshot,
    reference: SecretRef,
    consumer: ServiceId,
    scope: ScopeId,
    ctx: &OperationContext,
) -> Result<()> {
    if let Some(old) = snapshot
        .desired
        .read_set
        .secret_versions
        .insert(reference.secret_id.clone(), reference.version.clone())
    {
        if old != reference.version {
            return Err(fail(
                ctx,
                ErrorCode::PlanStale,
                "one logical secret has conflicting selected versions",
            ));
        }
    }
    if secrets
        .iter()
        .any(|secret| secret.reference == reference && secret.service == consumer)
    {
        return Ok(());
    }
    let material = app.secrets.resolve(
        &reference,
        &ConsumerGrant {
            consumer: consumer.clone(),
            scope,
        },
        ctx,
    )?;
    secrets.push(OwnedSecret {
        reference,
        service: consumer,
        material,
    });
    Ok(())
}
fn accepted(
    app: &HostApplication,
    state: &wire::RuntimeState,
    ctx: &OperationContext,
) -> Result<Option<wire::CandidateBundle>> {
    let Some(bundle) = &state.accepted else {
        return Ok(None);
    };
    let bytes = app
        .work
        .child("bundles")
        .and_then(|root| root.child(bundle.as_str()))
        .and_then(|dir| dir.read("manifest.json", wire::MAX_WIRE_BYTES))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "accepted manifest unavailable",
            )
        })?
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "accepted manifest missing",
            )
        })?;
    let candidate: wire::CandidateBundle = wire::decode(&bytes).map_err(|_| {
        fail(
            ctx,
            ErrorCode::RecoveryRequired,
            "accepted manifest corrupt",
        )
    })?;
    if &candidate.bundle_id != bundle
        || candidate.instance_id != state.instance_id
        || wire::candidate_digest(&candidate).ok().as_ref() != Some(&candidate.manifest_digest)
    {
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "accepted manifest identity invalid",
        ));
    }
    Ok(Some(candidate))
}
fn prior(
    app: &HostApplication,
    ctx: &OperationContext,
) -> Result<BTreeMap<ServiceId, Sha256Digest>> {
    type Active = wire::ActiveServiceArtifacts;
    let services = app
        .work
        .child("runtime")
        .and_then(|root| root.child("services"))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "service artifact inventory unavailable",
            )
        })?;
    let mut result = BTreeMap::new();
    for name in ["haproxy", "caddy", "xray", "unbound"] {
        let directory = match services.child(name) {
            Ok(directory) => directory,
            Err(hton_storage::Error::NotFound) => continue,
            Err(_) => {
                return Err(fail(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "service artifact state unsafe",
                ));
            }
        };
        if let Some(bytes) = directory.read("active.json", 65536).map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "active service metadata unavailable",
            )
        })? {
            let active: Active = wire::decode(&bytes).map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "active service metadata corrupt",
                )
            })?;
            if !active.loaded {
                return Err(fail(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "native artifact generation has not been acknowledged",
                ));
            }
            if active.service_id.as_str() != name {
                return Err(fail(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "active service metadata mismatched",
                ));
            }
            let _ = &active.bundle_id;
            result.insert(service(name), digest(&active.digest, ctx)?);
        }
    }
    Ok(result)
}
fn layout(instance: &InstanceId) -> NetworkLayout {
    let native = wire::private_network(
        &wire::InstanceId::new(instance.as_str()).expect("validated logical ID"),
    );
    NetworkLayout {
        network_id: id(&native.network_id),
        subnet: native.subnet,
        prefix: native.prefix,
        gateway: native.gateway,
        endpoints: native
            .endpoints
            .into_iter()
            .map(|(name, endpoint)| (service(name.as_str()), endpoint))
            .collect(),
        foreign_backends: BTreeMap::new(),
        public_addresses: BTreeSet::new(),
    }
}

fn assemble(
    app: &HostApplication,
    baseline: Option<&EffectPlan>,
    change: Option<(&IdentityBinding, bool)>,
    selection: Option<&crate::selection::SelectionCandidate>,
    ctx: &OperationContext,
) -> Result<Prepared> {
    let now = UnixMillis(app.control.now_ms());
    let current = app.desired(ctx)?;
    let loaded = if let Some(candidate) = selection {
        let root = id("root");
        let prospective = candidate.read_result();
        let mut before = current.read_set.clone();
        let mut after = prospective.read_set.clone();
        let root_revision = before.objects.remove(&root).map(|entry| entry.revision);
        after.objects.remove(&root);
        if !current.errors.is_empty()
            || current.read_set.coherence != ReadCoherence::Coherent
            || root_revision.as_ref() != Some(&candidate.change().expected_root_revision)
            || before != after
        {
            return Err(fail(
                ctx,
                ErrorCode::PlanStale,
                "prospective selection baseline changed before effect preparation",
            ));
        }
        prospective.clone()
    } else {
        current
    };
    if !loaded.errors.is_empty() {
        return Err(fail(
            ctx,
            ErrorCode::ConfigSyntax,
            "desired tree is invalid; accepted runtime retained",
        ));
    }
    let desired = loaded.snapshot.ok_or_else(|| {
        fail(
            ctx,
            ErrorCode::ConfigSyntax,
            "coherent desired snapshot unavailable",
        )
    })?;
    let catalogue = ModuleCatalogue::builtin().map_err(|_| {
        fail(
            ctx,
            ErrorCode::ConfigSyntax,
            "trusted catalogue unavailable",
        )
    })?;
    let normalized = config::normalize(&desired, &catalogue);
    if normalized.blocking {
        return Err(fail(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "desired normalization failed",
        ));
    }
    let mut snapshot = normalized.normalized_snapshot.ok_or_else(|| {
        fail(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "normalized desired snapshot missing",
        )
    })?;
    let site_index = match super::webroot::inspect_index(ctx)? {
        super::webroot::SiteIndexObservation::Present(index) => index,
        super::webroot::SiteIndexObservation::Absent { .. } => {
            return Err(fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "host site index is not prepared",
            ));
        }
    };
    let registry = app.registry(ctx)?;
    let runtime = app.runtime(ctx)?;
    if runtime.maintenance.is_some()
        || runtime.lease.as_ref().is_some_and(|lease| {
            matches!(
                lease.state,
                wire::LeaseState::Armed
                    | wire::LeaseState::Activating
                    | wire::LeaseState::Verifying
                    | wire::LeaseState::Compensating
                    | wire::LeaseState::RollingBack
                    | wire::LeaseState::RecoveryRequired
            )
        })
    {
        return Err(fail(
            ctx,
            ErrorCode::RecoveryRequired,
            "runtime maintenance or unfinished trial requires reconciliation",
        ));
    }
    let observed = observations::collect(app, ctx)?;
    if !observed.issues.is_empty() {
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "registered native observations are incomplete or drifted",
        ));
    }
    let domains: Vec<String> = snapshot.desired.platform.settings["core"]["edge"]["domains"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let vless = snapshot
        .desired
        .features
        .get(&module("vless"))
        .is_some_and(|selection| selection.enabled);
    if snapshot
        .desired
        .features
        .get(&module("foreign-amnezia"))
        .is_some_and(|selection| selection.enabled)
    {
        return Err(fail(
            ctx,
            ErrorCode::ModuleUnsupported,
            "exact installed Amnezia passthrough qualification is required before candidate preparation",
        ));
    }
    let mut required = BTreeSet::new();
    if !domains.is_empty() {
        references(
            &snapshot.desired.platform.settings["core"],
            &mut required,
            ctx,
        )?;
    }
    for (name, selection) in &snapshot.desired.features {
        if selection.enabled {
            if let Some(settings) = snapshot.desired.module_settings.get(name) {
                references(
                    &Value::Object(settings.settings.clone().into_iter().collect()),
                    &mut required,
                    ctx,
                )?;
            }
        }
    }
    let mut available = AvailableSecrets(BTreeMap::new());
    for name in required {
        if let Some(version) = app.secrets.current_version(&name, ctx)? {
            available.0.insert(name, version);
        }
    }
    let mut secrets = vec![];
    let mut resolution = ResolutionEvidence {
        observations: observed.runtime.clone(),
        now,
        ..Default::default()
    };
    if vless {
        let name = snapshot
            .desired
            .module_settings
            .get(&module("vless"))
            .and_then(|settings| settings.settings.get("path"))
            .and_then(|path| path.get("secretRef"))
            .and_then(Value::as_str)
            .and_then(config::canonical_secret_id)
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "VLESS path secret reference missing",
                )
            })?;
        let version = available.version(&name).ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "VLESS path secret is not provisioned",
            )
        })?;
        let reference = SecretRef {
            secret_id: name,
            version,
        };
        add_secret(
            app,
            &mut secrets,
            &mut snapshot,
            reference.clone(),
            service("caddy"),
            ScopeId::new("module.vless").expect("fixed scope"),
            ctx,
        )?;
        add_secret(
            app,
            &mut secrets,
            &mut snapshot,
            reference.clone(),
            service("xray"),
            ScopeId::new("module.vless").expect("fixed scope"),
            ctx,
        )?;
        let path = std::str::from_utf8(
            secrets
                .iter()
                .find(|secret| secret.reference == reference)
                .expect("just resolved")
                .material
                .expose(),
        )
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::ConfigSyntax,
                "VLESS path material encoding invalid",
            )
        })?;
        let path = path.strip_prefix('/').unwrap_or(path);
        if !(16..=128).contains(&path.len())
            || !path
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(fail(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "VLESS path material is not an opaque URL-safe token",
            ));
        }
        resolution.resolved_routes.insert(
            module("vless"),
            domains
                .iter()
                .map(|hostname| RouteClaim {
                    owner: module("vless"),
                    listener_service: service("caddy"),
                    matcher: RouteMatch::Http {
                        hostname: hostname.clone(),
                        path_prefix: format!("/{path}"),
                    },
                    target_service: service("xray"),
                })
                .collect(),
        );
    }
    let mut certificate = None;
    if !domains.is_empty() {
        let inventory = crate::host_crypto::application::certificate_inventory(app, ctx)?;
        let prepared = inventory.prepared(ctx)?.ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "configured HTTPS certificate has not been prepared",
            )
        })?;
        let verified = &prepared.candidate.certificate;
        if !verified.chain_trusted
            || !verified.private_key_matches
            || verified.not_before.0 > now.0
            || verified.not_after.0 <= now.0
        {
            return Err(fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "prepared certificate is not currently trusted/usable",
            ));
        }
        let qualified = QualifiedCertificate {
            service_id: service("caddy"),
            chain: prepared.chain.clone(),
            key: prepared.key.clone(),
            names: verified
                .names
                .iter()
                .map(|name| name.as_str().to_owned())
                .collect(),
            valid_until: verified.not_after,
            qualification: verified.fingerprint.clone(),
        };
        if domains.iter().any(|name| !qualified.covers(name)) {
            return Err(fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "prepared certificate does not cover every configured domain",
            ));
        }
        add_secret(
            app,
            &mut secrets,
            &mut snapshot,
            prepared.chain,
            service("caddy"),
            prepared.scope.clone(),
            ctx,
        )?;
        add_secret(
            app,
            &mut secrets,
            &mut snapshot,
            prepared.key,
            service("caddy"),
            prepared.scope,
            ctx,
        )?;
        let root = desired.read_set.objects.get(&id("root")).ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ConfigSyntax,
                "root content revision missing",
            )
        })?;
        resolution.publication_evidence.push(Evidence {
            check_id: CheckId::new("certificates.native").expect("fixed ID"),
            module_id: Some(module("core")),
            vantage: Vantage::NativeValidation,
            outcome: EvidenceOutcome::Pass,
            timestamp: now,
            input_fingerprint: root.revision.digest.clone(),
            details: SafeFields::default(),
        });
        certificate = Some(qualified);
    }
    let graph = catalogue.resolve(&snapshot, ReleasePhase::R1, &resolution);
    if !graph.conflicts.is_empty() {
        return Err(fail(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "resolved graph has missing qualification or conflicting claims",
        ));
    }
    // Unknown/co-tenant host listeners never acquire implicit HTON authority.
    // Complete collision evidence is checked before any local consent writes.
    let mut read_ctx = ctx.clone();
    read_ctx.execution_mode = ExecutionMode::ReadOnly;
    let reservations =
        observations::reservation_evidence_for(&registry, app.control.as_ref(), &read_ctx)?;
    let observed_now = app.control.now_ms();
    if !reservations.complete
        || reservations.observed_at.0 > observed_now
        || reservations.valid_until.0 <= observed_now
        || !observations::listener_conflicts(&graph, &reservations).is_empty()
    {
        return Err(fail(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "complete fresh host listener evidence is missing or conflicts with the candidate",
        ));
    }
    let authorized = approvals::authorize(app, &graph, ctx)?;
    let module_check = VlessCheck;
    let checks = BTreeMap::from([(module("vless"), &module_check as &dyn ModuleCheck)]);
    let ports = ValidationPorts {
        release: ReleasePhase::R1,
        resolution: &resolution,
        module_checks: checks,
        secret_checks: &available,
    };
    let validated = config::validate(
        &snapshot.desired,
        &catalogue,
        &registry,
        &authorized.approvals,
        &ports,
    );
    if validated.blocking {
        return Err(fail(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "configuration semantic checks failed; no effect planned",
        ));
    }
    let extra_versions = snapshot.desired.read_set.secret_versions.clone();
    snapshot = validated.normalized_snapshot.clone().ok_or_else(|| {
        fail(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "validated snapshot missing",
        )
    })?;
    snapshot
        .desired
        .read_set
        .secret_versions
        .extend(extra_versions);
    let identity_store = crate::host_crypto::identities::FileIdentityInventory::open(app);
    let identity_revision = identity_store.revision(ctx)?;
    let mut bindings = identity_store.bindings(ctx)?;
    let mut all_refs = BTreeSet::new();
    for binding in &bindings {
        if binding.service_id.as_str() != "xray"
            || binding.scope.as_str() != "module.vless"
            || !binding.secret.secret_id.as_str().starts_with("identity.")
            || !all_refs.insert((
                binding.secret.secret_id.clone(),
                binding.secret.version.clone(),
            ))
        {
            return Err(fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "identity grant inventory is ambiguous or unqualified",
            ));
        }
    }
    if let Some((binding, include)) = change {
        if !bindings.iter().any(|current| current == binding) {
            return Err(fail(
                ctx,
                ErrorCode::ResourceDrift,
                "prospective identity does not match current protected inventory",
            ));
        }
        if !include {
            bindings.retain(|current| current != binding);
        }
    }
    if let Some(plan) = baseline {
        for (name, version) in plan
            .secret_versions
            .iter()
            .filter(|(name, _)| name.as_str().starts_with("identity."))
        {
            if !all_refs.contains(&(name.clone(), version.clone())) {
                return Err(fail(
                    ctx,
                    ErrorCode::PlanStale,
                    "staged credential grant no longer matches protected identity inventory",
                ));
            }
        }
        bindings.retain(|binding| {
            plan.secret_versions.get(&binding.secret.secret_id) == Some(&binding.secret.version)
        });
    }
    let mut credentials = Vec::new();
    if vless {
        for binding in bindings {
            add_secret(
                app,
                &mut secrets,
                &mut snapshot,
                binding.secret.clone(),
                binding.service_id.clone(),
                binding.scope,
                ctx,
            )?;
            credentials.push(CredentialBinding {
                module_id: module("vless"),
                identity_id: IdentityId::new(binding.identity_id.as_str()).map_err(|_| {
                    fail(ctx, ErrorCode::ConfigSyntax, "credential identity invalid")
                })?,
                reference: binding.secret,
            });
        }
    }
    snapshot.desired.read_set.objects.insert(
        id("state.identities"),
        ReadDependency {
            revision: identity_revision.clone(),
            format_version: FormatVersion(1),
        },
    );
    snapshot
        .supported_versions
        .insert(id("state.identities"), FormatVersion(1));
    let policy = app
        .work
        .read("local-policy.json", 65536)
        .map_err(|_| fail(ctx, ErrorCode::IoUnavailable, "owned policy unavailable"))?
        .ok_or_else(|| fail(ctx, ErrorCode::ApprovalRequired, "owned policy missing"))?;
    snapshot.desired.read_set.objects.insert(
        id("state.localPolicy"),
        ReadDependency {
            revision: config::content_revision(id("state.localPolicy"), &policy),
            format_version: FormatVersion(1),
        },
    );
    snapshot
        .supported_versions
        .insert(id("state.localPolicy"), FormatVersion(1));
    let image_lock: ImageLock =
        serde_json::from_str(include_str!("../../../../platform/images.lock.json"))
            .map_err(|_| fail(ctx, ErrorCode::ConfigSyntax, "trusted image lock invalid"))?;
    let host = HostObserver::new();
    let cancel = Cancel { app, ctx };
    let mut images = BTreeMap::new();
    let mut services = BTreeMap::new();
    let restart_owners: Vec<_> = registry
        .entries
        .values()
        .filter(|entry| entry.membership == MembershipState::Installed)
        .flat_map(|entry| &entry.resources)
        .filter(|resource| resource.service_id.as_str() == "runtime")
        .map(|resource| {
            wire::ResourceId::new(resource.resource_id.as_str())
                .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "restart owner ID invalid"))
        })
        .collect::<Result<_>>()?;
    for name in ["haproxy", "caddy", "xray", "unbound"] {
        let resource = registry
            .entries
            .get(&module("core"))
            .and_then(|entry| {
                entry
                    .resources
                    .iter()
                    .find(|resource| resource.service_id.as_str() == name)
            })
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "owned physical engine setup is incomplete",
                )
            })?;
        let native = observed
            .native
            .get(&resource.resource_id)
            .cloned()
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "owned engine native identity unavailable",
                )
            })?;
        let metadata = image_lock.images.get(name).ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "selected engine image pin missing",
            )
        })?;
        let reference = ImageReference::new(metadata.reference.clone()).map_err(|_| {
            fail(
                ctx,
                ErrorCode::ConfigSyntax,
                "invalid immutable engine image reference",
            )
        })?;
        let facts = host
            .docker()
            .inspect_image(&reference, budget(app, ctx)?, &cancel)
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "pinned engine image is not cached/qualified; up never pulls",
                )
            })?;
        if facts.architecture != "amd64" || facts.os != "linux" {
            return Err(fail(
                ctx,
                ErrorCode::ModuleUnsupported,
                "cached engine platform not qualified",
            ));
        }
        let installed = registry.entries.get(&module("core")).and_then(|entry| {
            entry
                .installed_artifact_versions
                .get(&ArtifactId::new(format!("image.{name}")).expect("fixed ID"))
        });
        if installed != Some(&metadata.manifest_digest) {
            return Err(fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "installed engine pin differs; explicit image update required",
            ));
        }
        let owners = restart_owners.clone();
        services.insert(
            service(name),
            RuntimeServiceBinding {
                structural_fingerprint: resource.fingerprint.clone(),
                resource: native,
                image: wire::ImagePin::new(metadata.reference.clone()).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "native image pin conversion invalid",
                    )
                })?,
                restart_owner_resources: owners,
            },
        );
        images.insert(service(name), facts);
    }
    let accepted = accepted(app, &runtime, ctx)?;
    let proposed: BTreeSet<_> = snapshot
        .desired
        .read_set
        .secret_versions
        .iter()
        .filter(|(name, _)| name.as_str().starts_with("identity."))
        .map(|(name, version)| (name.as_str(), version.as_str()))
        .collect();
    let removes = accepted.as_ref().is_some_and(|candidate| {
        candidate
            .secrets
            .as_slice()
            .iter()
            .filter(|binding| binding.secret_id.as_str().starts_with("identity."))
            .any(|binding| {
                !proposed.contains(&(binding.secret_id.as_str(), binding.version.as_str()))
            })
    });
    if removes {
        return Err(fail(
            ctx,
            ErrorCode::RecoveryRequired,
            "credential removal/replacement requires a qualified policy-clean fallback before effects",
        ));
    }
    let credential_revision = runtime.fences.credential_revision;
    let anchor = if graph.service_consumers.contains_key(&service("unbound")) {
        Some(super::trustanchor::prepared(app, ctx)?)
    } else {
        None
    };
    let mut prepared = Prepared {
        now,
        context: authorized.context,
        catalogue,
        snapshot,
        validation: validated,
        graph,
        registry,
        runtime,
        observed,
        services,
        network: layout(&app.installation.instance_id),
        secrets,
        credentials,
        certificate,
        site_index,
        anchor,
        rendered: None,
        images,
        prior: prior(app, ctx)?,
        accepted,
        approvals: authorized.approvals,
        verified_ids: authorized.verified_ids,
        credential_revision,
    };
    let rendered = prepared.with_render(|inputs| render::preview(inputs, &renderers()))?;
    if identity_store.revision(ctx)? != identity_revision {
        return Err(fail(
            ctx,
            ErrorCode::PlanStale,
            "identity inventory changed during preparation",
        ));
    }
    prepared.rendered = Some(rendered);
    Ok(prepared)
}

fn rendered<'a>(prepared: &'a Prepared, ctx: &OperationContext) -> Result<&'a RenderedBundle> {
    prepared.rendered.as_ref().ok_or_else(|| {
        fail(
            ctx,
            ErrorCode::ModuleIncomplete,
            "native render result unavailable",
        )
    })
}
fn compose(prepared: &Prepared, ctx: &OperationContext) -> Result<Value> {
    let artifact = rendered(prepared, ctx)?
        .artifacts
        .iter()
        .find(|artifact| {
            artifact.service_id.as_str() == "haproxy" && artifact.name.as_str() == "compose.json"
        })
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "final Compose metadata unavailable",
            )
        })?;
    serde_json::from_slice(artifact.expose()).map_err(|_| {
        fail(
            ctx,
            ErrorCode::ConfigSyntax,
            "rendered Compose metadata invalid",
        )
    })
}
fn active_digest(prepared: &Prepared, name: &ServiceId, ctx: &OperationContext) -> Result<String> {
    let native: Vec<_> = rendered(prepared, ctx)?
        .artifacts
        .iter()
        .filter(|artifact| &artifact.service_id == name && artifact.name.as_str() != "compose.json")
        .map(|artifact| {
            Ok((
                wire::ArtifactId::new(artifact.artifact_id.as_str()).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "native artifact identity invalid",
                    )
                })?,
                wire_digest(&artifact.digest(), ctx)?,
            ))
        })
        .collect::<Result<_>>()?;
    let compose = compose(prepared, ctx)?;
    let spec = compose
        .get("services")
        .and_then(|services| services.get(name.as_str()))
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "native launch spec unavailable",
            )
        })?;
    wire::service_revision(&native, spec)
        .map(|digest| digest.as_str().to_owned())
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::ConfigSyntax,
                "native service revision invalid",
            )
        })
}

fn current_config(
    app: &HostApplication,
    name: &ServiceId,
    ctx: &OperationContext,
) -> Result<Option<String>> {
    let directory = match app
        .work
        .child("runtime")
        .and_then(|root| root.child("services"))
        .and_then(|root| root.child(name.as_str()))
    {
        Ok(directory) => directory,
        Err(hton_storage::Error::NotFound) => return Ok(None),
        Err(_) => {
            return Err(fail(
                ctx,
                ErrorCode::IoUnavailable,
                "native service metadata unavailable",
            ));
        }
    };
    let Some(bytes) = directory.read("active.json", 65536).map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "native service metadata unreadable",
        )
    })?
    else {
        return Ok(None);
    };
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| {
        fail(
            ctx,
            ErrorCode::RecoveryRequired,
            "native service metadata corrupt",
        )
    })?;
    if value.get("serviceId").and_then(Value::as_str) != Some(name.as_str()) {
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "native service metadata identity mismatch",
        ));
    }
    if value.get("loaded").and_then(Value::as_bool) != Some(true) {
        return Err(fail(
            ctx,
            ErrorCode::RecoveryRequired,
            "native generation has not acknowledged installed artifacts",
        ));
    }
    let hash = value
        .get("digest")
        .and_then(Value::as_str)
        .and_then(|value| Sha256Digest::new(value).ok())
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "native service digest invalid",
            )
        })?;
    Ok(Some(hash.as_str().to_owned()))
}
fn restart_token(policy: Option<&RestartPolicy>) -> Result<&'static str> {
    Ok(match policy {
        Some(RestartPolicy::DockerNo | RestartPolicy::SystemdDisabled) => "disabled",
        Some(RestartPolicy::DockerAlways | RestartPolicy::SystemdEnabled) => "always",
        Some(RestartPolicy::DockerUnlessStopped) => "unless_stopped",
        Some(RestartPolicy::DockerOnFailure { .. }) => "on_failure",
        None => "external",
    })
}
struct NativeState {
    structural: String,
    status: &'static str,
    restart: &'static str,
    config: Option<String>,
    identity: String,
    fingerprint: Sha256Digest,
}
fn calculate(
    app: &HostApplication,
    prepared: &Prepared,
    baseline: Option<&EffectPlan>,
    ctx: &OperationContext,
) -> Result<EffectPlan> {
    let rendered = rendered(prepared, ctx)?;
    let compose = compose(prepared, ctx)?;
    let images = prepared
        .services
        .iter()
        .map(|(name, binding)| {
            let pin = binding
                .image
                .as_str()
                .rsplit_once('@')
                .map(|(_, pin)| pin)
                .ok_or_else(|| fail(ctx, ErrorCode::ConfigSyntax, "engine image pin missing"))?;
            Ok((
                name.clone(),
                ImageDigest::new(pin).map_err(|_| {
                    fail(ctx, ErrorCode::ConfigSyntax, "engine image digest invalid")
                })?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut recreate_services = BTreeSet::new();
    for (name, facts) in prepared.images.iter().filter(|(name, _)| {
        compose
            .get("services")
            .and_then(|services| services.get(name.as_str()))
            .is_some()
    }) {
        let spec = compose
            .get("services")
            .and_then(|services| services.get(name.as_str()))
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "selected service Compose missing",
                )
            })?;
        let expected = expected_container_fingerprints(spec, facts, None, "stopped", "disabled")
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "native desired launch cannot be qualified",
                )
            })?;
        if prepared
            .services
            .get(name)
            .is_some_and(|binding| binding.structural_fingerprint.as_str() != expected.structural)
        {
            recreate_services.insert(name.clone());
        }
    }
    let accepted = prepared
        .runtime
        .accepted
        .as_ref()
        .map(|value| BundleId::new(value.as_str()))
        .transpose()
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "accepted identity conversion invalid",
            )
        })?;
    if accepted.is_none()
        && prepared
            .observed
            .runtime
            .resources
            .values()
            .any(|resource| resource.state == ObservedState::Running)
    {
        return Err(fail(
            ctx,
            ErrorCode::RecoveryRequired,
            "first installation has unaccepted running resources; explicit stopped baseline required",
        ));
    }
    let mut inputs = PlanInputs {
        catalogue: &prepared.catalogue,
        operation_id: ctx.operation_id.clone(),
        actor: &prepared.context.actor,
        now: prepared.now,
        expires_at: baseline
            .map(|plan| plan.expires_at)
            .unwrap_or(UnixMillis(prepared.now.0.saturating_add(300_000))),
        run_intent: match prepared.runtime.run_intent {
            wire::RunIntent::Running => RunIntent::Running,
            wire::RunIntent::Stopped => RunIntent::Stopped,
        },
        run_epoch: RunEpoch(prepared.runtime.run_epoch),
        runtime_revision: StateRevision(prepared.runtime.revision),
        accepted_bundle: accepted.clone(),
        prior_artifacts: prepared.prior.clone(),
        target_artifacts: rendered.service_artifact_digests.clone(),
        images,
        verified_approval_ids: prepared.verified_ids.clone(),
        rollback_target: match accepted {
            Some(bundle_id) => RollbackTarget::Accepted { bundle_id },
            None => RollbackTarget::PreInstall { originals: vec![] },
        },
        native_transitions: BTreeMap::new(),
        recreate_services,
        start_requested: false,
    };
    let requirements = plan::prepare_actions(
        &prepared.snapshot,
        &prepared.graph,
        &prepared.observed.runtime,
        &prepared.registry,
        &prepared.snapshot.desired.read_set,
        &prepared.approvals,
        &inputs,
    )?;
    let mut states = BTreeMap::<ResourceId, NativeState>::new();
    for action in &requirements {
        if !states.contains_key(&action.resource_id) {
            let observation = prepared
                .observed
                .runtime
                .resources
                .get(&action.resource_id)
                .ok_or_else(|| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "planned resource observation missing",
                    )
                })?;
            let native = prepared
                .observed
                .native
                .get(&action.resource_id)
                .ok_or_else(|| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "planned native identity missing",
                    )
                })?;
            let status = match observation.state {
                ObservedState::Running => "running",
                ObservedState::Stopped => "stopped",
                ObservedState::Failed => "failed",
                ObservedState::Unknown => "unknown",
            };
            let restart = restart_token(observation.restart_policy.as_ref())?;
            let config = current_config(app, &observation.identity.service_id, ctx)?;
            let fingerprint = native_state_fingerprint(
                native.identity_digest.as_str(),
                observation.identity.fingerprint.as_str(),
                status,
                restart,
                config.as_deref(),
            )
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "observed native state cannot be fingerprinted",
                )
            })?;
            if fingerprint != native.fingerprint.as_str() {
                return Err(fail(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "observed native state metadata differs",
                ));
            }
            states.insert(
                action.resource_id.clone(),
                NativeState {
                    structural: observation.identity.fingerprint.as_str().to_owned(),
                    status,
                    restart,
                    config,
                    identity: native.identity_digest.as_str().to_owned(),
                    fingerprint: Sha256Digest::new(fingerprint).map_err(|_| {
                        fail(ctx, ErrorCode::ResourceDrift, "native digest invalid")
                    })?,
                },
            );
        }
        let observation = &prepared.observed.runtime.resources[&action.resource_id];
        let name = &observation.identity.service_id;
        let state = states
            .get_mut(&action.resource_id)
            .expect("initialized native chain");
        let before = state.fingerprint.clone();
        let inverse_kind = match action.kind {
            ActionKind::Start | ActionKind::Restart | ActionKind::Reload | ActionKind::Recreate => {
                let spec = compose
                    .get("services")
                    .and_then(|services| services.get(name.as_str()))
                    .ok_or_else(|| {
                        fail(
                            ctx,
                            ErrorCode::ModuleIncomplete,
                            "activation Compose missing",
                        )
                    })?;
                let image = prepared.images.get(name).ok_or_else(|| {
                    fail(
                        ctx,
                        ErrorCode::ModuleIncomplete,
                        "activation cached image missing",
                    )
                })?;
                let target = active_digest(prepared, name, ctx)?;
                let expected = expected_container_fingerprints(
                    spec,
                    image,
                    Some(&target),
                    "running",
                    state.restart,
                )
                .map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "target native transition is not derivable",
                    )
                })?;
                state.structural = expected.structural;
                state.status = "running";
                state.config = Some(target);
                match action.kind {
                    ActionKind::Start => ActionKind::Stop,
                    kind => kind,
                }
            }
            ActionKind::Stop => {
                state.status = "stopped";
                ActionKind::Start
            }
            ActionKind::DisableAutostart => {
                state.restart = "disabled";
                ActionKind::RestoreAutostart
            }
            ActionKind::RestoreAutostart => {
                state.restart = "unless_stopped";
                ActionKind::DisableAutostart
            }
            _ => {
                return Err(fail(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "planned effect has no qualified native transition driver",
                ));
            }
        };
        let after = Sha256Digest::new(
            native_state_fingerprint(
                &state.identity,
                &state.structural,
                state.status,
                state.restart,
                state.config.as_deref(),
            )
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "target native fingerprint invalid",
                )
            })?,
        )
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "target native digest invalid",
            )
        })?;
        let inverse_id = StepId::new(format!(
            "inverse.{}",
            &config::digest(action.action_id.as_str().as_bytes()).as_str()[7..39]
        ))
        .expect("bounded digest ID");
        let compensation = Action {
            action_id: inverse_id,
            kind: inverse_kind,
            resource_id: action.resource_id.clone(),
            dependencies: BTreeSet::new(),
            preconditions: vec![Condition::ResourceFingerprint {
                resource_id: action.resource_id.clone(),
                fingerprint: after.clone(),
            }],
            postconditions: vec![Condition::ResourceFingerprint {
                resource_id: action.resource_id.clone(),
                fingerprint: before.clone(),
            }],
            timeout: action.timeout,
            compensation_ref: None,
        };
        inputs.native_transitions.insert(
            action.action_id.clone(),
            NativeTransitionBinding {
                before,
                after: after.clone(),
                compensation,
            },
        );
        state.fingerprint = after;
    }
    // Reverse every dependency edge for exact reverse-topological compensation.
    for action in &requirements {
        for dependency in &action.dependencies {
            let inverse = inputs
                .native_transitions
                .get(&action.action_id)
                .map(|binding| binding.compensation.action_id.clone())
                .ok_or_else(|| fail(ctx, ErrorCode::ResourceDrift, "native inverse missing"))?;
            inputs
                .native_transitions
                .get_mut(dependency)
                .ok_or_else(|| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "native dependency inverse missing",
                    )
                })?
                .compensation
                .dependencies
                .insert(inverse);
        }
    }
    plan::plan(
        &prepared.snapshot,
        &prepared.graph,
        &prepared.observed.runtime,
        &prepared.registry,
        &prepared.snapshot.desired.read_set,
        &prepared.approvals,
        &inputs,
    )
}

pub fn prepare(app: &HostApplication, ctx: &OperationContext) -> Result<EffectPlan> {
    let prepared = assemble(app, None, None, None, ctx)?;
    calculate(app, &prepared, None, ctx)
}
/// Explicit public cutover applies the saved declarative target. The optional
/// CLI port is an assertion, never a second configuration writer.
pub fn prepare_publication(
    app: &HostApplication,
    requested_port: Option<u16>,
    ctx: &OperationContext,
) -> Result<(EffectPlan, u16)> {
    let prepared = assemble(app, None, None, None, ctx)?;
    let listeners = prepared
        .graph
        .listener_claims
        .iter()
        .filter(|claim| {
            claim.namespace == ListenerNamespace::Host
                && claim.public
                && claim.service_id.as_str() == "haproxy"
                && claim.transport == Transport::Tcp
        })
        .collect::<Vec<_>>();
    let [listener] = listeners.as_slice() else {
        return Err(fail(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "public cutover requires one configured HAProxy TCP listener",
        ));
    };
    if requested_port.is_some_and(|port| port != listener.port) {
        return Err(fail(
            ctx,
            ErrorCode::ConfigReadConflict,
            "requested port differs from saved edge configuration; edit desired YAML before cutover",
        ));
    }
    Ok((calculate(app, &prepared, None, ctx)?, listener.port))
}
/// Prepare against a private selection without committing editable desired state.
pub fn prepare_selection(
    app: &HostApplication,
    candidate: &crate::selection::SelectionCandidate,
    ctx: &OperationContext,
) -> Result<crate::lifecycle::PreparedOperation> {
    let prepared = assemble(app, None, None, Some(candidate), ctx)?;
    let effect = calculate(app, &prepared, None, ctx)?;
    Ok(crate::lifecycle::PreparedOperation {
        selection: Some(candidate.clone()),
        effect: Some(effect),
        validation: Some(prepared.validation),
        target_graph: Some(prepared.graph),
        ..Default::default()
    })
}
pub fn prepare_identity(
    app: &HostApplication,
    binding: &IdentityBinding,
    include: bool,
    ctx: &OperationContext,
) -> Result<EffectPlan> {
    let prepared = assemble(app, None, Some((binding, include)), None, ctx)?;
    calculate(app, &prepared, None, ctx)
}
pub fn stage(app: &HostApplication, ctx: &OperationContext) -> Result<PlanId> {
    journal::ensure_mutating(ctx, app.control.as_ref())?;
    let prepared = assemble(app, None, None, None, ctx)?;
    let effect = calculate(app, &prepared, None, ctx)?;
    let plan_id = PlanId::new(format!(
        "plan.{}",
        &config::digest(
            &serde_json::to_vec(&(ctx.operation_id.clone(), &effect)).map_err(|_| fail(
                ctx,
                ErrorCode::IoUnavailable,
                "plan ID encoding invalid"
            ))?
        )
        .as_str()[7..39]
    ))
    .expect("digest ID");
    app.journal
        .stage_plan(plan_id.clone(), &effect, &prepared.context)?;
    Ok(plan_id)
}

#[derive(Clone)]
pub struct HostEffects {
    app: HostApplication,
}
impl HostEffects {
    pub fn new(app: &HostApplication) -> Self {
        Self { app: app.clone() }
    }
}
impl ApplyRecheck for HostEffects {
    fn refresh(&self, baseline: &EffectPlan, ctx: &OperationContext) -> Result<FreshApplyInputs> {
        let prepared = assemble(&self.app, Some(baseline), None, None, ctx)?;
        let plan = calculate(&self.app, &prepared, Some(baseline), ctx)?;
        Ok(FreshApplyInputs {
            expected_inputs: prepared.expected(&plan),
            instance_id: self.app.installation.instance_id.clone(),
            run_intent: match prepared.runtime.run_intent {
                wire::RunIntent::Running => RunIntent::Running,
                wire::RunIntent::Stopped => RunIntent::Stopped,
            },
            accepted_bundle: prepared
                .runtime
                .accepted
                .as_ref()
                .map(|id| BundleId::new(id.as_str()))
                .transpose()
                .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "accepted identity invalid"))?,
            originals: vec![],
            plan,
        })
    }
}
fn bundle_identity(plan: &EffectPlan, transaction: &TransactionId) -> BundleId {
    let digest =
        config::digest(format!("{}\0{}", plan.candidate_hash.as_str(), transaction).as_bytes());
    BundleId::new(format!("bundle.{}", &digest.as_str()[7..39])).expect("bounded digest identity")
}
fn read_plan_intent(
    app: &HostApplication,
    transaction: &TransactionId,
    ctx: &OperationContext,
) -> Result<EffectPlan> {
    let tx = app.journal.read(transaction, ctx)?;
    if tx.operation_id != ctx.operation_id || tx.actor != ctx.actor {
        return Err(fail(
            ctx,
            ErrorCode::OperationDenied,
            "native preparation has no exact current journal authority",
        ));
    }
    match tx.intent {
        Intent::Effects { plan } => Ok(plan),
        _ => Err(fail(
            ctx,
            ErrorCode::OperationDenied,
            "native preparation requires an effects intent",
        )),
    }
}
fn fallback(prepared: &Prepared, ctx: &OperationContext) -> Result<wire::FallbackCompatibility> {
    let target = if let Some(accepted) = &prepared.accepted {
        if accepted.fences.credential_revision < prepared.runtime.fences.credential_revision {
            return Err(fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "accepted fallback is behind the current credential floor",
            ));
        }
        for claim in accepted.claims.as_slice() {
            if matches!(claim,wire::Claim::Certificate{valid_until_ms,..} if *valid_until_ms<=prepared.now.0)
            {
                return Err(fail(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "accepted fallback certificate expired",
                ));
            }
        }
        wire::RollbackTarget::Accepted {
            bundle_id: accepted.bundle_id.clone(),
            manifest_digest: accepted.manifest_digest.clone(),
        }
    } else {
        let owned = prepared
            .registry
            .entries
            .values()
            .filter(|entry| entry.ownership == Ownership::Owned)
            .flat_map(|entry| &entry.resources)
            .map(|resource| {
                wire::ResourceId::new(resource.resource_id.as_str()).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "preinstall resource identity invalid",
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let preserved = prepared
            .registry
            .entries
            .values()
            .filter(|entry| entry.ownership == Ownership::Associated)
            .flat_map(|entry| &entry.resources)
            .map(|resource| {
                prepared
                    .observed
                    .native
                    .get(&resource.resource_id)
                    .cloned()
                    .ok_or_else(|| {
                        fail(
                            ctx,
                            ErrorCode::ResourceDrift,
                            "preserved associated identity unavailable",
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        wire::RollbackTarget::PreInstall {
            closed_listeners: items(owned, ctx)?,
            preserved_resources: items(preserved, ctx)?,
        }
    };
    let certificate_valid_until_ms = prepared.accepted.as_ref().and_then(|accepted| {
        accepted
            .claims
            .as_slice()
            .iter()
            .filter_map(|claim| match claim {
                wire::Claim::Certificate { valid_until_ms, .. } => Some(*valid_until_ms),
                _ => None,
            })
            .min()
    });
    let fences = wire::PolicyFences {
        credential_revision: prepared.runtime.fences.credential_revision,
    };
    let qualification_digest = wire::hash(
        &serde_json::to_vec(&(&target, &fences, certificate_valid_until_ms)).map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "fallback qualification digest invalid",
            )
        })?,
    );
    Ok(wire::FallbackCompatibility {
        target,
        fences,
        certificate_valid_until_ms,
        qualification_digest,
    })
}
fn authority(
    prepared: &Prepared,
    candidate: &wire::CandidateBundle,
    fallback: &wire::FallbackCompatibility,
    ctx: &OperationContext,
) -> Result<wire::RuntimeAuthority> {
    let mut resources = Vec::new();
    for entry in prepared.registry.entries.values() {
        for resource in &entry.resources {
            let Some(identity) = prepared.observed.native.get(&resource.resource_id).cloned()
            else {
                return Err(fail(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "authority resource lacks exact native observation",
                ));
            };
            let locator = match &resource.locator {
                ResourceLocator::DockerContainer { container_id } => {
                    wire::NativeLocator::Container {
                        container_id: container_id.as_str().to_owned(),
                    }
                }
                ResourceLocator::SystemdUnit { unit_id } => wire::NativeLocator::Systemd {
                    unit_name: unit_id.as_str().to_owned(),
                },
                ResourceLocator::ProtectedArtifact { .. } => wire::NativeLocator::Artifacts {
                    service_id: wire::ServiceId::new(resource.service_id.as_str()).map_err(
                        |_| fail(ctx, ErrorCode::ResourceDrift, "artifact scope invalid"),
                    )?,
                },
            };
            let dependencies = prepared
                .catalogue
                .services()
                .get(&resource.service_id)
                .map(|service| {
                    service
                        .dependencies
                        .iter()
                        .flat_map(|dependency| {
                            prepared
                                .registry
                                .entries
                                .values()
                                .flat_map(|entry| &entry.resources)
                                .filter(move |resource| &resource.service_id == dependency)
                        })
                        .map(|resource| {
                            wire::ResourceId::new(resource.resource_id.as_str()).map_err(|_| {
                                fail(ctx, ErrorCode::ResourceDrift, "shutdown dependency invalid")
                            })
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?
                .unwrap_or_default();
            let restart = prepared
                .observed
                .runtime
                .resources
                .get(&resource.resource_id)
                .and_then(|resource| resource.restart_policy.as_ref());
            let enabled_restart_policy = match restart {
                Some(RestartPolicy::DockerAlways | RestartPolicy::SystemdEnabled) => {
                    wire::RestartPolicy::Always
                }
                Some(RestartPolicy::DockerOnFailure { .. }) => wire::RestartPolicy::OnFailure,
                Some(RestartPolicy::DockerUnlessStopped) => wire::RestartPolicy::UnlessStopped,
                Some(RestartPolicy::SystemdDisabled) => wire::RestartPolicy::Always,
                _ => wire::RestartPolicy::UnlessStopped,
            };
            resources.push(wire::AuthorizedResource {
                identity,
                service_id: wire::ServiceId::new(resource.service_id.as_str()).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "native service identity invalid",
                    )
                })?,
                locator,
                structural_fingerprint: wire_digest(&resource.fingerprint, ctx)?,
                dependencies: items(dependencies, ctx)?,
                enabled_restart_policy,
            });
        }
    }
    let mut approvals = Vec::new();
    for approval in &prepared.approvals {
        let ids: Vec<_> = approval
            .resources
            .iter()
            .map(|id| {
                wire::ResourceId::new(id.as_str()).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ApprovalRequired,
                        "approval resource invalid",
                    )
                })
            })
            .collect::<Result<_>>()?;
        let actions: Vec<_> = candidate
            .actions
            .as_slice()
            .iter()
            .filter(|action| {
                approval
                    .resources
                    .iter()
                    .any(|resource| resource.as_str() == action.resource_id.as_str())
            })
            .map(|action| action.kind)
            .fold(Vec::new(), |mut kinds, kind| {
                if !kinds.contains(&kind) {
                    kinds.push(kind);
                }
                kinds
            });
        // Permission vocabulary is fixed and scope is the exact immutable candidate digest.
        approvals.push(wire::ApprovalEnvelope {
            approval_id: wire::ApprovalId::new(approval.approval_id.as_str()).map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "approval identity invalid",
                )
            })?,
            issuer: wire::CallerId::new(ctx.actor.actor_id.as_str())
                .map_err(|_| fail(ctx, ErrorCode::ApprovalRequired, "approval issuer invalid"))?,
            version: wire::ContractVersion::V1,
            scope_digest: candidate.manifest_digest.clone(),
            resources: items(ids, ctx)?,
            permitted_actions: items(actions, ctx)?,
            expires_ms: approval
                .expires_at
                .map(|expiry| expiry.0)
                .unwrap_or(ctx.deadline.0)
                .min(ctx.deadline.0),
        });
    }
    Ok(wire::RuntimeAuthority {
        schema_version: wire::StateSchemaVersion::V1,
        instance_id: candidate.instance_id.clone(),
        revision: prepared.registry.revision.0,
        run_epoch: prepared.runtime.run_epoch,
        bundle_id: candidate.bundle_id.clone(),
        bundle_digest: candidate.manifest_digest.clone(),
        resources: items(resources, ctx)?,
        approvals: items(approvals, ctx)?,
        native_evidence: candidate.native_evidence.clone(),
        staging: candidate.staging.clone(),
        fallbacks: items(vec![fallback.clone()], ctx)?,
        secrets: candidate.secrets.clone(),
        claims: candidate.claims.clone(),
    })
}
impl ApplyPreparation for HostEffects {
    fn mode(&self, _: &PlanId, _: &OperationContext) -> Result<QualificationMode> {
        Ok(QualificationMode::NativeOnly)
    }
    fn compile(
        &self,
        baseline: &EffectPlan,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<CompiledBundle> {
        let prepared = assemble(&self.app, Some(baseline), None, None, ctx)?;
        if calculate(&self.app, &prepared, Some(baseline), ctx)? != *baseline {
            return Err(fail(
                ctx,
                ErrorCode::PlanStale,
                "compile input changed after review",
            ));
        }
        let staging =
            BundleStaging::new(Arc::new(self.app.work.child("bundles").map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "bundle staging inventory unavailable",
                )
            })?));
        prepared.with_render(|render| {
            render::compile(
                &CompileInputs {
                    render,
                    plan: baseline,
                    transaction_id: transaction,
                    bundle_id: bundle_identity(baseline, transaction),
                    instance_id: self.app.installation.instance_id.clone(),
                    credential_revision: prepared.credential_revision,
                },
                &renderers(),
                &staging,
                &self.app.journal,
                ctx,
            )
        })
    }
    fn qualify(
        &self,
        bundle: &CompiledBundle,
        mode: QualificationMode,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<QualifiedApply> {
        let plan = read_plan_intent(&self.app, transaction, ctx)?;
        let prepared = assemble(&self.app, Some(&plan), None, None, ctx)?;
        if calculate(&self.app, &prepared, Some(&plan), ctx)? != plan {
            return Err(fail(
                ctx,
                ErrorCode::PlanStale,
                "qualification input changed after compilation",
            ));
        }
        if mode != QualificationMode::NativeOnly {
            return Err(fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "isolated semantic staging has no qualified attestation for this candidate",
            ));
        }
        let host = HostObserver::new();
        let cancel = Cancel {
            app: &self.app,
            ctx,
        };
        let mut candidate = bundle.candidate.clone();
        let mut evidence = Vec::new();
        for service in candidate.services.as_slice() {
            if service.engine == wire::Engine::Amnezia {
                return Err(fail(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "native foreign compatibility evidence is not available",
                ));
            }
            let image = ImageReference::new(service.image.as_str()).map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "native validator image invalid",
                )
            })?;
            host.docker_control()
                .validate_bundle(
                    self.app.installation.instance_id.as_str(),
                    candidate.bundle_id.as_str(),
                    service.service_id.as_str(),
                    &image,
                    engine(service.engine, ctx)?,
                    budget(&self.app, ctx)?,
                    &cancel,
                )
                .map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ConfigSemanticConflict,
                        "pinned native validator rejected exact compiled artifacts",
                    )
                })?;
            evidence.push(wire::Evidence {
                check_id: wire::CheckId::new(format!("native.{}", service.service_id)).map_err(
                    |_| {
                        fail(
                            ctx,
                            ErrorCode::ConfigSyntax,
                            "validator check identity invalid",
                        )
                    },
                )?,
                service_id: service.service_id.clone(),
                kind: wire::ProbeKind::NativeConfig,
                vantage: wire::Vantage::LocalNative,
                outcome: wire::EvidenceOutcome::Pass,
                timestamp_ms: self.app.control.now_ms(),
                bundle_digest: candidate.manifest_digest.clone(),
                input_digest: wire::hash(&serde_json::to_vec(service).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "validator input digest invalid",
                    )
                })?),
                observed_identity: None,
            });
        }
        candidate.native_evidence = items(evidence, ctx)?;
        render::publish_qualified(bundle, &candidate, &self.app.journal, ctx)?;
        let fallback = fallback(&prepared, ctx)?;
        let authority = authority(&prepared, &candidate, &fallback, ctx)?;
        super::runtime::publish_authority(&self.app, &authority, transaction, ctx)?;
        let manifest = bundle
            .directory
            .read("manifest.json", wire::MAX_WIRE_BYTES)
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "qualified manifest unreadable",
                )
            })?
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "qualified manifest missing",
                )
            })?;
        let manifest_digest = wire::hash(&manifest);
        let proof = wire::AdmissionProof {
            contract_version: wire::ContractVersion::V1,
            expected_run_epoch: prepared.runtime.run_epoch,
            expected_state_revision: prepared.runtime.revision,
            expected_accepted: prepared.runtime.accepted.clone(),
            actual_resources: items(
                authority
                    .resources
                    .as_slice()
                    .iter()
                    .map(|resource| resource.identity.clone())
                    .collect(),
                ctx,
            )?,
            approvals: authority.approvals.clone(),
            fallback,
            caller: wire::CallerId::new(ctx.actor.actor_id.as_str())
                .map_err(|_| fail(ctx, ErrorCode::ApprovalRequired, "runtime caller invalid"))?,
            protected_artifact_digest: manifest_digest.clone(),
        };
        Ok(QualifiedApply {
            candidate,
            proof,
            manifest_digest,
        })
    }
    fn save_pending(
        &self,
        bundle: &QualifiedApply,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<()> {
        let _ = read_plan_intent(&self.app, transaction, ctx)?;
        let runtime = self.app.work.child("runtime").map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "pending runtime inventory unavailable",
            )
        })?;
        let bytes=serde_json::to_vec(&json!({"schemaVersion":1,"bundleId":bundle.candidate.bundle_id,"manifestDigest":bundle.manifest_digest,"transactionId":transaction,"runEpoch":bundle.proof.expected_run_epoch})).map_err(|_|fail(ctx,ErrorCode::IoUnavailable,"pending metadata encoding failed"))?;
        let locked = runtime
            .lock(deadline(ctx, self.app.control.as_ref())?)
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::OperationConflict,
                    "pending publication lock unavailable",
                )
            })?;
        let old = locked.read("pending.json", 65536).map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "prior pending metadata unreadable",
            )
        })?;
        locked
            .compare_exchange("pending.json", old.as_deref(), &bytes)
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "pending publication durability uncertain",
                )
            })
    }
}

fn binding_record(
    app: &HostApplication,
    id: &ResourceId,
    ctx: &OperationContext,
) -> Result<Option<wire::NativeBindingRecord>> {
    let directory = match app
        .work
        .child("runtime")
        .and_then(|root| root.child("native-bindings"))
    {
        Ok(directory) => directory,
        Err(hton_storage::Error::NotFound) => return Ok(None),
        Err(_) => {
            return Err(fail(
                ctx,
                ErrorCode::IoUnavailable,
                "native replacement inventory unavailable",
            ));
        }
    };
    let name = format!("{id}.json");
    let Some(bytes) = directory.read(&name, wire::MAX_WIRE_BYTES).map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "native replacement record unreadable",
        )
    })?
    else {
        return Ok(None);
    };
    let record: wire::NativeBindingRecord = wire::decode(&bytes).map_err(|_| {
        fail(
            ctx,
            ErrorCode::RecoveryRequired,
            "native replacement record corrupt",
        )
    })?;
    if record.instance_id.as_str() != app.installation.instance_id.as_str()
        || record.resource.identity.resource_id.as_str() != id.as_str()
        || record.resource.identity.ownership != wire::Ownership::Owned
    {
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "native replacement authority mismatch",
        ));
    }
    Ok(Some(record))
}
fn concrete_resource(
    app: &HostApplication,
    id: &ResourceId,
    receipt: Option<&wire::ResourceReceipt>,
    ctx: &OperationContext,
) -> Result<(ResourceIdentity, ResourceObservation, Sha256Digest)> {
    let registry = app.registry(ctx)?;
    let (owner, previous) = registry
        .entries
        .values()
        .find_map(|entry| {
            entry
                .resources
                .iter()
                .find(|resource| &resource.resource_id == id)
                .map(|resource| (entry, resource))
        })
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "receipt resource is not registered",
            )
        })?;
    let record = binding_record(app, id, ctx)?;
    let (locator, expected_structural, identity_digest) = if let Some(record) = &record {
        if owner.ownership != Ownership::Owned
            || record.resource.service_id.as_str() != previous.service_id.as_str()
        {
            return Err(fail(
                ctx,
                ErrorCode::ResourceDrift,
                "native replacement service/ownership mismatch",
            ));
        }
        let locator = match &record.resource.locator {
            wire::NativeLocator::Container { container_id } => ResourceLocator::DockerContainer {
                container_id: DockerContainerId::new(container_id).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "replacement container identity invalid",
                    )
                })?,
            },
            wire::NativeLocator::Systemd { unit_name } => ResourceLocator::SystemdUnit {
                unit_id: ObjectId::new(unit_name).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "replacement unit identity invalid",
                    )
                })?,
            },
            wire::NativeLocator::Artifacts { .. } => {
                return Err(fail(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "artifact replacement requires its scoped owner",
                ));
            }
        };
        (
            locator,
            record.resource.structural_fingerprint.as_str().to_owned(),
            record.resource.identity.identity_digest.as_str().to_owned(),
        )
    } else {
        let digest = match &previous.locator {
            ResourceLocator::DockerContainer { container_id } => {
                config::digest(container_id.as_str().as_bytes())
            }
            ResourceLocator::SystemdUnit { unit_id } => config::digest(unit_id.as_str().as_bytes()),
            _ => {
                return Err(fail(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "artifact receipt needs its scoped owner",
                ));
            }
        };
        (
            previous.locator.clone(),
            previous.fingerprint.as_str().to_owned(),
            digest.as_str().to_owned(),
        )
    };
    if receipt.is_some_and(|receipt| {
        receipt.resource.resource_id.as_str() != id.as_str()
            || receipt.resource.identity_digest.as_str() != identity_digest
            || (receipt.resource.ownership == wire::Ownership::Associated)
                != (owner.ownership == Ownership::Associated)
    }) {
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "native receipt is not bound to exact installed/recreated identity",
        ));
    }
    let host = HostObserver::new();
    let cancel = Cancel { app, ctx };
    let config = current_config(app, &previous.service_id, ctx)?;
    let (structural, state_fingerprint, state, restart_policy) = match &locator {
        ResourceLocator::DockerContainer { container_id } => {
            let id = ContainerId::new(container_id.as_str())
                .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "receipt container invalid"))?;
            let facts = host
                .docker()
                .inspect_container(&id, budget(app, ctx)?, &cancel)
                .map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "receipt container reobservation unavailable",
                    )
                })?;
            let measured = container_fingerprints(&facts, config.as_deref()).map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "receipt native fingerprints unavailable",
                )
            })?;
            if measured.identity != identity_digest {
                return Err(fail(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "reobserved native identity mismatch",
                ));
            }
            let state = match measured.status {
                "running" => ObservedState::Running,
                "stopped" => ObservedState::Stopped,
                "failed" => ObservedState::Failed,
                _ => ObservedState::Unknown,
            };
            let restart = match measured.restart_policy {
                "disabled" => RestartPolicy::DockerNo,
                "always" => RestartPolicy::DockerAlways,
                "unless_stopped" => RestartPolicy::DockerUnlessStopped,
                "on_failure" => RestartPolicy::DockerOnFailure {
                    maximum_retry_count: facts.host_config.restart_policy.maximum_retry_count,
                },
                _ => {
                    return Err(fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "receipt restart policy unavailable",
                    ));
                }
            };
            (measured.structural, measured.state, state, restart)
        }
        ResourceLocator::SystemdUnit { unit_id } => {
            let name = hton_host::systemd::UnitName::new(unit_id.as_str())
                .map_err(|_| fail(ctx, ErrorCode::ResourceDrift, "receipt unit invalid"))?;
            let facts = host
                .systemd()
                .inspect(&name, budget(app, ctx)?, &cancel)
                .map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "receipt loaded unit reobservation unavailable",
                    )
                })?;
            let (state, status) = match facts.active_state.as_str() {
                "active" => (ObservedState::Running, "running"),
                "inactive" => (ObservedState::Stopped, "stopped"),
                "failed" => (ObservedState::Failed, "failed"),
                _ => (ObservedState::Unknown, "unknown"),
            };
            let (restart, token) = match facts.unit_file_state.as_str() {
                "enabled" | "enabled-runtime" => (RestartPolicy::SystemdEnabled, "always"),
                "disabled" | "masked" | "masked-runtime" => {
                    (RestartPolicy::SystemdDisabled, "disabled")
                }
                _ => {
                    return Err(fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "receipt unit restart owner unavailable",
                    ));
                }
            };
            let fingerprint = native_state_fingerprint(
                &identity_digest,
                &facts.structural_fingerprint,
                status,
                token,
                config.as_deref(),
            )
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "receipt unit fingerprint invalid",
                )
            })?;
            (facts.structural_fingerprint, fingerprint, state, restart)
        }
        _ => {
            return Err(fail(
                ctx,
                ErrorCode::ModuleUnsupported,
                "artifact receipt requires independent artifact owner",
            ));
        }
    };
    if structural != expected_structural
        || matches!(state, ObservedState::Unknown | ObservedState::Failed)
    {
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "reobserved structural/state identity is not qualified",
        ));
    }
    if receipt.is_some_and(|receipt| receipt.resource.fingerprint.as_str() != state_fingerprint) {
        return Err(fail(
            ctx,
            ErrorCode::ResourceDrift,
            "native receipt mutable fingerprint changed before mapping",
        ));
    }
    let identity = ResourceIdentity {
        resource_id: id.clone(),
        service_id: previous.service_id.clone(),
        locator,
        fingerprint: Sha256Digest::new(structural).map_err(|_| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "receipt structural digest invalid",
            )
        })?,
    };
    let now = app.control.now_ms();
    let observation = ResourceObservation {
        identity: identity.clone(),
        state,
        observed_at: UnixMillis(now),
        valid_until: UnixMillis(now.saturating_add(30_000)),
        restart_policy: Some(restart_policy),
        evidence: vec![],
    };
    Ok((
        identity,
        observation,
        Sha256Digest::new(state_fingerprint).map_err(|_| {
            fail(
                ctx,
                ErrorCode::ResourceDrift,
                "receipt mutable digest invalid",
            )
        })?,
    ))
}
fn receipt_key(step: &StepId) -> RequestId {
    RequestId::new(format!(
        "native.{}",
        &config::digest(step.as_str().as_bytes()).as_str()[7..39]
    ))
    .expect("bounded digest key")
}
fn steps_from_runtime(
    app: &HostApplication,
    plan: &EffectPlan,
    result: &wire::RuntimeResult,
    ctx: &OperationContext,
) -> Result<Vec<StepReceipt>> {
    let mut mapped = Vec::new();
    for action in &plan.actions {
        let outcome = result
            .receipts
            .as_slice()
            .iter()
            .find(|receipt| receipt.action_id.as_str() == action.action_id.as_str())
            .map(|receipt| &receipt.outcome);
        let mut receipt = StepReceipt {
            step_id: action.action_id.clone(),
            idempotency_key: receipt_key(&action.action_id),
            status: StepStatus::Uncertain,
            recorded_at: UnixMillis(app.control.now_ms()),
            resources: vec![],
            state_fingerprints: BTreeMap::new(),
            retained_artifacts: vec![],
            error: None,
        };
        match outcome {
            Some(
                wire::ActionOutcome::Completed { receipt: native }
                | wire::ActionOutcome::Unchanged { receipt: native },
            ) if result.status == wire::ResultStatus::Succeeded => {
                let (identity, _, _) = concrete_resource(app, &action.resource_id, None, ctx)?;
                // Independent committed runtime attests ordered intermediate postconditions;
                // later actions may already have changed restart state of this same identity.
                let exact_identity = match &identity.locator {
                    ResourceLocator::DockerContainer { container_id } => {
                        config::digest(container_id.as_str().as_bytes())
                    }
                    ResourceLocator::SystemdUnit { unit_id } => {
                        config::digest(unit_id.as_str().as_bytes())
                    }
                    _ => {
                        return Err(fail(
                            ctx,
                            ErrorCode::ResourceDrift,
                            "receipt has no native identity",
                        ));
                    }
                };
                if native.resource.identity_digest.as_str() != exact_identity.as_str()
                    || native.resource.resource_id.as_str() != action.resource_id.as_str()
                {
                    return Err(fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "forward receipt identity does not match final verified native binding",
                    ));
                }
                let fingerprint =
                    Sha256Digest::new(native.resource.fingerprint.as_str()).map_err(|_| {
                        fail(
                            ctx,
                            ErrorCode::ResourceDrift,
                            "forward native digest invalid",
                        )
                    })?;
                if !action.postconditions.iter().any(|condition|matches!(condition,Condition::ResourceFingerprint{resource_id,fingerprint:expected} if resource_id==&action.resource_id&&expected==&fingerprint)){return Err(fail(ctx,ErrorCode::ResourceDrift,"forward native receipt differs from exact approved after condition"));}
                receipt.status = StepStatus::Verified;
                receipt.recorded_at = UnixMillis(native.timestamp_ms);
                receipt.resources.push(identity);
                receipt
                    .state_fingerprints
                    .insert(action.resource_id.clone(), fingerprint);
            }
            Some(wire::ActionOutcome::Failed { .. }) => receipt.status = StepStatus::Failed,
            _ => (),
        }
        mapped.push(receipt);
    }
    Ok(mapped)
}
impl ApplyRuntime for HostEffects {
    fn activate(
        &self,
        bundle: &QualifiedApply,
        ctx: &OperationContext,
    ) -> Result<RuntimeApplyResult> {
        let tx = self
            .app
            .journal
            .unfinished(ctx)?
            .into_iter()
            .find(|tx| {
                tx.operation_id == ctx.operation_id && matches!(&tx.intent, Intent::Effects { .. })
            })
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "runtime activation effects journal missing",
                )
            })?;
        let plan = match tx.intent {
            Intent::Effects { plan } => plan,
            _ => unreachable!(),
        };
        let mut result = super::runtime::execute(
            &self.app,
            wire::ExecutorAction::Activate {
                bundle_id: bundle.candidate.bundle_id.clone(),
                proof: bundle.proof.clone(),
            },
            ctx,
        )?;
        let original_receipts = result.receipts.clone();
        loop {
            let state = self.app.runtime(ctx)?;
            let pending = state.lease.as_ref().filter(|lease| {
                matches!(
                    lease.state,
                    wire::LeaseState::Armed
                        | wire::LeaseState::Activating
                        | wire::LeaseState::Verifying
                        | wire::LeaseState::Compensating
                        | wire::LeaseState::RollingBack
                )
            });
            let Some(lease) = pending else { break };
            if self.app.control.now_ms() >= ctx.deadline.0
                || self.app.control.is_cancelled(&ctx.cancellation)
            {
                break;
            }
            if self.app.control.now_ms() >= lease.deadline_ms {
                result = super::runtime::execute(
                    &self.app,
                    wire::ExecutorAction::Recover {
                        lease_id: lease.lease_id.clone(),
                    },
                    ctx,
                )?;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
            result = super::runtime::execute(
                &self.app,
                wire::ExecutorAction::CommitTrial {
                    lease_id: lease.lease_id.clone(),
                },
                ctx,
            )?;
        }
        result.receipts = original_receipts;
        let state = self.app.runtime(ctx)?;
        let finality = if state.lease.as_ref().is_some_and(|lease| {
            matches!(
                lease.state,
                wire::LeaseState::Armed
                    | wire::LeaseState::Activating
                    | wire::LeaseState::Verifying
                    | wire::LeaseState::Compensating
                    | wire::LeaseState::RollingBack
            )
        }) {
            RuntimeFinality::TrialPending
        } else {
            RuntimeFinality::Final
        };
        let mut operation = super::runtime::map_result(&result, ctx)?;
        if operation.status == OperationStatus::Succeeded
            && result.accepted.as_ref() != Some(&bundle.candidate.bundle_id)
        {
            operation.status = OperationStatus::Failed;
        }
        let receipts = steps_from_runtime(&self.app, &plan, &result, ctx)?;
        Ok(RuntimeApplyResult {
            result: operation,
            receipts,
            finality,
        })
    }
}
impl AcceptedBindingPort for HostEffects {
    fn reconcile(
        &self,
        plan: &EffectPlan,
        _: &RuntimeApplyResult,
        ctx: &OperationContext,
    ) -> Result<()> {
        for id in plan
            .actions
            .iter()
            .map(|action| &action.resource_id)
            .collect::<BTreeSet<_>>()
        {
            let Some(_record) = binding_record(&self.app, id, ctx)? else {
                continue;
            };
            let (replacement, observation, fingerprint) =
                concrete_resource(&self.app, id, None, ctx)?;
            let registry = self.app.registry(ctx)?;
            let (owner, previous) = registry
                .entries
                .iter()
                .find_map(|(owner, entry)| {
                    entry
                        .resources
                        .iter()
                        .find(|resource| &resource.resource_id == id)
                        .map(|resource| (owner, resource))
                })
                .ok_or_else(|| {
                    fail(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "replacement registry member missing",
                    )
                })?;
            if previous == &replacement {
                continue;
            }
            let step = StepId::new(format!(
                "rebind.{}",
                &config::digest(id.as_str().as_bytes()).as_str()[7..39]
            ))
            .expect("digest ID");
            let receipt = StepReceipt {
                step_id: step.clone(),
                idempotency_key: receipt_key(&step),
                status: StepStatus::Verified,
                recorded_at: UnixMillis(self.app.control.now_ms()),
                resources: vec![replacement.clone()],
                state_fingerprints: BTreeMap::from([(id.clone(), fingerprint)]),
                retained_artifacts: vec![],
                error: None,
            };
            let preconditions = RegistryPreconditions {
                observed: BTreeMap::from([(id.clone(), observation)]),
                ..Default::default()
            };
            self.app.registry.transition(
                &self.app.installation.instance_id,
                registry.revision,
                RegistryChange::RebindOwned {
                    module_id: owner.clone(),
                    previous: previous.clone(),
                    replacement,
                    receipt,
                },
                &preconditions,
                ctx,
            )?;
        }
        Ok(())
    }
}

fn recovered_result(
    _app: &HostApplication,
    transaction: &Transaction,
    state: &wire::RuntimeState,
    status: OperationStatus,
    ctx: &OperationContext,
) -> Result<OperationResult> {
    Ok(OperationResult {
        operation_id: transaction.operation_id.clone(),
        status,
        transaction_id: Some(transaction.transaction_id.clone()),
        desired_saved: true,
        pending_bundle: None,
        accepted_bundle: state
            .accepted
            .as_ref()
            .map(|bundle| BundleId::new(bundle.as_str()))
            .transpose()
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "recovery accepted identity invalid",
                )
            })?,
        run_intent: match state.run_intent {
            wire::RunIntent::Running => RunIntent::Running,
            wire::RunIntent::Stopped => RunIntent::Stopped,
        },
        member_results: vec![],
        survivors: vec![],
        issues: if status == OperationStatus::Succeeded {
            vec![]
        } else {
            vec![config::issue(
                ErrorCode::RuntimePartial,
                None,
                None,
                "independent runtime compensated the interrupted trial; original artifacts retained",
            )]
        },
    })
}
impl ApplyReconciliation for HostEffects {
    fn reconcile(
        &self,
        transaction: &Transaction,
        ctx: &OperationContext,
    ) -> Result<Reconciliation> {
        let plan = match &transaction.intent {
            Intent::Effects { plan } => plan,
            // In the installed R1 entry OperationId is generated per-process from OS
            // randomness, never supplied by a caller or reused across process retries.
            _ if transaction.operation_id == ctx.operation_id => {
                return Ok(Reconciliation::ActiveParent);
            }
            _ => return Ok(Reconciliation::NeedsRecovery),
        };
        let state = self.app.runtime(ctx)?;
        let expected = bundle_identity(plan, &transaction.transaction_id);
        if state.lease.as_ref().is_some_and(|lease| {
            matches!(
                lease.state,
                wire::LeaseState::Armed
                    | wire::LeaseState::Activating
                    | wire::LeaseState::Verifying
                    | wire::LeaseState::Compensating
                    | wire::LeaseState::RollingBack
                    | wire::LeaseState::RecoveryRequired
            )
        }) {
            return Ok(Reconciliation::NeedsRecovery);
        }
        let mut original = ctx.clone();
        original.operation_id = transaction.operation_id.clone();
        original.actor = transaction.actor.clone();
        if state.accepted.as_ref().map(|bundle| bundle.as_str()) == Some(expected.as_str())
            && state.run_intent == wire::RunIntent::Running
        {
            let verified =
                super::runtime::execute(&self.app, wire::ExecutorAction::Observe, &original)?;
            if verified.status != wire::ResultStatus::Succeeded
                || verified.accepted.as_ref() != state.accepted.as_ref()
            {
                return Ok(Reconciliation::NeedsRecovery);
            }
            let mut receipts = Vec::new();
            for action in &plan.actions {
                let (identity, _, _) =
                    concrete_resource(&self.app, &action.resource_id, None, &original)?;
                let after = action
                    .postconditions
                    .iter()
                    .find_map(|condition| match condition {
                        Condition::ResourceFingerprint {
                            resource_id,
                            fingerprint,
                        } if resource_id == &action.resource_id => Some(fingerprint.clone()),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        fail(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "interrupted plan lacks exact after fingerprint",
                        )
                    })?;
                receipts.push(StepReceipt {
                    step_id: action.action_id.clone(),
                    idempotency_key: receipt_key(&action.action_id),
                    status: StepStatus::Verified,
                    recorded_at: UnixMillis(self.app.control.now_ms()),
                    resources: vec![identity],
                    state_fingerprints: BTreeMap::from([(action.resource_id.clone(), after)]),
                    retained_artifacts: vec![],
                    error: None,
                });
            }
            let result = recovered_result(
                &self.app,
                transaction,
                &state,
                OperationStatus::Succeeded,
                &original,
            )?;
            AcceptedBindingPort::reconcile(
                self,
                plan,
                &RuntimeApplyResult {
                    result: result.clone(),
                    receipts: receipts.clone(),
                    finality: RuntimeFinality::Final,
                },
                &original,
            )?;
            return Ok(Reconciliation::Resolved {
                status: TerminalStatus::Committed,
                receipts,
                result: Some(result),
            });
        }
        let compensated = state.lease.as_ref().is_some_and(|lease| {
            lease.candidate.as_str() == expected.as_str()
                && matches!(
                    lease.state,
                    wire::LeaseState::Compensated | wire::LeaseState::StoppedWithArtifacts
                )
        });
        if compensated {
            let mut earliest = BTreeMap::new();
            for action in &plan.actions {
                earliest
                    .entry(action.resource_id.clone())
                    .or_insert_with(|| {
                        action
                            .preconditions
                            .iter()
                            .find_map(|condition| match condition {
                                Condition::ResourceFingerprint {
                                    resource_id,
                                    fingerprint,
                                } if resource_id == &action.resource_id => {
                                    Some(fingerprint.clone())
                                }
                                _ => None,
                            })
                    });
            }
            for (resource, before) in earliest {
                let (_, _, actual) = concrete_resource(&self.app, &resource, None, &original)?;
                if before.as_ref() != Some(&actual) {
                    return Ok(Reconciliation::NeedsRecovery);
                }
            }
            let result = recovered_result(
                &self.app,
                transaction,
                &state,
                OperationStatus::Failed,
                &original,
            )?;
            AcceptedBindingPort::reconcile(
                self,
                plan,
                &RuntimeApplyResult {
                    result: result.clone(),
                    receipts: vec![],
                    finality: RuntimeFinality::Final,
                },
                &original,
            )?;
            return Ok(Reconciliation::Resolved {
                status: TerminalStatus::RolledBack,
                receipts: vec![],
                result: Some(result),
            });
        }
        Ok(Reconciliation::NeedsRecovery)
    }
}
fn coordinator<'a>(
    app: &'a HostApplication,
    backend: &'a HostEffects,
    lock: &'a SecureDir,
) -> ApplyCoordinator<'a> {
    ApplyCoordinator {
        journal: &app.journal,
        config_lock: lock,
        recheck: backend,
        preparation: backend,
        runtime: backend,
        bindings: backend,
        reconciliation: backend,
    }
}
pub fn apply(
    app: &HostApplication,
    effect: &EffectPlan,
    ctx: &OperationContext,
) -> Result<OperationResult> {
    journal::ensure_mutating(ctx, app.control.as_ref())?;
    let prepared = assemble(app, Some(effect), None, None, ctx)?;
    if calculate(app, &prepared, Some(effect), ctx)? != *effect {
        return Err(fail(
            ctx,
            ErrorCode::PlanStale,
            "requested effect no longer matches current qualified inputs",
        ));
    }
    let backend = HostEffects::new(app);
    let lock = app.work.child("locks").map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "configuration effect lock unavailable",
        )
    })?;
    coordinator(app, &backend, &lock).stage_and_apply(effect, &prepared.context)
}
pub fn apply_staged(
    app: &HostApplication,
    id: &PlanId,
    ctx: &OperationContext,
) -> Result<OperationResult> {
    journal::ensure_mutating(ctx, app.control.as_ref())?;
    let stored = app.journal.read_plan_metadata(id, &ctx.actor, ctx)?;
    let prepared = assemble(app, Some(&stored.plan), None, None, ctx)?;
    let current = calculate(app, &prepared, Some(&stored.plan), ctx)?;
    if current != stored.plan {
        return Err(fail(
            ctx,
            ErrorCode::PlanStale,
            "staged effect inputs changed",
        ));
    }
    let expected = prepared.expected(&current);
    let backend = HostEffects::new(app);
    let lock = app.work.child("locks").map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "configuration effect lock unavailable",
        )
    })?;
    coordinator(app, &backend, &lock).apply(id, &expected, &prepared.context)
}
impl EffectApplyPort for HostEffects {
    fn stage_and_apply(
        &self,
        effect: &EffectPlan,
        ctx: &OperationContext,
    ) -> Result<OperationResult> {
        apply(&self.app, effect, ctx)
    }
}
