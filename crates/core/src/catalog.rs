//! Trusted built-in descriptor catalogue and pure dependency/claim resolution.
use crate::config::{LoadLimits, decode_yaml, digest, issue};
use hton_contracts::{config::*, operations::*};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::{IpAddr, Ipv4Addr},
};

#[derive(Clone, Debug, Serialize)]
pub struct CatalogueEntry {
    pub descriptor: ModuleDescriptor,
    pub implemented: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceDescriptor {
    pub id: ServiceId,
    pub dependencies: BTreeSet<ServiceId>,
    pub mandatory_consumers: BTreeSet<ModuleId>,
    pub foreign: bool,
    pub image_required: bool,
    pub adapter: CapabilityId,
    pub change_action: ActionKind,
}
#[derive(Clone, Debug)]
pub struct ModuleCatalogue {
    version: CatalogueVersion,
    entries: BTreeMap<ModuleId, CatalogueEntry>,
    services: BTreeMap<ServiceId, ServiceDescriptor>,
    profile_suggestions: BTreeMap<ProfileId, Vec<ModuleId>>,
}
#[derive(Clone)]
pub struct ResolutionEvidence {
    /// External/native compatibility evidence. Foreign Amnezia requires this entry.
    pub qualified_modules: BTreeSet<ModuleId>,
    /// Protected resolved HTTP paths; supplied by secret owner, never generated here.
    pub resolved_routes: BTreeMap<ModuleId, Vec<RouteClaim>>,
    pub observations: RuntimeObservation,
    /// Fresh local publication checks bound to exact root bytes; never an
    /// implicit public DNS-A or external identity observation.
    pub publication_evidence: Vec<Evidence>,
    pub now: UnixMillis,
}
impl Default for ResolutionEvidence {
    fn default() -> Self {
        Self {
            qualified_modules: BTreeSet::new(),
            resolved_routes: BTreeMap::new(),
            observations: RuntimeObservation {
                resources: BTreeMap::new(),
            },
            publication_evidence: Vec::new(),
            now: UnixMillis(0),
        }
    }
}
impl std::fmt::Debug for ResolutionEvidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolutionEvidence")
            .field("qualified_modules", &self.qualified_modules)
            .field("resolved_routes", &"[protected]")
            .field("now", &self.now)
            .finish_non_exhaustive()
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Index {
    schema_version: FormatVersion,
    catalogue_version: CatalogueVersion,
    profile_suggestions: BTreeMap<ProfileId, Vec<ModuleId>>,
    modules: Vec<IndexEntry>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IndexEntry {
    id: ModuleId,
    phase: ReleasePhase,
    implemented: bool,
    descriptor: Option<ObjectId>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Services {
    schema_version: FormatVersion,
    services: Vec<ServiceDescriptor>,
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
fn builtin_error() -> CheckIssue {
    issue(
        ErrorCode::ConfigSemanticConflict,
        None,
        None,
        "Trusted module catalogue is invalid or inconsistent",
    )
}
impl ModuleCatalogue {
    pub fn builtin() -> std::result::Result<Self, CheckIssue> {
        let descriptors = BTreeMap::from([
            (
                object("vless"),
                include_bytes!("../../../platform/modules/vless.yaml").as_slice(),
            ),
            (
                object("foreign-amnezia"),
                include_bytes!("../../../platform/modules/foreign-amnezia.yaml").as_slice(),
            ),
        ]);
        Self::from_documents(
            include_bytes!("../../../platform/modules/index.yaml"),
            include_bytes!("../../../platform/modules/services.yaml"),
            &descriptors,
        )
    }
    /// Inputs belong to the protected distribution, never an executable plugin path.
    pub fn from_documents(
        index: &[u8],
        services: &[u8],
        descriptors: &BTreeMap<ObjectId, &[u8]>,
    ) -> std::result::Result<Self, CheckIssue> {
        let index: Index = serde_json::from_value(
            decode_yaml(index, LoadLimits::default()).map_err(|_| builtin_error())?,
        )
        .map_err(|_| builtin_error())?;
        let services: Services = serde_json::from_value(
            decode_yaml(services, LoadLimits::default()).map_err(|_| builtin_error())?,
        )
        .map_err(|_| builtin_error())?;
        if index.schema_version != FormatVersion(1) || services.schema_version != FormatVersion(1) {
            return Err(builtin_error());
        }
        let Index {
            catalogue_version,
            profile_suggestions,
            modules: index_modules,
            ..
        } = index;
        let mut service_map = BTreeMap::new();
        for entry in services.services {
            if !matches!(
                entry.change_action,
                ActionKind::Reload | ActionKind::Restart | ActionKind::Recreate
            ) || service_map.insert(entry.id.clone(), entry).is_some()
            {
                return Err(builtin_error());
            }
        }
        let mut entries = BTreeMap::new();
        for entry in index_modules {
            let descriptor = if let Some(ref descriptor_id) = entry.descriptor {
                let bytes = descriptors.get(descriptor_id).ok_or_else(builtin_error)?;
                let mut value =
                    decode_yaml(bytes, LoadLimits::default()).map_err(|_| builtin_error())?;
                value.as_object_mut().ok_or_else(builtin_error)?.insert(
                    "descriptorRevision".into(),
                    serde_json::to_value(digest(bytes)).map_err(|_| builtin_error())?,
                );
                let descriptor: ModuleDescriptor =
                    serde_json::from_value(value).map_err(|_| builtin_error())?;
                if descriptor.module_id != entry.id || descriptor.phase != entry.phase {
                    return Err(builtin_error());
                }
                descriptor
            } else {
                if entry.implemented {
                    return Err(builtin_error());
                }
                unavailable_descriptor(&entry.id, entry.phase)
            };
            if descriptor
                .services
                .iter()
                .any(|id| !service_map.contains_key(id))
            {
                return Err(builtin_error());
            }
            if entries
                .insert(
                    entry.id,
                    CatalogueEntry {
                        descriptor,
                        implemented: entry.implemented,
                    },
                )
                .is_some()
            {
                return Err(builtin_error());
            }
        }
        let expected_profiles: BTreeSet<_> = ["minimal", "home", "compat", "lab"]
            .into_iter()
            .map(|value| ProfileId::new(value).expect("fixed profile ID"))
            .collect();
        if profile_suggestions.keys().cloned().collect::<BTreeSet<_>>() != expected_profiles {
            return Err(builtin_error());
        }
        for suggested in profile_suggestions.values() {
            let mut unique = BTreeSet::new();
            if suggested.len() > 64
                || suggested.iter().any(|id| {
                    id.as_str() == "core"
                        || !unique.insert(id.clone())
                        || entries.get(id).is_none_or(|entry| {
                            !entry.implemented || entry.descriptor.phase != ReleasePhase::R1
                        })
                })
            {
                return Err(builtin_error());
            }
        }
        let catalogue = Self {
            version: catalogue_version,
            entries,
            services: service_map,
            profile_suggestions,
        };
        catalogue.validate_inventory()?;
        Ok(catalogue)
    }
    pub fn version(&self) -> CatalogueVersion {
        self.version
    }
    pub fn fingerprint(&self) -> Sha256Digest {
        digest(
            &serde_json::to_vec(&(
                self.version,
                &self.entries,
                &self.services,
                &self.profile_suggestions,
            ))
            .expect("trusted serializable catalogue records"),
        )
    }

    pub fn describe(&self, id: &ModuleId) -> Option<&CatalogueEntry> {
        self.entries.get(id)
    }
    pub fn entries(&self) -> &BTreeMap<ModuleId, CatalogueEntry> {
        &self.entries
    }
    pub fn services(&self) -> &BTreeMap<ServiceId, ServiceDescriptor> {
        &self.services
    }
    /// First-run hints only. Callers must not treat them as persisted selection
    /// or consent to install, associate, enable or mutate any module.
    pub fn profile_suggestions(&self, profile: &ProfileId) -> Option<&[ModuleId]> {
        self.profile_suggestions.get(profile).map(Vec::as_slice)
    }
    fn validate_inventory(&self) -> std::result::Result<(), CheckIssue> {
        let mut allocation_ids = BTreeSet::new();
        let mut allocation_services = BTreeSet::new();
        for entry in self.entries.values() {
            if entry
                .descriptor
                .dependencies
                .iter()
                .any(|id| !self.entries.contains_key(id))
            {
                return Err(builtin_error());
            }
            if entry.descriptor.listener_claims.iter().any(|claim| {
                claim.owner != entry.descriptor.module_id
                    || !entry.descriptor.services.contains(&claim.service_id)
            }) {
                return Err(builtin_error());
            }
            if let Some(binding) = &entry.descriptor.port_allocation {
                let expected_settings_document =
                    format!("module.{}", entry.descriptor.module_id.as_str());
                if !entry.descriptor.services.contains(&binding.service_id)
                    || !self.services.contains_key(&binding.service_id)
                    || binding.settings_document.as_str() != expected_settings_document.as_str()
                    || !allocation_ids.insert(binding.allocation_id.clone())
                    || !allocation_services.insert(binding.service_id.clone())
                    || binding.listener.namespace != ListenerNamespace::Host
                    || binding
                        .listener
                        .addresses
                        .iter()
                        .any(|address| address.is_ipv6())
                    || binding.fields.is_empty()
                    || binding.listener.addresses.is_empty()
                    || binding.fields.iter().any(|field| {
                        field.path.split('.').count() < 3
                            || !field.path.starts_with("settings.")
                            || field.path.split('.').any(str::is_empty)
                    })
                    || binding.fields.iter().enumerate().any(|(index, field)| {
                        binding.fields.iter().skip(index + 1).any(|other| {
                            field.path == other.path || field.transport == other.transport
                        })
                    })
                    || binding
                        .listener
                        .addresses
                        .iter()
                        .enumerate()
                        .any(|(index, address)| {
                            binding
                                .listener
                                .addresses
                                .iter()
                                .skip(index + 1)
                                .any(|other| address == other)
                        })
                {
                    return Err(builtin_error());
                }
            }
        }
        for entry in self.entries.values() {
            let descriptor = &entry.descriptor;
            if descriptor.route_claims.iter().any(|claim| {
                claim.owner != descriptor.module_id
                    || !self.services.contains_key(&claim.listener_service)
                    || !self.services.contains_key(&claim.target_service)
            }) || descriptor.certificate_claims.iter().any(|claim| {
                claim.owner != descriptor.module_id
                    || !self.services.contains_key(&claim.consumer)
                    || claim.names.iter().any(|name| !valid_hostname(name, true))
            }) || descriptor
                .state
                .iter()
                .any(|claim| claim.owner != descriptor.module_id)
            {
                return Err(builtin_error());
            }
        }
        let all_modules: BTreeSet<_> = self.entries.keys().cloned().collect();
        topo_modules(&all_modules, &self.entries).map_err(|_| builtin_error())?;
        for entry in self.services.values() {
            if entry
                .dependencies
                .iter()
                .any(|id| !self.services.contains_key(id))
                || entry
                    .mandatory_consumers
                    .iter()
                    .any(|id| id.as_str() != "core")
            {
                return Err(builtin_error());
            }
        }
        self.service_order(&self.services.keys().cloned().collect())
            .map_err(|_| builtin_error())?;
        Ok(())
    }
    pub fn service_order(
        &self,
        selected: &BTreeSet<ServiceId>,
    ) -> std::result::Result<Vec<ServiceId>, CheckIssue> {
        fn visit(
            id: &ServiceId,
            selected: &BTreeSet<ServiceId>,
            services: &BTreeMap<ServiceId, ServiceDescriptor>,
            visiting: &mut BTreeSet<ServiceId>,
            done: &mut BTreeSet<ServiceId>,
            result: &mut Vec<ServiceId>,
        ) -> std::result::Result<(), CheckIssue> {
            if done.contains(id) {
                return Ok(());
            }
            if !visiting.insert(id.clone()) {
                return Err(builtin_error());
            }
            for dep in &services.get(id).ok_or_else(builtin_error)?.dependencies {
                if !selected.contains(dep) {
                    return Err(builtin_error());
                }
                visit(dep, selected, services, visiting, done, result)?;
            }
            visiting.remove(id);
            done.insert(id.clone());
            result.push(id.clone());
            Ok(())
        }
        let mut result = Vec::new();
        let mut visiting = BTreeSet::new();
        let mut done = BTreeSet::new();
        for id in selected {
            visit(
                id,
                selected,
                &self.services,
                &mut visiting,
                &mut done,
                &mut result,
            )?;
        }
        Ok(result)
    }
    pub fn resolve(
        &self,
        snapshot: &NormalizedSnapshot,
        release: ReleasePhase,
        evidence: &ResolutionEvidence,
    ) -> DependencyGraph {
        self.resolve_inner(snapshot, release, evidence, true)
    }

    /// Resolve install structure before certificates are issued. This retains
    /// every structural/module conflict but defers only local HTTPS certificate
    /// readiness; the caller must still carry the ordinary `resolve` blockers
    /// into InstallPlan and may not treat this graph as activation-ready.
    pub(crate) fn resolve_for_install_preflight(
        &self,
        snapshot: &NormalizedSnapshot,
        release: ReleasePhase,
        evidence: &ResolutionEvidence,
    ) -> DependencyGraph {
        self.resolve_inner(snapshot, release, evidence, false)
    }

    fn resolve_inner(
        &self,
        snapshot: &NormalizedSnapshot,
        release: ReleasePhase,
        evidence: &ResolutionEvidence,
        require_publication_evidence: bool,
    ) -> DependencyGraph {
        let mut graph = DependencyGraph {
            enabled_modules: BTreeSet::from([module("core")]),
            service_consumers: BTreeMap::new(),
            topological_order: Vec::new(),
            listener_claims: Vec::new(),
            allocation_bindings: Vec::new(),
            route_claims: Vec::new(),
            certificate_claims: Vec::new(),
            state_claims: Vec::new(),
            conflicts: Vec::new(),
        };
        for (id, feature) in &snapshot.desired.features {
            if id.as_str() == "core" {
                graph.conflicts.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(id.clone()),
                    "Mandatory core cannot be toggled",
                ));
                continue;
            }
            if !feature.enabled {
                continue;
            }
            let Some(entry) = self.entries.get(id) else {
                graph.conflicts.push(issue(
                    ErrorCode::ModuleUnknown,
                    None,
                    Some(id.clone()),
                    "Enabled module is unknown",
                ));
                continue;
            };
            if !entry.implemented || !phase_available(entry.descriptor.phase, release) {
                graph.conflicts.push(issue(
                    ErrorCode::ModuleUnsupported,
                    None,
                    Some(id.clone()),
                    "Module implementation is unavailable in the selected release",
                ));
                continue;
            }
            graph.enabled_modules.insert(id.clone());
            if id.as_str() == "foreign-amnezia" && !evidence.qualified_modules.contains(id) {
                graph.conflicts.push(issue(ErrorCode::ModuleUnsupported, None, Some(id.clone()), "Amnezia native domain, certificate and restart behavior has not been qualified"));
            }
        }
        for descriptor in self
            .entries
            .values()
            .map(|e| &e.descriptor)
            .filter(|d| graph.enabled_modules.contains(&d.module_id))
        {
            for dependency in &descriptor.dependencies {
                if !graph.enabled_modules.contains(dependency) {
                    graph.conflicts.push(issue(
                        ErrorCode::ModuleIncomplete,
                        None,
                        Some(descriptor.module_id.clone()),
                        "A required feature is disabled; enable an explicit dependency batch",
                    ));
                }
            }
            for service in &descriptor.services {
                graph
                    .service_consumers
                    .entry(service.clone())
                    .or_default()
                    .insert(descriptor.module_id.clone());
            }
            graph
                .listener_claims
                .extend(descriptor.listener_claims.clone());
            if let Some(binding) = &descriptor.port_allocation {
                graph.allocation_bindings.push(binding.clone());
            }
            graph.route_claims.extend(descriptor.route_claims.clone());
            graph
                .certificate_claims
                .extend(descriptor.certificate_claims.clone());
            graph.state_claims.extend(descriptor.state.clone());
        }
        for entry in self.services.values() {
            if !entry.mandatory_consumers.is_empty() {
                graph
                    .service_consumers
                    .entry(entry.id.clone())
                    .or_default()
                    .extend(entry.mandatory_consumers.clone());
            }
        }
        // Service dependencies add private engines, never optional feature enablement.
        loop {
            let mut additions = Vec::new();
            for (id, consumers) in &graph.service_consumers {
                if let Some(entry) = self.services.get(id) {
                    for dep in &entry.dependencies {
                        for consumer in consumers {
                            if !graph
                                .service_consumers
                                .get(dep)
                                .is_some_and(|set| set.contains(consumer))
                            {
                                additions.push((dep.clone(), consumer.clone()));
                            }
                        }
                    }
                }
            }
            if additions.is_empty() {
                break;
            }
            for (id, consumer) in additions {
                graph
                    .service_consumers
                    .entry(id)
                    .or_default()
                    .insert(consumer);
            }
        }
        let enabled = graph
            .enabled_modules
            .iter()
            .filter(|id| id.as_str() != "core")
            .cloned()
            .collect();
        match topo_modules(&enabled, &self.entries) {
            Ok(order) => {
                graph.topological_order.push(module("core"));
                graph.topological_order.extend(order);
            }
            Err(()) => graph.conflicts.push(builtin_error()),
        }
        append_dynamic_claims(snapshot, evidence, &mut graph, require_publication_evidence);
        graph.conflicts.extend(check_claims(&graph));
        graph
    }

    /// Materialize allocated host listeners from the same trusted descriptor mapping
    /// used by the allocation planner. It performs no probes and changes no config.
    pub fn bind_allocation(
        &self,
        graph: &DependencyGraph,
        candidate: &PortAllocationCandidate,
        snapshot: &NormalizedSnapshot,
    ) -> DependencyGraph {
        let mut bound = graph.clone();
        let expected_services: BTreeSet<_> = graph
            .allocation_bindings
            .iter()
            .map(|binding| binding.service_id.clone())
            .collect();
        if candidate
            .allocations
            .keys()
            .any(|service| !expected_services.contains(service))
        {
            bound.conflicts.push(issue(
                ErrorCode::ConfigSemanticConflict,
                None,
                None,
                "Port allocation candidate contains an undeclared service",
            ));
            return bound;
        }
        for binding in &graph.allocation_bindings {
            let Some(allocation) = candidate.allocations.get(&binding.service_id) else {
                bound.conflicts.push(issue(
                    ErrorCode::ModuleIncomplete,
                    None,
                    Some(binding.settings_document.clone()),
                    "Selected VPN service has no port allocation candidate",
                ));
                continue;
            };
            if allocation.settings_document != binding.settings_document
                || allocation.binding_group != binding.binding_group
                || allocation.fields.len() != binding.fields.len()
                || binding.fields.iter().any(|field| {
                    !allocation.fields.iter().any(|candidate_field| {
                        field.path == candidate_field.path
                            && field.transport == candidate_field.transport
                    })
                })
                || allocation.prior_fields.len() != binding.fields.len()
                || binding
                    .fields
                    .iter()
                    .any(|field| !allocation.prior_fields.contains_key(&field.path))
            {
                bound.conflicts.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(binding.settings_document.clone()),
                    "Port allocation candidate does not match the trusted service mapping",
                ));
                continue;
            }
            let module_id = self
                .entries
                .values()
                .find(|entry| entry.descriptor.port_allocation.as_ref() == Some(binding))
                .map(|entry| entry.descriptor.module_id.clone());
            let Some(module_id) = module_id else {
                bound.conflicts.push(builtin_error());
                continue;
            };
            let Some(owner) = self.describe(&module_id) else {
                bound.conflicts.push(builtin_error());
                continue;
            };
            if allocation.descriptor_revision != owner.descriptor.descriptor_revision {
                bound.conflicts.push(issue(
                    ErrorCode::ConfigReadConflict,
                    Some(binding.settings_document.clone()),
                    Some(module_id),
                    "allocation candidate was planned against another module descriptor revision",
                ));
                continue;
            }
            let settings = snapshot.desired.module_settings.get(&module_id);
            let settings_value = match settings {
                Some(settings) => settings.settings.clone(),
                None if candidate
                    .read_set
                    .absent_objects
                    .contains(&binding.settings_document)
                    && !candidate
                        .read_set
                        .objects
                        .contains_key(&binding.settings_document) =>
                {
                    owner.descriptor.defaults.settings.clone()
                }
                None => {
                    bound.conflicts.push(issue(
                        ErrorCode::ModuleIncomplete,
                        None,
                        Some(module_id),
                        "Port allocation settings document is neither present nor proven absent",
                    ));
                    continue;
                }
            };
            let fields_match = binding.fields.iter().all(|field| {
                let Ok(observed) = setting_port(&settings_value, &field.path) else {
                    return false;
                };
                allocation.prior_fields.get(&field.path).copied() == Some(observed)
                    && allocation.fields.iter().any(|candidate_field| {
                        candidate_field.path == field.path
                            && candidate_field.transport == field.transport
                            && candidate_field.saved_port == observed
                    })
            });
            if !fields_match {
                bound.conflicts.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(module_id),
                    "Port allocation candidate settings do not match the validated snapshot",
                ));
                continue;
            }
            for field in &binding.fields {
                for address in &binding.listener.addresses {
                    bound.listener_claims.push(ListenerClaim {
                        namespace: binding.listener.namespace.clone(),
                        owner: module_id.clone(),
                        service_id: binding.service_id.clone(),
                        transport: field.transport,
                        address: *address,
                        port: allocation.port,
                        public: binding.listener.exposure == ListenerExposure::Public,
                    });
                }
            }
        }
        bound.conflicts.extend(check_claims(&bound));
        bound
    }

    /// Return selected descriptor bindings enriched with the exact saved leaf
    /// values from one validated snapshot. This is the allocation planner's only
    /// source for reading existing service ports.
    pub fn allocation_bindings_for(
        &self,
        graph: &DependencyGraph,
        snapshot: &NormalizedSnapshot,
    ) -> std::result::Result<Vec<ServicePortBinding>, Vec<CheckIssue>> {
        let mut bindings = Vec::with_capacity(graph.allocation_bindings.len());
        let mut issues = Vec::new();
        for declared in &graph.allocation_bindings {
            let owner = self
                .entries
                .values()
                .find(|entry| entry.descriptor.port_allocation.as_ref() == Some(declared));
            let Some(owner) = owner else {
                issues.push(builtin_error());
                continue;
            };
            let module_id = &owner.descriptor.module_id;
            let settings = snapshot.desired.module_settings.get(module_id);
            if settings.is_none()
                && (snapshot.desired.read_set.coherence != ReadCoherence::Coherent
                    || snapshot
                        .desired
                        .read_set
                        .objects
                        .contains_key(&declared.settings_document))
            {
                issues.push(issue(
                    ErrorCode::ModuleIncomplete,
                    Some(declared.settings_document.clone()),
                    Some(module_id.clone()),
                    "selected VPN settings are unreadable and cannot be treated as absent",
                ));
                continue;
            }
            let settings_value = settings
                .map(|settings| settings.settings.clone())
                .unwrap_or_else(|| owner.descriptor.defaults.settings.clone());
            let mut binding = declared.clone();
            binding.descriptor_revision = Some(owner.descriptor.descriptor_revision.clone());
            let mut valid = true;
            for field in &mut binding.fields {
                match setting_port(&settings_value, &field.path) {
                    Ok(saved_port) => field.saved_port = saved_port,
                    Err(()) => valid = false,
                }
            }
            if !valid {
                issues.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(module_id.clone()),
                    "declared VPN port setting has an invalid type or shape",
                ));
            } else {
                bindings.push(binding);
            }
        }
        if issues.is_empty() {
            Ok(bindings)
        } else {
            Err(issues)
        }
    }

    /// Project saved allocations for installed services not selected in the
    /// current graph, so disabled/retained modules still reserve their number.
    pub fn retained_allocations_for(
        &self,
        selected: &BTreeSet<ServiceId>,
        retained_services: &BTreeSet<ServiceId>,
        snapshot: &NormalizedSnapshot,
    ) -> std::result::Result<Vec<SavedAllocation>, Vec<CheckIssue>> {
        let mut saved = Vec::new();
        let mut issues = Vec::new();
        for service_id in retained_services.difference(selected) {
            let owners = self
                .entries
                .values()
                .filter_map(|entry| {
                    entry
                        .descriptor
                        .port_allocation
                        .as_ref()
                        .filter(|binding| &binding.service_id == service_id)
                        .map(|binding| (&entry.descriptor, binding))
                })
                .collect::<Vec<_>>();
            if owners.is_empty() {
                let known_service = self
                    .entries
                    .values()
                    .any(|entry| entry.descriptor.services.contains(service_id));
                if known_service {
                    continue;
                }
                issues.push(builtin_error());
                continue;
            }
            if owners.len() != 1 {
                issues.push(builtin_error());
                continue;
            }
            let (descriptor, binding) = owners[0];
            let module_id = &descriptor.module_id;
            let Some(settings) = snapshot.desired.module_settings.get(module_id) else {
                issues.push(issue(
                    ErrorCode::ModuleIncomplete,
                    None,
                    Some(module_id.clone()),
                    "retained VPN service settings are unavailable",
                ));
                continue;
            };
            let values = binding
                .fields
                .iter()
                .map(|field| setting_port(&settings.settings, &field.path))
                .collect::<std::result::Result<Vec<_>, _>>();
            let Ok(values) = values else {
                issues.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(module_id.clone()),
                    "retained VPN port setting has an invalid type or shape",
                ));
                continue;
            };
            let ports = values.iter().flatten().copied().collect::<BTreeSet<_>>();
            match ports.len() {
                0 => {}
                1 if values.iter().all(Option::is_some) => {
                    let port = *ports.iter().next().expect("one retained port");
                    saved.push(SavedAllocation {
                        service_id: service_id.clone(),
                        settings_document: binding.settings_document.clone(),
                        binding_group: binding.binding_group.clone(),
                        port,
                    });
                }
                _ => issues.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(module_id.clone()),
                    "retained VPN TCP/UDP port fields are incomplete or inconsistent",
                )),
            }
        }
        if issues.is_empty() {
            Ok(saved)
        } else {
            Err(issues)
        }
    }
}

