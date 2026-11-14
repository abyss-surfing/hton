//! Ownership-bounded maintenance removal; retained configuration is never purged.
use super::{HostApplication, observations, runtime, systemd_installation};
use crate::{
    config,
    journal::{self, Intent, TerminalStatus, Transaction},
};
use hton_contracts::{config::*, operations::*};
use hton_runtime_contract as wire;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn failure(ctx: &OperationContext, code: ErrorCode, message: &'static str) -> Error {
    journal::failure(ctx, code, message)
}

fn transactions(app: &HostApplication, ctx: &OperationContext) -> Result<Vec<Transaction>> {
    let directory = app.work.child("journal").map_err(|_| {
        failure(
            ctx,
            ErrorCode::IoUnavailable,
            "uninstall journal unavailable",
        )
    })?;
    let mut transactions = Vec::new();
    for name in directory.list(10_000).map_err(|_| {
        failure(
            ctx,
            ErrorCode::IoUnavailable,
            "uninstall journal inventory exceeds bounds",
        )
    })? {
        if let Some(id) = name
            .strip_prefix("intent-")
            .and_then(|name| name.strip_suffix(".json"))
        {
            let id = TransactionId::new(id).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "uninstall journal identity is invalid",
                )
            })?;
            transactions.push(app.journal.read(&id, ctx)?);
        }
    }
    Ok(transactions)
}

fn validate_registry(
    original: &RegistrySnapshot,
    current: &RegistrySnapshot,
    ctx: &OperationContext,
) -> Result<()> {
    if original.instance_id != current.instance_id
        || original.entries.len() != current.entries.len()
    {
        return Err(failure(
            ctx,
            ErrorCode::ResourceDrift,
            "installed composition changed during uninstall",
        ));
    }
    for (id, entry) in &original.entries {
        if entry.ownership != Ownership::Owned
            || entry.adoption_scope.is_some()
            || entry.resources.iter().any(|resource| {
                !matches!(resource.locator, ResourceLocator::DockerContainer { .. })
            })
        {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "foreign restoration or unsupported resource ownership blocks uninstall before effects",
            ));
        }
        let Some(now) = current.entries.get(id) else {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "module disappeared during uninstall",
            ));
        };
        // A resumed uninstall may already have cleared this exact owned entry.
        if now != entry && *now != crate::registry::uninstalled_owned_projection(entry) {
            return Err(failure(
                ctx,
                ErrorCode::ResourceDrift,
                "module was re-added or changed during uninstall",
            ));
        }
    }
    Ok(())
}

