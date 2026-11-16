//! Journalled install coordinator: persisted input/allocation, owned setup, certificate and ingress.
use super::*;
use crate::config::ConfigSource;
use crate::journal::{Intent, TerminalStatus, deadline, ensure_mutating};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::{Ipv4Addr, SocketAddrV4},
    time::{Duration, Instant},
};

/// Read-only catalogue-backed install selection offer. The shell may display the
/// trusted choices, but it never discovers modules or edits desired config.
pub fn selection_offer(
    paths: &HostPaths,
    args: &[String],
    ctx: &OperationContext,
) -> Result<Value> {
    if ctx.execution_mode != ExecutionMode::ReadOnly {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "install selection offer is strictly read-only",
        ));
    }
    selection_offer_inner(paths, args, ctx)
}

fn selection_offer_inner(
    paths: &HostPaths,
    args: &[String],
    ctx: &OperationContext,
) -> Result<Value> {
    let mut profile = None;
    let mut menu = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--menu" if !menu => {
                menu = true;
                index += 1;
            }
            "--profile" if profile.is_none() => {
                let value = args.get(index + 1).ok_or_else(|| {
                    journal::failure(ctx, ErrorCode::OperationDenied, "install profile missing")
                })?;
                profile = Some(ProfileId::new(value).map_err(|_| {
                    journal::failure(ctx, ErrorCode::OperationDenied, "install profile invalid")
                })?);
                index += 2;
            }
            _ => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "invalid install selection-offer arguments",
                ));
            }
        }
    }
    paths.validate(false).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "untrusted installation paths",
        )
    })?;
    if has_pending_port_allocation(paths, ctx)? {
        return Ok(json!({
            "operation":"install.selection-offer",
            "status":"recoveryPending",
            "writes":false,
            "detail":"an exact VPN allocation transaction must be recovered before module selection"
        }));
    }
    let catalogue = crate::catalog::ModuleCatalogue::builtin().map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "trusted module catalogue unavailable",
        )
    })?;
    let suggestions = match profile.as_ref() {
        Some(profile) => catalogue.profile_suggestions(profile).ok_or_else(|| {
            journal::failure(ctx, ErrorCode::OperationDenied, "unknown install profile")
        })?,
        None => &[],
    };
    let mut saved = BTreeSet::<ModuleId>::new();
    let mut root_revision = None;
    let mut expected_root_absent = false;
    let mut recovered_allocation_selection = false;
    let source = match SecureDir::open(&paths.config) {
        Ok(_) => {
            let source = config::FsConfigSource::open(&paths.config).map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "configuration inventory unavailable or unsafe",
                )
            })?;
            let root_bytes = source
                .read(&["hton.yaml"], config::LoadLimits::default().file_bytes)
                .map_err(|_| {
                    journal::failure(ctx, ErrorCode::IoUnavailable, "root config unavailable")
                })?;
            if root_bytes.is_none() {
                let modules = source
                    .list_modules(config::LoadLimits::default().files)
                    .map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "module settings inventory unavailable",
                        )
                    })?;
                let docker = source
                    .list_docker(config::LoadLimits::default().files)
                    .map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "Docker settings inventory unavailable",
                        )
                    })?;
                if !modules.is_empty() || !docker.is_empty() {
                    let recovered = if docker.is_empty() {
                        rootless_allocation_selection(paths, &modules, ctx)?
                    } else {
                        None
                    };
                    match recovered {
                        Some(RootlessAllocationSelection::Pending) => {
                            return Ok(json!({
                                "operation":"install.selection-offer",
                                "status":"recoveryPending",
                                "writes":false,
                                "detail":"a journaled first-install port candidate is unfinished; recover its exact settings before module selection"
                            }));
                        }
                        Some(RootlessAllocationSelection::Committed { modules, .. }) => {
                            saved.extend(modules);
                            recovered_allocation_selection = true;
                        }
                        None => {
                            return Err(journal::failure(
                                ctx,
                                ErrorCode::ModuleIncomplete,
                                "orphan config documents exist without hton.yaml; repair before install selection",
                            ));
                        }
                    }
                }
                expected_root_absent = true;
                None
            } else {
                let loaded = config::load(&source, &BTreeSet::from([FormatVersion(1)]));
                if !loaded.errors.is_empty() {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "saved configuration is invalid; selection offer unavailable",
                    ));
                }
                let snapshot = loaded.snapshot.ok_or_else(|| {
                    journal::failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "coherent desired snapshot unavailable",
                    )
                })?;
                root_revision = loaded
                    .read_set
                    .objects
                    .get(&ObjectId::new("root").expect("fixed ID"))
                    .map(|dependency| dependency.revision.clone());
                if root_revision.is_none() || loaded.read_set.coherence != ReadCoherence::Coherent {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "root selection revision unavailable",
                    ));
                }
                saved.extend(snapshot.features.iter().filter_map(|(id, choice)| {
                    (choice.enabled && id.as_str() != "core").then_some(id.clone())
                }));
                Some(source)
            }
        }
        Err(hton_storage::Error::NotFound) => {
            expected_root_absent = true;
            None
        }
        Err(_) => {
            return Err(journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "configuration root unavailable or unsafe",
            ));
        }
    };
    let _ = source;
    let selectable: Vec<ModuleId> = catalogue
        .entries()
        .iter()
        .filter_map(|(id, entry)| {
            (id.as_str() != "core"
                && entry.implemented
                && entry.descriptor.phase == ReleasePhase::R1)
                .then_some(id.clone())
        })
        .collect();
    let suggested: Vec<ModuleId> = if expected_root_absent && recovered_allocation_selection {
        saved.iter().cloned().collect()
    } else if expected_root_absent {
        suggestions.to_vec()
    } else {
        Vec::new()
    };
    let catalogue_fingerprint = catalogue.fingerprint();
    let scope = serde_json::to_vec(&(
        catalogue.version(),
        &catalogue_fingerprint,
        &profile,
        &root_revision,
        expected_root_absent,
        &saved,
        &suggested,
        &selectable,
    ))
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "selection offer encoding failed",
        )
    })?;
    let offer_digest = config::digest(&scope);
    Ok(json!({
        "operation":"install.selection-offer",
        "status":"offer",
        "writes":false,
        "catalogueVersion":catalogue.version(),
        "catalogueFingerprint":catalogue_fingerprint,
        "offerDigest":offer_digest,
        "profile":profile,
        "profileIgnored":profile.is_some() && !expected_root_absent,
        "suggestedModuleIds":suggested,
        "savedModuleIds":saved,
        "selectableModules":selectable,
        "rootRevision":root_revision,
        "expectedRootAbsent":expected_root_absent,
        "menuRequested":menu
    }))
}

enum RootlessAllocationSelection {
    Pending,
    Committed {
        modules: BTreeSet<ModuleId>,
        candidate: PortAllocationCandidate,
        instance_id: InstanceId,
        transaction_id: TransactionId,
    },
}

/// Recognize the one narrow rootless state created when a first-install port
/// allocation CAS completed before root seeding. Every module document must be
/// named by one same-instance candidate; committed bytes must match exactly.
fn rootless_allocation_selection(
    paths: &HostPaths,
    module_files: &[String],
    ctx: &OperationContext,
) -> Result<Option<RootlessAllocationSelection>> {
    if module_files.is_empty() {
        return Ok(None);
    }
    let Some(work) = open_optional_private_dir(&paths.work, ctx)? else {
        return Ok(None);
    };
    let Some(config_dir) = open_optional_private_dir(&paths.config, ctx)? else {
        return Ok(None);
    };
    let installation = read_installation_record(Some(&work), ctx)?;
    let journal_directory = match work.child("journal") {
        Ok(directory) => Arc::new(directory),
        Err(hton_storage::Error::NotFound) => return Ok(None),
        Err(_) => {
            return Err(journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "rootless install journal is unsafe or unavailable",
            ));
        }
    };
    let store = JournalStore::new(journal_directory, Arc::new(SystemControl));
    let allocations = store
        .history(ctx)?
        .into_iter()
        .filter_map(|(transaction, terminal)| {
            let Intent::PortAllocation {
                candidate,
                instance_id: Some(instance_id),
            } = transaction.intent
            else {
                return None;
            };
            let same_instance = installation
                .as_ref()
                .is_none_or(|record| record.instance_id == instance_id);
            let active = terminal
                .as_ref()
                .is_none_or(|terminal| terminal.status == TerminalStatus::RecoveryRequired);
            let committed = terminal
                .as_ref()
                .is_some_and(|terminal| terminal.status == TerminalStatus::Committed);
            (same_instance && (active || committed)).then_some((
                candidate,
                instance_id,
                transaction.transaction_id,
                active,
                committed,
            ))
        })
        .collect::<Vec<_>>();
    if allocations.len() != 1 {
        return Ok(None);
    }
    let (candidate, instance_id, transaction_id, active, committed) = allocations
        .into_iter()
        .next()
        .expect("one candidate checked");
    let expected_files = candidate
        .allocations
        .values()
        .map(|allocation| {
            allocation
                .settings_document
                .as_str()
                .strip_prefix("module.")
                .map(|module| format!("{module}.yaml"))
                .ok_or_else(|| {
                    journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "allocation settings document identity is invalid",
                    )
                })
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let observed_files = module_files.iter().cloned().collect::<BTreeSet<_>>();
    if active {
        if !observed_files.is_subset(&expected_files) {
            return Ok(None);
        }
        return Ok(Some(RootlessAllocationSelection::Pending));
    }
    if !committed || expected_files != observed_files {
        return Ok(None);
    }
    let modules = expected_files
        .iter()
        .filter_map(|filename| filename.strip_suffix(".yaml"))
        .map(|id| {
            ModuleId::new(id).map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "allocation module settings identity is invalid",
                )
            })
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let catalogue = Arc::new(crate::catalog::ModuleCatalogue::builtin().map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "trusted catalogue unavailable for rootless allocation verification",
        )
    })?);
    let writer = crate::port_allocation::PortSettingsWriter::new(
        &config_dir,
        catalogue,
        Arc::new(store),
        instance_id,
        ctx,
    )?;
    if !writer.candidate_is_saved(&transaction_id, &candidate, ctx)? {
        return Ok(None);
    }
    Ok(Some(RootlessAllocationSelection::Committed {
        modules,
        candidate,
        instance_id,
        transaction_id,
    }))
}

