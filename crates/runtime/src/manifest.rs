//! Independent, fail-closed admission of exact immutable runtime material.
use crate::{Clock, error};
use hton_runtime_contract::*;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;

/// Implementations authenticate persisted evidence and enforce reads/deadlines.
/// Candidate-supplied booleans, digests or caller strings never satisfy this port.
pub trait AdmissionPorts: Send + Sync {
    fn manifest_bytes(
        &self,
        bundle: &BundleId,
        context: &OperationContext,
    ) -> Result<Vec<u8>, RuntimeError>;
    fn artifact_bytes(
        &self,
        bundle: &BundleId,
        artifact: &Artifact,
        limit: usize,
        context: &OperationContext,
    ) -> Result<Vec<u8>, RuntimeError>;
    fn observe(
        &self,
        expected: &ResourceIdentity,
        context: &OperationContext,
    ) -> Result<ResourceIdentity, RuntimeError>;
    fn image_cached(
        &self,
        image: &ImagePin,
        context: &OperationContext,
    ) -> Result<bool, RuntimeError>;
    fn secret_available(
        &self,
        binding: &SecretBinding,
        context: &OperationContext,
    ) -> Result<bool, RuntimeError>;
    fn approval_valid(
        &self,
        approval: &ApprovalEnvelope,
        caller: &CallerId,
        bundle_digest: &Sha256Digest,
        context: &OperationContext,
    ) -> Result<bool, RuntimeError>;
    fn native_evidence_valid(
        &self,
        bundle: &CandidateBundle,
        service: &Service,
        evidence: &Evidence,
        context: &OperationContext,
    ) -> Result<bool, RuntimeError>;
    fn fallback_valid(
        &self,
        fallback: &FallbackCompatibility,
        context: &OperationContext,
    ) -> Result<bool, RuntimeError>;
    fn staging_valid(
        &self,
        bundle: &CandidateBundle,
        attestation: &StagingAttestation,
        context: &OperationContext,
    ) -> Result<bool, RuntimeError>;
    fn claims_valid(
        &self,
        bundle: &CandidateBundle,
        context: &OperationContext,
    ) -> Result<bool, RuntimeError>;
}

/// Cannot be constructed or deserialized outside admission; retained bytes cannot change.
pub struct AdmittedBundle {
    candidate: CandidateBundle,
    artifacts: BTreeMap<ArtifactId, Vec<u8>>,
    resources: BTreeMap<ResourceId, ResourceIdentity>,
    activation_ready: bool,
}
impl AdmittedBundle {
    pub fn activation_ready(&self) -> bool {
        self.activation_ready
    }
    pub fn candidate(&self) -> &CandidateBundle {
        &self.candidate
    }
    pub fn resource(&self, id: &ResourceId) -> Option<&ResourceIdentity> {
        self.resources.get(id)
    }
    pub fn artifact_bytes(&self, id: &ArtifactId) -> Option<&[u8]> {
        self.artifacts.get(id).map(Vec::as_slice)
    }
}

pub use hton_runtime_contract::{candidate_digest, hash};

fn invalid(context: &OperationContext) -> RuntimeError {
    error(ErrorCode::InvalidManifest, context)
}
fn unique<'a, T: Ord + 'a>(items: impl Iterator<Item = &'a T>) -> bool {
    let mut set = BTreeSet::new();
    items.into_iter().all(|v| set.insert(v))
}

pub fn admit(
    bundle_id: &BundleId,
    proof: &AdmissionProof,
    state: &RuntimeState,
    ports: &dyn AdmissionPorts,
    clock: &dyn Clock,
    context: &OperationContext,
) -> Result<AdmittedBundle, RuntimeError> {
    if proof.caller != context.caller
        || proof.expected_run_epoch != state.run_epoch
        || proof.expected_state_revision != state.revision
        || proof.expected_accepted != state.accepted
    {
        return Err(error(ErrorCode::StaleInput, context));
    }
    if state.maintenance.is_some() || state.run_intent == RunIntent::Stopped {
        return Err(error(ErrorCode::Stopped, context));
    }
    if state.lease.as_ref().is_some_and(|l| {
        matches!(
            l.state,
            LeaseState::Armed
                | LeaseState::Activating
                | LeaseState::Verifying
                | LeaseState::Compensating
                | LeaseState::RollingBack
                | LeaseState::RecoveryRequired
        )
    }) {
        return Err(error(ErrorCode::LeaseConflict, context));
    }
    let bytes = ports.manifest_bytes(bundle_id, context)?;
    if hash(&bytes) != proof.protected_artifact_digest {
        return Err(invalid(context));
    }
    let bundle: CandidateBundle = decode(&bytes).map_err(|_| invalid(context))?;
    validate_material(bundle_id, &bundle, state, ports, clock, true, context)?;
    if proof.fallback.fences.credential_revision < state.fences.credential_revision
        || proof
            .fallback
            .certificate_valid_until_ms
            .is_some_and(|v| v <= clock.now_ms())
        || !ports.fallback_valid(&proof.fallback, context)?
    {
        return Err(error(ErrorCode::FallbackIncompatible, context));
    }
    match &proof.fallback.target {
        RollbackTarget::Accepted { bundle_id, .. }
            if state.accepted.as_ref() != Some(bundle_id) =>
        {
            return Err(error(ErrorCode::FallbackIncompatible, context));
        }
        RollbackTarget::PreInstall { .. } if state.accepted.is_some() => {
            return Err(error(ErrorCode::FallbackIncompatible, context));
        }
        _ => (),
    }
    let mut approved = BTreeSet::new();
    for approval in proof.approvals.as_slice() {
        if approval.expires_ms <= clock.now_ms()
            || !ports.approval_valid(approval, &proof.caller, &bundle.manifest_digest, context)?
        {
            return Err(error(ErrorCode::ApprovalRequired, context));
        }
        for resource in approval.resources.as_slice() {
            for action in approval.permitted_actions.as_slice() {
                approved.insert((resource.clone(), *action as u8));
            }
        }
    }
    for action in bundle.actions.as_slice() {
        if !approved.contains(&(action.resource_id.clone(), action.kind as u8)) {
            return Err(error(ErrorCode::ApprovalRequired, context));
        }
    }
    let actual: BTreeMap<_, _> = proof
        .actual_resources
        .as_slice()
        .iter()
        .map(|r| (&r.resource_id, r))
        .collect();
    if actual.len() != proof.actual_resources.as_slice().len() {
        return Err(invalid(context));
    }
    for (_, resource, _) in resource_scopes(&bundle) {
        let observed = ports.observe(resource, context)?;
        if actual.get(&resource.resource_id).copied() != Some(&observed) || &observed != resource {
            return Err(error(ErrorCode::ResourceDrift, context));
        }
    }
    let resources: BTreeMap<_, _> = actual
        .into_iter()
        .map(|(id, r)| (id.clone(), r.clone()))
        .collect();
    for (_, _, owners) in resource_scopes(&bundle) {
        for owner in owners {
            let expected = resources
                .get(owner)
                .ok_or_else(|| error(ErrorCode::ResourceDrift, context))?;
            if ports.observe(expected, context)? != *expected {
                return Err(error(ErrorCode::ResourceDrift, context));
            }
        }
    }
    collect_artifacts(bundle, resources, ports, context)
}

