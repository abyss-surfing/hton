//! Module membership composition over the shared selection/lifecycle/effect owners.
use super::{HostApplication, effects};
use crate::{catalog::ModuleCatalogue, config, journal, lifecycle, selection};
use hton_contracts::{config::*, operations::*};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn operation(
    app: &HostApplication,
    operation: &str,
    args: &[String],
    ctx: &OperationContext,
) -> Result<Value> {
    let module_id = match args {
        [id] => ModuleId::new(id).map_err(|_| {
            journal::failure(ctx, ErrorCode::ModuleUnknown, "invalid module identity")
        })?,
        _ => {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "module membership operation expects exactly one module ID",
            ));
        }
    };
    let catalogue = ModuleCatalogue::builtin().map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "trusted module catalogue unavailable",
        )
    })?;
    let descriptor = catalogue
        .describe(&module_id)
        .ok_or_else(|| journal::failure(ctx, ErrorCode::ModuleUnknown, "unknown module"))?;
    if module_id.as_str() == "core"
        || !descriptor.implemented
        || descriptor.descriptor.phase != ReleasePhase::R1
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "module lifecycle is outside the selected release",
        ));
    }
    let registry = app.registry(ctx)?;
    if operation == "modules.purge" {
        return purge_config(app, &module_id, ctx);
    }
    let entry = registry.entries.get(&module_id).ok_or_else(|| {
        journal::failure(ctx, ErrorCode::ModuleUnknown, "module is not registered")
    })?;
    if operation == "modules.update" {
        // The trusted R1 descriptors deliberately expose no feature-owned update
        // target: VLESS uses Core-owned engines and Amnezia stays native-owned.
        if descriptor.descriptor.capabilities.update.is_none() {
            return Err(journal::failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "module has no module-owned update target; shared and foreign images are outside this operation",
            ));
        }
        return Err(journal::failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "trusted module update adapter is unavailable",
        ));
    }
    if operation != "modules.remove" {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "unsupported module membership operation",
        ));
    }
    if entry.ownership != Ownership::Owned || !entry.resources.is_empty() {
        return Err(journal::failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "module removal requires a qualified exact resource stop adapter",
        ));
    }
    if entry.membership == MembershipState::RemovedRetained {
        let desired = app.desired(ctx)?;
        if !desired.errors.is_empty()
            || desired.read_set.coherence != ReadCoherence::Coherent
            || desired.snapshot.as_ref().is_none_or(|snapshot| {
                snapshot
                    .features
                    .get(&module_id)
                    .is_some_and(|selection| selection.enabled)
            })
            || !app.journal.unfinished(ctx)?.is_empty()
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "removed module participation or pending recovery requires reconciliation",
            ));
        }
        return Ok(json!({"status":"succeeded", "operation":operation,
            "moduleId":module_id, "alreadyRemoved":true, "retainsData":true, "writes":false}));
    }
    if entry.membership != MembershipState::Installed {
        return Err(journal::failure(
            ctx,
            ErrorCode::ModuleIncomplete,
            "module is not installed or requires cleanup reconciliation",
        ));
    }
    let desired = app.desired(ctx)?;
    let root = desired
        .documents
        .iter()
        .find(|document| document.object_id.as_str() == "root")
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "module removal requires a coherent root configuration",
            )
        })?;
    let source = config::FsConfigSource::open(&app.paths.config).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "configuration source unavailable",
        )
    })?;
    let candidate = selection::prepare(
        &source,
        BTreeSet::from([module_id.clone()]),
        false,
        &root.revision,
        ctx,
    )?;
    let prepared = effects::prepare_selection(app, &candidate, ctx)?;
    let snapshot = lifecycle::LifecycleSnapshot {
        desired: Some(desired),
        registry,
        runtime: app.runtime(ctx)?,
        observations: RuntimeObservation {
            resources: BTreeMap::new(),
        },
    };
    let request = LifecycleRequest::Remove {
        module_id: module_id.clone(),
        approval_refs: BTreeSet::new(),
    };
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        let plan = lifecycle::plan(request, &snapshot, &prepared, &catalogue, ctx)?;
        return Ok(
            json!({"status":"preview", "operation":operation, "writes":false,
            "plan":plan, "retainsData":true}),
        );
    }
    let result = lifecycle::execute_resource_free_remove(
        request,
        &snapshot,
        &prepared,
        &catalogue,
        &app.registry,
        &app.journal,
        |plan| {
            let saved = selection::SelectionWriter::new(app.config.clone(), app.journal.clone())
                .commit(&candidate, &candidate.change().expected_root_revision, ctx)?;
            let effect = plan.effect_plan.as_ref().ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "removal lost its prepared effect",
                )
            })?;
            match effects::apply(app, effect, ctx) {
                Ok(mut result) => {
                    result.desired_saved = true;
                    Ok(result)
                }
                Err(error) => Ok(OperationResult {
                    operation_id: ctx.operation_id.clone(),
                    status: OperationStatus::RecoveryRequired,
                    transaction_id: saved.transaction_id,
                    desired_saved: true,
                    pending_bundle: None,
                    accepted_bundle: snapshot
                        .runtime
                        .accepted
                        .as_ref()
                        .map(|id| BundleId::new(id.as_str()).expect("wire ID grammar")),
                    run_intent: if snapshot.runtime.run_intent
                        == hton_runtime_contract::RunIntent::Running
                    {
                        RunIntent::Running
                    } else {
                        RunIntent::Stopped
                    },
                    member_results: vec![MemberResult {
                        module_id: module_id.clone(),
                        status: OperationStatus::RecoveryRequired,
                        resources: vec![],
                        error: Some(error),
                    }],
                    survivors: vec![],
                    issues: vec![],
                }),
            }
        },
        ctx,
    )?;
    serde_json::to_value(result).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "module result encoding failed",
        )
    })
}

