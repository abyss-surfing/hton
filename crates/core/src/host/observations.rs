//! Exact registered observations for composition; no ownership is inferred from discovery.
use super::*;
use crate::certificates::CertificateName;
use hton_host::{
    HostObserver,
    docker::{ContainerId, container_fingerprints},
    systemd::UnitName,
};
use hton_runtime_contract as wire;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::Ipv4Addr,
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct ObservedResources {
    pub runtime: RuntimeObservation,
    pub native: BTreeMap<ResourceId, wire::ResourceIdentity>,
    pub issues: Vec<CheckIssue>,
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
struct ReservationCancel<'a> {
    control: &'a dyn crate::journal::OperationControl,
    cancellation: &'a CancellationId,
}
impl hton_host::Cancellation for ReservationCancel<'_> {
    fn cancelled(&self) -> bool {
        self.control.is_cancelled(self.cancellation)
    }
}
type ActiveArtifacts = wire::ActiveServiceArtifacts;
fn active(
    app: &HostApplication,
    service: &ServiceId,
    ctx: &OperationContext,
) -> Result<Option<ActiveArtifacts>> {
    let runtime = app.work.child("runtime").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "runtime metadata unavailable",
        )
    })?;
    let services = runtime.child("services").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "service metadata unavailable",
        )
    })?;
    let directory = match services.child(service.as_str()) {
        Ok(dir) => dir,
        Err(hton_storage::Error::NotFound) => return Ok(None),
        Err(_) => {
            return Err(journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "service metadata unsafe or unavailable",
            ));
        }
    };
    let Some(bytes) = directory.read("active.json", 65536).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "active service metadata unavailable",
        )
    })?
    else {
        return Ok(None);
    };
    let metadata: ActiveArtifacts = wire::decode(&bytes).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "active service metadata corrupt",
        )
    })?;
    if !metadata.loaded {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "native service generation is not acknowledged",
        ));
    }
    if metadata.service_id.as_str() != service.as_str() {
        return Err(journal::failure(
            ctx,
            ErrorCode::ResourceDrift,
            "active service identity mismatch",
        ));
    }
    Ok(Some(metadata))
}
pub fn collect(app: &HostApplication, ctx: &OperationContext) -> Result<ObservedResources> {
    collect_filtered(app, ctx, false, 1_048_576)
}

/// Capture complete collision-only TCP/UDP reservations and bind Docker identities
/// to a service only through the exact protected registry. Procfs sockets and
/// unregistered containers remain unknown holders.
pub fn reservation_evidence(
    app: &HostApplication,
    ctx: &OperationContext,
) -> Result<ReservationEvidence> {
    let registry = app.registry(ctx)?;
    reservation_evidence_for(&registry, app.control.as_ref(), ctx)
}