pub fn reconcile(paths: &HostPaths, args: &[String], ctx: &OperationContext) -> Result<Value> {
    let parsed = parse_install_args(args).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "invalid installation input arguments",
        )
    })?;
    paths
        .validate(ctx.execution_mode == ExecutionMode::Mutating && !parsed.input_preflight)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "untrusted installation paths",
            )
        })?;
    if parsed.input_preflight && has_pending_port_allocation(paths, ctx)? {
        return Ok(json!({
            "status":"recoveryPending",
            "operation":"install.reconcile",
            "writes":false,
            "detail":"an approved VPN port allocation is unfinished; install will recover its exact journaled values before continuing"
        }));
    }
    if ctx.execution_mode == ExecutionMode::Mutating && !parsed.input_preflight {
        if let Some(recovered) = recover_pending_port_allocation(paths, ctx)? {
            return Ok(recovered);
        }
    }
    let selection = validate_selection_candidate(paths, &parsed, ctx)?;
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        if parsed.input_preflight {
            let result = input_preflight(paths, parsed.input.as_ref(), selection.as_ref(), ctx)?;
            return with_selection_candidate(result, selection, ctx);
        }
        return Ok(
            json!({"status":"preview","operation":"install.reconcile","writes":false,"nativeSetup":"unverified","publication":"disabled-until-configured"}),
        );
    }

    if parsed.input_preflight {
        let result = input_preflight(paths, parsed.input.as_ref(), selection.as_ref(), ctx)?;
        return with_selection_candidate(result, selection, ctx);
    }

    // Input discovery is deliberately before SecureDir::create, journal creation,
    // setup, or any other installation effect. The shell may then collect missing
    // values from a trusted TTY and retry this fixed operation once.
    let missing = missing_publication_inputs(paths, parsed.input.as_ref());
    if !missing.is_empty() {
        return Ok(input_required_result(paths, missing));
    }

    // Normal mutating reconciliation owns a fresh execution preflight. It does
    // not trust the earlier shell preview because reservations, config, registry
    // or runtime may have changed between calls.
    let install_plan = preflight_plan(paths, parsed.input.as_ref(), selection.as_ref(), ctx)?;
    if !install_plan.conflicts.is_empty() {
        return Ok(install_blocked_result(&install_plan));
    }
    if install_plan
        .approvals
        .iter()
        .any(|approval| matches!(approval, InstallApprovalRequirement::ForeignRemap { .. }))
        || !install_plan.foreign_bindings.is_empty()
        || install_plan
            .graph
            .enabled_modules
            .iter()
            .any(|module| module.as_str() == "foreign-amnezia")
    {
        return Ok(json!({
            "status":"blocked",
            "operation":"install.reconcile",
            "reason":"foreignExecutionUnavailable",
            "writes":false,
            "installPlan":install_plan,
            "detail":"qualified foreign snapshot/remap execution is not available; no host or foreign state was changed"
        }));
    }

    let root_id = ObjectId::new("root").expect("fixed object ID");
    let first_root = install_plan
        .expected_inputs
        .read_set
        .absent_objects
        .contains(&root_id);
    let has_non_allocation_approval = install_plan.approvals.iter().any(|approval| {
        !matches!(
            approval,
            InstallApprovalRequirement::AllocationChange { .. }
                | InstallApprovalRequirement::InputChange { .. }
        )
    });
    if !first_root && has_non_allocation_approval {
        return Ok(json!({
            "status":"blocked",
            "operation":"install.reconcile",
            "reason":"existingValueReviewUnavailable",
            "writes":false,
            "installPlan":install_plan,
            "detail":"changing saved site or module selections requires the exact install review path; existing values were retained"
        }));
    }
    if !first_root
        && selection
            .as_ref()
            .is_some_and(|candidate| candidate.selected_module_ids != candidate.previous_module_ids)
    {
        return Ok(json!({
            "status":"blocked",
            "operation":"install.reconcile",
            "reason":"existingSelectionReviewUnavailable",
            "writes":false,
            "installPlan":install_plan,
            "detail":"saved module selection was retained because the exact menu-change execution path is unavailable"
        }));
    }

    // A saved value is never direct first-install consent. Review the exact
    // candidate before allocations, metadata, source or configuration writes.
    let input_approval = if !first_root {
        match install_plan.input_candidate.as_ref() {
            Some(candidate)
                if candidate
                    .changes
                    .iter()
                    .any(|change| change.previous.is_some()) =>
            {
                let app = HostApplication::open(paths.clone(), ctx)?;
                let review = super::approvals::authorize_install_input(&app, candidate, ctx)?;
                Some(super::approvals::verify_install_input(
                    &app, candidate, &review, ctx,
                )?)
            }
            _ => None,
        }
    } else {
        None
    };

    let allocation_candidate = install_plan.port_allocation_candidate.clone();
    let allocation_needs_approval = allocation_candidate.as_ref().is_some_and(|candidate| {
        candidate.allocations.values().any(|allocation| {
            allocation
                .prior_fields
                .values()
                .any(|prior| prior.is_some_and(|prior| prior != allocation.port))
        })
    });
    let stores_ready =
        read_installation_record(open_optional_private_dir(&paths.work, ctx)?.as_ref(), ctx)?
            .is_some();
    let app_for_allocation = if allocation_needs_approval && stores_ready {
        Some(HostApplication::open(paths.clone(), ctx)?)
    } else {
        None
    };
    let allocation_approval = match allocation_candidate.as_ref() {
        Some(candidate) if allocation_needs_approval => {
            Some(if let Some(app) = &app_for_allocation {
                authorize_allocation(app, candidate, ctx)?
            } else {
                authorize_first_install_allocation(
                    &install_plan.instance_id,
                    candidate,
                    ctx,
                    &SystemControl,
                )?
            })
        }
        _ => None,
    };

    recheck_install_plan_inputs(paths, &install_plan, first_root, ctx)?;
    if rustix::process::geteuid().as_raw() != 0 {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "installation requires root or sudo",
        ));
    }
    let control: Arc<dyn OperationControl> = Arc::new(SystemControl);
    ensure_mutating(ctx, control.as_ref())?;
    let config = Arc::new(SecureDir::create(&paths.config).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "private config initialization failed",
        )
    })?);
    let work = Arc::new(SecureDir::create(&paths.work).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "private work initialization failed",
        )
    })?);
    for name in ["modules", "docker", "secrets", "backup-identity"] {
        config.create_child(name).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "private config inventory initialization failed",
            )
        })?;
    }
    for name in [
        "journal",
        "locks",
        "bundles",
        "runtime",
        "certificates",
        "backups",
        "identities",
        "docker-client",
        "setup",
    ] {
        work.create_child(name).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "private working inventory initialization failed",
            )
        })?;
    }
    let runtime = work.child("runtime").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "runtime directory unavailable",
        )
    })?;
    for name in [
        "authorities",
        "services",
        "effects",
        "collectors",
        "native-bindings",
    ] {
        runtime.create_child(name).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "runtime inventory initialization failed",
            )
        })?;
    }
    let journal =
        Arc::new(JournalStore::new(
            Arc::new(work.child("journal").map_err(|_| {
                journal::failure(ctx, ErrorCode::IoUnavailable, "journal unavailable")
            })?),
            control.clone(),
        ));
    // Journal and commit the exact allocation before publishing installation
    // identity or root config. A crash cannot leave a seeded installation that
    // has to redraw entropy on its next invocation.
    let allocation_receipt = if let Some(candidate) = allocation_candidate.as_ref() {
        recheck_install_allocation_before_commit(paths, &install_plan, ctx)?;
        let catalogue = Arc::new(crate::catalog::ModuleCatalogue::builtin().map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "trusted catalogue unavailable for VPN allocation commit",
            )
        })?);
        let writer = crate::port_allocation::PortSettingsWriter::new(
            &config,
            catalogue,
            journal.clone(),
            install_plan.instance_id.clone(),
            ctx,
        )?;
        Some(writer.commit(
            candidate,
            &candidate.read_set,
            allocation_approval.as_ref(),
            ctx,
        )?)
    } else {
        None
    };
    let request_digest = config::digest(&serde_json::to_vec(&("installation-v1", args)).map_err(
        |_| journal::failure(ctx, ErrorCode::ConfigSyntax, "installation request invalid"),
    )?);
    let tx = journal.prepare_intent(Intent::Installation { request_digest }, vec![], ctx)?;
    let record = {
        let locked = work.lock(deadline(ctx, control.as_ref())?).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "installation metadata lock unavailable",
            )
        })?;
        if let Some(bytes) = locked.read("installation.json", 16_384).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "installation metadata unavailable",
            )
        })? {
            let record: InstallationRecord = serde_json::from_slice(&bytes).map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "installation metadata corrupt; retained",
                )
            })?;
            if record.schema_version != 1 {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "unsupported installation metadata",
                ));
            }
            record
        } else {
            let instance_id = match config
                .read("hton.yaml", config::LoadLimits::default().file_bytes)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "existing root configuration unavailable",
                    )
                })? {
                Some(bytes) => {
                    let value = config::decode_yaml(&bytes, config::LoadLimits::default())
                        .map_err(|_| {
                            journal::failure(
                                ctx,
                                ErrorCode::ConfigSyntax,
                                "existing root is invalid; retained for repair",
                            )
                        })?;
                    let name = value
                        .get("instance")
                        .and_then(|value| value.get("name"))
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            journal::failure(
                                ctx,
                                ErrorCode::ConfigSyntax,
                                "existing instance logical name missing",
                            )
                        })?;
                    InstanceId::new(name).map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::ConfigSyntax,
                            "instance logical name must be a valid ID",
                        )
                    })?
                }
                None => install_plan.instance_id.clone(),
            };
            let record = InstallationRecord {
                schema_version: 1,
                instance_id,
            };
            let bytes = serde_json::to_vec(&record).map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "installation metadata encoding failed",
                )
            })?;
            locked
                .compare_exchange("installation.json", None, &bytes)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "installation identity publication uncertain",
                    )
                })?;
            record
        }
    };
    let root_exists = config
        .read("hton.yaml", config::LoadLimits::default().file_bytes)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "existing root configuration unavailable",
            )
        })?
        .is_some();
    let rootbytes = if root_exists {
        Vec::new()
    } else {
        let root_id = ObjectId::new("root").expect("fixed object ID");
        let selected_modules = install_plan
            .selection_candidate
            .as_ref()
            .map(|candidate| candidate.selected_module_ids.clone())
            .unwrap_or_default();
        let existing_module_ids = install_plan
            .expected_inputs
            .read_set
            .objects
            .keys()
            .filter_map(|object| object.as_str().strip_prefix("module."))
            .filter_map(|module| ModuleId::new(module).ok())
            .collect::<BTreeSet<_>>();
        let modules_to_seed = selected_modules
            .difference(&existing_module_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        let initial_read_set = install_plan.expected_inputs.read_set.clone();
        let catalogue = crate::catalog::ModuleCatalogue::builtin().map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "trusted module catalogue unavailable during bootstrap",
            )
        })?;
        let mut initial = config::initial_desired(
            &record.instance_id,
            &modules_to_seed,
            &initial_read_set,
            &catalogue,
        )
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "canonical initial configuration could not be composed",
            )
        })?;
        apply_selection_candidate(
            initial.snapshot.as_mut().ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "initial selection snapshot is unavailable",
                )
            })?,
            install_plan.selection_candidate.as_ref(),
            ctx,
        )?;
        let seeded_input = apply_input_candidate(&mut initial, parsed.input.as_ref(), ctx)?;
        if seeded_input
            .as_ref()
            .map(|candidate| &candidate.candidate_digest)
            != install_plan
                .input_candidate
                .as_ref()
                .map(|candidate| &candidate.candidate_digest)
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::PlanStale,
                "first-install publication input changed after preflight",
            ));
        }
        if initial
            .snapshot
            .as_ref()
            .is_none_or(|snapshot| snapshot.read_set != install_plan.expected_inputs.read_set)
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "first-install root candidate differs from its virtual read set",
            ));
        }
        let root = config::root_document(initial.snapshot.as_ref().ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "canonical initial configuration snapshot is unavailable",
            )
        })?)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "canonical initial root document is invalid",
            )
        })?;
        serde_yaml::to_string(&root)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "bootstrap configuration encoding failed",
                )
            })?
            .into_bytes()
    };
    seed(&config, "hton.yaml", &rootbytes, ctx, control.as_ref())?;
    let app = HostApplication::open(paths.clone(), ctx)?;
    // Recheck the exact source observation from preflight and seed only if absent,
    // before Caddy renders the captured body into its immutable response route.
    verify_site_index_basis(&install_plan, ctx)?;
    let observation = webroot::inspect_index(ctx)?;
    if site_index_basis(&observation) != install_plan.site_index {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "host site index changed before installation setup",
        ));
    }
    let source_index = webroot::ensure_index(ctx, &observation)?;
    let registry = app.registry(ctx)?;
    if !first_root {
        if let Some(candidate) = install_plan.input_candidate.as_ref() {
            let writer = crate::host::install_input_writer::InstallInputWriter::new(&app);
            if let Some(approval) = input_approval.as_ref() {
                writer.commit(candidate, &candidate.read_set, approval, ctx)?;
            } else {
                writer.commit_for_install(candidate, &candidate.read_set, ctx)?;
            }
        }
        if let Some(candidate) = install_plan.selection_candidate.as_ref() {
            let enable = candidate
                .selected_module_ids
                .difference(&candidate.previous_module_ids)
                .cloned()
                .map(|module| (module, true));
            let disable = candidate
                .previous_module_ids
                .difference(&candidate.selected_module_ids)
                .cloned()
                .map(|module| (module, false));
            let changes = enable.chain(disable).collect::<Vec<_>>();
            if !changes.is_empty() {
                let source = config::FsConfigSource::open(&paths.config).map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "install selection config source unavailable",
                    )
                })?;
                let writer =
                    crate::selection::SelectionWriter::new(app.config.clone(), app.journal.clone());
                let mut root_revision = app
                    .desired(ctx)?
                    .documents
                    .into_iter()
                    .find(|document| document.object_id.as_str() == "root")
                    .map(|document| document.revision)
                    .ok_or_else(|| {
                        journal::failure(
                            ctx,
                            ErrorCode::ConfigReadConflict,
                            "install selection root revision unavailable",
                        )
                    })?;
                for (module, enabled) in changes {
                    let selection = crate::selection::prepare(
                        &source,
                        BTreeSet::from([module]),
                        enabled,
                        &root_revision,
                        ctx,
                    )?;
                    let committed = writer.commit(&selection, &root_revision, ctx)?;
                    root_revision = committed.revision;
                }
            }
        }
    }
    let core = ModuleId::new("core").expect("fixed ID");
    if !registry.entries.contains_key(&core) {
        app.registry.transition(
            &record.instance_id,
            registry.revision,
            RegistryChange::Install {
                entry: RegistryEntry {
                    module_id: core,
                    ownership: Ownership::Owned,
                    membership: MembershipState::Installed,
                    resources: vec![],
                    setup_state: SetupState::Incomplete {
                        blocked_step: None,
                        required_operator_action: None,
                    },
                    checkpoints: vec![],
                    installed_artifact_versions: BTreeMap::new(),
                    original_policies: BTreeMap::new(),
                    adoption_scope: None,
                    retained_artifacts: vec![],
                },
            },
            &Default::default(),
            ctx,
        )?;
    }
    let runtime_metadata = serde_json::to_vec(&record).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "runtime identity encoding failed",
        )
    })?;
    {
        let locked = runtime
            .lock(deadline(ctx, control.as_ref())?)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "runtime identity lock unavailable",
                )
            })?;
        let old = locked.read("instance.json", 16384).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "runtime identity unavailable",
            )
        })?;
        if old.as_ref().is_some_and(|old| old != &runtime_metadata) {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "runtime bootstrap identity differs from installation; retained",
            ));
        }
        locked
            .compare_exchange("instance.json", old.as_deref(), &runtime_metadata)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "runtime identity durability uncertain",
                )
            })?;
    }
    super::approvals::initialize(&app, &tx, ctx)?;
    let _logs = super::logging::project(&app, &tx, ctx)?;
    super::systemd_installation::install(&app, ctx)?;
    let setup = super::setup::complete_core(&app, ctx)?;
    let mut member_results = vec![MemberResult {
        module_id: setup.module_id.clone(),
        status: if setup.complete {
            OperationStatus::Succeeded
        } else {
            OperationStatus::Partial
        },
        resources: setup
            .receipts
            .iter()
            .flat_map(|receipt| {
                receipt
                    .resources
                    .iter()
                    .map(|resource| resource.resource_id.clone())
            })
            .collect(),
        error: setup.error.clone(),
    }];
    let mut selected_setup_complete = true;
    for module_id in install_plan
        .graph
        .enabled_modules
        .iter()
        .filter(|id| id.as_str() != "core")
    {
        match super::setup::add_module(&app, module_id, ctx) {
            Ok(module_setup) => {
                selected_setup_complete &= module_setup.complete;
                member_results.push(MemberResult {
                    module_id: module_setup.module_id,
                    status: if module_setup.complete {
                        OperationStatus::Succeeded
                    } else {
                        OperationStatus::Partial
                    },
                    resources: module_setup
                        .receipts
                        .iter()
                        .flat_map(|receipt| {
                            receipt
                                .resources
                                .iter()
                                .map(|resource| resource.resource_id.clone())
                        })
                        .collect(),
                    error: module_setup.error,
                });
            }
            Err(error) => {
                selected_setup_complete = false;
                member_results.push(MemberResult {
                    module_id: module_id.clone(),
                    status: OperationStatus::Partial,
                    resources: Vec::new(),
                    error: Some(error),
                });
            }
        }
    }
    let observed = app.runtime(ctx)?;
    let run_intent = match observed.run_intent {
        hton_runtime_contract::RunIntent::Running => RunIntent::Running,
        hton_runtime_contract::RunIntent::Stopped => RunIntent::Stopped,
    };
    let accepted = observed
        .accepted
        .as_ref()
        .map(|id| BundleId::new(id.as_str()))
        .transpose()
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "accepted identity invalid",
            )
        })?;
    let mut result = OperationResult {
        operation_id: ctx.operation_id.clone(),
        // Terminal success requires both the trusted site and qualified native
        // Amnezia route, in addition to certificate delivery and ingress apply.
        status: OperationStatus::Partial,
        transaction_id: Some(tx.clone()),
        desired_saved: true,
        pending_bundle: None,
        accepted_bundle: accepted,
        run_intent,
        member_results,
        survivors: vec![],
        issues: vec![],
    };
    // This bootstrap persists desired values/allocations and completes only the
    // selected native setup owners. It never labels ingress ready without the
    // certificate, admitted apply and local served-site verification phases.
    if !setup.complete || !selected_setup_complete {
        journal.finish_operation(&tx, TerminalStatus::Partial, vec![], &result, ctx)?;
        return Ok(json!({
            "status":"partial",
            "operation":"install.reconcile",
            "reason":"selectedSetupIncomplete",
            "instanceId":record.instance_id,
            "desiredSaved":true,
            "writes":true,
            "publication":"not-changed",
            "allocation":allocation_receipt,
            "ready":false,
            "operationResult":result
        }));
    }
    if !installation_dns_credentials_available(&app, ctx)? {
        journal.finish_operation(&tx, TerminalStatus::Partial, vec![], &result, ctx)?;
        return Ok(json!({
            "status":"partial",
            "operation":"install.reconcile",
            "reason":"certificateCredentialsRequired",
            "instanceId":record.instance_id,
            "desiredSaved":true,
            "writes":true,
            "publication":"not-changed",
            "allocation":allocation_receipt,
            "ready":false,
            "requiredNext":["importInternetbsDnsCredentials"],
            "operationResult":result
        }));
    }

    let certificate = match crate::host_crypto::application::handle(&app, "certs.issue", &[], ctx) {
        Ok(result) if result.get("status").and_then(Value::as_str) == Some("succeeded") => result,
        Ok(certificate_result) => {
            journal.finish_operation(&tx, TerminalStatus::Partial, vec![], &result, ctx)?;
            return Ok(json!({
                "status":"partial",
                "operation":"install.reconcile",
                "reason":"certificateIssueIncomplete",
                "instanceId":record.instance_id,
                "desiredSaved":true,
                "writes":true,
                "publication":"not-changed",
                "allocation":allocation_receipt,
                "certificate":certificate_result,
                "ready":false,
                "operationResult":result
            }));
        }
        Err(error) => {
            journal.finish_operation(&tx, TerminalStatus::Partial, vec![], &result, ctx)?;
            return Ok(json!({
                "status":"partial",
                "operation":"install.reconcile",
                "reason":"certificateIssueFailed",
                "instanceId":record.instance_id,
                "desiredSaved":true,
                "writes":true,
                "publication":"not-changed",
                "allocation":allocation_receipt,
                "error":error,
                "ready":false,
                "operationResult":result
            }));
        }
    };
    let source_unchanged_before_render = matches!(
        webroot::inspect_index(ctx)?,
        webroot::SiteIndexObservation::Present(ref index)
            if index.identity_digest() == source_index.identity_digest()
                && index.content_digest() == source_index.content_digest()
                && index.bytes() == source_index.bytes()
    );
    if !source_unchanged_before_render {
        journal.finish_operation(&tx, TerminalStatus::Partial, vec![], &result, ctx)?;
        return Ok(json!({
            "status":"partial",
            "operation":"install.reconcile",
            "reason":"siteSourceChanged",
            "instanceId":record.instance_id,
            "desiredSaved":true,
            "writes":true,
            "publication":"not-changed",
            "allocation":allocation_receipt,
            "certificate":certificate,
            "ready":false,
            "operationResult":result
        }));
    }
    let effect = match effects::prepare(&app, ctx) {
        Ok(effect) => effect,
        Err(error) => {
            journal.finish_operation(&tx, TerminalStatus::Partial, vec![], &result, ctx)?;
            return Ok(json!({
                "status":"partial",
                "operation":"install.reconcile",
                "reason":"ingressPreflightFailed",
                "instanceId":record.instance_id,
                "desiredSaved":true,
                "writes":true,
                "publication":"not-changed",
                "allocation":allocation_receipt,
                "certificate":certificate,
                "error":error,
                "ready":false,
                "operationResult":result
            }));
        }
    };
    let applied = match effects::apply(&app, &effect, ctx) {
        Ok(applied) => applied,
        Err(error) => {
            journal.finish_operation(&tx, TerminalStatus::Partial, vec![], &result, ctx)?;
            return Ok(json!({
                "status":"partial",
                "operation":"install.reconcile",
                "reason":"ingressApplyIncomplete",
                "instanceId":record.instance_id,
                "desiredSaved":true,
                "writes":true,
                "publication":"unverified",
                "allocation":allocation_receipt,
                "certificate":certificate,
                "error":error,
                "ready":false,
                "operationResult":result
            }));
        }
    };
    // Preserve the admitted runtime outcome even if apply was partial or a later
    // served-route probe fails. The enclosing install keeps its own transaction.
    result.pending_bundle = applied.pending_bundle.clone();
    result.accepted_bundle = applied.accepted_bundle.clone();
    result.run_intent = applied.run_intent.clone();
    result.member_results.extend(applied.member_results.clone());
    result.survivors = applied.survivors.clone();
    result.issues.extend(applied.issues.clone());
    if applied.status != OperationStatus::Succeeded {
        journal.finish_operation(&tx, TerminalStatus::Partial, vec![], &result, ctx)?;
        return Ok(json!({
            "status":"partial",
            "operation":"install.reconcile",
            "reason":"ingressApplyPartial",
            "instanceId":record.instance_id,
            "desiredSaved":true,
            "writes":true,
            "publication":"applied-unverified",
            "allocation":allocation_receipt,
            "certificate":certificate,
            "ingressResult":applied,
            "servedState":"unverified",
            "ready":false,
            "operationResult":result
        }));
    }
    let served = verify_local_https_site(&app, &source_index, ctx).unwrap_or(false);
    if !served {
        journal.finish_operation(&tx, TerminalStatus::Partial, vec![], &result, ctx)?;
        return Ok(json!({
            "status":"partial",
            "operation":"install.reconcile",
            "reason":"servedHttpsRouteUnverified",
            "instanceId":record.instance_id,
            "desiredSaved":true,
            "writes":true,
            "publication":"applied-unverified",
            "allocation":allocation_receipt,
            "certificate":certificate,
            "ingressResult":applied,
            "servedState":"unverified",
            "ready":false,
            "operationResult":result
        }));
    }
    // The site probe is not evidence that native Amnezia traffic reaches its
    // qualified backend. Foreign execution currently fails closed before effects,
    // so a site-only success must remain partial until both SNI paths and the
    // native Amnezia handshake have a qualified verifier.
    journal.finish_operation(&tx, TerminalStatus::Partial, vec![], &result, ctx)?;
    Ok(json!({
        "status":"partial",
        "operation":"install.reconcile",
        "reason":"amneziaRouteUnverified",
        "instanceId":record.instance_id,
        "desiredSaved":true,
        "writes":true,
        "publication":"site-applied-amnezia-unverified",
        "allocation":allocation_receipt,
        "certificate":certificate,
        "ingressResult":applied,
        "siteReady":true,
        "amneziaRoute":"unverified",
        "externalDns":"unverified",
        "ready":false,
        "operationResult":result
    }))
}