/// Boot authority comes from durable accepted state, not a client-created proof.
/// Ports authenticate persisted native qualification without starting new validators.
pub fn admit_accepted(
    bundle_id: &BundleId,
    state: &RuntimeState,
    ports: &dyn AdmissionPorts,
    clock: &dyn Clock,
    context: &OperationContext,
) -> Result<AdmittedBundle, RuntimeError> {
    if state.maintenance.is_some() || state.run_intent != RunIntent::Running {
        return Err(error(ErrorCode::Stopped, context));
    }
    if state.lease.as_ref().is_some_and(|lease| {
        matches!(
            lease.state,
            LeaseState::Armed
                | LeaseState::Activating
                | LeaseState::Verifying
                | LeaseState::Compensating
                | LeaseState::RollingBack
                | LeaseState::RecoveryRequired
        )
    }) {
        return Err(error(ErrorCode::LeaseConflict, context));
    }
    if state.accepted.as_ref() != Some(bundle_id) {
        return Err(error(ErrorCode::StaleInput, context));
    }
    let bundle: CandidateBundle =
        decode(&ports.manifest_bytes(bundle_id, context)?).map_err(|_| invalid(context))?;
    validate_material(bundle_id, &bundle, state, ports, clock, true, context)?;
    let resources = recovery_resources(&bundle, state, context)?;
    collect_artifacts(bundle, resources, ports, context)
}