/// Capture complete host reservations against the exact registry snapshot
/// supplied by the caller. This variant does not open or initialize an
/// application store, so first-install virtual preflight can use an empty
/// expected registry before creating bootstrap state.
pub fn reservation_evidence_for(
    registry: &RegistrySnapshot,
    control: &dyn crate::journal::OperationControl,
    ctx: &OperationContext,
) -> Result<ReservationEvidence> {
    let remaining = ctx.deadline.0.saturating_sub(control.now_ms()).min(10_000);
    if remaining == 0 {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDeadline,
            "port reservation observation deadline expired",
        ));
    }
    let host_budget =
        hton_host::Budget::new(Instant::now() + Duration::from_millis(remaining), 1_048_576)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::OperationDeadline,
                    "port reservation observation budget invalid",
                )
            })?;
    let observed = HostObserver::new()
        .port_reservations(
            host_budget,
            &ReservationCancel {
                control,
                cancellation: &ctx.cancellation,
            },
        )
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "complete host TCP/UDP reservation evidence unavailable",
            )
        })?;
    let mut identity_by_container = BTreeMap::new();
    for entry in registry.entries.values() {
        for resource in &entry.resources {
            let ResourceLocator::DockerContainer { container_id } = &resource.locator else {
                continue;
            };
            let identity = (
                resource.service_id.clone(),
                resource.fingerprint.as_str().to_owned(),
            );
            if identity_by_container
                .insert(container_id.as_str().to_owned(), identity.clone())
                .is_some_and(|previous| previous != identity)
            {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "Docker reservation identity is ambiguously registered",
                ));
            }
        }
    }
    let mut identity_digest = Vec::with_capacity(observed.len());
    let mut reservations = Vec::with_capacity(observed.len());
    for reservation in observed {
        let transport = match reservation.protocol {
            hton_host::docker::TransportProtocol::Tcp => Transport::Tcp,
            hton_host::docker::TransportProtocol::Udp => Transport::Udp,
        };
        let container_id = reservation
            .container_id
            .as_ref()
            .map(|id| id.as_str().to_owned());
        let container_fingerprint = reservation.container_fingerprint.clone();
        let holder_service = container_id.as_ref().and_then(|id| {
            identity_by_container
                .get(id)
                .and_then(|(service, fingerprint)| {
                    (container_fingerprint.as_ref() == Some(fingerprint)).then(|| service.clone())
                })
        });
        identity_digest.push((
            transport,
            reservation.port,
            container_id,
            container_fingerprint,
            holder_service.clone(),
        ));
        reservations.push(PortReservation {
            transport,
            port: reservation.port,
            holder_service,
        });
    }
    let digest_bytes = serde_json::to_vec(&identity_digest).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "port reservation digest encoding failed",
        )
    })?;
    let observed_at = UnixMillis(control.now_ms());
    Ok(ReservationEvidence {
        observed_at,
        valid_until: UnixMillis(observed_at.0.saturating_add(30_000)),
        digest: config::digest(&digest_bytes),
        complete: true,
        reservations,
    })
}