fn artifact_id() -> ArtifactId {
    ArtifactId::new("config.vless.v1").expect("fixed artifact")
}
fn config_id() -> ObjectId {
    ObjectId::new("module.vless").expect("fixed object")
}
fn digest_entry(entry: &RegistryEntry, ctx: &OperationContext) -> Result<Sha256Digest> {
    serde_json::to_vec(entry)
        .map(|bytes| config::digest(&bytes))
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "module ownership digest unavailable",
            )
        })
}
fn purge_scope(
    app: &HostApplication,
    actor: &ActorId,
    operation: &OperationId,
    module: &ModuleId,
    registry_revision: StateRevision,
    entry: &Sha256Digest,
    revision: &ContentRevision,
    ctx: &OperationContext,
) -> Result<Sha256Digest> {
    serde_json::to_vec(&(
        "module-config-purge-v1",
        &app.installation.instance_id,
        actor,
        operation,
        module,
        registry_revision,
        entry,
        revision,
        artifact_id(),
    ))
    .map(|bytes| config::digest(&bytes))
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "module purge approval digest unavailable",
        )
    })
}
fn purge_snapshot(
    app: &HostApplication,
    ctx: &OperationContext,
) -> Result<lifecycle::LifecycleSnapshot> {
    Ok(lifecycle::LifecycleSnapshot {
        desired: Some(app.desired(ctx)?),
        registry: app.registry(ctx)?,
        runtime: app.runtime(ctx)?,
        observations: RuntimeObservation {
            resources: BTreeMap::new(),
        },
    })
}

fn purge_config(app: &HostApplication, module: &ModuleId, ctx: &OperationContext) -> Result<Value> {
    if module.as_str() != "vless" {
        return Err(journal::failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "module does not expose an owned retained-config purge target",
        ));
    }
    let pending = app.journal.unfinished(ctx)?;
    let same: Vec<_> = pending
        .iter()
        .filter(|transaction| {
            matches!(&transaction.intent,
        journal::Intent::ModulePurge { module_id, .. } if module_id == module)
        })
        .collect();
    if !same.is_empty() {
        if same.len() != 1
            || pending.iter().any(|other| {
                other.transaction_id != same[0].transaction_id
                    && !(other.operation_id == same[0].operation_id
                        && matches!(&other.intent, journal::Intent::Registry {
                        instance_id, change: RegistryChange::Purge { module_id }, ..
                    } if instance_id == &app.installation.instance_id && module_id == module))
            })
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "concurrent unfinished operations block exact module purge recovery",
            ));
        }
        if ctx.execution_mode == ExecutionMode::ReadOnly {
            return Ok(
                json!({"status":"recoveryRequired", "operation":"modules.purge",
                "moduleId":module, "writes":false, "transactionId":same[0].transaction_id}),
            );
        }
        for child in &pending {
            if child.transaction_id != same[0].transaction_id {
                reconcile_purge_registry_child(app, same[0], child, ctx)?;
            }
        }
        return finish_purge(app, same[0], ctx);
    }
    let snapshot = purge_snapshot(app, ctx)?;
    lifecycle::validate_resource_free_config_purge(&snapshot, module, &artifact_id(), ctx)?;
    if !pending.is_empty() {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "unfinished operations block retained configuration purge",
        ));
    }
    let modules = app.config.child("modules").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "retained module configuration inventory unavailable",
        )
    })?;
    let original = modules
        .read("vless.yaml", config::LoadLimits::default().file_bytes)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "retained module configuration unavailable",
            )
        })?
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ResourceDrift,
                "retained configuration disappeared without a purge intent",
            )
        })?;
    let revision = config::content_revision(config_id(), &original);
    let entry_digest = digest_entry(&snapshot.registry.entries[module], ctx)?;
    let scope = purge_scope(
        app,
        &ctx.actor.actor_id,
        &ctx.operation_id,
        module,
        snapshot.registry.revision,
        &entry_digest,
        &revision,
        ctx,
    )?;
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        return Ok(
            json!({"status":"preview", "operation":"modules.purge", "moduleId":module,
            "writes":false, "artifactId":artifact_id(), "revision":revision,
            "requiresLocalConfirmation":true}),
        );
    }
    let approval =
        super::approvals::authorize_module_purge(app, module, &artifact_id(), &scope, ctx)?;
    let tx = app.journal.prepare_intent(
        journal::Intent::ModulePurge {
            module_id: module.clone(),
            expected_registry_revision: snapshot.registry.revision,
            entry_digest,
            config_revision: revision,
            approval_digest: approval.digest().clone(),
        },
        vec![journal::ProtectedOriginal {
            artifact_id: artifact_id(),
            digest: config::digest(&original),
            was_present: true,
            bytes: original,
        }],
        ctx,
    )?;
    let transaction = app
        .journal
        .unfinished(ctx)?
        .into_iter()
        .find(|candidate| candidate.transaction_id == tx)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "module purge intent unavailable after preparation",
            )
        })?;
    finish_purge(app, &transaction, ctx)
}