fn install_blocked_result(plan: &InstallPlan) -> Value {
    json!({
        "status":"blocked",
        "operation":"install.reconcile",
        "reason":"installPreflightBlocked",
        "writes":false,
        "installPlan":plan,
        "detail":"install preflight found unresolved dependency, listener, allocation or host-evidence issues; no installation effects were started"
    })
}

fn site_index_basis(observation: &webroot::SiteIndexObservation) -> InstallSiteIndexBasis {
    match observation {
        webroot::SiteIndexObservation::Absent { identity_digest } => {
            InstallSiteIndexBasis::Absent {
                identity_digest: identity_digest.clone(),
            }
        }
        webroot::SiteIndexObservation::Present(index) => InstallSiteIndexBasis::Present {
            content_digest: index.content_digest().clone(),
            identity_digest: index.identity_digest(),
        },
    }
}

fn verify_site_index_basis(plan: &InstallPlan, ctx: &OperationContext) -> Result<()> {
    let current = webroot::inspect_index(ctx)?;
    if site_index_basis(&current) != plan.site_index {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "host site index changed after install preflight",
        ));
    }
    Ok(())
}

struct InstallProbeCancellation<'a> {
    app: &'a HostApplication,
    ctx: &'a OperationContext,
}
impl hton_host::Cancellation for InstallProbeCancellation<'_> {
    fn cancelled(&self) -> bool {
        self.app.control.is_cancelled(&self.ctx.cancellation)
    }
}