fn collect_filtered(
    app: &HostApplication,
    ctx: &OperationContext,
    docker_only: bool,
    max_output_bytes: usize,
) -> Result<ObservedResources> {
    let registry = app.registry(ctx)?;
    if registry
        .entries
        .values()
        .map(|entry| entry.resources.len())
        .sum::<usize>()
        > 512
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "resource observation inventory exceeds runtime bounds",
        ));
    }
    let host = HostObserver::new();
    let cancel = Cancel { app, ctx };
    let mut result = ObservedResources {
        runtime: RuntimeObservation {
            resources: BTreeMap::new(),
        },
        native: BTreeMap::new(),
        issues: vec![],
    };
    for (module, entry) in &registry.entries {
        for resource in &entry.resources {
            if docker_only && !matches!(&resource.locator, ResourceLocator::DockerContainer { .. })
            {
                continue;
            }
            let remaining = ctx
                .deadline
                .0
                .saturating_sub(app.control.now_ms())
                .min(10_000);
            let budget = hton_host::Budget::new(
                Instant::now() + Duration::from_millis(remaining),
                max_output_bytes,
            )
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::OperationDeadline,
                    "native observation budget invalid",
                )
            })?;
            let observed=(||->Result<(ResourceIdentity,ObservedState,Option<RestartPolicy>,wire::ResourceIdentity)>{
                let active=active(app,&resource.service_id,ctx)?;
                let active_digest=active.as_ref().map(|metadata|metadata.digest.as_str());
                let (structural,identity,state_fp,state,policy,kind)=match &resource.locator{
                    ResourceLocator::DockerContainer{container_id}=>{
                        let id=ContainerId::new(container_id.as_str()).map_err(|_|journal::failure(ctx,ErrorCode::ResourceDrift,"invalid installed container locator"))?;
                        let facts=host.docker().inspect_container(&id,budget,&cancel).map_err(|_|journal::failure(ctx,ErrorCode::IoUnavailable,"exact container observation unavailable"))?;
                        let fingerprints=container_fingerprints(&facts,active_digest).map_err(|_|journal::failure(ctx,ErrorCode::ResourceDrift,"native container fingerprint unavailable"))?;
                        let state=match fingerprints.status{"running"=>ObservedState::Running,"stopped"=>ObservedState::Stopped,"failed"=>ObservedState::Failed,_=>ObservedState::Unknown};
                        let policy=match facts.host_config.restart_policy.name.as_str(){"no"|""=>Some(RestartPolicy::DockerNo),"always"=>Some(RestartPolicy::DockerAlways),"unless-stopped"=>Some(RestartPolicy::DockerUnlessStopped),"on-failure"=>Some(RestartPolicy::DockerOnFailure{maximum_retry_count:facts.host_config.restart_policy.maximum_retry_count}),_=>None};
                        (fingerprints.structural,fingerprints.identity,fingerprints.state,state,policy,wire::ResourceKind::Container)
                    }
                    ResourceLocator::SystemdUnit{unit_id}=>{
                        let name=UnitName::new(unit_id.as_str()).map_err(|_|journal::failure(ctx,ErrorCode::ResourceDrift,"invalid installed unit locator"))?;
                        let facts=host.systemd().inspect(&name,budget,&cancel).map_err(|_|journal::failure(ctx,ErrorCode::IoUnavailable,"exact loaded unit observation unavailable"))?;
                        let state=match facts.active_state.as_str(){"active"=>ObservedState::Running,"inactive"=>ObservedState::Stopped,"failed"=>ObservedState::Failed,_=>ObservedState::Unknown};
                        let (policy,restart)=match facts.unit_file_state.as_str(){"disabled"|"masked"|"masked-runtime"=>(Some(RestartPolicy::SystemdDisabled),"disabled"),"enabled"|"enabled-runtime"=>(Some(RestartPolicy::SystemdEnabled),"always"),_=>(None,"external")};
                        let identity=config::digest(unit_id.as_str().as_bytes()).as_str().to_owned();
                        let state_token=match state{ObservedState::Running=>"running",ObservedState::Stopped=>"stopped",ObservedState::Failed=>"failed",ObservedState::Unknown=>"unknown"};
                        let fp=hton_host::docker::native_state_fingerprint(&identity,&facts.structural_fingerprint,state_token,restart,active_digest).map_err(|_|journal::failure(ctx,ErrorCode::ResourceDrift,"loaded unit fingerprint unavailable"))?;
                        (facts.structural_fingerprint,identity,fp,state,policy,wire::ResourceKind::SystemdUnit)
                    }
                    ResourceLocator::ProtectedArtifact{..}=>return Err(journal::failure(ctx,ErrorCode::ModuleUnsupported,"artifact observation requires its scoped archive/bundle owner")),
                };
                let structural=Sha256Digest::new(structural).map_err(|_|journal::failure(ctx,ErrorCode::ResourceDrift,"invalid observed structural digest"))?;
                let actual=ResourceIdentity{fingerprint:structural,..resource.clone()};
                let native=wire::ResourceIdentity{
                    resource_id:wire::ResourceId::new(resource.resource_id.as_str()).map_err(|_|journal::failure(ctx,ErrorCode::ResourceDrift,"resource identity invalid"))?,kind,
                    identity_digest:wire::Sha256Digest::new(identity).map_err(|_|journal::failure(ctx,ErrorCode::ResourceDrift,"native identity digest invalid"))?,
                    fingerprint:wire::Sha256Digest::new(state_fp).map_err(|_|journal::failure(ctx,ErrorCode::ResourceDrift,"native state digest invalid"))?,
                    ownership:match entry.ownership{Ownership::Owned=>wire::Ownership::Owned,Ownership::Associated=>wire::Ownership::Associated},
                };
                Ok((actual,state,policy,native))
            })();
            let now = app.control.now_ms();
            let (identity, state, policy) = match observed {
                Ok((identity, state, policy, native)) => {
                    if identity != *resource {
                        result.issues.push(config::issue(
                            ErrorCode::ResourceDrift,
                            None,
                            Some(module.clone()),
                            "installed launch metadata differs from recorded identity",
                        ));
                    } else {
                        result.native.insert(resource.resource_id.clone(), native);
                    }
                    (identity, state, policy)
                }
                Err(error) => {
                    result.issues.push(config::issue(
                        error.code,
                        None,
                        Some(module.clone()),
                        "native resource observation is unavailable or unqualified",
                    ));
                    (resource.clone(), ObservedState::Unknown, None)
                }
            };
            result.runtime.resources.insert(
                resource.resource_id.clone(),
                ResourceObservation {
                    identity,
                    state,
                    observed_at: UnixMillis(now),
                    valid_until: UnixMillis(now.saturating_add(30_000)),
                    restart_policy: policy,
                    evidence: vec![],
                },
            );
        }
    }
    Ok(result)
}

