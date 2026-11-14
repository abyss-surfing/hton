//! Pure exact-input effect planning. Preview never persists a plan ID or changes resources.
use crate::{
    catalog::ModuleCatalogue,
    config::{content_revision, digest},
};
use hton_contracts::{config::*, operations::*};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Supplied by trusted state/admission readers; never deserialize this from a frontend.
pub struct PlanInputs<'a> {
    pub catalogue: &'a ModuleCatalogue,
    pub operation_id: OperationId,
    pub actor: &'a ActorContext,
    pub now: UnixMillis,
    pub expires_at: UnixMillis,
    pub run_intent: RunIntent,
    pub run_epoch: RunEpoch,
    pub runtime_revision: StateRevision,
    pub accepted_bundle: Option<BundleId>,
    pub prior_artifacts: BTreeMap<ServiceId, Sha256Digest>,
    pub target_artifacts: BTreeMap<ServiceId, Sha256Digest>,
    pub images: BTreeMap<ServiceId, ImageDigest>,
    /// Locally verified signatures/protected provenance, not client asserted approval IDs.
    pub verified_approval_ids: BTreeSet<ApprovalId>,
    pub rollback_target: RollbackTarget,
    pub native_transitions: BTreeMap<StepId, NativeTransitionBinding>,
    /// Trusted native desired-launch comparison, never inferred after plan approval.
    pub recreate_services: BTreeSet<ServiceId>,
    /// Explicit whole-server up may prepare native starts while actual intent stays stopped.
    pub start_requested: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTransitionBinding {
    pub before: Sha256Digest,
    pub after: Sha256Digest,
    pub compensation: Action,
}
fn failure(inputs: &PlanInputs<'_>, code: ErrorCode, message: &'static str) -> Error {
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
fn object(value: &str) -> ObjectId {
    ObjectId::new(value).expect("trusted object ID")
}
fn step(prefix: &str, service: &ServiceId, resource: &ResourceId) -> StepId {
    // Exact identities are bound separately; action IDs stay bounded even with long input IDs.
    let hash = digest(format!("{prefix}\0{service}\0{resource}").as_bytes());
    StepId::new(format!("{prefix}.{}", &hash.as_str()[7..39])).expect("canonical step ID")
}
pub fn plan(
    snapshot: &NormalizedSnapshot,
    graph: &DependencyGraph,
    observations: &RuntimeObservation,
    registry: &RegistrySnapshot,
    read_set: &ReadSet,
    approvals: &[ApprovalEnvelope],
    inputs: &PlanInputs<'_>,
) -> Result<EffectPlan> {
    match calculate(
        snapshot,
        graph,
        observations,
        registry,
        read_set,
        approvals,
        inputs,
        false,
    )? {
        Calculation::Final(plan) => Ok(plan),
        Calculation::Requirements(_) => unreachable!("final calculation mode"),
    }
}

/// Internal read-only requirements for trusted native transition observation; never a stageable plan.
pub fn prepare_actions(
    snapshot: &NormalizedSnapshot,
    graph: &DependencyGraph,
    observations: &RuntimeObservation,
    registry: &RegistrySnapshot,
    read_set: &ReadSet,
    approvals: &[ApprovalEnvelope],
    inputs: &PlanInputs<'_>,
) -> Result<Vec<Action>> {
    match calculate(
        snapshot,
        graph,
        observations,
        registry,
        read_set,
        approvals,
        inputs,
        true,
    )? {
        Calculation::Requirements(actions) => Ok(actions),
        Calculation::Final(_) => unreachable!("requirements calculation mode"),
    }
}

enum Calculation {
    Requirements(Vec<Action>),
    Final(EffectPlan),
}

fn calculate(
    snapshot: &NormalizedSnapshot,
    graph: &DependencyGraph,
    observations: &RuntimeObservation,
    registry: &RegistrySnapshot,
    read_set: &ReadSet,
    approvals: &[ApprovalEnvelope],
    inputs: &PlanInputs<'_>,
    requirements_only: bool,
) -> Result<Calculation> {
    if read_set.coherence != ReadCoherence::Coherent || snapshot.desired.read_set != *read_set {
        return Err(failure(
            inputs,
            ErrorCode::ConfigReadConflict,
            "Plan inputs do not share one coherent configuration read set",
        ));
    }
    if !graph.conflicts.is_empty()
        || graph
            .enabled_modules
            .iter()
            .any(|id| id.as_str() == "core" && snapshot.desired.features.contains_key(id))
    {
        return Err(failure(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Dependency graph has unresolved conflicts or an illegal core toggle",
        ));
    }
    if inputs.expires_at.0 <= inputs.now.0
        || inputs.expires_at.0.saturating_sub(inputs.now.0) > 900_000
    {
        return Err(failure(
            inputs,
            ErrorCode::PlanExpired,
            "Plan lifetime must be positive and no more than fifteen minutes",
        ));
    }
    for module in &graph.enabled_modules {
        if module.as_str() == "core" {
            continue;
        }
        let Some(entry) = registry.entries.get(module) else {
            return Err(failure(
                inputs,
                ErrorCode::ModuleIncomplete,
                "Enabled module is not installed",
            ));
        };
        if entry.membership != MembershipState::Installed
            || entry.setup_state != SetupState::Complete
        {
            return Err(failure(
                inputs,
                ErrorCode::ModuleIncomplete,
                "Enabled module setup or cleanup is incomplete",
            ));
        }
    }
    let mut approved = Vec::new();
    let mut seen_approvals = BTreeSet::new();
    for approval in approvals {
        if !seen_approvals.insert(approval.approval_id.clone())
            || !inputs.verified_approval_ids.contains(&approval.approval_id)
            || !inputs
                .actor
                .local_approval_refs
                .contains(&approval.approval_id)
            || approval
                .expires_at
                .is_some_and(|expiry| expiry.0 <= inputs.now.0)
        {
            return Err(failure(
                inputs,
                ErrorCode::ApprovalRequired,
                "Approval provenance, scope or expiry is invalid",
            ));
        }
        approved.push(approval.clone());
    }
    approved.sort_by(|a, b| a.approval_id.cmp(&b.approval_id));
    for claim in graph.listener_claims.iter().filter(|claim| claim.public) {
        if !approved
            .iter()
            .any(|approval| approval.listener_scopes.contains(claim))
        {
            return Err(failure(
                inputs,
                ErrorCode::ApprovalRequired,
                "Public listener is outside the exact locally approved scope",
            ));
        }
    }
    for id in &graph.enabled_modules {
        if let Some(entry) = inputs.catalogue.describe(id) {
            for privilege in &entry.descriptor.privileges {
                if !approved
                    .iter()
                    .any(|approval| approval.privileges.contains(privilege))
                {
                    return Err(failure(
                        inputs,
                        ErrorCode::ApprovalRequired,
                        "Module privilege is outside the locally approved scope",
                    ));
                }
            }
        }
    }
    let mut resources = BTreeMap::<ResourceId, (&ResourceIdentity, Ownership)>::new();
    for entry in registry
        .entries
        .values()
        .filter(|entry| entry.membership != MembershipState::RemovedRetained)
    {
        for resource in &entry.resources {
            if let Some((previous, ownership)) =
                resources.insert(resource.resource_id.clone(), (resource, entry.ownership))
            {
                if previous != resource || ownership != entry.ownership {
                    return Err(failure(
                        inputs,
                        ErrorCode::ResourceDrift,
                        "Registry resources have inconsistent exact identities or ownership",
                    ));
                }
            }
        }
    }
    if resources.values().any(|(resource, ownership)| {
        *ownership == Ownership::Owned
            && !inputs
                .catalogue
                .services()
                .contains_key(&resource.service_id)
    }) {
        return Err(failure(
            inputs,
            ErrorCode::RecoveryRequired,
            "Registry contains an owned service without a trusted lifecycle descriptor",
        ));
    }
    let selected_services: BTreeSet<_> = graph.service_consumers.keys().cloned().collect();
    let order = inputs
        .catalogue
        .service_order(&selected_services)
        .map_err(|_| {
            failure(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Service dependencies are inconsistent",
            )
        })?;
    let mut actions = Vec::new();
    let mut before = BTreeMap::new();
    let mut last_service_action = BTreeMap::<ServiceId, Vec<StepId>>::new();
    let mut disruption = Disruption::None;
    let mut images = BTreeMap::new();
    // Foreign route targets are observed and bound, but never reconfigured here.
    for (identity, ownership) in resources
        .values()
        .filter(|(identity, _)| selected_services.contains(&identity.service_id))
    {
        if *ownership != Ownership::Associated {
            continue;
        }
        let observed = observations
            .resources
            .get(&identity.resource_id)
            .ok_or_else(|| {
                failure(
                    inputs,
                    ErrorCode::ResourceDrift,
                    "Associated route target observation is missing",
                )
            })?;
        if observed.identity != **identity
            || observed.valid_until.0 <= inputs.now.0
            || observed.observed_at.0 > inputs.now.0
            || matches!(
                observed.state,
                ObservedState::Unknown | ObservedState::Failed
            )
        {
            return Err(failure(
                inputs,
                ErrorCode::ResourceDrift,
                "Associated route target observation is stale or has drifted",
            ));
        }
        before.insert(identity.resource_id.clone(), identity.fingerprint.clone());
    }

    for service_id in &order {
        let descriptor = inputs.catalogue.services().get(service_id).ok_or_else(|| {
            failure(
                inputs,
                ErrorCode::ModuleUnknown,
                "Service is not in the trusted catalogue",
            )
        })?;
        if descriptor.foreign {
            continue;
        } // Lifecycle owns associated start/stop; config never takes over native configuration.
        if service_id.as_str() == "runtime" {
            continue;
        } // Supervisor lifecycle is independent of generated traffic artifacts.
        if descriptor.image_required {
            images.insert(
                service_id.clone(),
                inputs.images.get(service_id).cloned().ok_or_else(|| {
                    failure(
                        inputs,
                        ErrorCode::ModuleIncomplete,
                        "A selected service has no pinned image digest",
                    )
                })?,
            );
        }
        let target = inputs.target_artifacts.get(service_id).ok_or_else(|| {
            failure(
                inputs,
                ErrorCode::ModuleIncomplete,
                "A selected service has no exact target artifact fingerprint",
            )
        })?;
        let service_resources: Vec<_> = resources
            .values()
            .filter(|(resource, ownership)| {
                resource.service_id == *service_id && *ownership == Ownership::Owned
            })
            .collect();
        if service_resources.is_empty() {
            return Err(failure(
                inputs,
                ErrorCode::ModuleIncomplete,
                "A selected service has no installed exact owned resource",
            ));
        }
        for (identity, _) in service_resources {
            let observed = observations
                .resources
                .get(&identity.resource_id)
                .ok_or_else(|| {
                    failure(
                        inputs,
                        ErrorCode::ResourceDrift,
                        "Resource observation is missing",
                    )
                })?;
            if observed.identity != **identity
                || observed.valid_until.0 <= inputs.now.0
                || observed.observed_at.0 > inputs.now.0
                || matches!(
                    observed.state,
                    ObservedState::Unknown | ObservedState::Failed
                )
            {
                return Err(failure(
                    inputs,
                    ErrorCode::ResourceDrift,
                    "Resource observation is stale, uncertain or has changed identity",
                ));
            }
            before.insert(identity.resource_id.clone(), identity.fingerprint.clone());
            // Saving configuration while stopped never starts or reloads traffic services.
            if inputs.run_intent == RunIntent::Stopped && !inputs.start_requested {
                continue;
            }
            let changed = inputs.prior_artifacts.get(service_id) != Some(target);
            let kind = match (observed.state, changed) {
                (ObservedState::Stopped, _) => Some(ActionKind::Start),
                (ObservedState::Running, true) if inputs.recreate_services.contains(service_id) => {
                    Some(ActionKind::Recreate)
                }
                (ObservedState::Running, true) => Some(descriptor.change_action),
                (ObservedState::Running, false) => None,
                _ => None,
            };
            let needs_autostart = matches!(
                observed.restart_policy,
                Some(RestartPolicy::DockerNo | RestartPolicy::SystemdDisabled)
            ) || inputs.recreate_services.contains(service_id);
            if observed.restart_policy.is_none() {
                return Err(failure(
                    inputs,
                    ErrorCode::ResourceDrift,
                    "selected restart policy is unavailable",
                ));
            }
            let mut kinds: Vec<_> = kind.into_iter().collect();
            if needs_autostart {
                kinds.push(ActionKind::RestoreAutostart);
            }
            for kind in kinds {
                let action_id = step(
                    if kind == ActionKind::RestoreAutostart {
                        "autostart"
                    } else {
                        "activate"
                    },
                    service_id,
                    &identity.resource_id,
                );
                let mut dependencies: BTreeSet<StepId> = descriptor
                    .dependencies
                    .iter()
                    .flat_map(|dependency| {
                        last_service_action
                            .get(dependency)
                            .into_iter()
                            .flatten()
                            .cloned()
                    })
                    .collect();
                if kind == ActionKind::RestoreAutostart {
                    dependencies.extend(
                        last_service_action
                            .get(service_id)
                            .into_iter()
                            .flatten()
                            .cloned(),
                    );
                }
                let mut preconditions = vec![
                    Condition::ResourceFingerprint {
                        resource_id: identity.resource_id.clone(),
                        fingerprint: identity.fingerprint.clone(),
                    },
                    Condition::RunEpoch {
                        epoch: inputs.run_epoch,
                    },
                ];
                for approval in &approved {
                    preconditions.push(Condition::Approval {
                        approval_id: approval.approval_id.clone(),
                    });
                }
                actions.push(Action {
                    action_id: action_id.clone(),
                    kind,
                    resource_id: identity.resource_id.clone(),
                    dependencies,
                    preconditions,
                    postconditions: if kind == ActionKind::RestoreAutostart {
                        vec![Condition::Autostart {
                            resource_id: identity.resource_id.clone(),
                            enabled: true,
                        }]
                    } else {
                        vec![Condition::ResourceRunning {
                            resource_id: identity.resource_id.clone(),
                            running: true,
                        }]
                    },
                    timeout: DurationMillis(30_000),
                    compensation_ref: None,
                });
                last_service_action
                    .entry(service_id.clone())
                    .or_default()
                    .push(action_id);
                disruption = match kind {
                    ActionKind::RestoreAutostart => disruption,
                    ActionKind::Reload if disruption == Disruption::None => Disruption::Reload,
                    ActionKind::Reload => disruption,
                    _ => Disruption::ConnectionsInterrupted,
                };
            }
        }
    }
    // Stop only installed owned services without any surviving mandatory/feature consumer.
    let deselected: BTreeSet<_> = resources
        .values()
        .filter(|(resource, ownership)| {
            *ownership == Ownership::Owned
                && !selected_services.contains(&resource.service_id)
                && inputs
                    .catalogue
                    .services()
                    .contains_key(&resource.service_id)
        })
        .map(|(resource, _)| resource.service_id.clone())
        .collect();
    let mut remaining = deselected.clone();
    let mut stop_order = Vec::new();
    while !remaining.is_empty() {
        let ready: Vec<_> = remaining
            .iter()
            .filter(|id| {
                !remaining.iter().any(|other| {
                    inputs
                        .catalogue
                        .services()
                        .get(other)
                        .is_some_and(|descriptor| descriptor.dependencies.contains(*id))
                })
            })
            .cloned()
            .collect();
        if ready.is_empty() {
            return Err(failure(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Retired service dependency cycle",
            ));
        }
        for id in ready {
            remaining.remove(&id);
            stop_order.push(id);
        }
    }
    let mut stop_actions = BTreeMap::<ServiceId, Vec<StepId>>::new();
    for service_id in stop_order {
        for (identity, ownership) in resources
            .values()
            .filter(|(identity, _)| identity.service_id == service_id)
        {
            if *ownership != Ownership::Owned {
                continue;
            }
            let observed = observations
                .resources
                .get(&identity.resource_id)
                .ok_or_else(|| {
                    failure(
                        inputs,
                        ErrorCode::ResourceDrift,
                        "Retired resource observation is missing",
                    )
                })?;
            if observed.identity != **identity
                || observed.valid_until.0 <= inputs.now.0
                || observed.observed_at.0 > inputs.now.0
                || matches!(
                    observed.state,
                    ObservedState::Unknown | ObservedState::Failed
                )
            {
                return Err(failure(
                    inputs,
                    ErrorCode::ResourceDrift,
                    "Retired resource observation is stale or uncertain",
                ));
            }
            let autostart = match observed.restart_policy {
                Some(RestartPolicy::DockerNo | RestartPolicy::SystemdDisabled) => false,
                Some(_) => true,
                None => {
                    return Err(failure(
                        inputs,
                        ErrorCode::ResourceDrift,
                        "Retired resource restart-owner observation is unavailable",
                    ));
                }
            };
            if !autostart && observed.state == ObservedState::Stopped {
                continue;
            }
            before.insert(identity.resource_id.clone(), identity.fingerprint.clone());
            let mut dependencies: BTreeSet<StepId> = stop_actions
                .iter()
                .filter(|(consumer, _)| {
                    inputs
                        .catalogue
                        .services()
                        .get(*consumer)
                        .is_some_and(|descriptor| descriptor.dependencies.contains(&service_id))
                })
                .flat_map(|(_, ids)| ids.iter().cloned())
                .collect();
            if autostart {
                let disable_id = step("disable-autostart", &service_id, &identity.resource_id);
                actions.push(Action {
                    action_id: disable_id.clone(),
                    kind: ActionKind::DisableAutostart,
                    resource_id: identity.resource_id.clone(),
                    dependencies,
                    preconditions: vec![
                        Condition::ResourceFingerprint {
                            resource_id: identity.resource_id.clone(),
                            fingerprint: identity.fingerprint.clone(),
                        },
                        Condition::RunEpoch {
                            epoch: inputs.run_epoch,
                        },
                    ],
                    postconditions: vec![Condition::Autostart {
                        resource_id: identity.resource_id.clone(),
                        enabled: false,
                    }],
                    timeout: DurationMillis(15_000),
                    compensation_ref: None,
                });
                if observed.state == ObservedState::Stopped {
                    stop_actions
                        .entry(service_id.clone())
                        .or_default()
                        .push(disable_id);
                    continue;
                }
                dependencies = BTreeSet::from([disable_id]);
            }
            let action_id = step("stop", &service_id, &identity.resource_id);
            actions.push(Action {
                action_id: action_id.clone(),
                kind: ActionKind::Stop,
                resource_id: identity.resource_id.clone(),
                dependencies,
                preconditions: vec![
                    Condition::ResourceFingerprint {
                        resource_id: identity.resource_id.clone(),
                        fingerprint: identity.fingerprint.clone(),
                    },
                    Condition::RunEpoch {
                        epoch: inputs.run_epoch,
                    },
                ],
                postconditions: vec![
                    Condition::ResourceRunning {
                        resource_id: identity.resource_id.clone(),
                        running: false,
                    },
                    Condition::Autostart {
                        resource_id: identity.resource_id.clone(),
                        enabled: false,
                    },
                ],
                timeout: DurationMillis(30_000),
                compensation_ref: None,
            });
            stop_actions
                .entry(service_id.clone())
                .or_default()
                .push(action_id);
            disruption = Disruption::ConnectionsInterrupted;
        }
    }
    for action in &mut actions {
        let matching: Vec<_> = approved
            .iter()
            .filter(|approval| approval.resources.contains(&action.resource_id))
            .collect();
        if matching.is_empty() {
            return Err(failure(
                inputs,
                ErrorCode::ApprovalRequired,
                "Action resource is outside the exact locally approved scope",
            ));
        }
        for approval in matching {
            let condition = Condition::Approval {
                approval_id: approval.approval_id.clone(),
            };
            if !action.preconditions.contains(&condition) {
                action.preconditions.push(condition);
            }
        }
    }
    if requirements_only {
        return Ok(Calculation::Requirements(actions));
    }
    let mut compensations = Vec::new();
    let mut inverse_ids = BTreeSet::new();
    let mut current = BTreeMap::new();
    if inputs.native_transitions.len() != actions.len() {
        return Err(failure(
            inputs,
            ErrorCode::ResourceDrift,
            "Every planned effect needs exactly one trusted native transition binding",
        ));
    }
    let forward_ids: BTreeSet<_> = actions
        .iter()
        .map(|action| action.action_id.clone())
        .collect();
    for action in &mut actions {
        let transition = inputs
            .native_transitions
            .get(&action.action_id)
            .ok_or_else(|| {
                failure(
                    inputs,
                    ErrorCode::ResourceDrift,
                    "Exact native before/after transition is unavailable",
                )
            })?;
        let inverse = &transition.compensation;
        let fingerprint = |conditions: &[Condition], expected: &Sha256Digest| {
            conditions.iter().filter(|condition|matches!(condition,Condition::ResourceFingerprint {..})).count()==1 && conditions.iter().any(|condition|matches!(condition,Condition::ResourceFingerprint {resource_id,fingerprint} if resource_id==&action.resource_id && fingerprint==expected))
        };
        let inverse_kind = matches!(
            (action.kind, inverse.kind),
            (ActionKind::Start, ActionKind::Stop)
                | (ActionKind::Stop, ActionKind::Start)
                | (ActionKind::DisableAutostart, ActionKind::RestoreAutostart)
                | (ActionKind::RestoreAutostart, ActionKind::DisableAutostart)
                | (ActionKind::Reload, ActionKind::Reload)
                | (ActionKind::Restart, ActionKind::Restart)
                | (ActionKind::Recreate, ActionKind::Recreate)
                | (ActionKind::PublishArtifacts, ActionKind::PublishArtifacts)
                | (ActionKind::SuspendJob, ActionKind::ResumeJob)
                | (ActionKind::ResumeJob, ActionKind::SuspendJob)
        );
        if transition.before == transition.after
            || current
                .get(&action.resource_id)
                .is_some_and(|value| value != &transition.before)
            || inverse.resource_id != action.resource_id
            || !inverse_kind
            || inverse.compensation_ref.is_some()
            || forward_ids.contains(&inverse.action_id)
            || !inverse_ids.insert(inverse.action_id.clone())
            || !fingerprint(&inverse.preconditions, &transition.after)
            || !fingerprint(&inverse.postconditions, &transition.before)
            || inverse.timeout.0 == 0
            || inverse.timeout.0 > 120_000
        {
            return Err(failure(
                inputs,
                ErrorCode::ResourceDrift,
                "Native effect or inverse does not bind one exact reversible transition",
            ));
        }
        let mut inverse = inverse.clone();
        for approval in approved
            .iter()
            .filter(|approval| approval.resources.contains(&inverse.resource_id))
        {
            let condition = Condition::Approval {
                approval_id: approval.approval_id.clone(),
            };
            if !inverse.preconditions.contains(&condition) {
                inverse.preconditions.push(condition);
            }
        }
        inverse
            .preconditions
            .retain(|condition| !matches!(condition, Condition::RunEpoch { .. }));
        inverse.preconditions.push(Condition::RunEpoch {
            epoch: inputs.run_epoch,
        });
        action
            .preconditions
            .retain(|condition| !matches!(condition, Condition::ResourceFingerprint { .. }));
        action
            .postconditions
            .retain(|condition| !matches!(condition, Condition::ResourceFingerprint { .. }));
        action.preconditions.push(Condition::ResourceFingerprint {
            resource_id: action.resource_id.clone(),
            fingerprint: transition.before.clone(),
        });
        action.postconditions.push(Condition::ResourceFingerprint {
            resource_id: action.resource_id.clone(),
            fingerprint: transition.after.clone(),
        });
        action.compensation_ref = Some(inverse.action_id.clone());
        current.insert(action.resource_id.clone(), transition.after.clone());
        compensations.push(inverse);
    }
    if compensations.iter().any(|action| {
        action
            .dependencies
            .iter()
            .any(|id| !inverse_ids.contains(id))
    }) {
        return Err(failure(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Compensation dependency is outside the approved inverse action set",
        ));
    }
    let mut resolved = BTreeSet::new();
    while resolved.len() < compensations.len() {
        let count = resolved.len();
        for inverse in &compensations {
            if inverse.dependencies.iter().all(|id| resolved.contains(id)) {
                resolved.insert(inverse.action_id.clone());
            }
        }
        if count == resolved.len() {
            return Err(failure(
                inputs,
                ErrorCode::ConfigSemanticConflict,
                "Compensation dependency graph contains a cycle",
            ));
        }
    }
    let descriptor_versions = graph
        .enabled_modules
        .iter()
        .filter_map(|id| {
            inputs
                .catalogue
                .describe(id)
                .map(|entry| (id.clone(), entry.descriptor.descriptor_revision.clone()))
        })
        .collect();
    let mut bound_read_set = read_set.clone();
    bind(&mut bound_read_set, "state.registry", registry, inputs)?;
    bind(
        &mut bound_read_set,
        "state.runtime",
        &RuntimeBinding {
            revision: inputs.runtime_revision,
            run_intent: inputs.run_intent,
            run_epoch: inputs.run_epoch,
            accepted_bundle: &inputs.accepted_bundle,
        },
        inputs,
    )?;
    bind(
        &mut bound_read_set,
        "state.catalogue",
        &inputs.catalogue.fingerprint(),
        inputs,
    )?;
    bind(
        &mut bound_read_set,
        "plan.targetArtifacts",
        &inputs.target_artifacts,
        inputs,
    )?;
    bind(
        &mut bound_read_set,
        "plan.priorArtifacts",
        &inputs.prior_artifacts,
        inputs,
    )?;
    let encoded = serde_json::to_vec(&(
        snapshot,
        graph,
        &bound_read_set,
        &images,
        &before,
        &approved,
        &actions,
        &compensations,
        &inputs.rollback_target,
    ))
    .map_err(|_| {
        failure(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Plan inputs could not be encoded",
        )
    })?;
    Ok(Calculation::Final(EffectPlan {
        candidate_hash: digest(&encoded),
        read_set: bound_read_set,
        catalogue_version: inputs.catalogue.version(),
        descriptor_versions,
        image_versions: images,
        secret_versions: read_set.secret_versions.clone(),
        resource_before: before,
        approvals: approved,
        actions,
        compensations,
        expected_disruption: disruption,
        expires_at: inputs.expires_at,
        rollback_target: inputs.rollback_target.clone(),
    }))
}
fn bind<T: Serialize>(
    read_set: &mut ReadSet,
    name: &str,
    value: &T,
    inputs: &PlanInputs<'_>,
) -> Result<()> {
    let object_id = object(name);
    if read_set.objects.contains_key(&object_id) {
        return Err(failure(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Configuration input used a reserved runtime planning identity",
        ));
    }
    let bytes = serde_json::to_vec(value).map_err(|_| {
        failure(
            inputs,
            ErrorCode::ConfigSemanticConflict,
            "Planning state could not be encoded",
        )
    })?;
    read_set.objects.insert(
        object_id.clone(),
        ReadDependency {
            revision: content_revision(object_id, &bytes),
            format_version: FormatVersion(1),
        },
    );
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeBinding<'a> {
    revision: StateRevision,
    run_intent: RunIntent,
    run_epoch: RunEpoch,
    accepted_bundle: &'a Option<BundleId>,
}

/// Exact synthetic state binding, shared with trusted host recheck/publication.
pub fn runtime_binding_revision(
    revision: StateRevision,
    run_intent: RunIntent,
    run_epoch: RunEpoch,
    accepted_bundle: &Option<BundleId>,
) -> std::result::Result<ContentRevision, serde_json::Error> {
    let binding = RuntimeBinding {
        revision,
        run_intent,
        run_epoch,
        accepted_bundle,
    };
    Ok(content_revision(
        object("state.runtime"),
        &serde_json::to_vec(&binding)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_binding_revision_is_deterministic() {
        let a = runtime_binding_revision(StateRevision(0), RunIntent::Running, RunEpoch(0), &None)
            .unwrap();
        let b = runtime_binding_revision(StateRevision(0), RunIntent::Running, RunEpoch(0), &None)
            .unwrap();
        assert_eq!(a, b);
        assert_eq!(a.object_id, ObjectId::new("state.runtime").unwrap());
    }

    #[test]
    fn runtime_binding_revision_varies_with_inputs() {
        let base =
            runtime_binding_revision(StateRevision(0), RunIntent::Running, RunEpoch(0), &None)
                .unwrap();
        let revision =
            runtime_binding_revision(StateRevision(1), RunIntent::Running, RunEpoch(0), &None)
                .unwrap();
        let intent =
            runtime_binding_revision(StateRevision(0), RunIntent::Stopped, RunEpoch(0), &None)
                .unwrap();
        let epoch =
            runtime_binding_revision(StateRevision(0), RunIntent::Running, RunEpoch(1), &None)
                .unwrap();
        let bundle = runtime_binding_revision(
            StateRevision(0),
            RunIntent::Running,
            RunEpoch(0),
            &Some(BundleId::new("bundle1").unwrap()),
        )
        .unwrap();
        assert_ne!(base.digest, revision.digest);
        assert_ne!(base.digest, intent.digest);
        assert_ne!(base.digest, epoch.digest);
        assert_ne!(base.digest, bundle.digest);
    }

    #[test]
    fn step_id_is_bounded_and_stable() {
        let svc = ServiceId::new("svc").unwrap();
        let res = ResourceId::new("res").unwrap();
        let a = step("activate", &svc, &res);
        let b = step("activate", &svc, &res);
        assert_eq!(a, b);
        assert!(a.as_str().starts_with("activate."));
        assert_eq!(a.as_str().len(), "activate.".len() + 32);

        let long_svc = ServiceId::new(&"s".repeat(128)).unwrap();
        let long_res = ResourceId::new(&"r".repeat(128)).unwrap();
        let c = step("activate", &long_svc, &long_res);
        assert_eq!(c.as_str().len(), "activate.".len() + 32);
        assert_ne!(a, c);
    }
}