/// Require trusted local TLS for the configured SNI name and the exact shipped
/// site body through the local public listener. This does not assert public DNS.
fn verify_local_https_site(
    app: &HostApplication,
    expected_source: &webroot::SiteIndex,
    ctx: &OperationContext,
) -> Result<bool> {
    let source = config::FsConfigSource::open(&app.paths.config).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "site probe config unavailable",
        )
    })?;
    let desired = config::load(&source, &BTreeSet::from([FormatVersion(1)]));
    if !desired.errors.is_empty() {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "site probe config is invalid",
        ));
    }
    let snapshot = desired.snapshot.ok_or_else(|| {
        journal::failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "site probe config unavailable",
        )
    })?;
    let domain = snapshot
        .platform
        .settings
        .get("core")
        .and_then(Value::as_object)
        .and_then(|core| core.get("edge"))
        .and_then(Value::as_object)
        .and_then(|edge| edge.get("domains"))
        .and_then(Value::as_array)
        .and_then(|domains| domains.first())
        .and_then(Value::as_str)
        .filter(|domain| valid_site_domain(domain))
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "site domain unavailable",
            )
        })?;
    let source_before = match webroot::inspect_index(ctx)? {
        webroot::SiteIndexObservation::Present(index)
            if index.identity_digest() == expected_source.identity_digest()
                && index.content_digest() == expected_source.content_digest() =>
        {
            index
        }
        _ => return Ok(false),
    };
    if source_before.bytes() != expected_source.bytes() {
        return Ok(false);
    }
    let remaining = ctx
        .deadline
        .0
        .saturating_sub(app.control.now_ms())
        .min(10_000);
    if remaining == 0 {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDeadline,
            "site probe deadline elapsed",
        ));
    }
    let budget = hton_host::Budget::new(
        Instant::now() + Duration::from_millis(remaining),
        1024 * 1024 + 1024,
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::OperationDeadline,
            "site probe budget invalid",
        )
    })?;
    let cancel = InstallProbeCancellation { app, ctx };
    let reply = hton_host::HostObserver::new().probes().http(
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, 443),
        Some(domain),
        None,
        budget,
        &cancel,
    );
    let Ok(reply) = reply else { return Ok(false) };
    let source_after = match webroot::inspect_index(ctx)? {
        webroot::SiteIndexObservation::Present(index)
            if index.identity_digest() == source_before.identity_digest()
                && index.content_digest() == source_before.content_digest() =>
        {
            index
        }
        _ => return Ok(false),
    };
    Ok(reply.status == 200
        && reply.body == source_before.bytes()
        && source_after.identity_digest() == source_before.identity_digest()
        && source_after.content_digest() == source_before.content_digest()
        && source_after.bytes() == source_before.bytes())
}

/// Recheck the exact preflight basis after all operation-scoped TTY approval and
/// before creating bootstrap state. The point-in-time reservation digest is
/// refreshed; a stale plan cannot silently draw replacement entropy here.
fn recheck_install_plan_inputs(
    paths: &HostPaths,
    plan: &InstallPlan,
    first_root: bool,
    ctx: &OperationContext,
) -> Result<()> {
    verify_site_index_basis(plan, ctx)?;
    let control = SystemControl;
    let now = UnixMillis(control.now_ms());
    if plan.created_at.0 > now.0 || plan.expires_at.0 <= now.0 {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanExpired,
            "install plan expired before bootstrap",
        ));
    }
    if first_root {
        verify_first_install_inventory_absent(paths, ctx)?;
        recheck_first_install_config_readset(paths, &plan.expected_inputs.read_set, ctx)?;
        if plan
            .expected_inputs
            .read_set
            .objects
            .keys()
            .any(|object| object.as_str().starts_with("module."))
        {
            verify_rootless_allocation_work_inventory(paths, ctx)?;
        } else {
            verify_first_install_work_inventory(paths, ctx)?;
        }
        if !plan
            .expected_inputs
            .read_set
            .absent_objects
            .contains(&ObjectId::new("root").expect("fixed object ID"))
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::PlanStale,
                "first-install plan no longer binds absent root config",
            ));
        }
    } else {
        let source = config::FsConfigSource::open(&paths.config).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "install configuration unavailable during execution recheck",
            )
        })?;
        let desired = config::load(&source, &BTreeSet::from([FormatVersion(1)]));
        if !desired.errors.is_empty()
            || desired.read_set.coherence != ReadCoherence::Coherent
            || desired.read_set != plan.expected_inputs.read_set
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::PlanStale,
                "saved configuration changed after install preflight",
            ));
        }
    }

    let mut read_ctx = ctx.clone();
    read_ctx.execution_mode = ExecutionMode::ReadOnly;
    let work = open_optional_private_dir(&paths.work, &read_ctx)?;
    let observed_registry = match work.as_ref() {
        Some(directory) => crate::registry::RegistryStore::inspect(directory, &read_ctx)?,
        None => None,
    };
    let registry = observed_registry.unwrap_or_else(|| RegistrySnapshot {
        instance_id: plan.instance_id.clone(),
        revision: StateRevision(0),
        entries: BTreeMap::new(),
        diagnostics: Vec::new(),
        recovery_originals: Vec::new(),
    });
    if registry.instance_id != plan.instance_id
        || registry.revision != plan.expected_inputs.registry_revision
        || registry.diagnostics.len() > 0
        || registry_resource_fingerprints(&registry, ctx)? != plan.expected_inputs.resources
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "registry or resource fingerprints changed after install preflight",
        ));
    }
    if let Some(record) = read_installation_record(work.as_ref(), ctx)?
        && record.instance_id != plan.instance_id
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "installation identity changed after preflight",
        ));
    }
    let runtime_observed = read_runtime_for_preflight(work.as_ref(), &read_ctx)?;
    let runtime = runtime_observed.unwrap_or_else(|| {
        hton_runtime_contract::initial_state(
            hton_runtime_contract::InstanceId::new(plan.instance_id.as_str())
                .expect("Core and Runtime instance ID grammars match"),
        )
    });
    if runtime.instance_id.as_str() != plan.instance_id.as_str()
        || runtime.revision != plan.expected_inputs.runtime_revision.0
        || RunEpoch(runtime.run_epoch) != plan.expected_inputs.run_epoch
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "runtime state changed after install preflight",
        ));
    }
    recheck_install_reservations(
        &registry,
        plan,
        plan.port_allocation_candidate.as_ref(),
        &control,
        ctx,
    )
}

fn recheck_first_install_config_readset(
    paths: &HostPaths,
    read_set: &ReadSet,
    ctx: &OperationContext,
) -> Result<()> {
    let source = config::FsConfigSource::open(&paths.config).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "first-install config unavailable",
        )
    })?;
    for (object_id, dependency) in &read_set.objects {
        let components = match object_id.as_str() {
            "root" => vec!["hton.yaml".to_owned()],
            value => match value.strip_prefix("module.") {
                Some(module) => vec!["modules".to_owned(), format!("{module}.yaml")],
                None => {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "first-install read set contains an unsupported object",
                    ));
                }
            },
        };
        let refs = components.iter().map(String::as_str).collect::<Vec<_>>();
        let bytes = source
            .read(&refs, config::LoadLimits::default().file_bytes)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "first-install object read failed",
                )
            })?
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "first-install object disappeared after preflight",
                )
            })?;
        if config::content_revision(object_id.clone(), &bytes) != dependency.revision {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "first-install object revision changed after preflight",
            ));
        }
    }
    for object_id in &read_set.absent_objects {
        let components = match object_id.as_str() {
            "root" => vec!["hton.yaml".to_owned()],
            value => match value.strip_prefix("module.") {
                Some(module) => vec!["modules".to_owned(), format!("{module}.yaml")],
                None => {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "first-install absence set contains an unsupported object",
                    ));
                }
            },
        };
        let refs = components.iter().map(String::as_str).collect::<Vec<_>>();
        if source
            .read(&refs, config::LoadLimits::default().file_bytes)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "first-install absence check failed",
                )
            })?
            .is_some()
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "first-install absent object appeared after preflight",
            ));
        }
    }
    Ok(())
}

fn registry_resource_fingerprints(
    registry: &RegistrySnapshot,
    ctx: &OperationContext,
) -> Result<BTreeMap<ResourceId, Sha256Digest>> {
    let mut resources = BTreeMap::new();
    for entry in registry.entries.values() {
        for resource in &entry.resources {
            if resources
                .insert(resource.resource_id.clone(), resource.fingerprint.clone())
                .is_some_and(|previous| previous != resource.fingerprint)
            {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "registry contains conflicting resource fingerprints",
                ));
            }
        }
    }
    Ok(resources)
}

fn recheck_install_reservations(
    registry: &RegistrySnapshot,
    plan: &InstallPlan,
    candidate: Option<&PortAllocationCandidate>,
    control: &dyn OperationControl,
    ctx: &OperationContext,
) -> Result<()> {
    let evidence = observations::reservation_evidence_for(registry, control, ctx)?;
    let now = UnixMillis(control.now_ms());
    if !evidence.complete
        || evidence.observed_at.0 > now.0
        || evidence.valid_until.0 <= now.0
        || evidence.digest != plan.host_reservation_evidence.digest
        || candidate.is_some_and(|candidate| {
            candidate.reservation_digest != evidence.digest
                || candidate.expires_at.0 <= now.0
                || candidate.registry_revision != registry.revision
        })
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "host TCP/UDP reservations changed or expired after install review",
        ));
    }
    Ok(())
}

/// Last exact-basis check immediately before the allocation writer journals its
/// candidate. First-root scaffolding directories are permitted, while any
/// configuration documents, registry/runtime state, or listener drift blocks.
fn recheck_install_allocation_before_commit(
    paths: &HostPaths,
    plan: &InstallPlan,
    ctx: &OperationContext,
) -> Result<()> {
    verify_site_index_basis(plan, ctx)?;
    let first_root = plan
        .expected_inputs
        .read_set
        .absent_objects
        .contains(&ObjectId::new("root").expect("fixed object ID"));
    if !first_root {
        return recheck_install_plan_inputs(paths, plan, false, ctx);
    }
    verify_first_install_inventory_absent(paths, ctx)?;
    let control = SystemControl;
    let mut read_ctx = ctx.clone();
    read_ctx.execution_mode = ExecutionMode::ReadOnly;
    let work = open_optional_private_dir(&paths.work, &read_ctx)?;
    let observed_registry = match work.as_ref() {
        Some(directory) => crate::registry::RegistryStore::inspect(directory, &read_ctx)?,
        None => None,
    };
    let registry = observed_registry.unwrap_or_else(|| RegistrySnapshot {
        instance_id: plan.instance_id.clone(),
        revision: StateRevision(0),
        entries: BTreeMap::new(),
        diagnostics: Vec::new(),
        recovery_originals: Vec::new(),
    });
    if registry.instance_id != plan.instance_id
        || registry.revision != plan.expected_inputs.registry_revision
        || !registry.diagnostics.is_empty()
        || registry_resource_fingerprints(&registry, ctx)? != plan.expected_inputs.resources
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "registry changed before first VPN port assignment",
        ));
    }
    let runtime = read_runtime_for_preflight(work.as_ref(), &read_ctx)?.unwrap_or_else(|| {
        hton_runtime_contract::initial_state(
            hton_runtime_contract::InstanceId::new(plan.instance_id.as_str())
                .expect("Core and Runtime instance ID grammars match"),
        )
    });
    if runtime.instance_id.as_str() != plan.instance_id.as_str()
        || runtime.revision != plan.expected_inputs.runtime_revision.0
        || RunEpoch(runtime.run_epoch) != plan.expected_inputs.run_epoch
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "runtime changed before first VPN port assignment",
        ));
    }
    recheck_install_reservations(
        &registry,
        plan,
        plan.port_allocation_candidate.as_ref(),
        &control,
        ctx,
    )
}

fn recheck_install_allocation_evidence(
    app: &HostApplication,
    plan: &InstallPlan,
    candidate: &PortAllocationCandidate,
    ctx: &OperationContext,
) -> Result<()> {
    let registry = app.registry(ctx)?;
    if registry.revision != plan.expected_inputs.registry_revision
        || registry.instance_id != plan.instance_id
        || registry_resource_fingerprints(&registry, ctx)? != plan.expected_inputs.resources
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "registry changed before install port settings commit",
        ));
    }
    recheck_install_reservations(&registry, plan, Some(candidate), app.control.as_ref(), ctx)
}