/// CheckCoordinator adapter for one bounded, read-only host snapshot.
/// External DNS is reached only from the explicit identity command.
struct LocalObservationPort<'a> {
    app: &'a HostApplication,
    observed: &'a ObservedResources,
    local_instance_known: bool,
    dns: Mutex<BTreeMap<String, hton_host::network::DnsIpv4>>,
}

impl crate::checks::ObservationPort for LocalObservationPort<'_> {
    fn observe(
        &self,
        query: &crate::checks::ObservationQuery,
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<crate::checks::ObservedSample> {
        let (outcome, resource, bytes_used) = match &query.target {
            crate::checks::ObservationTarget::Host => (
                if self.local_instance_known {
                    EvidenceOutcome::Pass
                } else {
                    EvidenceOutcome::Unknown
                },
                None,
                32,
            ),
            crate::checks::ObservationTarget::Resource(expected) => {
                let sample = self
                    .observed
                    .runtime
                    .resources
                    .get(&expected.resource_id)
                    .cloned()
                    .ok_or_else(|| {
                        journal::failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "registered resource observation unavailable",
                        )
                    })?;
                let outcome = match sample.state {
                    ObservedState::Running | ObservedState::Stopped => EvidenceOutcome::Pass,
                    ObservedState::Failed => EvidenceOutcome::Fail,
                    ObservedState::Unknown => EvidenceOutcome::Unknown,
                };
                (outcome, Some(sample), 256)
            }
            crate::checks::ObservationTarget::Domain(domain) => {
                let remaining = budget.deadline.0.saturating_sub(self.app.control.now_ms());
                let native_budget = hton_host::Budget::new(
                    Instant::now() + Duration::from_millis(remaining.min(10_000)),
                    65_536,
                )
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::OperationDeadline,
                        "DNS observation budget invalid",
                    )
                })?;
                let cancel = Cancel { app: self.app, ctx };
                let answer = HostObserver::new()
                    .network()
                    .dns_ipv4(domain.host(), native_budget, &cancel)
                    .map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "configured domain DNS observation unavailable",
                        )
                    })?;
                let outcome = if answer.addresses.is_empty() {
                    EvidenceOutcome::Fail
                } else {
                    EvidenceOutcome::Pass
                };
                self.dns
                    .lock()
                    .map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "identity evidence aggregation unavailable",
                        )
                    })?
                    .insert(domain.host().to_owned(), answer);
                (outcome, None, 65_536)
            }
            crate::checks::ObservationTarget::Certificate(_) => (EvidenceOutcome::Unknown, None, 0),
        };
        let timestamp = resource
            .as_ref()
            .map(|sample| sample.observed_at)
            .unwrap_or_else(|| UnixMillis(self.app.control.now_ms()));
        Ok(crate::checks::ObservedSample {
            evidence: Evidence {
                check_id: query.check_id.clone(),
                module_id: query.module_id.clone(),
                outcome,
                timestamp,
                vantage: query.vantage,
                input_fingerprint: query.input_fingerprint.clone(),
                details: SafeFields {
                    resource_id: resource
                        .as_ref()
                        .map(|sample| sample.identity.resource_id.clone()),
                    count: Some(1),
                    ..Default::default()
                },
            },
            resource,
            bytes_used,
        })
    }
}