fn finish_purge(
    app: &HostApplication,
    transaction: &journal::Transaction,
    ctx: &OperationContext,
) -> Result<Value> {
    journal::ensure_mutating(ctx, app.control.as_ref())?;
    if ctx.actor.source != ActorSource::Local
        || ctx.actor.actor_id.as_str() != "os.uid.0"
        || rustix::process::geteuid().as_raw() != 0
        || transaction.actor.source != ActorSource::Local
        || transaction.actor.actor_id.as_str() != "os.uid.0"
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "module purge recovery requires its original local root authority",
        ));
    }
    let journal::Intent::ModulePurge {
        module_id,
        expected_registry_revision,
        entry_digest,
        config_revision,
        approval_digest,
    } = &transaction.intent
    else {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "not a module purge intent",
        ));
    };
    let [original] = transaction.originals.as_slice() else {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "module purge originals are incomplete; retained",
        ));
    };
    if module_id.as_str() != "vless"
        || original.artifact_id != artifact_id()
        || !original.was_present
        || original.digest != config::digest(&original.bytes)
        || *config_revision != config::content_revision(config_id(), &original.bytes)
        || *approval_digest
            != purge_scope(
                app,
                &transaction.actor.actor_id,
                &transaction.operation_id,
                module_id,
                *expected_registry_revision,
                entry_digest,
                config_revision,
                ctx,
            )?
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "module purge evidence or approval is corrupt; originals retained",
        ));
    }
    let mut original_ctx = ctx.clone();
    original_ctx.operation_id = transaction.operation_id.clone();
    original_ctx.actor = transaction.actor.clone();
    let root_lock = app
        .config
        .lock(journal::deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "module purge configuration lock unavailable",
            )
        })?;
    let modules = app.config.child("modules").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "module configuration inventory unavailable",
        )
    })?;
    let module_lock = modules
        .lock(journal::deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "module purge artifact lock unavailable",
            )
        })?;
    let snapshot = purge_snapshot(app, ctx)?;
    lifecycle::validate_purge_participation(&snapshot, module_id, ctx)?;
    let current = module_lock
        .read("vless.yaml", config::LoadLimits::default().file_bytes)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "module purge artifact recheck unavailable",
            )
        })?;
    match snapshot.registry.entries.get(module_id) {
        Some(entry) => {
            lifecycle::validate_resource_free_config_purge(
                &snapshot,
                module_id,
                &artifact_id(),
                ctx,
            )?;
            if digest_entry(entry, ctx)? != *entry_digest {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "removed module was changed or re-added after purge approval",
                ));
            }
            match current.as_deref() {
                Some(bytes) if bytes == original.bytes.as_slice() => {
                    module_lock.remove("vless.yaml", bytes).map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "module purge deletion uncertain; originals retained",
                        )
                    })?;
                }
                None => (),
                _ => {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "retained module configuration changed after purge approval",
                    ));
                }
            }
            let desired = snapshot
                .desired
                .as_ref()
                .expect("purge snapshot includes desired");
            let root = desired
                .documents
                .iter()
                .find(|document| document.object_id.as_str() == "root")
                .ok_or_else(|| {
                    journal::failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "module purge root configuration unavailable",
                    )
                })?;
            if root_lock
                .read("hton.yaml", config::LoadLimits::default().file_bytes)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "module purge root recheck unavailable",
                    )
                })?
                .as_deref()
                != Some(root.source_bytes.as_slice())
            {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "module participation changed during purge; originals retained",
                ));
            }
            let preconditions = crate::registry::RegistryPreconditions {
                recovery_released: BTreeSet::from([module_id.clone()]),
                ..Default::default()
            };
            app.registry.transition(
                &snapshot.registry.instance_id,
                snapshot.registry.revision,
                RegistryChange::Purge {
                    module_id: module_id.clone(),
                },
                &preconditions,
                &original_ctx,
            )?;
        }
        None if current.is_none()
            && snapshot.registry.revision.0 > expected_registry_revision.0 =>
        {
            ()
        }
        _ => {
            return Err(journal::failure(
                ctx,
                ErrorCode::ResourceDrift,
                "module purge registry and retained artifact are inconsistent",
            ));
        }
    }
    let state = app.runtime(ctx)?;
    let result = OperationResult {
        operation_id: transaction.operation_id.clone(),
        status: OperationStatus::Succeeded,
        transaction_id: Some(transaction.transaction_id.clone()),
        desired_saved: true,
        pending_bundle: None,
        accepted_bundle: state
            .accepted
            .as_ref()
            .map(|id| BundleId::new(id.as_str()).expect("wire ID grammar")),
        run_intent: if state.run_intent == hton_runtime_contract::RunIntent::Running {
            RunIntent::Running
        } else {
            RunIntent::Stopped
        },
        member_results: vec![MemberResult {
            module_id: module_id.clone(),
            status: OperationStatus::Succeeded,
            resources: vec![],
            error: None,
        }],
        survivors: vec![],
        issues: vec![],
    };
    app.journal.finish_operation(
        &transaction.transaction_id,
        journal::TerminalStatus::Committed,
        vec![],
        &result,
        &original_ctx,
    )?;
    Ok(
        json!({"status":"succeeded", "operation":"modules.purge", "moduleId":module_id,
        "purgedArtifacts":[artifact_id()], "operationResult":result}),
    )
}