fn verify_first_install_work_inventory(paths: &HostPaths, ctx: &OperationContext) -> Result<()> {
    let directory = match SecureDir::open_private(&paths.work) {
        Ok(directory) => directory,
        Err(hton_storage::Error::NotFound) => return Ok(()),
        Err(_) => {
            return Err(journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "first-install work inventory became unsafe or unavailable",
            ));
        }
    };
    let names = directory.list(64).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "first-install work inventory could not be checked",
        )
    })?;
    if names.iter().any(|name| name != "distribution.lock")
        || (names.iter().any(|name| name == "distribution.lock")
            && directory
                .read("distribution.lock", 16_384)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "distribution lock inventory is unsafe",
                    )
                })?
                .is_none())
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "first-install work inventory contains managed or unknown state",
        ));
    }
    Ok(())
}

fn verify_rootless_allocation_work_inventory(
    paths: &HostPaths,
    ctx: &OperationContext,
) -> Result<()> {
    let directory = SecureDir::open_private(&paths.work).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "rootless allocation work state unavailable",
        )
    })?;
    let allowed = BTreeSet::from([
        "distribution.lock",
        "journal",
        "locks",
        "bundles",
        "runtime",
        "certificates",
        "backups",
        "identities",
        "docker-client",
        "setup",
        "installation.json",
    ]);
    let names = directory.list(64).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "rootless allocation work inventory unavailable",
        )
    })?;
    if names.iter().any(|name| !allowed.contains(name.as_str())) {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "rootless allocation work inventory contains unknown state",
        ));
    }
    Ok(())
}

fn installation_dns_credentials_available(
    app: &HostApplication,
    ctx: &OperationContext,
) -> Result<bool> {
    let desired = app.desired(ctx)?;
    if !desired.errors.is_empty() || desired.read_set.coherence != ReadCoherence::Coherent {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "certificate settings are not readable as one coherent snapshot",
        ));
    }
    let certificates = desired
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.platform.settings.get("core"))
        .and_then(|core| core.get("certificates"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ModuleIncomplete,
                "certificate issuer settings are unavailable",
            )
        })?;
    for field in ["apiKey", "apiPassword"] {
        let reference = certificates
            .get("credentials")
            .and_then(|credentials| credentials.get(field))
            .and_then(|credential| credential.get("secretRef"))
            .and_then(Value::as_str)
            .and_then(config::canonical_secret_id)
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "InternetBS SecretRef is missing or invalid",
                )
            })?;
        if app.secrets.current_version(&reference, ctx)?.is_none() {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Complete an already approved exact port-settings transaction before any
/// retry can compute a new preflight candidate. The settings writer reconciles
/// only protected expected-old/expected-new bytes from its journal intent.
fn recover_pending_port_allocation(
    paths: &HostPaths,
    ctx: &OperationContext,
) -> Result<Option<Value>> {
    if ctx.execution_mode != ExecutionMode::Mutating {
        return Ok(None);
    }
    let pending = pending_port_allocation_intents(paths, ctx)?;
    if pending.is_empty() {
        return Ok(None);
    }

    let work = SecureDir::open_private(&paths.work).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "port recovery work state unavailable",
        )
    })?;
    let config_dir = SecureDir::open_private(&paths.config).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "port recovery config unavailable",
        )
    })?;
    let journal_store = Arc::new(JournalStore::new(
        Arc::new(work.child("journal").map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "port recovery journal unavailable",
            )
        })?),
        Arc::new(SystemControl),
    ));
    let unfinished = journal_store.unfinished(ctx)?;
    let pending = unfinished
        .iter()
        .filter(|transaction| matches!(&transaction.intent, Intent::PortAllocation { .. }))
        .cloned()
        .collect::<Vec<_>>();
    if pending.is_empty() {
        return Ok(None);
    }
    if pending.len() != 1 {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "multiple unfinished VPN port allocations require explicit reconciliation",
        ));
    }
    let candidate_digest = match &pending[0].intent {
        Intent::PortAllocation { candidate, .. } => &candidate.candidate_digest,
        _ => unreachable!("pending allocation inventory guarantees the intent kind"),
    };
    if let Some(parent) = unfinished.iter().find(|transaction| {
        matches!(
            &transaction.intent,
            Intent::Lifecycle { plan }
                if plan.port_allocation_candidate.as_ref().is_some_and(|candidate| {
                    &candidate.candidate_digest == candidate_digest
                })
        )
    }) {
        return Ok(Some(json!({
            "status":"partial",
            "operation":"install.reconcile",
            "reason":"allocationRecoveryOwnedByLifecycle",
            "writes":false,
            "lifecycleTransactionId":parent.transaction_id.clone(),
            "detail":"this VPN allocation belongs to an unfinished lifecycle transaction; resume lifecycle reconciliation before install"
        })));
    }

    let catalogue = Arc::new(crate::catalog::ModuleCatalogue::builtin().map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "trusted module catalogue unavailable for port recovery",
        )
    })?);
    let instance_id = match &pending[0].intent {
        Intent::PortAllocation { instance_id, .. } => instance_id
            .clone()
            .or(read_installation_record(Some(&work), ctx)?.map(|record| record.instance_id))
            .unwrap_or_else(|| InstanceId::new("port-recovery").expect("fixed ID")),
        _ => unreachable!("pending allocation inventory guarantees the intent kind"),
    };
    let writer = crate::port_allocation::PortSettingsWriter::new(
        &config_dir,
        catalogue,
        journal_store,
        instance_id,
        ctx,
    )?;
    let receipt = writer.recover(&pending[0].transaction_id, ctx)?;
    Ok(Some(json!({
        "status":"partial",
        "operation":"install.reconcile",
        "reason":"portAllocationRecovered",
        "writes":true,
        "publication":"not-changed",
        "portAllocation":receipt,
        "detail":"the exact previously approved VPN ports were recovered and saved; rerun install to continue"
    })))
}

fn has_pending_port_allocation(paths: &HostPaths, ctx: &OperationContext) -> Result<bool> {
    Ok(!pending_port_allocation_intents(paths, ctx)?.is_empty())
}

fn pending_port_allocation_intents(
    paths: &HostPaths,
    ctx: &OperationContext,
) -> Result<Vec<crate::journal::Transaction>> {
    let Some(work) = open_optional_private_dir(&paths.work, ctx)? else {
        return Ok(Vec::new());
    };
    let journal_directory = match work.child("journal") {
        Ok(directory) => directory,
        Err(hton_storage::Error::NotFound) => return Ok(Vec::new()),
        Err(_) => {
            return Err(journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "port allocation journal is unavailable",
            ));
        }
    };
    let mut read_ctx = ctx.clone();
    read_ctx.execution_mode = ExecutionMode::ReadOnly;
    let journal = JournalStore::new(Arc::new(journal_directory), Arc::new(SystemControl));
    Ok(journal
        .unfinished(&read_ctx)?
        .into_iter()
        .filter(|transaction| matches!(&transaction.intent, Intent::PortAllocation { .. }))
        .collect())
}

struct ParsedInstallArgs {
    input_preflight: bool,
    input: Option<InstallInputArgs>,
    selection_offer_digest: Option<Sha256Digest>,
    selected_modules: BTreeSet<ModuleId>,
    selection_profile: Option<ProfileId>,
}

#[derive(Clone, Debug)]
struct InstallInputArgs {
    site_domain: Option<String>,
    edge_port: Option<u16>,
    certificate_email: Option<String>,
}

fn parse_install_args(args: &[String]) -> std::result::Result<ParsedInstallArgs, ()> {
    if args.is_empty() {
        return Ok(ParsedInstallArgs {
            input_preflight: false,
            input: None,
            selection_offer_digest: None,
            selected_modules: BTreeSet::new(),
            selection_profile: None,
        });
    }
    let mut input_preflight = false;
    let mut domain = None;
    let mut edge = None;
    let mut certificate_email = None;
    let mut selection_offer_digest = None;
    let mut selected_modules = BTreeSet::new();
    let mut selection_profile = None;
    let mut index = 0;
    while index < args.len() {
        let option = args[index].as_str();
        if option == "--input-preflight" && !input_preflight {
            input_preflight = true;
            index += 1;
            continue;
        }
        let value = args.get(index + 1).ok_or(())?.as_str();
        if value.starts_with("--") || value.is_empty() {
            return Err(());
        }
        match option {
            "--selection-offer-digest" if selection_offer_digest.is_none() => {
                selection_offer_digest = Some(Sha256Digest::new(value).map_err(|_| ())?);
            }
            "--selected-module" => {
                let id = ModuleId::new(value).map_err(|_| ())?;
                if !selected_modules.insert(id) {
                    return Err(());
                }
            }
            "--profile" if selection_profile.is_none() => {
                selection_profile = Some(ProfileId::new(value).map_err(|_| ())?);
            }
            "--site-domain" if domain.is_none() => {
                if !valid_site_domain(value) {
                    return Err(());
                }
                domain = Some(value.to_owned());
            }
            "--edge-port" if edge.is_none() => {
                let port = parse_port(value)?;
                if port != 443 {
                    return Err(());
                }
                edge = Some(port);
            }
            "--certificate-email" if certificate_email.is_none() => {
                if !crate::host::install_input::valid_certificate_email(value) {
                    return Err(());
                }
                certificate_email = Some(value.to_owned());
            }
            _ => return Err(()),
        }
        index += 2;
    }
    let any_input = domain.is_some() || edge.is_some() || certificate_email.is_some();
    let input = if any_input {
        Some(InstallInputArgs {
            site_domain: domain,
            edge_port: edge,
            certificate_email,
        })
    } else {
        None
    };
    if selection_offer_digest.is_none()
        && (!selected_modules.is_empty() || selection_profile.is_some())
    {
        return Err(());
    }
    Ok(ParsedInstallArgs {
        input_preflight,
        input,
        selection_offer_digest,
        selected_modules,
        selection_profile,
    })
}

fn validate_selection_candidate(
    paths: &HostPaths,
    parsed: &ParsedInstallArgs,
    ctx: &OperationContext,
) -> Result<Option<InstallSelectionCandidate>> {
    let Some(expected_digest) = &parsed.selection_offer_digest else {
        return Ok(None);
    };
    let profile_args = parsed
        .selection_profile
        .as_ref()
        .map(|profile| vec!["--profile".to_owned(), profile.to_string()])
        .unwrap_or_default();
    let offer = selection_offer_inner(paths, &profile_args, ctx)?;
    if offer.get("offerDigest").and_then(Value::as_str) != Some(expected_digest.as_str()) {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "install selection offer or source revision changed; request a fresh offer",
        ));
    }
    let selectable: BTreeSet<ModuleId> = offer
        .get("selectableModules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|value| ModuleId::new(value).ok())
        .collect();
    if parsed
        .selected_modules
        .iter()
        .any(|id| !selectable.contains(id))
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ModuleUnsupported,
            "install selection contains an unknown or unavailable module",
        ));
    }
    let catalogue_version = offer
        .get("catalogueVersion")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "install catalogue version is invalid",
            )
        })?;
    let previous_module_ids = offer
        .get("savedModuleIds")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "saved install selection is invalid",
            )
        })?;
    let root_state = if offer.get("expectedRootAbsent").and_then(Value::as_bool) == Some(true) {
        InstallRootState::Absent
    } else {
        let revision = offer
            .get("rootRevision")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "install root revision is unavailable",
                )
            })?;
        InstallRootState::Present { revision }
    };
    let candidate_material = serde_json::to_vec(&(
        "install-selection-candidate-v1",
        &catalogue_version,
        expected_digest,
        &parsed.selected_modules,
        &previous_module_ids,
        &root_state,
    ))
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "install selection candidate encoding failed",
        )
    })?;
    let candidate_digest = config::digest(&candidate_material);
    Ok(Some(InstallSelectionCandidate {
        catalogue_version,
        offer_digest: expected_digest.clone(),
        selected_module_ids: parsed.selected_modules.clone(),
        previous_module_ids,
        root_state,
        candidate_digest,
    }))
}

fn with_selection_candidate(
    mut result: Value,
    selection: Option<InstallSelectionCandidate>,
    ctx: &OperationContext,
) -> Result<Value> {
    if let (Some(object), Some(selection)) = (result.as_object_mut(), selection) {
        let selection = serde_json::to_value(selection).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "install selection candidate encoding failed",
            )
        })?;
        object.insert("selectionCandidate".into(), selection);
    }
    Ok(result)
}