fn observation_port<'a>(
    app: &'a HostApplication,
    observed: &'a ObservedResources,
) -> LocalObservationPort<'a> {
    LocalObservationPort {
        app,
        observed,
        local_instance_known: true,
        dns: Mutex::new(BTreeMap::new()),
    }
}

fn check_budget(app: &HostApplication, ctx: &OperationContext, observations: u32) -> Budget {
    Budget {
        deadline: UnixMillis(
            ctx.deadline
                .0
                .min(app.control.now_ms().saturating_add(75_000)),
        ),
        max_concurrency: 1,
        max_bytes: 2 * 1024 * 1024,
        max_observations: observations,
    }
}

fn resource_queries(
    app: &HostApplication,
    ctx: &OperationContext,
    docker_only: bool,
) -> Result<Vec<crate::checks::ObservationQuery>> {
    let registry = app.registry(ctx)?;
    let entries: Vec<_> = registry
        .entries
        .iter()
        .flat_map(|(module, entry)| {
            entry
                .resources
                .iter()
                .filter(move |resource| {
                    !docker_only
                        || matches!(&resource.locator, ResourceLocator::DockerContainer { .. })
                })
                .map(move |resource| (module, resource))
        })
        .collect();
    if entries.len() > 128 {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "diagnostic resource inventory exceeds 128 observations",
        ));
    }
    entries
        .into_iter()
        .map(|(module, resource)| {
            let digest = config::digest(resource.resource_id.as_str().as_bytes());
            let check_id = CheckId::new(format!("diag.resource.{}", &digest.as_str()[7..31]))
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "diagnostic check identity invalid",
                    )
                })?;
            Ok(crate::checks::ObservationQuery {
                check_id,
                module_id: Some(module.clone()),
                section: crate::checks::CheckSection::Resources,
                target: crate::checks::ObservationTarget::Resource(resource.clone()),
                vantage: Vantage::LocalRuntime,
                input_fingerprint: resource.fingerprint.clone(),
                required: false,
            })
        })
        .collect()
}

