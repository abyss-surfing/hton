//! Semantic runtime verification. Missing or unknown evidence remains blocking.
use crate::{
    Cancellation, Clock, check_budget, error,
    manifest::{AdmittedBundle, hash},
};
use hton_runtime_contract::*;
use std::collections::BTreeSet;

/// A qualified native client/probe implementation must check actual protocol,
/// route/auth and observed backend identity under its bounded context.
pub trait ProbePort: Send + Sync {
    fn probe(
        &self,
        bundle: &AdmittedBundle,
        service: &Service,
        kind: ProbeKind,
        context: &OperationContext,
    ) -> Result<Evidence, RuntimeError>;
}

pub fn required(service: &Service, bundle: &CandidateBundle) -> Vec<ProbeKind> {
    let mut kinds = vec![ProbeKind::ResourceIdentity, ProbeKind::RouteIdentity];
    if matches!(service.engine, Engine::Xray | Engine::Amnezia) {
        kinds.extend([
            ProbeKind::AuthenticationAllowed,
            ProbeKind::AuthenticationDenied,
        ]);
    }
    if bundle.claims.as_slice().iter().any(|claim| matches!(claim, Claim::Certificate { service: id, .. } if id == &service.service_id)) {
        kinds.push(ProbeKind::CertificateCoverage);
    }
    kinds
}

pub fn verify(
    bundle: &AdmittedBundle,
    port: &dyn ProbePort,
    clock: &dyn Clock,
    cancellation: &dyn Cancellation,
    context: &OperationContext,
) -> Result<ProbeReport, RuntimeError> {
    verify_services(bundle, None, port, clock, cancellation, context)
}

/// Verify only the exact selected services after a scoped action. Unknown evidence
/// remains blocking, while unrelated accepted members do not change this result.
pub fn verify_selected(
    bundle: &AdmittedBundle,
    selected: &[ServiceId],
    port: &dyn ProbePort,
    clock: &dyn Clock,
    cancellation: &dyn Cancellation,
    context: &OperationContext,
) -> Result<ProbeReport, RuntimeError> {
    if selected.is_empty() {
        return Err(error(ErrorCode::InvalidManifest, context));
    }
    verify_services(bundle, Some(selected), port, clock, cancellation, context)
}

fn verify_services(
    bundle: &AdmittedBundle,
    selected: Option<&[ServiceId]>,
    port: &dyn ProbePort,
    clock: &dyn Clock,
    cancellation: &dyn Cancellation,
    context: &OperationContext,
) -> Result<ProbeReport, RuntimeError> {
    check_budget(context, clock, cancellation)?;
    let candidate = bundle.candidate();
    let selected_set: Option<BTreeSet<_>> = selected.map(|ids| ids.iter().cloned().collect());
    if selected.is_some_and(|ids| {
        selected_set
            .as_ref()
            .is_none_or(|set| set.len() != ids.len())
            || ids.iter().any(|id| {
                !candidate
                    .services
                    .as_slice()
                    .iter()
                    .any(|service| &service.service_id == id)
            })
    }) {
        return Err(error(ErrorCode::InvalidManifest, context));
    }
    let count: usize = candidate
        .services
        .as_slice()
        .iter()
        .filter(|service| {
            selected_set
                .as_ref()
                .is_none_or(|ids| ids.contains(&service.service_id))
        })
        .map(|s| required(s, candidate).len())
        .sum();
    if count > usize::from(context.budget.max_observations.get()) || count > MAX_ITEMS {
        return Err(error(ErrorCode::InvalidManifest, context));
    }
    let mut evidence = Vec::new();
    let mut blocking = false;
    for service in candidate.services.as_slice().iter().filter(|service| {
        selected_set
            .as_ref()
            .is_none_or(|ids| ids.contains(&service.service_id))
    }) {
        for kind in required(service, candidate) {
            check_budget(context, clock, cancellation)?;
            let expected_input = hash(
                &serde_json::to_vec(service)
                    .map_err(|_| error(ErrorCode::InvalidManifest, context))?,
            );
            let observed = port.probe(bundle, service, kind, context);
            let valid = observed.as_ref().is_ok_and(|e| {
                e.service_id == service.service_id
                    && e.kind == kind
                    && e.bundle_digest == candidate.manifest_digest
                    && e.input_digest == expected_input
                    && e.timestamp_ms <= clock.now_ms()
                    && e.timestamp_ms >= clock.now_ms().saturating_sub(60_000)
                    && e.observed_identity.as_ref()
                        == bundle
                            .resource(&service.resource.resource_id)
                            .map(|resource| &resource.identity_digest)
            });
            if valid {
                let e = observed.map_err(|_| error(ErrorCode::RecoveryRequired, context))?;
                blocking |= e.outcome != EvidenceOutcome::Pass;
                evidence.push(e);
            } else {
                blocking = true;
                evidence.push(Evidence {
                    check_id: CheckId::new(format!(
                        "probe-{}-{}",
                        &hash(service.service_id.as_str().as_bytes()).as_str()[7..23],
                        kind as u8
                    ))
                    .map_err(|_| error(ErrorCode::InvalidManifest, context))?,
                    service_id: service.service_id.clone(),
                    kind,
                    vantage: Vantage::LocalRuntime,
                    outcome: EvidenceOutcome::Unknown,
                    timestamp_ms: clock.now_ms(),
                    bundle_digest: candidate.manifest_digest.clone(),
                    input_digest: expected_input,
                    observed_identity: None,
                });
            }
        }
    }
    Ok(ProbeReport {
        bundle_id: candidate.bundle_id.clone(),
        manifest_digest: candidate.manifest_digest.clone(),
        evidence: Items::new(evidence).map_err(|_| error(ErrorCode::InvalidManifest, context))?,
        blocking,
    })
}