fn input_preflight(
    paths: &HostPaths,
    input: Option<&InstallInputArgs>,
    selection: Option<&InstallSelectionCandidate>,
    ctx: &OperationContext,
) -> Result<Value> {
    let missing = missing_publication_inputs(paths, input);
    if !missing.is_empty() {
        return Ok(input_required_result(paths, missing));
    }
    let plan = preflight_plan(paths, input, selection, ctx)?;
    let executable = plan.conflicts.is_empty() && plan.approvals.is_empty();
    let plan = serde_json::to_value(plan).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "install preflight plan encoding failed",
        )
    })?;
    Ok(json!({
        "status":if executable { "ready" } else { "preflight" },
        "operation":"install.reconcile",
        "writes":false,
        "installPlan":plan
    }))
}

/// Compose a bounded read-only plan from one coherent desired/registry/runtime
/// basis. Missing qualification, certificate or listener evidence remains a
/// blocker in the plan; no bootstrap state or journal is created here.
fn restore_rootless_allocation_settings(
    paths: &HostPaths,
    desired: &mut DesiredReadResult,
    candidate: &PortAllocationCandidate,
    modules: &BTreeSet<ModuleId>,
    catalogue: &crate::catalog::ModuleCatalogue,
    ctx: &OperationContext,
) -> Result<()> {
    let source = config::FsConfigSource::open(&paths.config).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "rootless module settings source unavailable",
        )
    })?;
    let snapshot = desired.snapshot.as_mut().ok_or_else(|| {
        journal::failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "rootless desired snapshot unavailable",
        )
    })?;
    let mut expected_modules = BTreeSet::new();
    for allocation in candidate.allocations.values() {
        let module = allocation
            .settings_document
            .as_str()
            .strip_prefix("module.")
            .and_then(|name| ModuleId::new(name).ok())
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "rootless allocation settings identity is invalid",
                )
            })?;
        expected_modules.insert(module);
    }
    if &expected_modules != modules {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "rootless module set does not match committed allocation candidate",
        ));
    }
    for module in modules {
        let descriptor = catalogue.describe(module).ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ModuleUnknown,
                "rootless allocated module is not in the trusted catalogue",
            )
        })?;
        let filename = descriptor
            .location
            .components
            .last()
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "module settings location is invalid",
                )
            })?
            .as_str()
            .to_owned();
        let bytes = source
            .read(
                &["modules", &filename],
                config::LoadLimits::default().file_bytes,
            )
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "rootless module settings read failed",
                )
            })?
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "committed rootless module settings disappeared",
                )
            })?;
        let value = config::decode_yaml(&bytes, config::LoadLimits::default()).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "committed rootless module settings are invalid",
            )
        })?;
        let version = FormatVersion(
            value
                .get("schemaVersion")
                .and_then(Value::as_u64)
                .unwrap_or(1) as u32,
        );
        let settings = value
            .get("settings")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "committed rootless module settings shape is invalid",
                )
            })?
            .clone()
            .into_iter()
            .collect();
        let object_id = ObjectId::new(format!("module.{}", module.as_str())).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "rootless module object identity is invalid",
            )
        })?;
        let revision = config::content_revision(object_id.clone(), &bytes);
        snapshot.read_set.absent_objects.remove(&object_id);
        snapshot.read_set.objects.insert(
            object_id.clone(),
            ReadDependency {
                revision: revision.clone(),
                format_version: version,
            },
        );
        snapshot
            .features
            .insert(module.clone(), FeatureSelection { enabled: true });
        snapshot.module_settings.insert(
            module.clone(),
            ModuleSettings {
                schema_version: version,
                settings,
            },
        );
        desired.documents.push(ConfigDocument {
            object_id,
            location: descriptor.location.clone(),
            source_bytes: bytes,
            revision,
            schema_version: Some(version),
            diagnostics: Vec::new(),
        });
    }
    desired.read_set = snapshot.read_set.clone();
    Ok(())
}

fn preflight_plan(
    paths: &HostPaths,
    input: Option<&InstallInputArgs>,
    selection: Option<&InstallSelectionCandidate>,
    ctx: &OperationContext,
) -> Result<InstallPlan> {
    let mut read_ctx = ctx.clone();
    read_ctx.execution_mode = ExecutionMode::ReadOnly;
    let site_index_observation = webroot::inspect_index(&read_ctx)?;
    let site_index = site_index_basis(&site_index_observation);
    let catalogue = crate::catalog::ModuleCatalogue::builtin().map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "trusted install catalogue unavailable",
        )
    })?;
    let work = open_optional_private_dir(&paths.work, ctx)?;
    let config_dir = open_optional_private_dir(&paths.config, ctx)?;
    let registry_observed = match work.as_ref() {
        Some(directory) => crate::registry::RegistryStore::inspect(directory, &read_ctx)?,
        None => None,
    };
    let runtime_observed = read_runtime_for_preflight(work.as_ref(), ctx)?;
    let install_record = read_installation_record(work.as_ref(), ctx)?;
    let root_bytes = config_dir
        .as_ref()
        .map(|directory| {
            directory
                .read("hton.yaml", config::LoadLimits::default().file_bytes)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "installation root config is unavailable or unsafe",
                    )
                })
        })
        .transpose()?
        .flatten();
    let rootless_allocation = if root_bytes.is_none() {
        match config_dir.as_ref() {
            Some(_) => {
                let source = config::FsConfigSource::open(&paths.config).map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "rootless allocation inventory unavailable",
                    )
                })?;
                let modules = source
                    .list_modules(config::LoadLimits::default().files)
                    .map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "rootless module inventory unavailable",
                        )
                    })?;
                let docker = source
                    .list_docker(config::LoadLimits::default().files)
                    .map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "rootless Docker inventory unavailable",
                        )
                    })?;
                if modules.is_empty() && docker.is_empty() {
                    None
                } else if docker.is_empty() {
                    rootless_allocation_selection(paths, &modules, ctx)?
                } else {
                    None
                }
            }
            None => None,
        }
    } else {
        None
    };
    let root_value = root_bytes
        .as_deref()
        .map(|bytes| config::decode_yaml(bytes, config::LoadLimits::default()))
        .transpose()
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "existing installation root config is invalid",
            )
        })?;
    let root_instance = root_value
        .as_ref()
        .and_then(|value| value.pointer("/instance/name"))
        .and_then(Value::as_str)
        .map(InstanceId::new)
        .transpose()
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "existing installation instance identity is invalid",
            )
        })?;
    let registry_instance = registry_observed
        .as_ref()
        .map(|value| value.instance_id.clone());
    let runtime_instance = runtime_observed
        .as_ref()
        .map(|value| InstanceId::new(value.instance_id.as_str()))
        .transpose()
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "runtime instance identity is invalid",
            )
        })?;
    let record_instance = install_record.map(|record| record.instance_id);
    let mut known_instances = [
        record_instance.as_ref(),
        root_instance.as_ref(),
        registry_instance.as_ref(),
        runtime_instance.as_ref(),
        rootless_allocation.as_ref().map(|state| match state {
            RootlessAllocationSelection::Committed { instance_id, .. } => instance_id,
            RootlessAllocationSelection::Pending => {
                unreachable!("pending allocation is returned before plan composition")
            }
        }),
    ]
    .into_iter()
    .flatten();
    let instance_id = if let Some(first) = known_instances.next() {
        if known_instances.any(|candidate| candidate != first) {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "installation, config, registry and runtime instance identities disagree",
            ));
        }
        first.clone()
    } else {
        generate_instance_id(ctx)?
    };
    let registry = registry_observed.unwrap_or_else(|| RegistrySnapshot {
        instance_id: instance_id.clone(),
        revision: StateRevision(0),
        entries: BTreeMap::new(),
        diagnostics: Vec::new(),
        recovery_originals: Vec::new(),
    });
    if registry.instance_id != instance_id {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "install registry belongs to another instance",
        ));
    }
    let runtime = runtime_observed.unwrap_or_else(|| {
        hton_runtime_contract::initial_state(
            hton_runtime_contract::InstanceId::new(instance_id.as_str())
                .expect("Core and Runtime instance ID grammars match"),
        )
    });
    if runtime.instance_id.as_str() != instance_id.as_str() {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "install runtime state belongs to another instance",
        ));
    }

    let selected = selection
        .map(|candidate| candidate.selected_module_ids.clone())
        .unwrap_or_default();
    let mut desired = if let Some(root_bytes) = root_bytes.as_deref() {
        let source = config::FsConfigSource::open(&paths.config).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "configuration inventory is unavailable or unsafe",
            )
        })?;
        let loaded = config::load(&source, &BTreeSet::from([FormatVersion(1)]));
        if loaded.read_set.coherence != ReadCoherence::Coherent
            || !loaded.errors.is_empty()
            || loaded.snapshot.is_none()
            || loaded.documents.is_empty()
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "install preflight requires one complete coherent configuration snapshot",
            ));
        }
        let root_id = ObjectId::new("root").expect("fixed object ID");
        match root_bytes.as_deref() {
            Some(bytes)
                if loaded
                    .read_set
                    .objects
                    .get(&root_id)
                    .map_or(true, |dependency| {
                        dependency.revision != config::content_revision(root_id.clone(), bytes)
                    }) =>
            {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "root configuration changed while install preflight was reading it",
                ));
            }
            None if !loaded.read_set.absent_objects.contains(&root_id) => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "root configuration appeared while install preflight was reading it",
                ));
            }
            _ => {}
        }
        let mut desired = loaded;
        apply_selection_candidate(
            desired.snapshot.as_mut().expect("snapshot checked"),
            selection,
            ctx,
        )?;
        let _ = root_bytes;
        desired
    } else {
        let (modules, docker) = match config_dir.as_ref() {
            Some(_) => {
                let source = config::FsConfigSource::open(&paths.config).map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "configuration inventory is unavailable or unsafe",
                    )
                })?;
                let modules = source
                    .list_modules(config::LoadLimits::default().files)
                    .map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "module settings inventory is unavailable",
                        )
                    })?;
                let docker = source
                    .list_docker(config::LoadLimits::default().files)
                    .map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "Docker settings inventory is unavailable",
                        )
                    })?;
                (modules, docker)
            }
            None => (Vec::new(), Vec::new()),
        };
        let recovered_modules = match rootless_allocation.as_ref() {
            Some(RootlessAllocationSelection::Committed { modules, .. })
                if docker.is_empty() && !modules.is_empty() && selected == *modules =>
            {
                modules.clone()
            }
            _ => BTreeSet::new(),
        };
        if (!modules.is_empty() && recovered_modules.is_empty())
            || !docker.is_empty()
            || !registry.entries.is_empty()
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::ModuleIncomplete,
                "root config is absent while managed settings or registry entries exist; repair before install",
            ));
        }
        if !modules.is_empty() && selected != recovered_modules {
            return Err(journal::failure(
                ctx,
                ErrorCode::PlanStale,
                "selected module set differs from the committed rootless allocation",
            ));
        }
        let absent_selected = selected
            .difference(&recovered_modules)
            .cloned()
            .collect::<BTreeSet<_>>();
        let absent_objects = std::iter::once(ObjectId::new("root").expect("fixed ID"))
            .chain(absent_selected.iter().map(|module| {
                ObjectId::new(format!("module.{}", module.as_str()))
                    .expect("catalogue module ID forms valid object ID")
            }))
            .collect();
        let read_set = ReadSet {
            objects: BTreeMap::new(),
            absent_objects,
            secret_versions: BTreeMap::new(),
            coherence: ReadCoherence::Coherent,
        };
        let mut desired =
            config::initial_desired(&instance_id, &absent_selected, &read_set, &catalogue)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::ConfigSemanticConflict,
                        "virtual first-install desired state is invalid",
                    )
                })?;
        if selection
            .is_some_and(|candidate| !matches!(candidate.root_state, InstallRootState::Absent))
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::PlanStale,
                "install selection was not offered against an absent root config",
            ));
        }
        if !recovered_modules.is_empty() {
            if let Some(RootlessAllocationSelection::Committed { candidate, .. }) =
                rootless_allocation.as_ref()
            {
                restore_rootless_allocation_settings(
                    paths,
                    &mut desired,
                    candidate,
                    &recovered_modules,
                    &catalogue,
                    ctx,
                )?;
                verify_first_install_inventory_absent(paths, ctx)?;
            }
        } else {
            // Recheck both inventories after the virtual absence snapshot was formed.
            verify_first_install_inventory_absent(paths, ctx)?;
        }
        desired
    };
    let input_candidate = apply_input_candidate(&mut desired, input, ctx)?;
    let Some(snapshot) = desired.snapshot.as_mut() else {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "install desired snapshot is unavailable",
        ));
    };
    let normalized_report = config::normalize(snapshot, &catalogue);
    let mut conflicts = normalized_report.issues;
    let Some(normalized) = normalized_report.normalized_snapshot else {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "install settings could not be normalized",
        ));
    };
    if normalized.desired.read_set != desired.read_set {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "install normalized snapshot changed its source read set",
        ));
    }

    let mut resolution = crate::catalog::ResolutionEvidence::default();
    let control = SystemControl;
    let host_reservation_evidence =
        observations::reservation_evidence_for(&registry, &control, &read_ctx)?;
    let now = UnixMillis(control.now_ms());
    resolution.now = now;
    let mut graph =
        catalogue.resolve_for_install_preflight(&normalized, ReleasePhase::R1, &resolution);
    conflicts.extend(graph.conflicts.clone());
    if !host_reservation_evidence.complete
        || host_reservation_evidence.observed_at.0 > now.0
        || host_reservation_evidence.valid_until.0 <= now.0
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "complete fresh host reservation evidence is unavailable",
        ));
    }
    let host_conflicts = observations::listener_conflicts(&graph, &host_reservation_evidence);
    conflicts.extend(host_conflicts.clone());

    let plan_expires_at = UnixMillis(
        now.0
            .saturating_add(30_000)
            .min(ctx.deadline.0)
            .min(host_reservation_evidence.valid_until.0),
    );
    if plan_expires_at.0 <= now.0 {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanExpired,
            "install preflight evidence expires before a plan can be issued",
        ));
    }
    let mut port_allocation_candidate = None;
    let allocation_preflight_clear = conflicts.is_empty();
    if allocation_preflight_clear && !graph.allocation_bindings.is_empty() {
        let bindings = catalogue
            .allocation_bindings_for(&graph, &normalized)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "trusted VPN allocation settings are invalid",
                )
            })?;
        let needs_entropy = bindings.iter().any(|binding| {
            binding
                .fields
                .iter()
                .any(|field| field.saved_port.is_none())
        });
        let mut entropy = vec![0; if needs_entropy { 8192 } else { 0 }];
        if needs_entropy {
            getrandom::fill(&mut entropy).map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "OS cryptographic randomness unavailable for VPN port assignment",
                )
            })?;
        }
        let budget = Budget::bounded_default(plan_expires_at);
        let planned = crate::port_allocation::plan_graph(
            &catalogue,
            &graph,
            &normalized,
            &registry,
            &host_reservation_evidence,
            &crate::port_allocation::EntropyBytes(entropy),
            now,
            plan_expires_at,
            &budget,
        );
        match planned {
            Ok(planned) => {
                graph = planned.graph;
                if !planned.candidate.allocations.is_empty() {
                    port_allocation_candidate = Some(planned.candidate);
                }
            }
            Err(error) => {
                conflicts.extend(allocation_plan_issues(error));
            }
        }
    }

    let public_edge = public_edge_from_snapshot(&normalized, ctx)?;
    let mut approvals = Vec::new();
    if !desired
        .read_set
        .absent_objects
        .contains(&ObjectId::new("root").expect("fixed object ID"))
    {
        if let Some(candidate) = input_candidate.as_ref() {
            if candidate
                .changes
                .iter()
                .any(|change| change.previous.is_some())
            {
                approvals.push(InstallApprovalRequirement::InputChange {
                    candidate_digest: candidate.candidate_digest.clone(),
                });
            }
        }
    }
    if let Some(candidate) = &port_allocation_candidate {
        let changed = candidate.allocations.values().any(|allocation| {
            allocation
                .prior_fields
                .values()
                .any(|prior| prior.is_some_and(|prior| prior != allocation.port))
        });
        if changed {
            approvals.push(InstallApprovalRequirement::AllocationChange {
                candidate_digest: candidate.candidate_digest.clone(),
            });
        }
    }
    let mut resources = BTreeMap::new();
    for entry in registry.entries.values() {
        for resource in &entry.resources {
            if resources
                .insert(resource.resource_id.clone(), resource.fingerprint.clone())
                .is_some_and(|previous| previous != resource.fingerprint)
            {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "registry contains conflicting resource fingerprints",
                ));
            }
        }
    }
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(|_| {
        journal::failure(ctx, ErrorCode::IoUnavailable, "OS randomness unavailable")
    })?;
    let suffix: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(InstallPlan {
        plan_id: PlanId::new(format!("install.{suffix}")).expect("random canonical plan ID"),
        instance_id: instance_id.clone(),
        expected_inputs: ExpectedInputs {
            read_set: desired.read_set.clone(),
            registry_revision: registry.revision,
            runtime_revision: StateRevision(runtime.revision),
            run_epoch: RunEpoch(runtime.run_epoch),
            resources,
        },
        host_reservation_evidence,
        selection_candidate: selection.cloned(),
        input_candidate,
        graph: graph.clone(),
        public_edge,
        site_index,
        port_allocation_candidate,
        foreign_bindings: Vec::new(),
        certificate_requirements: graph.certificate_claims.clone(),
        conflicts,
        approvals,
        phases: vec![
            InstallEffectPhase::PersistInputs,
            InstallEffectPhase::PersistSelection,
            InstallEffectPhase::PersistAllocations,
            InstallEffectPhase::IssueOrReuseCertificate,
            InstallEffectPhase::SnapshotForeign,
            InstallEffectPhase::RemapForeign,
            InstallEffectPhase::ApplyIngress,
            InstallEffectPhase::VerifyIngress,
            InstallEffectPhase::Compensate,
        ],
        created_at: now,
        expires_at: plan_expires_at,
    })
}