/// Bounded local service/runtime diagnostics. Unsupported sections stay explicit.
pub fn diagnose(
    app: &HostApplication,
    sections: &BTreeSet<String>,
    verbose: bool,
    ctx: &OperationContext,
) -> Result<Value> {
    let supported: BTreeSet<_> = ["services", "runtime", "docker"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let requested = if sections.is_empty() {
        supported.clone()
    } else {
        sections.clone()
    };
    let mut unknown: Vec<_> = requested.difference(&supported).cloned().collect();
    unknown.sort();
    let wants_services = requested.contains("services");
    let wants_docker = !wants_services && requested.contains("docker");
    let needs_native = wants_services || wants_docker;
    let mut counts = BTreeMap::<&'static str, u64>::new();
    let mut issues = Vec::<String>::new();
    if needs_native {
        let registry = app.registry(ctx)?;
        let resource_count = registry
            .entries
            .values()
            .flat_map(|entry| &entry.resources)
            .filter(|resource| {
                !wants_docker
                    || matches!(&resource.locator, ResourceLocator::DockerContainer { .. })
            })
            .count();
        if resource_count > 128 {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "diagnostic resource inventory exceeds 128 observations",
            ));
        }
        // 128 exact resources × 32 KiB keeps diagnostic native output within 4 MiB.
        let observed = collect_filtered(app, ctx, wants_docker, 32_768)?;
        let queries = resource_queries(app, ctx, wants_docker)?;
        issues.extend(
            observed
                .issues
                .iter()
                .map(|issue| format!("{:?}", issue.code)),
        );
        let port = observation_port(app, &observed);
        let checks = crate::checks::CheckCoordinator {
            port: &port,
            control: app.control.as_ref(),
            max_age_ms: 30_000,
        };
        let report = checks.observe(&queries, &check_budget(app, ctx, 128), ctx)?;
        for sample in report.samples {
            let key = match (sample.freshness, sample.evidence.outcome) {
                (crate::checks::Freshness::Fresh, EvidenceOutcome::Pass) => "observed",
                (_, EvidenceOutcome::Fail) => "failed",
                _ => "unknown",
            };
            *counts.entry(key).or_default() += 1;
            if let Some(code) = sample.error_code {
                issues.push(format!("{code:?}"));
            }
        }
    }
    if requested.contains("runtime") {
        let runtime_dir = app.work.child("runtime");
        match runtime_dir.and_then(|dir| dir.read("state.json", wire::MAX_WIRE_BYTES)) {
            Ok(Some(_)) => match app.runtime(ctx) {
                Ok(runtime) => {
                    counts.insert("runtimeResources", runtime.resources.len() as u64);
                    counts.insert(
                        match runtime.run_intent {
                            wire::RunIntent::Running => "runIntentRunning",
                            wire::RunIntent::Stopped => "runIntentStopped",
                        },
                        1,
                    );
                }
                Err(_) => issues.push("runtimeUnavailable".to_owned()),
            },
            Ok(None) | Err(hton_storage::Error::NotFound) => {
                issues.push("runtimeStateAbsent".to_owned());
            }
            Err(_) => issues.push("runtimeUnavailable".to_owned()),
        }
    }
    let unverified = if unknown.is_empty() {
        vec![
            "configuration",
            "ports",
            "certificates",
            "journal",
            "tunnel",
            "admin",
            "externalReachability",
        ]
    } else {
        vec!["requestedSectionNotImplemented", "externalReachability"]
    };
    let failed = counts.get("failed").copied().unwrap_or_default() > 0;
    let unknown_observations = counts.get("unknown").copied().unwrap_or_default() > 0;
    Ok(json!({
        "apiVersion": 1,
        "operation": "diag.observe",
        "status": if failed { "failed" } else if unknown_observations || !unknown.is_empty() || counts.is_empty() { "unverified" } else { "succeeded" },
        "requestedSections": requested,
        "observedSections": requested.intersection(&supported).collect::<Vec<_>>(),
        "unsupportedSections": unknown,
        "counts": counts,
        "issues": issues,
        "verbose": verbose,
        "unverified": unverified,
        "writes": false,
        "networkRequests": 0
    }))
}