fn validate_material(
    id: &BundleId,
    bundle: &CandidateBundle,
    state: &RuntimeState,
    ports: &dyn AdmissionPorts,
    clock: &dyn Clock,
    activation_ready: bool,
    context: &OperationContext,
) -> Result<(), RuntimeError> {
    if &bundle.bundle_id != id
        || bundle.instance_id != state.instance_id
        || candidate_digest(bundle).map_err(|_| invalid(context))? != bundle.manifest_digest
        || bundle.services.as_slice().is_empty()
        || bundle.services.as_slice().len() > 512
        || bundle
            .services
            .as_slice()
            .iter()
            .map(|s| s.restart_owner_resources.as_slice().len())
            .sum::<usize>()
            > 1024
        || !unique(bundle.services.as_slice().iter().map(|s| &s.service_id))
        || !unique(
            bundle
                .services
                .as_slice()
                .iter()
                .map(|s| &s.resource.resource_id),
        )
        || !unique(bundle.artifacts.as_slice().iter().map(|a| &a.artifact_id))
        || !unique(bundle.artifacts.as_slice().iter().map(|a| &a.path))
        || !unique(bundle.actions.as_slice().iter().map(|a| &a.action_id))
    {
        return Err(invalid(context));
    }
    if bundle.services.as_slice().len() + bundle.retired_services.as_slice().len() > 512
        || !unique(resource_scopes(bundle).map(|(service, _, _)| service))
        || !unique(resource_scopes(bundle).map(|(_, resource, _)| &resource.resource_id))
        || bundle
            .retired_services
            .as_slice()
            .iter()
            .any(|service| service.resource.ownership != Ownership::Owned)
    {
        return Err(invalid(context));
    }
    if bundle.fences.credential_revision < state.fences.credential_revision {
        return Err(error(ErrorCode::FenceConflict, context));
    }
    let services: BTreeMap<_, _> = bundle
        .services
        .as_slice()
        .iter()
        .map(|s| (&s.service_id, s))
        .collect();
    let artifacts: BTreeMap<_, _> = bundle
        .artifacts
        .as_slice()
        .iter()
        .map(|a| (&a.artifact_id, a))
        .collect();
    for service in bundle.services.as_slice() {
        if activation_ready && !ports.image_cached(&service.image, context)? {
            return Err(error(ErrorCode::MissingImage, context));
        }
        if service
            .dependencies
            .as_slice()
            .iter()
            .any(|d| !services.contains_key(d))
            || service.artifacts.as_slice().iter().any(|a| {
                !artifacts
                    .get(a)
                    .is_some_and(|artifact| artifact.service_id == service.service_id)
            })
        {
            return Err(invalid(context));
        }
        if activation_ready {
            let evidence = bundle
                .native_evidence
                .as_slice()
                .iter()
                .find(|e| e.service_id == service.service_id && e.kind == ProbeKind::NativeConfig)
                .ok_or_else(|| error(ErrorCode::NativeEvidenceMissing, context))?;
            if evidence.outcome != EvidenceOutcome::Pass
                || evidence.bundle_digest != bundle.manifest_digest
                || evidence.timestamp_ms > clock.now_ms()
                || !ports.native_evidence_valid(bundle, service, evidence, context)?
            {
                return Err(error(ErrorCode::NativeEvidenceMissing, context));
            }
        }
    }
    // Reject dependency cycles before either startup or shutdown ordering.
    let mut resolved = BTreeSet::new();
    loop {
        let before = resolved.len();
        for service in bundle.services.as_slice() {
            if service
                .dependencies
                .as_slice()
                .iter()
                .all(|d| resolved.contains(d))
            {
                resolved.insert(service.service_id.clone());
            }
        }
        if resolved.len() == services.len() {
            break;
        }
        if before == resolved.len() {
            return Err(invalid(context));
        }
    }
    for artifact in bundle.artifacts.as_slice() {
        if !services.contains_key(&artifact.service_id)
            || artifact.size_bytes > MAX_ARTIFACT_BYTES as u64
        {
            return Err(invalid(context));
        }
    }
    for secret in bundle.secrets.as_slice() {
        if !artifacts
            .get(&secret.artifact_id)
            .is_some_and(|a| a.service_id == secret.service_id)
            || (activation_ready && !ports.secret_available(secret, context)?)
        {
            return Err(error(ErrorCode::SecretUnavailable, context));
        }
    }
    let compensation_ids: BTreeSet<_> = bundle
        .actions
        .as_slice()
        .iter()
        .filter_map(|a| a.compensation.as_ref())
        .collect();
    if compensation_ids.len()
        != bundle
            .actions
            .as_slice()
            .iter()
            .filter(|a| a.compensation.is_some())
            .count()
        || compensation_ids.iter().any(|id| {
            bundle
                .actions
                .as_slice()
                .iter()
                .find(|a| &a.action_id == *id)
                .is_none_or(|a| a.compensation.is_some())
        })
    {
        return Err(invalid(context));
    }
    for action in bundle
        .actions
        .as_slice()
        .iter()
        .filter(|a| !compensation_ids.contains(&a.action_id))
    {
        let inverse = action.compensation.as_ref().and_then(|id| {
            bundle
                .actions
                .as_slice()
                .iter()
                .find(|a| &a.action_id == id)
        });
        if inverse.is_none_or(|inverse| {
            inverse.resource_id != action.resource_id
                || inverse.service_id != action.service_id
                || inverse.compensation.is_some()
        }) || action
            .dependencies
            .as_slice()
            .iter()
            .any(|id| compensation_ids.contains(id))
        {
            return Err(invalid(context));
        }

        if action.before.is_none()
            || action.after.is_none()
            || action.before == action.after
            || action.compensation.is_none()
        {
            return Err(invalid(context));
        }
        if action.kind == NativeActionKind::Recreate
            && services
                .get(&action.service_id)
                .is_some_and(|s| s.resource.ownership == Ownership::Associated)
        {
            return Err(invalid(context));
        }
    }
    let action_ids: BTreeSet<_> = bundle
        .actions
        .as_slice()
        .iter()
        .map(|a| &a.action_id)
        .collect();
    for action in bundle.actions.as_slice() {
        let (_, resource, owners) = resource_scopes(bundle)
            .find(|(id, _, _)| *id == &action.service_id)
            .ok_or_else(|| invalid(context))?;
        let retired = bundle
            .retired_services
            .as_slice()
            .iter()
            .any(|s| s.service_id == action.service_id);
        let allowed_retirement = if compensation_ids.contains(&action.action_id) {
            matches!(
                action.kind,
                NativeActionKind::Start | NativeActionKind::EnableAutostart
            )
        } else {
            matches!(
                action.kind,
                NativeActionKind::Stop | NativeActionKind::DisableAutostart
            )
        };
        if action.timeout_ms.get() > 120_000
            || (retired && !allowed_retirement)
            || (action.resource_id != resource.resource_id && !owners.contains(&action.resource_id))
            || action
                .dependencies
                .as_slice()
                .iter()
                .any(|id| !action_ids.contains(id))
            || action
                .compensation
                .as_ref()
                .is_some_and(|id| !action_ids.contains(id))
        {
            return Err(invalid(context));
        }
    }
    let mut resolved = BTreeSet::new();
    loop {
        let before = resolved.len();
        for action in bundle.actions.as_slice() {
            if action
                .dependencies
                .as_slice()
                .iter()
                .all(|id| resolved.contains(id))
            {
                resolved.insert(action.action_id.clone());
            }
        }
        if resolved.len() == action_ids.len() {
            break;
        }
        if before == resolved.len() {
            return Err(invalid(context));
        }
    }
    if activation_ready && !ports.claims_valid(bundle, context)? {
        return Err(invalid(context));
    }
    for claim in bundle.claims.as_slice() {
        if let Claim::Certificate { valid_until_ms, .. } = claim {
            if activation_ready && *valid_until_ms <= clock.now_ms() {
                return Err(invalid(context));
            }
        }
    }
    if let Some(staging) = bundle.staging.as_ref().filter(|_| activation_ready) {
        if staging.bundle_digest != bundle.manifest_digest
            || staging.timestamp_ms > clock.now_ms()
            || !ports.staging_valid(bundle, staging, context)?
        {
            return Err(error(ErrorCode::StagingEvidenceInvalid, context));
        }
    }
    Ok(())
}