fn open_optional_private_dir(
    path: &std::path::Path,
    ctx: &OperationContext,
) -> Result<Option<SecureDir>> {
    match SecureDir::open_private(path) {
        Ok(directory) => Ok(Some(directory)),
        Err(hton_storage::Error::NotFound) => Ok(None),
        Err(_) => Err(journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "installation state directory is unsafe or unavailable",
        )),
    }
}

fn read_runtime_for_preflight(
    work: Option<&SecureDir>,
    ctx: &OperationContext,
) -> Result<Option<hton_runtime_contract::RuntimeState>> {
    let Some(work) = work else { return Ok(None) };
    let runtime = match work.child("runtime") {
        Ok(runtime) => runtime,
        Err(hton_storage::Error::NotFound) => return Ok(None),
        Err(_) => {
            return Err(journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "runtime state directory is unsafe or unavailable",
            ));
        }
    };
    let Some(bytes) = runtime
        .read("state.json", hton_runtime_contract::MAX_WIRE_BYTES)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "runtime state is unavailable",
            )
        })?
    else {
        return Ok(None);
    };
    hton_runtime_contract::decode(&bytes)
        .map(Some)
        .map_err(|_| journal::failure(ctx, ErrorCode::RecoveryRequired, "runtime state is corrupt"))
}

fn read_installation_record(
    work: Option<&SecureDir>,
    ctx: &OperationContext,
) -> Result<Option<InstallationRecord>> {
    let Some(work) = work else { return Ok(None) };
    let Some(bytes) = work.read("installation.json", 16_384).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "installation identity is unavailable",
        )
    })?
    else {
        return Ok(None);
    };
    let record: InstallationRecord = serde_json::from_slice(&bytes).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "installation identity is corrupt",
        )
    })?;
    if record.schema_version != 1 {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "installation identity version is unsupported",
        ));
    }
    Ok(Some(record))
}

fn generate_instance_id(ctx: &OperationContext) -> Result<InstanceId> {
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(|_| {
        journal::failure(ctx, ErrorCode::IoUnavailable, "OS randomness unavailable")
    })?;
    let suffix: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(InstanceId::new(format!("hton-{suffix}")).expect("random canonical ID"))
}

fn verify_first_install_inventory_absent(paths: &HostPaths, ctx: &OperationContext) -> Result<()> {
    let directory = match SecureDir::open_private(&paths.config) {
        Ok(directory) => directory,
        Err(hton_storage::Error::NotFound) => return Ok(()),
        Err(_) => {
            return Err(journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "first-install config inventory became unsafe or unavailable",
            ));
        }
    };
    let _ = directory;
    let source = config::FsConfigSource::open(&paths.config).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "first-install config inventory became unavailable",
        )
    })?;
    let root = source
        .read(&["hton.yaml"], config::LoadLimits::default().file_bytes)
        .map_err(|_| journal::failure(ctx, ErrorCode::IoUnavailable, "root read failed"))?;
    let modules = source
        .list_modules(config::LoadLimits::default().files)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "module inventory read failed",
            )
        })?;
    let docker = source
        .list_docker(config::LoadLimits::default().files)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "Docker inventory read failed",
            )
        })?;
    let rootless_allocation_is_exact = if modules.is_empty() && docker.is_empty() {
        true
    } else if docker.is_empty() {
        matches!(
            rootless_allocation_selection(paths, &modules, ctx)?,
            Some(RootlessAllocationSelection::Committed { .. })
        )
    } else {
        false
    };
    if root.is_some() || !rootless_allocation_is_exact {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "first-install config inventory changed outside the exact committed allocation",
        ));
    }
    Ok(())
}

fn apply_selection_candidate(
    snapshot: &mut DesiredSnapshot,
    selection: Option<&InstallSelectionCandidate>,
    ctx: &OperationContext,
) -> Result<()> {
    let Some(candidate) = selection else {
        return Ok(());
    };
    let root = ObjectId::new("root").expect("fixed object ID");
    match &candidate.root_state {
        InstallRootState::Absent if !snapshot.read_set.absent_objects.contains(&root) => {
            return Err(journal::failure(
                ctx,
                ErrorCode::PlanStale,
                "install selection root basis changed",
            ));
        }
        InstallRootState::Present { revision }
            if snapshot
                .read_set
                .objects
                .get(&root)
                .map(|dependency| &dependency.revision)
                != Some(revision) =>
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::PlanStale,
                "install selection root revision changed",
            ));
        }
        _ => {}
    }
    for (module_id, feature) in &mut snapshot.features {
        feature.enabled = candidate.selected_module_ids.contains(module_id);
    }
    if candidate
        .selected_module_ids
        .iter()
        .any(|id| !snapshot.features.contains_key(id))
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ModuleUnknown,
            "install selection contains an unavailable module",
        ));
    }
    Ok(())
}