/// Deliberate owner-facing identity report. External checks are caused only by this
/// explicitly invoked command; the general diagnostic path never reaches the network.
pub fn whoami(app: &HostApplication, ctx: &OperationContext) -> Result<Value> {
    const MAX_DOMAINS: usize = 8;
    let desired = app.desired(ctx)?;
    let mut configured = BTreeSet::<CertificateName>::new();
    let mut configured_domain_count = 0usize;
    if let Some(snapshot) = desired.snapshot.as_ref() {
        let domains = snapshot
            .platform
            .settings
            .get("core")
            .and_then(|core| core.get("edge"))
            .and_then(|edge| edge.get("domains"))
            .and_then(Value::as_array);
        if let Some(domains) = domains {
            configured_domain_count = domains.iter().filter(|value| value.is_string()).count();
            for value in domains.iter().filter_map(Value::as_str).take(MAX_DOMAINS) {
                if let Ok(domain) = CertificateName::new(value) {
                    configured.insert(domain);
                }
            }
        }
    }
    let host = HostObserver::new();
    let cancel = Cancel { app, ctx };
    let remaining = ctx
        .deadline
        .0
        .saturating_sub(app.control.now_ms())
        .min(10_000);
    let ip_budget =
        hton_host::Budget::new(Instant::now() + Duration::from_millis(remaining), 65_536).map_err(
            |_| journal::failure(ctx, ErrorCode::OperationDeadline, "identity budget invalid"),
        )?;
    let public_ipv4 = host.network().public_ipv4(ip_budget, &cancel).ok();

    let empty = ObservedResources {
        runtime: RuntimeObservation {
            resources: BTreeMap::new(),
        },
        native: BTreeMap::new(),
        issues: Vec::new(),
    };
    let port = observation_port(app, &empty);
    let checks = crate::checks::CheckCoordinator {
        port: &port,
        control: app.control.as_ref(),
        max_age_ms: 30_000,
    };
    let report = checks.identity_evidence(
        &configured,
        &check_budget(app, ctx, MAX_DOMAINS as u32 + 1),
        ctx,
    )?;
    let dns = port.dns.lock().map_err(|_| {
        journal::failure(ctx, ErrorCode::IoUnavailable, "identity result unavailable")
    })?;
    let domains: Vec<_> = configured
        .iter()
        .map(|domain| {
            let answer = dns.get(domain.host());
            json!({
                "domain": domain.as_str(),
                "queryName": domain.host(),
                "addresses": answer.map(|answer| answer.addresses.iter().map(ToString::to_string).collect::<Vec<_>>()).unwrap_or_default(),
                "ttlSeconds": answer.map(|answer| answer.ttl_seconds),
                "status": if answer.is_some_and(|answer| !answer.addresses.is_empty()) { "observed" } else { "unverified" }
            })
        })
        .collect();
    let dns_evidence = report
        .checks
        .evidence
        .iter()
        .filter(|evidence| evidence.check_id.as_str().starts_with("identity.dns."))
        .filter(|evidence| evidence.outcome != EvidenceOutcome::Pass)
        .count();
    let mut issues: Vec<_> = desired
        .errors
        .iter()
        .map(|issue| format!("{:?}", issue.code))
        .collect();
    issues.extend(
        report
            .checks
            .issues
            .iter()
            .map(|issue| format!("{:?}", issue.code)),
    );
    if public_ipv4.is_none() {
        issues.push("publicIpv4Unavailable".to_owned());
    }
    let domains_omitted = configured_domain_count.saturating_sub(configured.len());
    let mut unprobed = vec![
        "ipv6",
        "ptr",
        "whois",
        "dns-aaaa-ns-mx-txt-ds",
        "local-nic-addresses",
        "local-listening-ports",
        "external-port-reachability",
    ];
    if domains_omitted > 0 {
        unprobed.push("configured-domains-over-limit");
    }
    Ok(json!({
        "apiVersion": 1,
        "operation": "identity.whoami",
        "status": "unverified",
        "probeStatus": if public_ipv4.is_some() && dns_evidence == 0 && desired.errors.is_empty() { "observed" } else { "partial" },
        "instanceId": app.installation.instance_id,
        "externalIpv4": public_ipv4.map(|address| address.to_string()),
        "externalIpv4Status": if public_ipv4.is_some() { "observed" } else { "unverified" },
        "domains": domains,
        "configuredDomainCount": configured_domain_count,
        "domainsOmitted": domains_omitted,
        "issues": issues,
        "unprobed": unprobed,
        "domainLimit": MAX_DOMAINS,
        "writes": false
    }))
}

/// Shared conservative host TCP/UDP listener collision policy.

pub(crate) fn listener_conflicts(
    graph: &DependencyGraph,
    evidence: &ReservationEvidence,
) -> Vec<CheckIssue> {
    let mut issues = Vec::new();
    for claim in &graph.listener_claims {
        if claim.namespace != ListenerNamespace::Host {
            continue;
        }
        if evidence.reservations.iter().any(|reservation| {
            reservation.transport == claim.transport
                && reservation.port == claim.port
                && reservation.holder_service.as_ref() != Some(&claim.service_id)
        }) {
            issues.push(config::issue(
                ErrorCode::ConfigSemanticConflict,
                None,
                Some(claim.owner.clone()),
                "planned listener conflicts with a host TCP/UDP reservation",
            ));
        }
    }
    issues
}