pub fn execute(app: &HostApplication, args: &[String], ctx: &OperationContext) -> Result<Value> {
    let mut flags = BTreeSet::new();
    for argument in args {
        if !matches!(
            argument.as_str(),
            "--keep-certs" | "--keep-env" | "--foreign" | "--purge"
        ) || !flags.insert(argument.as_str())
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "invalid or duplicate uninstall option",
            ));
        }
    }
    if flags.contains("--purge") || flags.contains("--foreign") {
        return Err(failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "full data purge or foreign deletion requires a separately qualified exact deletion inventory",
        ));
    }
    let current = app.registry(ctx)?;
    let pending = app
        .journal
        .unfinished(ctx)?
        .into_iter()
        .filter(|transaction| matches!(transaction.intent, Intent::InstallationUninstall { .. }))
        .collect::<Vec<_>>();
    if pending.len() > 1 {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "multiple uninstall operations require reconciliation",
        ));
    }
    let original = match pending.first().map(|transaction| &transaction.intent) {
        Some(Intent::InstallationUninstall {
            registry,
            keep_certs,
            keep_env,
        }) => {
            if *keep_certs != flags.contains("--keep-certs")
                || *keep_env != flags.contains("--keep-env")
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "resume uninstall with its original options",
                ));
            }
            registry.clone()
        }
        _ => current.clone(),
    };
    validate_registry(&original, &current, ctx)?;
    let registered = original
        .entries
        .values()
        .flat_map(|entry| entry.resources.iter())
        .map(|resource| resource.resource_id.as_str())
        .collect::<BTreeSet<_>>();
    if app
        .runtime(ctx)?
        .resources
        .as_slice()
        .iter()
        .any(|receipt| {
            receipt.resource.ownership != wire::Ownership::Owned
                || !registered.contains(receipt.resource.resource_id.as_str())
        })
    {
        return Err(failure(
            ctx,
            ErrorCode::ResourceDrift,
            "runtime composition contains unrecorded or foreign uninstall resources",
        ));
    }
    systemd_installation::preflight_uninstall(app, ctx)?;
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        return Ok(
            json!({"status":"preview", "operation":"installation.uninstall",
            "writes":false, "modules":original.entries.keys().collect::<Vec<_>>(),
            "retainsConfiguration":true, "retainsRecovery":true}),
        );
    }
    journal::ensure_mutating(ctx, app.control.as_ref())?;
    let tx = match pending.first() {
        Some(transaction) => transaction.transaction_id.clone(),
        None => app.journal.prepare_intent(
            Intent::InstallationUninstall {
                registry: original.clone(),
                keep_certs: flags.contains("--keep-certs"),
                keep_env: flags.contains("--keep-env"),
            },
            vec![],
            ctx,
        )?,
    };
    let mut operation_ctx = ctx.clone();
    if let Some(transaction) = pending.first() {
        operation_ctx.operation_id = transaction.operation_id.clone();
    }
    // Reconcile only the exact nested registry transitions of this uninstall;
    // unrelated pending operations retain their own recovery authority.
    for transaction in app.journal.unfinished(&operation_ctx)? {
        if transaction.operation_id == operation_ctx.operation_id
            && matches!(
                &transaction.intent,
                Intent::Registry {
                    change: RegistryChange::UninstallOwned { .. },
                    ..
                }
            )
        {
            app.registry
                .recover_uninstall(&transaction.transaction_id, &operation_ctx)?;
        }
    }
    let state = app.runtime(&operation_ctx)?;
    if state.run_intent != wire::RunIntent::Stopped
        || state.resources.as_slice().iter().any(|receipt| {
            receipt.status != wire::ResourceStatus::Stopped
                || receipt.restart_policy != wire::RestartPolicy::Disabled
        })
    {
        let stopped = runtime::execute(app, wire::ExecutorAction::StopAll, &operation_ctx)?;
        if stopped.status != wire::ResultStatus::Succeeded
            || !stopped.survivors.as_slice().is_empty()
        {
            return Ok(
                json!({"status":"partial", "operation":"installation.uninstall",
                "reason":"shutdownIncomplete", "transactionId":tx, "ready":false,
                "runtimeResult":stopped, "retainsRecovery":true}),
            );
        }
    }
    validate_registry(&original, &app.registry(&operation_ctx)?, &operation_ctx)?;
    let children = transactions(app, &operation_ctx)?
        .into_iter()
        .filter(|transaction| {
            transaction.operation_id == operation_ctx.operation_id
                && matches!(transaction.intent, Intent::UninstallNative { .. })
        })
        .collect::<Vec<_>>();
    if children.len() > 1 {
        return Err(failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "multiple native uninstall target sets require reconciliation",
        ));
    }
    let (native_tx, targets) = if let Some(transaction) = children.first() {
        let Intent::UninstallNative { resources } = &transaction.intent else {
            unreachable!()
        };
        (transaction.transaction_id.clone(), resources.clone())
    } else {
        let observed = observations::collect(app, &operation_ctx)?;
        let mut targets = Vec::new();
        let mut unique = BTreeSet::new();
        for entry in original.entries.values() {
            for resource in &entry.resources {
                let target = observed
                    .native
                    .get(&resource.resource_id)
                    .filter(|target| {
                        target.kind == wire::ResourceKind::Container
                            && target.ownership == wire::Ownership::Owned
                    })
                    .ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::ResourceDrift,
                            "exact owned native uninstall target is unavailable",
                        )
                    })?;
                if !unique.insert(target.resource_id.clone()) {
                    return Err(failure(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "duplicate native uninstall target",
                    ));
                }
                targets.push(target.clone());
            }
        }
        let child = app.journal.prepare_intent(
            Intent::UninstallNative {
                resources: targets.clone(),
            },
            vec![],
            &operation_ctx,
        )?;
        (child, targets)
    };
    let expected_ids = original
        .entries
        .values()
        .flat_map(|entry| entry.resources.iter())
        .map(|resource| resource.resource_id.as_str())
        .collect::<BTreeSet<_>>();
    if targets
        .iter()
        .map(|resource| resource.resource_id.as_str())
        .collect::<BTreeSet<_>>()
        != expected_ids
        || targets.len() != expected_ids.len()
    {
        return Err(failure(
            ctx,
            ErrorCode::ResourceDrift,
            "native uninstall target set differs from recorded ownership",
        ));
    }
    let removed = runtime::execute(
        app,
        wire::ExecutorAction::UninstallOwned {
            resources: wire::Items::new(targets.clone()).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "native uninstall set exceeds bounds",
                )
            })?,
        },
        &operation_ctx,
    )?;
    let verified = removed
        .receipts
        .as_slice()
        .iter()
        .filter_map(|receipt| {
            if let wire::ActionOutcome::Removed { resource, .. } = &receipt.outcome {
                Some(resource)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if removed.status != wire::ResultStatus::Succeeded
        || verified.len() != targets.len()
        || targets.iter().any(|target| !verified.contains(&target))
        || !removed.survivors.as_slice().is_empty()
    {
        return Ok(
            json!({"status":"partial", "operation":"installation.uninstall",
            "reason":"nativeRemovalIncomplete", "transactionId":tx,
            "runtimeResult":removed, "retainsRecovery":true}),
        );
    }
    if app.journal.terminal(&native_tx, &operation_ctx)?.is_none() {
        app.journal.finish(
            &native_tx,
            TerminalStatus::Committed,
            vec![],
            &operation_ctx,
        )?;
    }
    for (module_id, entry) in &original.entries {
        let current = app.registry(&operation_ctx)?;
        if current.entries.get(module_id).is_some_and(|entry| {
            entry.membership == MembershipState::RemovedRetained && entry.resources.is_empty()
        }) {
            continue;
        }
        validate_registry(&original, &current, &operation_ctx)?;
        let fingerprints = entry
            .resources
            .iter()
            .map(|resource| {
                serde_json::to_vec(&("uninstall-owned-v1", resource))
                    .map(|bytes| (resource.resource_id.clone(), config::digest(&bytes)))
            })
            .collect::<std::result::Result<BTreeMap<_, _>, _>>()
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "native uninstall receipt encoding failed",
                )
            })?;
        let receipt = StepReceipt {
            step_id: StepId::new("uninstall-owned").expect("fixed step ID"),
            idempotency_key: operation_ctx.actor.request_id.clone(),
            status: StepStatus::Verified,
            recorded_at: UnixMillis(app.control.now_ms()),
            resources: entry.resources.clone(),
            state_fingerprints: fingerprints,
            retained_artifacts: vec![],
            error: None,
        };
        app.registry.transition(
            &original.instance_id,
            current.revision,
            RegistryChange::UninstallOwned {
                module_id: module_id.clone(),
                removed_resources: entry.resources.clone(),
                receipt,
            },
            &Default::default(),
            &operation_ctx,
        )?;
    }
    systemd_installation::uninstall(app, &operation_ctx)?;
    let state = app.runtime(&operation_ctx)?;
    let result = OperationResult {
        operation_id: operation_ctx.operation_id.clone(),
        status: OperationStatus::Succeeded,
        transaction_id: Some(tx.clone()),
        desired_saved: false,
        pending_bundle: None,
        accepted_bundle: state
            .accepted
            .as_ref()
            .map(|id| BundleId::new(id.as_str()).expect("wire ID grammar")),
        run_intent: RunIntent::Stopped,
        member_results: vec![],
        survivors: vec![],
        issues: vec![],
    };
    app.journal.finish_operation(
        &tx,
        TerminalStatus::Committed,
        vec![],
        &result,
        &operation_ctx,
    )?;
    Ok(
        json!({"status":"succeeded", "operation":"installation.uninstall",
        "operationResult":result, "retainsConfiguration":true, "retainsCertificates":true,
        "retainsCode":true, "retainsRecovery":true}),
    )
}