fn collect_artifacts(
    bundle: CandidateBundle,
    resources: BTreeMap<ResourceId, ResourceIdentity>,
    ports: &dyn AdmissionPorts,
    context: &OperationContext,
) -> Result<AdmittedBundle, RuntimeError> {
    let mut artifacts = BTreeMap::new();
    let mut remaining = MAX_ARTIFACT_BYTES;
    for artifact in bundle.artifacts.as_slice() {
        let limit = usize::try_from(artifact.size_bytes).map_err(|_| invalid(context))?;
        if limit > remaining {
            return Err(invalid(context));
        }
        let bytes = ports.artifact_bytes(&bundle.bundle_id, artifact, limit, context)?;
        if bytes.len() != limit || hash(&bytes) != artifact.digest {
            return Err(error(ErrorCode::MissingArtifact, context));
        }
        remaining -= bytes.len();
        artifacts.insert(artifact.artifact_id.clone(), bytes);
    }
    Ok(AdmittedBundle {
        candidate: bundle,
        artifacts,
        resources,
        activation_ready: true,
    })
}

/// Recovery authority is the durable lease, not a client-chosen candidate/fallback.
pub(crate) fn admit_trial(
    state: &RuntimeState,
    ports: &dyn AdmissionPorts,
    clock: &dyn Clock,
    context: &OperationContext,
) -> Result<AdmittedBundle, RuntimeError> {
    let lease = state
        .lease
        .as_ref()
        .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
    let bundle: CandidateBundle =
        decode(&ports.manifest_bytes(&lease.candidate, context)?).map_err(|_| invalid(context))?;
    validate_material(
        &lease.candidate,
        &bundle,
        state,
        ports,
        clock,
        true,
        context,
    )?;
    let resources = recovery_resources(&bundle, state, context)?;
    collect_artifacts(bundle, resources, ports, context)
}

pub(crate) fn admit_fallback(
    state: &RuntimeState,
    ports: &dyn AdmissionPorts,
    clock: &dyn Clock,
    context: &OperationContext,
) -> Result<Option<AdmittedBundle>, RuntimeError> {
    let lease = state
        .lease
        .as_ref()
        .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
    if lease.fallback.fences.credential_revision < state.fences.credential_revision
        || lease
            .fallback
            .certificate_valid_until_ms
            .is_some_and(|v| v <= clock.now_ms())
        || !ports.fallback_valid(&lease.fallback, context)?
    {
        return Err(error(ErrorCode::FallbackIncompatible, context));
    }
    let RollbackTarget::Accepted {
        bundle_id,
        manifest_digest,
    } = &lease.fallback.target
    else {
        return Ok(None);
    };
    let bundle: CandidateBundle =
        decode(&ports.manifest_bytes(bundle_id, context)?).map_err(|_| invalid(context))?;
    if &bundle.manifest_digest != manifest_digest {
        return Err(error(ErrorCode::FallbackIncompatible, context));
    }
    validate_material(bundle_id, &bundle, state, ports, clock, true, context)?;
    let resources = recovery_resources(&bundle, state, context)?;
    collect_artifacts(bundle, resources, ports, context).map(Some)
}

fn recovery_resources(
    bundle: &CandidateBundle,
    state: &RuntimeState,
    context: &OperationContext,
) -> Result<BTreeMap<ResourceId, ResourceIdentity>, RuntimeError> {
    let mut resources = BTreeMap::new();
    for receipt in state.resources.as_slice() {
        resources.insert(
            receipt.resource.resource_id.clone(),
            receipt.resource.clone(),
        );
    }
    for (_, resource, owners) in resource_scopes(bundle) {
        let observed = state
            .resources
            .as_slice()
            .iter()
            .find(|r| r.resource.resource_id == (*resource).resource_id);
        let identity = match observed {
            Some(receipt)
                if receipt.resource.kind != (*resource).kind
                    || receipt.resource.ownership != (*resource).ownership =>
            {
                return Err(error(ErrorCode::ResourceDrift, context));
            }
            Some(receipt)
                if receipt.effective_bundle.as_ref() == Some(&bundle.bundle_id)
                    || state.lease.as_ref().is_some_and(|lease| {
                        receipt.effective_bundle.as_ref() == Some(&lease.candidate)
                    }) =>
            {
                receipt.resource.clone()
            }
            Some(receipt) if receipt.resource.identity_digest == (*resource).identity_digest => {
                receipt.resource.clone()
            }
            Some(_) => return Err(error(ErrorCode::ResourceDrift, context)),
            None => (*resource).clone(),
        };
        resources.insert((*resource).resource_id.clone(), identity);
        for owner in owners {
            if !resources.contains_key(owner) {
                return Err(error(ErrorCode::ResourceDrift, context));
            }
        }
    }
    Ok(resources)
}

