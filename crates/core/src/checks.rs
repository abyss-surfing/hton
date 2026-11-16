//! Bounded observation aggregation; semantic configuration checks remain in config.
use crate::certificates::CertificateName;
use crate::journal::{OperationControl, failure};
use hton_contracts::{config::*, operations::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckSection {
    Host,
    Dns,
    Resources,
    Certificates,
    ExternalReachability,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationTarget {
    Host,
    Domain(CertificateName),
    Resource(ResourceIdentity),
    Certificate(ObjectId),
}
#[derive(Clone, Debug)]
pub struct ObservationQuery {
    pub check_id: CheckId,
    pub module_id: Option<ModuleId>,
    pub section: CheckSection,
    pub target: ObservationTarget,
    pub vantage: Vantage,
    pub input_fingerprint: Sha256Digest,
    pub required: bool,
}
#[derive(Clone, Debug)]
pub struct ObservedSample {
    pub evidence: Evidence,
    pub resource: Option<ResourceObservation>,
    pub bytes_used: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
    Unavailable,
}
#[derive(Clone, Debug)]
pub struct ObservationSample {
    pub evidence: Evidence,
    pub freshness: Freshness,
    pub age_ms: Option<u64>,
    pub error_code: Option<ErrorCode>,
}
#[derive(Clone, Debug)]
pub struct ObservationReport {
    pub checks: CheckReport,
    pub runtime: RuntimeObservation,
    pub samples: Vec<ObservationSample>,
}
/// Driver resolves only configured logical targets, bounds subprocess/network/output,
/// and returns measured evidence. Timeout does not prove censorship or exposure.
pub trait ObservationPort: Send + Sync {
    fn observe(
        &self,
        query: &ObservationQuery,
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ObservedSample>;
}
pub struct CheckCoordinator<'a> {
    pub port: &'a dyn ObservationPort,
    pub control: &'a dyn OperationControl,
    pub max_age_ms: u64,
}
impl CheckCoordinator<'_> {
    pub fn observe(
        &self,
        queries: &[ObservationQuery],
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ObservationReport> {
        if budget.max_concurrency == 0
            || budget.max_concurrency > 8
            || budget.max_bytes == 0
            || budget.max_bytes > 4 * 1024 * 1024
            || budget.max_observations == 0
            || budget.max_observations > 4096
            || queries.len() > budget.max_observations as usize
            || self.max_age_ms == 0
            || self.max_age_ms > 300_000
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "observation budget is outside trusted bounds",
            ));
        }
        let mut unique = BTreeSet::new();
        if !queries.iter().all(|q| unique.insert(q.check_id.clone())) {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "duplicate observation identity",
            ));
        }
        let mut bounded = budget.clone();
        bounded.deadline = UnixMillis(bounded.deadline.0.min(ctx.deadline.0));
        let mut samples = vec![];
        let mut evidence = vec![];
        let mut issues = vec![];
        let mut resources = BTreeMap::new();
        let mut remaining = bounded.max_bytes;
        let mut blocking = false;
        for query in queries {
            let now = self.control.now_ms();
            let attempt = if self.control.is_cancelled(&ctx.cancellation) {
                Err(failure(
                    ctx,
                    ErrorCode::OperationCancelled,
                    "observation cancelled",
                ))
            } else if now >= bounded.deadline.0 || remaining == 0 {
                Err(failure(
                    ctx,
                    ErrorCode::OperationDeadline,
                    "observation budget exhausted",
                ))
            } else {
                let mut individual = bounded.clone();
                individual.max_bytes = remaining;
                individual.max_observations = 1;
                self.port.observe(query, &individual, ctx)
            };
            let valid = attempt.as_ref().is_ok_and(|sample| {
                sample.bytes_used <= remaining
                    && sample.evidence.check_id == query.check_id
                    && sample.evidence.module_id == query.module_id
                    && sample.evidence.vantage == query.vantage
                    && sample.evidence.input_fingerprint == query.input_fingerprint
                    && sample.evidence.timestamp.0 <= self.control.now_ms()
                    && self.control.now_ms() < bounded.deadline.0
                    && !self.control.is_cancelled(&ctx.cancellation)
                    && (!matches!(query.target, ObservationTarget::Resource(_))
                        || sample.resource.is_some())
            });
            let error_code = if valid {
                None
            } else {
                Some(
                    attempt
                        .as_ref()
                        .err()
                        .map(|e| e.code)
                        .unwrap_or(ErrorCode::ResourceDrift),
                )
            };
            let (value, freshness, age) = if valid {
                let sample = attempt?;
                remaining -= sample.bytes_used;
                let age = self
                    .control
                    .now_ms()
                    .saturating_sub(sample.evidence.timestamp.0);
                let mut fresh = age <= self.max_age_ms;
                let mut resource_state = None;
                if fresh {
                    if let Some(resource) = sample.resource {
                        let matched = matches!(&query.target,ObservationTarget::Resource(expected) if expected==&resource.identity);
                        if matched
                            && resource.observed_at.0 <= self.control.now_ms()
                            && resource.valid_until.0 > self.control.now_ms()
                        {
                            fresh &= self.control.now_ms().saturating_sub(resource.observed_at.0)
                                <= self.max_age_ms;
                            resource_state = Some(resource.state);
                            if fresh {
                                resources.insert(resource.identity.resource_id.clone(), resource);
                            }
                        } else {
                            fresh = false;
                            blocking |= query.required;
                            issues.push(issue(
                                query,
                                "resource observation identity or freshness is invalid",
                            ));
                        }
                    }
                }
                let mut e = sample.evidence;
                if !fresh || resource_state == Some(ObservedState::Unknown) {
                    e.outcome = EvidenceOutcome::Unknown;
                } else if resource_state == Some(ObservedState::Failed) {
                    e.outcome = EvidenceOutcome::Fail;
                }
                (
                    e,
                    if fresh {
                        Freshness::Fresh
                    } else {
                        Freshness::Stale
                    },
                    Some(age),
                )
            } else {
                (
                    Evidence {
                        check_id: query.check_id.clone(),
                        module_id: query.module_id.clone(),
                        outcome: EvidenceOutcome::Unknown,
                        timestamp: UnixMillis(self.control.now_ms()),
                        vantage: query.vantage,
                        input_fingerprint: query.input_fingerprint.clone(),
                        details: SafeFields::default(),
                    },
                    Freshness::Unavailable,
                    None,
                )
            };
            if freshness != Freshness::Fresh || value.outcome != EvidenceOutcome::Pass {
                blocking |= query.required;
                issues.push(issue(
                    query,
                    "observation failed or is unavailable; external blocking is not established",
                ));
            }
            evidence.push(value.clone());
            samples.push(ObservationSample {
                evidence: value,
                freshness,
                age_ms: age,
                error_code,
            });
        }
        Ok(ObservationReport {
            checks: CheckReport {
                issues,
                evidence,
                blocking,
                normalized_snapshot: None,
            },
            runtime: RuntimeObservation { resources },
            samples,
        })
    }
    pub fn identity_evidence(
        &self,
        configured_domains: &BTreeSet<CertificateName>,
        budget: &Budget,
        ctx: &OperationContext,
    ) -> Result<ObservationReport> {
        let mut queries = Vec::new();
        let domains: BTreeSet<_> = configured_domains
            .iter()
            .map(CertificateName::host)
            .collect();
        for host in domains {
            let domain = CertificateName::new(host).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "configured identity domain is invalid",
                )
            })?;
            let fingerprint = crate::config::digest(domain.host().as_bytes());
            let check_id = CheckId::new(format!("identity.dns.{}", &fingerprint.as_str()[7..39]))
                .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "identity check identifier invalid",
                )
            })?;
            queries.push(ObservationQuery {
                check_id,
                module_id: None,
                section: CheckSection::Dns,
                target: ObservationTarget::Domain(domain.clone()),
                vantage: Vantage::ExternalProbe,
                input_fingerprint: fingerprint,
                required: false,
            });
        }
        let fp = crate::config::digest(b"hton.configured-host.identity.v1");
        queries.push(ObservationQuery {
            check_id: CheckId::new("identity.host").expect("fixed identifier"),
            module_id: None,
            section: CheckSection::Host,
            target: ObservationTarget::Host,
            vantage: Vantage::LocalRuntime,
            input_fingerprint: fp,
            required: false,
        });
        self.observe(&queries, budget, ctx)
    }
}
fn issue(query: &ObservationQuery, message: &'static str) -> CheckIssue {
    CheckIssue {
        code: ErrorCode::ResourceDrift,
        severity: if query.required {
            Severity::Error
        } else {
            Severity::Warning
        },
        module_id: query.module_id.clone(),
        object_id: None,
        path: None,
        message: message.into(),
        repair_hint: None,
    }
}