fn setting_port(settings: &BTreeMap<String, Value>, path: &str) -> Result<Option<u16>, ()> {
    let mut parts = path.split('.');
    if parts.next() != Some("settings") {
        return Err(());
    }
    let Some(first) = parts.next() else {
        return Err(());
    };
    let Some(mut value) = settings.get(first) else {
        return Ok(None);
    };
    for part in parts {
        let Some(next) = value.get(part) else {
            return if value.is_object() { Ok(None) } else { Err(()) };
        };
        value = next;
    }
    value
        .as_u64()
        .and_then(|port| u16::try_from(port).ok())
        .map(Some)
        .ok_or(())
}
fn unavailable_descriptor(id: &ModuleId, phase: ReleasePhase) -> ModuleDescriptor {
    ModuleDescriptor {
        module_id: id.clone(),
        phase,
        location: KnownLocation {
            inventory_id: object("modules"),
            components: vec![object(&format!("{id}.yaml"))],
        },
        config_schema: object("module-v1"),
        config_version: FormatVersion(1),
        descriptor_revision: digest(id.as_str().as_bytes()),
        defaults: ModuleSettings {
            schema_version: FormatVersion(1),
            settings: BTreeMap::new(),
        },
        dependencies: BTreeSet::new(),
        services: BTreeSet::new(),
        listener_claims: Vec::new(),
        port_allocation: None,
        route_claims: Vec::new(),
        certificate_claims: Vec::new(),
        privileges: BTreeSet::new(),
        state: Vec::new(),
        capabilities: CapabilityBindings {
            check: CapabilityId::new("unavailable").expect("static capability ID"),
            observe: None,
            render: None,
            setup: None,
            update: None,
            foreign_lifecycle: None,
        },
    }
}
fn phase_available(phase: ReleasePhase, release: ReleasePhase) -> bool {
    match release {
        ReleasePhase::R1 => phase == ReleasePhase::R1,
        ReleasePhase::R2 => matches!(phase, ReleasePhase::R1 | ReleasePhase::R2),
        ReleasePhase::LaterManagement => matches!(
            phase,
            ReleasePhase::R1 | ReleasePhase::R2 | ReleasePhase::LaterManagement
        ),
        _ => phase == release || phase == ReleasePhase::R1,
    }
}
fn topo_modules(
    selected: &BTreeSet<ModuleId>,
    entries: &BTreeMap<ModuleId, CatalogueEntry>,
) -> std::result::Result<Vec<ModuleId>, ()> {
    fn visit(
        id: &ModuleId,
        selected: &BTreeSet<ModuleId>,
        entries: &BTreeMap<ModuleId, CatalogueEntry>,
        visiting: &mut BTreeSet<ModuleId>,
        done: &mut BTreeSet<ModuleId>,
        result: &mut Vec<ModuleId>,
    ) -> std::result::Result<(), ()> {
        if done.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id.clone()) {
            return Err(());
        }
        for dependency in &entries.get(id).ok_or(())?.descriptor.dependencies {
            if selected.contains(dependency) {
                visit(dependency, selected, entries, visiting, done, result)?;
            }
        }
        visiting.remove(id);
        done.insert(id.clone());
        result.push(id.clone());
        Ok(())
    }
    let mut result = Vec::new();
    let mut visiting = BTreeSet::new();
    let mut done = BTreeSet::new();
    for id in selected {
        visit(id, selected, entries, &mut visiting, &mut done, &mut result)?;
    }
    Ok(result)
}
fn append_dynamic_claims(
    snapshot: &NormalizedSnapshot,
    evidence: &ResolutionEvidence,
    graph: &mut DependencyGraph,
    require_publication_evidence: bool,
) {
    let Some(core) = snapshot.desired.platform.settings.get("core") else {
        graph.conflicts.push(issue(
            ErrorCode::ConfigSemanticConflict,
            None,
            None,
            "Mandatory ingress configuration is absent",
        ));
        return;
    };
    let edge = &core["edge"];
    let listen = &edge["listen"];
    let families = listen["addressFamilies"].as_array();
    if families.is_none_or(|families| families.len() != 1 || families[0] != json!("ipv4")) {
        graph.conflicts.push(issue(
            ErrorCode::ModuleUnsupported,
            None,
            None,
            "R1 ingress requires IPv4 only",
        ));
    }
    let port = listen["port"]
        .as_u64()
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port != 0);
    let scope = listen["scope"].as_str().unwrap_or("public");
    let address = match scope {
        "public" | "project" => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        "host-loopback" => IpAddr::V4(Ipv4Addr::LOCALHOST),
        _ => {
            graph.conflicts.push(issue(
                ErrorCode::ConfigSemanticConflict,
                None,
                None,
                "Ingress listener scope is invalid",
            ));
            return;
        }
    };
    if let Some(port) = port {
        graph.listener_claims.push(ListenerClaim {
            namespace: if scope == "project" {
                ListenerNamespace::Project {
                    network_id: object("vpn-net"),
                }
            } else {
                ListenerNamespace::Host
            },
            owner: module("core"),
            service_id: service("haproxy"),
            transport: Transport::Tcp,
            address,
            port,
            public: scope == "public",
        });
    } else {
        graph.conflicts.push(issue(
            ErrorCode::ConfigSemanticConflict,
            None,
            None,
            "Ingress listener port is invalid",
        ));
    }
    let domains: Vec<String> = edge["domains"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|v| v.to_ascii_lowercase())
        .collect();
    if scope == "public" && domains.is_empty() {
        graph.conflicts.push(issue(ErrorCode::ModuleIncomplete,None,None,"Public HTTPS activation requires a real configured domain and native certificate; unpublished setup may remain incomplete"));
    }
    if require_publication_evidence && !domains.is_empty() {
        let root_revision = snapshot
            .desired
            .read_set
            .objects
            .get(&object("root"))
            .map(|dependency| &dependency.revision.digest);
        // Publication readiness is based on local certificate evidence. Public
        // DNS-A is an explicit identity observation and must not be probed by
        // catalogue resolution or installation (OR-8 / D94).
        for name in ["certificates.native"] {
            let found = evidence.publication_evidence.iter().any(|check| {
                check.check_id.as_str() == name
                    && check
                        .module_id
                        .as_ref()
                        .is_some_and(|id| id.as_str() == "core")
                    && check.outcome == EvidenceOutcome::Pass
                    && Some(&check.input_fingerprint) == root_revision
                    && check.timestamp.0 <= evidence.now.0
                    && evidence.now.0.saturating_sub(check.timestamp.0) <= 300_000
            });
            if !found {
                graph.conflicts.push(issue(ErrorCode::ModuleIncomplete,None,None,"HTTPS publication needs fresh local certificate evidence bound to the current root configuration"));
            }
        }
    }
    for domain in &domains {
        if !valid_hostname(domain, false) {
            graph.conflicts.push(issue(
                ErrorCode::ConfigSemanticConflict,
                None,
                None,
                "An ingress domain is not a valid concrete DNS hostname",
            ));
        }
    }
    // The ordinary website owns the HTTPS fallback; exact foreign SNI bypasses it.
    graph.route_claims.push(RouteClaim {
        owner: module("core"),
        listener_service: service("haproxy"),
        matcher: RouteMatch::DefaultHttps,
        target_service: service("caddy"),
    });
    if !domains.is_empty() {
        graph.certificate_claims.push(CertificateClaim {
            owner: module("core"),
            names: domains.iter().cloned().collect(),
            consumer: service("caddy"),
            foreign_owned: false,
        });
    }
    for id in &graph.enabled_modules {
        if let Some(routes) = evidence.resolved_routes.get(id) {
            if routes.iter().any(|r| {
                r.owner != *id
                    || !graph.service_consumers.contains_key(&r.target_service)
                    || !graph.service_consumers.contains_key(&r.listener_service)
            }) {
                graph.conflicts.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(id.clone()),
                    "Supplied route claims do not match their owner or service closure",
                ));
            } else {
                graph.route_claims.extend(routes.clone());
            }
        }
        if id.as_str() == "vless" {
            if domains.is_empty() {
                graph.conflicts.push(issue(
                    ErrorCode::ModuleIncomplete,
                    None,
                    Some(id.clone()),
                    "Published VLESS requires a configured domain",
                ));
            }
            if evidence
                .resolved_routes
                .get(id)
                .is_none_or(|routes| routes.is_empty())
            {
                graph.conflicts.push(issue(
                    ErrorCode::ModuleIncomplete,
                    None,
                    Some(id.clone()),
                    "VLESS route path has no protected resolved-secret evidence",
                ));
            }
        }
        if id.as_str() == "foreign-amnezia" {
            let names = snapshot
                .desired
                .module_settings
                .get(id)
                .and_then(|settings| settings.settings.get("serverNames"))
                .and_then(Value::as_array);
            if names.is_none_or(Vec::is_empty) {
                graph.conflicts.push(issue(
                    ErrorCode::ModuleIncomplete,
                    None,
                    Some(id.clone()),
                    "Associated Amnezia needs explicit native certificate server names",
                ));
            }
            for name in names.into_iter().flatten() {
                let Some(name) = name.as_str() else {
                    continue;
                };
                let name = name.to_ascii_lowercase();
                if !valid_hostname(&name, false) {
                    graph.conflicts.push(issue(
                        ErrorCode::ConfigSemanticConflict,
                        None,
                        Some(id.clone()),
                        "Amnezia passthrough requires a concrete DNS SNI hostname",
                    ));
                    continue;
                }
                if domains.contains(&name) {
                    graph.conflicts.push(issue(
                        ErrorCode::ConfigSemanticConflict,
                        None,
                        Some(id.clone()),
                        "Amnezia passthrough SNI conflicts with the ordinary HTTPS domain",
                    ));
                }
                graph.route_claims.push(RouteClaim {
                    owner: id.clone(),
                    listener_service: service("haproxy"),
                    matcher: RouteMatch::Sni {
                        hostname: name.clone(),
                    },
                    target_service: service("foreign-amnezia"),
                });
                graph.certificate_claims.push(CertificateClaim {
                    owner: id.clone(),
                    names: BTreeSet::from([name]),
                    consumer: service("foreign-amnezia"),
                    foreign_owned: true,
                });
            }
        }
    }
}
pub fn valid_hostname(name: &str, wildcard: bool) -> bool {
    let name = if wildcard {
        name.strip_prefix("*.").unwrap_or(name)
    } else {
        name
    };
    name.len() <= 253
        && name.contains('.')
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
        })
}
pub fn listener_overlap(left: &ListenerClaim, right: &ListenerClaim) -> bool {
    if left.transport != right.transport
        || left.port != right.port
        || left.namespace != right.namespace
    {
        return false;
    }
    if left.address == right.address {
        return true;
    }
    match (left.address, right.address) {
        (IpAddr::V4(a), IpAddr::V4(b)) => a.is_unspecified() || b.is_unspecified(),
        (IpAddr::V6(a), IpAddr::V6(b)) => a.is_unspecified() || b.is_unspecified(),
        (IpAddr::V6(a), IpAddr::V4(b)) | (IpAddr::V4(b), IpAddr::V6(a)) => {
            a.is_unspecified()
                || a.to_ipv4_mapped()
                    .is_some_and(|a| a == b || a.is_unspecified() || b.is_unspecified())
        }
    }
}
fn hostname_overlap(left: &str, right: &str) -> bool {
    let left = left.to_ascii_lowercase();
    let right = right.to_ascii_lowercase();
    if left == right {
        return true;
    }
    fn wildcard_matches(pattern: &str, name: &str) -> bool {
        pattern.strip_prefix("*.").is_some_and(|suffix| {
            name.strip_suffix(suffix).is_some_and(|prefix| {
                prefix.ends_with('.')
                    && prefix[..prefix.len() - 1].find('.').is_none()
                    && prefix.len() > 1
            })
        })
    }
    wildcard_matches(&left, &right) || wildcard_matches(&right, &left)
}
fn path_overlap(left: &str, right: &str) -> bool {
    left.starts_with(right) || right.starts_with(left)
}
pub fn check_claims(graph: &DependencyGraph) -> Vec<CheckIssue> {
    let mut issues = Vec::new();
    for (index, left) in graph.listener_claims.iter().enumerate() {
        if left.port == 0 {
            issues.push(issue(
                ErrorCode::ConfigSemanticConflict,
                None,
                Some(left.owner.clone()),
                "Listener port must be nonzero",
            ));
        }
        for right in graph.listener_claims.iter().skip(index + 1) {
            if listener_overlap(left, right) {
                issues.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(right.owner.clone()),
                    "Listener ownership overlaps an existing address, transport and port claim",
                ));
            }
        }
    }
    for (index, left) in graph.route_claims.iter().enumerate() {
        let hostname = match &left.matcher {
            RouteMatch::Sni { hostname } | RouteMatch::Http { hostname, .. } => Some(hostname),
            RouteMatch::DefaultHttps => None,
        };
        if hostname.is_some_and(|name| !valid_hostname(name, true)) {
            issues.push(issue(
                ErrorCode::ConfigSemanticConflict,
                None,
                Some(left.owner.clone()),
                "Route hostname is invalid",
            ));
        }
        if let RouteMatch::Http { path_prefix, .. } = &left.matcher {
            if !path_prefix.starts_with('/')
                || path_prefix.contains(['?', '#', '\\'])
                || path_prefix.contains("..")
                || path_prefix.contains('%')
                || path_prefix.contains("//")
                || path_prefix
                    .bytes()
                    .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
            {
                issues.push(issue(ErrorCode::ConfigSemanticConflict,None,Some(left.owner.clone()),"HTTP route path must be an unambiguous absolute path without encoded separators"));
            }
        }
        for right in graph.route_claims.iter().skip(index + 1) {
            if left.listener_service != right.listener_service {
                continue;
            }
            let overlap = match (&left.matcher, &right.matcher) {
                (RouteMatch::DefaultHttps, RouteMatch::DefaultHttps) => true,
                (RouteMatch::Sni { hostname: a }, RouteMatch::Sni { hostname: b }) => {
                    hostname_overlap(a, b)
                }
                (
                    RouteMatch::Http {
                        hostname: a,
                        path_prefix: p,
                    },
                    RouteMatch::Http {
                        hostname: b,
                        path_prefix: q,
                    },
                ) => hostname_overlap(a, b) && path_overlap(p, q),
                (RouteMatch::Sni { hostname: a }, RouteMatch::Http { hostname: b, .. })
                | (RouteMatch::Http { hostname: b, .. }, RouteMatch::Sni { hostname: a }) => {
                    hostname_overlap(a, b)
                }
                _ => false,
            };
            if overlap {
                issues.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(right.owner.clone()),
                    "SNI or HTTP path ownership overlaps another route",
                ));
            }
        }
    }
    for (index, left) in graph.state_claims.iter().enumerate() {
        for right in graph.state_claims.iter().skip(index + 1) {
            if left.object_id == right.object_id
                && (left.exclusive || right.exclusive)
                && left.owner != right.owner
            {
                issues.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(right.owner.clone()),
                    "Exclusive state is claimed by multiple modules",
                ));
            }
        }
    }
    for (index, left) in graph.certificate_claims.iter().enumerate() {
        for right in graph.certificate_claims.iter().skip(index + 1) {
            if left.foreign_owned != right.foreign_owned
                && left
                    .names
                    .iter()
                    .any(|a| right.names.iter().any(|b| hostname_overlap(a, b)))
            {
                issues.push(issue(
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    Some(right.owner.clone()),
                    "Native foreign and HTON certificate ownership overlap",
                ));
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    fn claim(transport: Transport, port: u16, address: IpAddr) -> ListenerClaim {
        ListenerClaim {
            namespace: ListenerNamespace::Host,
            owner: ModuleId::new("owner").unwrap(),
            service_id: ServiceId::new("svc").unwrap(),
            transport,
            address,
            port,
            public: false,
        }
    }

    #[test]
    fn valid_hostname_accepts_well_formed() {
        for name in [
            "example.com",
            "a.example.com",
            "a-b.example.com",
            "sub.domain.tld",
            "xn--e1afmkfd.xn--p1ai",
        ] {
            assert!(valid_hostname(name, false), "should accept {name:?}");
        }
    }

    #[test]
    fn valid_hostname_rejects_malformed() {
        for name in [
            "example",        // no dot
            ".example.com",   // empty first label
            "example.com.",   // empty last label
            "a..example.com", // empty middle label
            "-example.com",   // leading dash
            "example-.com",   // trailing dash
            "exam ple.com",   // whitespace
            "exam!ple.com",   // invalid char
        ] {
            assert!(!valid_hostname(name, false), "should reject {name:?}");
        }
    }

    #[test]
    fn valid_hostname_enforces_label_length() {
        let label63 = "a".repeat(63);
        let label64 = "a".repeat(64);
        assert!(valid_hostname(&format!("{label63}.com"), false));
        assert!(!valid_hostname(&format!("{label64}.com"), false));
    }

    #[test]
    fn wildcard_hostname_is_prefix_only() {
        assert!(valid_hostname("*.example.com", true));
        assert!(valid_hostname("example.com", true)); // wildcard optional
        assert!(!valid_hostname("*.example.com", false)); // not allowed without flag
    }

    #[test]
    fn listener_overlap_same_claim() {
        let a = claim(Transport::Tcp, 443, "192.0.2.1".parse().unwrap());
        let b = claim(Transport::Tcp, 443, "192.0.2.1".parse().unwrap());
        assert!(listener_overlap(&a, &b));
    }

    #[test]
    fn listener_overlap_distinct_port_or_transport() {
        let a = claim(Transport::Tcp, 443, "192.0.2.1".parse().unwrap());
        assert!(!listener_overlap(
            &a,
            &claim(Transport::Tcp, 8443, "192.0.2.1".parse().unwrap())
        ));
        assert!(!listener_overlap(
            &a,
            &claim(Transport::Udp, 443, "192.0.2.1".parse().unwrap())
        ));
    }

    #[test]
    fn listener_overlap_unspecified_v4_wildcard() {
        let wild = claim(Transport::Tcp, 443, "0.0.0.0".parse().unwrap());
        let specific = claim(Transport::Tcp, 443, "192.0.2.1".parse().unwrap());
        assert!(listener_overlap(&wild, &specific));
        assert!(!listener_overlap(
            &claim(Transport::Tcp, 443, "192.0.2.1".parse().unwrap()),
            &claim(Transport::Tcp, 443, "198.51.100.7".parse().unwrap())
        ));
    }

    #[test]
    fn listener_overlap_unspecified_v6_wildcard() {
        let wild = claim(Transport::Tcp, 443, "::".parse().unwrap());
        let specific = claim(Transport::Tcp, 443, "2001:db8::1".parse().unwrap());
        assert!(listener_overlap(&wild, &specific));
    }

    #[test]
    fn listener_overlap_v4_mapped_matches_v4() {
        let mapped = claim(Transport::Tcp, 443, "::ffff:192.0.2.1".parse().unwrap());
        let v4 = claim(Transport::Tcp, 443, "192.0.2.1".parse().unwrap());
        assert!(listener_overlap(&mapped, &v4));
    }

    #[test]
    fn listener_overlap_distinct_namespace() {
        let host = claim(Transport::Tcp, 443, "192.0.2.1".parse().unwrap());
        let project = ListenerClaim {
            namespace: ListenerNamespace::Project {
                network_id: ObjectId::new("net").unwrap(),
            },
            ..claim(Transport::Tcp, 443, "192.0.2.1".parse().unwrap())
        };
        assert!(!listener_overlap(&host, &project));
    }
}