fn apply_input_candidate(
    desired: &mut DesiredReadResult,
    input: Option<&InstallInputArgs>,
    ctx: &OperationContext,
) -> Result<Option<InstallConfigCandidate>> {
    let Some(input_args) = input else {
        return Ok(None);
    };
    let snapshot = desired.snapshot.as_ref().ok_or_else(|| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "install desired snapshot is unavailable",
        )
    })?;
    let core = snapshot
        .platform
        .settings
        .get("core")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "core publication settings are unavailable",
            )
        })?;
    let edge = core.get("edge").and_then(Value::as_object).ok_or_else(|| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "site edge settings are unavailable",
        )
    })?;
    let saved_domain = edge
        .get("domains")
        .and_then(Value::as_array)
        .and_then(|domains| domains.first())
        .and_then(Value::as_str);
    let domain = input_args
        .site_domain
        .as_deref()
        .or(saved_domain)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "install site domain is required",
            )
        })?;
    let site_domain_changed = input_args.site_domain.is_some();
    let edge_changed = input_args.edge_port.is_some();
    let requested_port = input_args.edge_port.unwrap_or(443);
    let saved_certificate_email = core
        .get("certificates")
        .map(|certificates| {
            crate::host::install_input::existing_certificate_email(certificates, ctx)
        })
        .transpose()?
        .flatten();
    let certificate_email = input_args
        .certificate_email
        .as_deref()
        .or(saved_certificate_email.as_deref())
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "certificate account email is required for DNS-01 publication",
            )
        })?;
    let already_matches = saved_domain == Some(domain)
        && edge
            .get("listen")
            .and_then(Value::as_object)
            .is_some_and(|listen| {
                listen.get("transport").and_then(Value::as_str) == Some("tcp")
                    && listen.get("port").and_then(Value::as_u64) == Some(requested_port as u64)
                    && listen.get("scope").and_then(Value::as_str) == Some("public")
                    && listen
                        .get("addressFamilies")
                        .and_then(Value::as_array)
                        .is_some_and(|families| {
                            families.len() == 1 && families[0].as_str() == Some("ipv4")
                        })
            })
        && saved_certificate_email.as_deref() == Some(certificate_email);
    if already_matches {
        return Ok(None);
    }
    let install_input = InstallInput {
        site_domain: domain.to_owned(),
        public_edge: HostEndpoint {
            transport: Transport::Tcp,
            address: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            port: requested_port,
        },
        certificate_email: input_args.certificate_email.clone(),
        provenance: InstallInputProvenance {
            site_domain: if site_domain_changed {
                InstallValueSource::CommandLine
            } else {
                InstallValueSource::Existing
            },
            public_edge: if edge_changed {
                InstallValueSource::CommandLine
            } else {
                InstallValueSource::Existing
            },
            certificate_email: input_args
                .certificate_email
                .as_ref()
                .map(|_| InstallValueSource::CommandLine),
        },
    };
    let candidate =
        crate::host::install_input::prepare_input_candidate(&install_input, desired, ctx)?;
    let core = desired
        .snapshot
        .as_mut()
        .and_then(|snapshot| snapshot.platform.settings.get_mut("core"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "core publication settings are unavailable",
            )
        })?;
    let edge = core
        .get_mut("edge")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "site edge settings are unavailable",
            )
        })?;
    for change in &candidate.changes {
        match &change.proposed {
            InstallConfigValue::SiteDomain(domain) => {
                edge.insert("domains".into(), json!([domain]));
            }
            InstallConfigValue::PublicEdge(endpoint) => {
                let listen = edge
                    .get_mut("listen")
                    .and_then(Value::as_object_mut)
                    .ok_or_else(|| {
                        journal::failure(
                            ctx,
                            ErrorCode::ConfigSemanticConflict,
                            "site listener settings are unavailable",
                        )
                    })?;
                listen.insert("transport".into(), json!(endpoint.transport));
                listen.insert("port".into(), json!(endpoint.port));
                listen.insert("scope".into(), json!("public"));
                listen.insert("addressFamilies".into(), json!(["ipv4"]));
            }
            InstallConfigValue::CertificateEmail(email) => {
                if let Some(certificates) = core.get_mut("certificates") {
                    let certificates = certificates.as_object_mut().ok_or_else(|| {
                        journal::failure(
                            ctx,
                            ErrorCode::ConfigSemanticConflict,
                            "certificate settings are malformed",
                        )
                    })?;
                    certificates.insert("email".into(), json!(email));
                } else {
                    core.insert(
                        "certificates".into(),
                        json!({
                            "ca":"letsencrypt",
                            "challenge":"dns-01",
                            "provider":"internetbs",
                            "email":email,
                            "credentials":{
                                "apiKey":{"secretRef":"dns/internetbs/api-key"},
                                "apiPassword":{"secretRef":"dns/internetbs/api-password"}
                            }
                        }),
                    );
                }
            }
        }
    }
    Ok(Some(candidate))
}

fn allocation_plan_issues(error: crate::port_allocation::AllocationPlanError) -> Vec<CheckIssue> {
    match error {
        crate::port_allocation::AllocationPlanError::Catalogue(issues)
        | crate::port_allocation::AllocationPlanError::Graph(issues)
        | crate::port_allocation::AllocationPlanError::Registry(issues) => issues,
        crate::port_allocation::AllocationPlanError::Allocation(blocker) => {
            let (code, module, message) = match blocker {
                crate::port_allocation::AllocationBlocker::NoFreePort => (
                    ErrorCode::ModuleIncomplete,
                    None,
                    "no free VPN backend port is available in 5000-6000",
                ),
                crate::port_allocation::AllocationBlocker::SavedAllocationConflict { .. } => (
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    "saved VPN backend port conflicts with a host reservation",
                ),
                crate::port_allocation::AllocationBlocker::InvalidSavedAllocation { .. }
                | crate::port_allocation::AllocationBlocker::DuplicateSavedAllocation { .. }
                | crate::port_allocation::AllocationBlocker::InvalidBinding => (
                    ErrorCode::ConfigSemanticConflict,
                    None,
                    "saved VPN backend port state is inconsistent",
                ),
                crate::port_allocation::AllocationBlocker::ReservationEvidenceMissing
                | crate::port_allocation::AllocationBlocker::ReservationEvidenceStale => (
                    ErrorCode::IoUnavailable,
                    None,
                    "fresh host port reservation evidence is unavailable",
                ),
                crate::port_allocation::AllocationBlocker::CandidateExpired => (
                    ErrorCode::PlanExpired,
                    None,
                    "VPN port allocation candidate expired",
                ),
                crate::port_allocation::AllocationBlocker::EntropyUnavailable => (
                    ErrorCode::IoUnavailable,
                    None,
                    "secure random bytes are unavailable for first VPN port assignment",
                ),
                crate::port_allocation::AllocationBlocker::IncoherentReadSet => (
                    ErrorCode::ConfigReadConflict,
                    None,
                    "VPN port allocation requires a coherent settings snapshot",
                ),
            };
            vec![config::issue(code, None, module, message)]
        }
    }
}

fn public_edge_from_snapshot(
    snapshot: &NormalizedSnapshot,
    ctx: &OperationContext,
) -> Result<Option<HostEndpoint>> {
    let edge = snapshot
        .desired
        .platform
        .settings
        .get("core")
        .and_then(|core| core.get("edge"))
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "site edge settings are unavailable",
            )
        })?;
    let listen = edge
        .get("listen")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "site listener settings are unavailable",
            )
        })?;
    let Some(port) = listen
        .get("port")
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
    else {
        return Ok(None);
    };
    let transport = match listen.get("transport").and_then(Value::as_str) {
        Some("tcp") => Transport::Tcp,
        Some("udp") => Transport::Udp,
        _ => return Ok(None),
    };
    if listen.get("scope").and_then(Value::as_str) != Some("public") {
        return Ok(None);
    }
    Ok(Some(HostEndpoint {
        transport,
        address: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
        port,
    }))
}

fn input_required_result(paths: &HostPaths, missing: Vec<&'static str>) -> Value {
    let first_publication = !has_configured_site_domain(paths);
    let mut suggested = serde_json::Map::new();
    if first_publication && missing.contains(&"siteDomain") {
        suggested.insert("siteDomain".into(), Value::String("example.com".into()));
    }
    if first_publication && missing.contains(&"edgePort") {
        suggested.insert("edgePort".into(), Value::from(443));
    }
    let mut result = json!({
        "status":"inputRequired",
        "operation":"install.reconcile",
        "requiredInputs":missing,
        "writes":false
    });
    if !suggested.is_empty() {
        result
            .as_object_mut()
            .expect("install input result is an object")
            .insert("suggestedValues".into(), Value::Object(suggested));
    }
    result
}

fn edge_change_without_review(
    paths: &HostPaths,
    input: Option<&InstallInputArgs>,
) -> Option<(Value, Value)> {
    // The seeded loopback 4443 value is an unpublished development setting.
    // With no saved site name, the first-publication wizard may propose public
    // TCP/443 without treating that seed as an owner-reviewed live value.
    if !has_configured_site_domain(paths) {
        return None;
    }
    let current = saved_edge_config(paths)?;
    let candidate_port = input.and_then(|value| value.edge_port).unwrap_or(443);
    let current_matches = current.get("transport").and_then(Value::as_str) == Some("tcp")
        && current.get("port").and_then(Value::as_u64) == Some(candidate_port as u64)
        && current.get("scope").and_then(Value::as_str) == Some("public");
    if current_matches {
        return None;
    }
    Some((
        current,
        json!({"transport":"tcp","port":candidate_port,"scope":"public"}),
    ))
}

fn has_configured_site_domain(paths: &HostPaths) -> bool {
    let Some(root) = saved_root_config(paths) else {
        return false;
    };
    root.pointer("/core/edge/domains")
        .and_then(Value::as_array)
        .is_some_and(|domains| {
            domains
                .iter()
                .any(|domain| domain.as_str().is_some_and(valid_site_domain))
        })
}

fn other_input_change_without_review(
    paths: &HostPaths,
    input: Option<&InstallInputArgs>,
) -> Option<(Value, Value)> {
    let input = input?;
    let root = saved_root_config(paths)?;
    let mut current = serde_json::Map::new();
    let mut proposed = serde_json::Map::new();
    if let Some(domain) = &input.site_domain {
        let saved = root
            .pointer("/core/edge/domains")
            .and_then(Value::as_array)
            .and_then(|domains| domains.iter().find_map(Value::as_str));
        if saved.is_some_and(|saved| saved != domain) {
            current.insert("siteDomain".into(), json!(saved));
            proposed.insert("siteDomain".into(), json!(domain));
        }
    }
    (!proposed.is_empty()).then(|| (Value::Object(current), Value::Object(proposed)))
}

fn saved_edge_config(paths: &HostPaths) -> Option<Value> {
    let root = saved_root_config(paths)?;
    root.pointer("/core/edge/listen").cloned()
}

fn saved_root_config(paths: &HostPaths) -> Option<Value> {
    let directory = SecureDir::open(&paths.config).ok()?;
    let bytes = directory
        .read("hton.yaml", config::LoadLimits::default().file_bytes)
        .ok()??;
    config::decode_yaml(&bytes, config::LoadLimits::default()).ok()
}

fn parse_port(value: &str) -> std::result::Result<u16, ()> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(());
    }
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or(())
}

pub(super) fn valid_site_domain(value: &str) -> bool {
    if value.len() > 253
        || value.is_empty()
        || value.ends_with('.')
        || !value.is_ascii()
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return false;
    }
    value.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label.as_bytes()[0].is_ascii_alphanumeric()
            && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn missing_publication_inputs(
    paths: &HostPaths,
    input: Option<&InstallInputArgs>,
) -> Vec<&'static str> {
    let Ok(config) = SecureDir::open(&paths.config) else {
        return vec!["siteDomain", "edgePort", "certificateEmail"];
    };
    let root = config
        .read("hton.yaml", config::LoadLimits::default().file_bytes)
        .ok()
        .flatten()
        .and_then(|bytes| config::decode_yaml(&bytes, config::LoadLimits::default()).ok());
    let mut missing = Vec::new();
    let domain_present = root
        .as_ref()
        .and_then(|root| root.pointer("/core/edge/domains"))
        .and_then(Value::as_array)
        .is_some_and(|domains| {
            domains
                .iter()
                .any(|domain| domain.as_str().is_some_and(valid_site_domain))
        });
    if input.and_then(|value| value.site_domain.as_ref()).is_none() && !domain_present {
        missing.push("siteDomain");
    }
    let public_edge = root
        .as_ref()
        .and_then(|root| root.pointer("/core/edge/listen"))
        .is_some_and(|listen| {
            listen.get("transport").and_then(Value::as_str) == Some("tcp")
                && listen.get("port").and_then(Value::as_u64) == Some(443)
                && listen.get("scope").and_then(Value::as_str) == Some("public")
        });
    if input.and_then(|value| value.edge_port) != Some(443) && !public_edge {
        missing.push("edgePort");
    }

    let certificate_email = root
        .as_ref()
        .and_then(|root| root.pointer("/core/certificates/email"))
        .and_then(Value::as_str)
        .filter(|email| crate::host::install_input::valid_certificate_email(email));
    let certificates_absent = root
        .as_ref()
        .and_then(|root| root.pointer("/core/certificates"))
        .is_none();
    let first_publication =
        !domain_present && input.and_then(|value| value.site_domain.as_ref()).is_none();
    if (domain_present
        || input.and_then(|value| value.site_domain.as_ref()).is_some()
        || first_publication)
        && certificates_absent
        && input
            .and_then(|value| value.certificate_email.as_ref())
            .is_none()
        && certificate_email.is_none()
    {
        missing.push("certificateEmail");
    }

    missing
}
pub(super) fn seed(
    directory: &SecureDir,
    name: &str,
    bytes: &[u8],
    ctx: &OperationContext,
    control: &dyn OperationControl,
) -> Result<bool> {
    let locked = directory.lock(deadline(ctx, control)?).map_err(|_| {
        journal::failure(ctx, ErrorCode::OperationConflict, "seed lock unavailable")
    })?;
    if locked
        .read(name, config::LoadLimits::default().file_bytes)
        .map_err(|_| journal::failure(ctx, ErrorCode::IoUnavailable, "existing seed unavailable"))?
        .is_some()
    {
        return Ok(false);
    }
    locked.compare_exchange(name, None, bytes).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "seed publication uncertain; reconcile before retry",
        )
    })?;
    Ok(true)
}