/// Authenticate cleanup authority without requiring the failed candidate to be
/// runnable: missing candidate images/certs/secrets must not prevent its shutdown.
pub(crate) fn admit_cleanup(
    state: &RuntimeState,
    ports: &dyn AdmissionPorts,
    clock: &dyn Clock,
    context: &OperationContext,
) -> Result<AdmittedBundle, RuntimeError> {
    let lease = state
        .lease
        .as_ref()
        .ok_or_else(|| error(ErrorCode::LeaseConflict, context))?;
    let bundle: CandidateBundle =
        decode(&ports.manifest_bytes(&lease.candidate, context)?).map_err(|_| invalid(context))?;
    validate_material(
        &lease.candidate,
        &bundle,
        state,
        ports,
        clock,
        false,
        context,
    )?;
    let resources = recovery_resources(&bundle, state, context)?;
    Ok(AdmittedBundle {
        candidate: bundle,
        artifacts: BTreeMap::new(),
        resources,
        activation_ready: false,
    })
}

fn resource_scopes(
    bundle: &CandidateBundle,
) -> impl Iterator<Item = (&ServiceId, &ResourceIdentity, &[ResourceId])> {
    bundle
        .services
        .as_slice()
        .iter()
        .map(|s| {
            (
                &s.service_id,
                &s.resource,
                s.restart_owner_resources.as_slice(),
            )
        })
        .chain(bundle.retired_services.as_slice().iter().map(|s| {
            (
                &s.service_id,
                &s.resource,
                s.restart_owner_resources.as_slice(),
            )
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestClock(u64);
    impl Clock for TestClock {
        fn now_ms(&self) -> u64 {
            self.0
        }
    }

    const ARTIFACT_CONTENT: &[u8] = b"abc";

    fn dig(bytes: &[u8]) -> Sha256Digest {
        hash(bytes)
    }
    fn bundle_id(s: &str) -> BundleId {
        BundleId::new(s).unwrap()
    }
    fn instance_id(s: &str) -> InstanceId {
        InstanceId::new(s).unwrap()
    }
    fn service_id(s: &str) -> ServiceId {
        ServiceId::new(s).unwrap()
    }
    fn module_id(s: &str) -> ModuleId {
        ModuleId::new(s).unwrap()
    }
    fn resource_id(s: &str) -> ResourceId {
        ResourceId::new(s).unwrap()
    }
    fn artifact_id(s: &str) -> ArtifactId {
        ArtifactId::new(s).unwrap()
    }
    fn action_id(s: &str) -> ActionId {
        ActionId::new(s).unwrap()
    }
    fn check_id(s: &str) -> CheckId {
        CheckId::new(s).unwrap()
    }
    fn caller_id(s: &str) -> CallerId {
        CallerId::new(s).unwrap()
    }
    fn approval_id(s: &str) -> ApprovalId {
        ApprovalId::new(s).unwrap()
    }
    fn operation_id(s: &str) -> OperationId {
        OperationId::new(s).unwrap()
    }
    fn lease_id(s: &str) -> LeaseId {
        LeaseId::new(s).unwrap()
    }

    fn image() -> ImagePin {
        ImagePin::new(format!("registry/img@sha256:{}", "a".repeat(64))).unwrap()
    }

    fn resource(rid: &str) -> ResourceIdentity {
        ResourceIdentity {
            resource_id: resource_id(rid),
            kind: ResourceKind::Container,
            identity_digest: dig(format!("identity:{rid}").as_bytes()),
            fingerprint: dig(format!("fingerprint:{rid}").as_bytes()),
            ownership: Ownership::Owned,
        }
    }

    /// One service, one artifact and a Start/Stop compensation action pair.
    fn minimal_bundle() -> CandidateBundle {
        let svc = service_id("svc");
        let mut bundle = CandidateBundle {
            bundle_id: bundle_id("bundle1"),
            instance_id: instance_id("inst1"),
            contract_version: ContractVersion::V1,
            manifest_digest: dig(b"placeholder"),
            artifacts: Items::new(vec![Artifact {
                artifact_id: artifact_id("art-svc"),
                path: ArtifactPath::new("bundle/data.json").unwrap(),
                digest: dig(ARTIFACT_CONTENT),
                size_bytes: ARTIFACT_CONTENT.len() as u64,
                service_id: svc.clone(),
            }])
            .unwrap(),
            services: Items::new(vec![Service {
                service_id: svc.clone(),
                module_id: module_id("mod"),
                engine: Engine::Xray,
                image: image(),
                resource: resource("res-svc"),
                dependencies: Items::default(),
                artifacts: Items::new(vec![artifact_id("art-svc")]).unwrap(),
                restart_owner_resources: Items::default(),
            }])
            .unwrap(),
            retired_services: Items::default(),
            claims: Items::default(),
            secrets: Items::default(),
            actions: Items::new(vec![
                FixedAction {
                    action_id: action_id("act-start"),
                    kind: NativeActionKind::Start,
                    resource_id: resource_id("res-svc"),
                    service_id: svc.clone(),
                    dependencies: Items::default(),
                    before: Some(dig(b"before")),
                    after: Some(dig(b"after")),
                    timeout_ms: std::num::NonZeroU32::new(30_000).unwrap(),
                    compensation: Some(action_id("act-stop")),
                },
                FixedAction {
                    action_id: action_id("act-stop"),
                    kind: NativeActionKind::Stop,
                    resource_id: resource_id("res-svc"),
                    service_id: svc.clone(),
                    dependencies: Items::default(),
                    before: None,
                    after: None,
                    timeout_ms: std::num::NonZeroU32::new(30_000).unwrap(),
                    compensation: None,
                },
            ])
            .unwrap(),
            fences: PolicyFences {
                credential_revision: 0,
            },
            native_evidence: Items::new(vec![Evidence {
                check_id: check_id("chk1"),
                service_id: svc,
                kind: ProbeKind::NativeConfig,
                vantage: Vantage::LocalNative,
                outcome: EvidenceOutcome::Pass,
                timestamp_ms: 1,
                bundle_digest: dig(b"placeholder"),
                input_digest: dig(b"in"),
                observed_identity: None,
            }])
            .unwrap(),
            staging: None,
        };
        finalize(&mut bundle);
        bundle
    }

    /// Recompute the canonical manifest digest after a test mutates a bundle.
    fn finalize(bundle: &mut CandidateBundle) {
        let md = candidate_digest(bundle).unwrap();
        bundle.manifest_digest = md.clone();
        let evidence: Vec<Evidence> = bundle
            .native_evidence
            .as_slice()
            .iter()
            .cloned()
            .map(|mut e| {
                e.bundle_digest = md.clone();
                e
            })
            .collect();
        bundle.native_evidence = Items::new(evidence).unwrap();
    }

    fn running_state() -> RuntimeState {
        let mut state = initial_state(instance_id("inst1"));
        state.run_intent = RunIntent::Running;
        state
    }

    fn context() -> OperationContext {
        OperationContext {
            operation_id: operation_id("op1"),
            caller: caller_id("caller1"),
            budget: Budget {
                deadline_ms: 1_000_000,
                max_concurrency: std::num::NonZeroU16::new(4).unwrap(),
                max_bytes: std::num::NonZeroU32::new(1024).unwrap(),
                max_observations: std::num::NonZeroU16::new(16).unwrap(),
            },
            cancellation_id: None,
        }
    }

    fn approval() -> ApprovalEnvelope {
        ApprovalEnvelope {
            approval_id: approval_id("appr1"),
            issuer: caller_id("issuer1"),
            version: ContractVersion::V1,
            scope_digest: dig(b"scope"),
            resources: Items::new(vec![resource_id("res-svc")]).unwrap(),
            permitted_actions: Items::new(vec![NativeActionKind::Start, NativeActionKind::Stop])
                .unwrap(),
            expires_ms: 1_000_000,
        }
    }

    fn preinstall_fallback() -> FallbackCompatibility {
        FallbackCompatibility {
            target: RollbackTarget::PreInstall {
                closed_listeners: Items::default(),
                preserved_resources: Items::default(),
            },
            fences: PolicyFences {
                credential_revision: 0,
            },
            certificate_valid_until_ms: None,
            qualification_digest: dig(b"qualification"),
        }
    }

    fn proof(
        bundle: &CandidateBundle,
        ctx: &OperationContext,
        state: &RuntimeState,
    ) -> AdmissionProof {
        AdmissionProof {
            contract_version: ContractVersion::V1,
            expected_run_epoch: state.run_epoch,
            expected_state_revision: state.revision,
            expected_accepted: state.accepted.clone(),
            actual_resources: Items::new(vec![resource("res-svc")]).unwrap(),
            approvals: Items::new(vec![approval()]).unwrap(),
            fallback: preinstall_fallback(),
            caller: ctx.caller.clone(),
            protected_artifact_digest: hash(&serde_json::to_vec(bundle).unwrap()),
        }
    }

    struct MockPorts {
        manifest: Vec<u8>,
        drift: Option<ResourceIdentity>,
        artifact_corrupt: bool,
        image_cached: bool,
        secret_available: bool,
        approval_valid: bool,
        native_evidence_valid: bool,
        fallback_valid: bool,
        staging_valid: bool,
        claims_valid: bool,
    }

    impl MockPorts {
        fn for_bundle(bundle: &CandidateBundle) -> Self {
            MockPorts {
                manifest: serde_json::to_vec(bundle).unwrap(),
                drift: None,
                artifact_corrupt: false,
                image_cached: true,
                secret_available: true,
                approval_valid: true,
                native_evidence_valid: true,
                fallback_valid: true,
                staging_valid: true,
                claims_valid: true,
            }
        }
    }

    impl AdmissionPorts for MockPorts {
        fn manifest_bytes(
            &self,
            _bundle: &BundleId,
            _context: &OperationContext,
        ) -> Result<Vec<u8>, RuntimeError> {
            Ok(self.manifest.clone())
        }
        fn artifact_bytes(
            &self,
            _bundle: &BundleId,
            _artifact: &Artifact,
            limit: usize,
            _context: &OperationContext,
        ) -> Result<Vec<u8>, RuntimeError> {
            let bytes: &[u8] = if self.artifact_corrupt {
                b"abd"
            } else {
                ARTIFACT_CONTENT
            };
            assert_eq!(bytes.len(), limit);
            Ok(bytes.to_vec())
        }
        fn observe(
            &self,
            expected: &ResourceIdentity,
            _context: &OperationContext,
        ) -> Result<ResourceIdentity, RuntimeError> {
            Ok(self.drift.clone().unwrap_or_else(|| expected.clone()))
        }
        fn image_cached(
            &self,
            _image: &ImagePin,
            _context: &OperationContext,
        ) -> Result<bool, RuntimeError> {
            Ok(self.image_cached)
        }
        fn secret_available(
            &self,
            _binding: &SecretBinding,
            _context: &OperationContext,
        ) -> Result<bool, RuntimeError> {
            Ok(self.secret_available)
        }
        fn approval_valid(
            &self,
            _approval: &ApprovalEnvelope,
            _caller: &CallerId,
            _bundle_digest: &Sha256Digest,
            _context: &OperationContext,
        ) -> Result<bool, RuntimeError> {
            Ok(self.approval_valid)
        }
        fn native_evidence_valid(
            &self,
            _bundle: &CandidateBundle,
            _service: &Service,
            _evidence: &Evidence,
            _context: &OperationContext,
        ) -> Result<bool, RuntimeError> {
            Ok(self.native_evidence_valid)
        }
        fn fallback_valid(
            &self,
            _fallback: &FallbackCompatibility,
            _context: &OperationContext,
        ) -> Result<bool, RuntimeError> {
            Ok(self.fallback_valid)
        }
        fn staging_valid(
            &self,
            _bundle: &CandidateBundle,
            _attestation: &StagingAttestation,
            _context: &OperationContext,
        ) -> Result<bool, RuntimeError> {
            Ok(self.staging_valid)
        }
        fn claims_valid(
            &self,
            _bundle: &CandidateBundle,
            _context: &OperationContext,
        ) -> Result<bool, RuntimeError> {
            Ok(self.claims_valid)
        }
    }

    fn admit_err(
        bundle: &CandidateBundle,
        state: &RuntimeState,
        ports: &MockPorts,
        ctx: &OperationContext,
    ) -> RuntimeError {
        admit(
            &bundle.bundle_id,
            &proof(bundle, ctx, state),
            state,
            ports,
            &TestClock(100),
            ctx,
        )
        .err()
        .expect("admission should fail")
    }

    #[test]
    fn unique_detects_duplicate_items() {
        let a = service_id("svc");
        let b = service_id("other");
        assert!(unique([&a, &b].into_iter()));
        assert!(!unique([&a, &a].into_iter()));
        assert!(unique(std::iter::empty::<&ServiceId>()));
    }

    #[test]
    fn admit_rejects_stale_caller() {
        let bundle = minimal_bundle();
        let state = running_state();
        let ctx = context();
        let mut p = proof(&bundle, &ctx, &state);
        p.caller = caller_id("someone-else");
        let err = admit(
            &bundle.bundle_id,
            &p,
            &state,
            &MockPorts::for_bundle(&bundle),
            &TestClock(100),
            &ctx,
        )
        .err()
        .expect("admission should fail");
        assert_eq!(err.code, ErrorCode::StaleInput);
    }

    #[test]
    fn admit_rejects_stopped_intent() {
        let bundle = minimal_bundle();
        let mut state = running_state();
        state.run_intent = RunIntent::Stopped;
        let ctx = context();
        assert_eq!(
            admit_err(&bundle, &state, &MockPorts::for_bundle(&bundle), &ctx).code,
            ErrorCode::Stopped
        );
    }

    #[test]
    fn admit_rejects_maintenance_reservation() {
        let bundle = minimal_bundle();
        let mut state = running_state();
        state.maintenance = Some(MaintenanceReservation {
            reservation_id: lease_id("resv1"),
            operation_id: operation_id("op1"),
            run_epoch: 0,
        });
        let ctx = context();
        assert_eq!(
            admit_err(&bundle, &state, &MockPorts::for_bundle(&bundle), &ctx).code,
            ErrorCode::Stopped
        );
    }

    #[test]
    fn admit_rejects_active_lease() {
        let bundle = minimal_bundle();
        let mut state = running_state();
        state.lease = Some(Lease {
            started_from_stopped: false,
            lease_id: lease_id("lease1"),
            candidate: bundle.bundle_id.clone(),
            fallback: preinstall_fallback(),
            run_epoch: 0,
            deadline_ms: 1_000_000,
            state: LeaseState::Armed,
        });
        let ctx = context();
        assert_eq!(
            admit_err(&bundle, &state, &MockPorts::for_bundle(&bundle), &ctx).code,
            ErrorCode::LeaseConflict
        );
    }

    #[test]
    fn admit_rejects_manifest_digest_mismatch() {
        let bundle = minimal_bundle();
        let state = running_state();
        let ctx = context();
        let mut p = proof(&bundle, &ctx, &state);
        p.protected_artifact_digest = dig(b"wrong");
        let err = admit(
            &bundle.bundle_id,
            &p,
            &state,
            &MockPorts::for_bundle(&bundle),
            &TestClock(100),
            &ctx,
        )
        .err()
        .expect("admission should fail");
        assert_eq!(err.code, ErrorCode::InvalidManifest);
    }

    #[test]
    fn admit_rejects_undecodable_manifest() {
        let bundle = minimal_bundle();
        let state = running_state();
        let ctx = context();
        let mut ports = MockPorts::for_bundle(&bundle);
        ports.manifest = b"not-json".to_vec();
        let mut p = proof(&bundle, &ctx, &state);
        p.protected_artifact_digest = hash(b"not-json");
        let err = admit(&bundle.bundle_id, &p, &state, &ports, &TestClock(100), &ctx)
            .err()
            .expect("admission should fail");
        assert_eq!(err.code, ErrorCode::InvalidManifest);
    }

    #[test]
    fn validate_rejects_empty_services() {
        let mut bundle = minimal_bundle();
        bundle.services = Items::default();
        finalize(&mut bundle);
        let state = running_state();
        let ctx = context();
        assert_eq!(
            admit_err(&bundle, &state, &MockPorts::for_bundle(&bundle), &ctx).code,
            ErrorCode::InvalidManifest
        );
    }

    #[test]
    fn validate_rejects_duplicate_service_ids() {
        let mut bundle = minimal_bundle();
        let first = bundle.services.as_slice()[0].clone();
        let mut second = first.clone();
        second.resource = resource("res-svc2");
        bundle.services = Items::new(vec![first, second]).unwrap();
        finalize(&mut bundle);
        let state = running_state();
        let ctx = context();
        assert_eq!(
            admit_err(&bundle, &state, &MockPorts::for_bundle(&bundle), &ctx).code,
            ErrorCode::InvalidManifest
        );
    }

    #[test]
    fn validate_rejects_artifact_service_mismatch() {
        let mut bundle = minimal_bundle();
        let mut artifacts = bundle.artifacts.clone().into_vec();
        artifacts[0].service_id = service_id("other");
        bundle.artifacts = Items::new(artifacts).unwrap();
        finalize(&mut bundle);
        let state = running_state();
        let ctx = context();
        assert_eq!(
            admit_err(&bundle, &state, &MockPorts::for_bundle(&bundle), &ctx).code,
            ErrorCode::InvalidManifest
        );
    }

    #[test]
    fn validate_rejects_oversized_artifact() {
        let mut bundle = minimal_bundle();
        let mut artifacts = bundle.artifacts.clone().into_vec();
        artifacts[0].size_bytes = (MAX_ARTIFACT_BYTES as u64) + 1;
        bundle.artifacts = Items::new(artifacts).unwrap();
        finalize(&mut bundle);
        let state = running_state();
        let ctx = context();
        assert_eq!(
            admit_err(&bundle, &state, &MockPorts::for_bundle(&bundle), &ctx).code,
            ErrorCode::InvalidManifest
        );
    }

    #[test]
    fn validate_rejects_fence_regression() {
        let bundle = minimal_bundle();
        let mut state = running_state();
        state.fences.credential_revision = 1;
        let ctx = context();
        assert_eq!(
            admit_err(&bundle, &state, &MockPorts::for_bundle(&bundle), &ctx).code,
            ErrorCode::FenceConflict
        );
    }

    #[test]
    fn validate_rejects_missing_native_evidence() {
        let mut bundle = minimal_bundle();
        bundle.native_evidence = Items::default();
        finalize(&mut bundle);
        let state = running_state();
        let ctx = context();
        assert_eq!(
            admit_err(&bundle, &state, &MockPorts::for_bundle(&bundle), &ctx).code,
            ErrorCode::NativeEvidenceMissing
        );
    }

    #[test]
    fn validate_rejects_equal_before_and_after() {
        let mut bundle = minimal_bundle();
        let mut actions = bundle.actions.clone().into_vec();
        let before = actions[0].before.clone();
        actions[0].after = before;
        bundle.actions = Items::new(actions).unwrap();
        finalize(&mut bundle);
        let state = running_state();
        let ctx = context();
        assert_eq!(
            admit_err(&bundle, &state, &MockPorts::for_bundle(&bundle), &ctx).code,
            ErrorCode::InvalidManifest
        );
    }

    #[test]
    fn admit_accepted_rejects_stopped() {
        let bundle = minimal_bundle();
        let mut state = running_state();
        state.run_intent = RunIntent::Stopped;
        state.accepted = Some(bundle.bundle_id.clone());
        let ctx = context();
        let err = admit_accepted(
            &bundle.bundle_id,
            &state,
            &MockPorts::for_bundle(&bundle),
            &TestClock(100),
            &ctx,
        )
        .err()
        .expect("admission should fail");
        assert_eq!(err.code, ErrorCode::Stopped);
    }

    #[test]
    fn admit_accepted_rejects_mismatched_bundle() {
        let bundle = minimal_bundle();
        let mut state = running_state();
        state.accepted = Some(bundle_id("other-bundle"));
        let ctx = context();
        let err = admit_accepted(
            &bundle.bundle_id,
            &state,
            &MockPorts::for_bundle(&bundle),
            &TestClock(100),
            &ctx,
        )
        .err()
        .expect("admission should fail");
        assert_eq!(err.code, ErrorCode::StaleInput);
    }

    #[test]
    fn admit_rejects_resource_drift() {
        let bundle = minimal_bundle();
        let state = running_state();
        let ctx = context();
        let mut ports = MockPorts::for_bundle(&bundle);
        let mut drifted = resource("res-svc");
        drifted.fingerprint = dig(b"drifted");
        ports.drift = Some(drifted);
        assert_eq!(
            admit_err(&bundle, &state, &ports, &ctx).code,
            ErrorCode::ResourceDrift
        );
    }

    #[test]
    fn admit_rejects_corrupted_artifact_bytes() {
        let bundle = minimal_bundle();
        let state = running_state();
        let ctx = context();
        let mut ports = MockPorts::for_bundle(&bundle);
        ports.artifact_corrupt = true;
        assert_eq!(
            admit_err(&bundle, &state, &ports, &ctx).code,
            ErrorCode::MissingArtifact
        );
    }

    #[test]
    fn admit_returns_admitted_bundle() {
        let bundle = minimal_bundle();
        let state = running_state();
        let ctx = context();
        let admitted = admit(
            &bundle.bundle_id,
            &proof(&bundle, &ctx, &state),
            &state,
            &MockPorts::for_bundle(&bundle),
            &TestClock(100),
            &ctx,
        )
        .unwrap();
        assert!(admitted.activation_ready());
        assert_eq!(admitted.candidate().bundle_id, bundle_id("bundle1"));
        assert_eq!(
            admitted.resource(&resource_id("res-svc")),
            Some(&resource("res-svc"))
        );
        assert_eq!(
            admitted.artifact_bytes(&artifact_id("art-svc")),
            Some(ARTIFACT_CONTENT)
        );
    }
}