fn reconcile_purge_registry_child(
    app: &HostApplication,
    parent: &journal::Transaction,
    child: &journal::Transaction,
    ctx: &OperationContext,
) -> Result<()> {
    let journal::Intent::ModulePurge {
        module_id,
        entry_digest,
        ..
    } = &parent.intent
    else {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "purge parent is invalid",
        ));
    };
    let journal::Intent::Registry {
        instance_id,
        expected_revision,
        change: RegistryChange::Purge {
            module_id: child_module,
        },
    } = &child.intent
    else {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "unfinished operation is not the exact purge registry child",
        ));
    };
    if child.operation_id != parent.operation_id
        || child.actor != parent.actor
        || instance_id != &app.installation.instance_id
        || child_module != module_id
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ResourceDrift,
            "purge registry child authority mismatch",
        ));
    }
    let [original] = child.originals.as_slice() else {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "purge registry child originals incomplete",
        ));
    };
    let original_registry: crate::registry::RegistryDocument =
        serde_json::from_slice(&original.bytes).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "purge registry child originals corrupt",
            )
        })?;
    if !original.was_present
        || original.digest != config::digest(&original.bytes)
        || original_registry.snapshot.instance_id != *instance_id
        || original_registry.snapshot.revision != *expected_revision
        || original_registry
            .snapshot
            .entries
            .get(module_id)
            .map(|entry| digest_entry(entry, ctx))
            .transpose()?
            .as_ref()
            != Some(entry_digest)
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ResourceDrift,
            "purge registry child original ownership mismatch",
        ));
    }
    let registry = app.registry(ctx)?;
    let terminal = match registry.entries.get(module_id) {
        Some(entry) if digest_entry(entry, ctx)? == *entry_digest => {
            journal::TerminalStatus::RolledBack
        }
        None if registry.revision.0 > expected_revision.0 => {
            let modules = app.config.child("modules").map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "purge configuration inventory unavailable",
                )
            })?;
            if modules
                .read("vless.yaml", config::LoadLimits::default().file_bytes)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "purge configuration observation unavailable",
                    )
                })?
                .is_some()
            {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "purged module configuration was recreated",
                ));
            }
            journal::TerminalStatus::Committed
        }
        _ => {
            return Err(journal::failure(
                ctx,
                ErrorCode::ResourceDrift,
                "purge registry child cannot reconcile changed ownership",
            ));
        }
    };
    let mut original_ctx = ctx.clone();
    original_ctx.operation_id = child.operation_id.clone();
    original_ctx.actor = child.actor.clone();
    app.journal
        .finish(&child.transaction_id, terminal, vec![], &original_ctx)
}
