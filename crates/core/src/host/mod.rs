//! Installed application composition; domain decisions remain in their owning modules.
pub mod approvals;
pub mod bundle_validation;
pub mod effects;
pub mod foreign_observations;
pub(crate) mod install_input;
pub(crate) mod install_input_writer;
pub mod installation;
pub mod lifecycle;
pub mod logging;
pub mod modules;
pub mod observations;
pub mod publication;
pub mod runtime;
pub mod setup;
pub(crate) mod systemd_installation;
pub mod trustanchor;
pub mod uninstall;
pub(crate) mod webroot;

pub(crate) use approvals::{
    AllocationApproval, InstallInputApproval, authorize_allocation,
    authorize_first_install_allocation,
};

use crate::{
    catalog::{ModuleCatalogue, ResolutionEvidence},
    config,
    journal::{self, JournalStore, OperationControl, SystemControl},
    port_allocation::{self, EntropyBytes, PortSettingsWriter},
    registry::RegistryStore,
    secrets::SecretStore,
};
use hton_contracts::{config::*, operations::*};
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct HostPaths {
    pub config: PathBuf,
    pub work: PathBuf,
}
impl HostPaths {
    pub fn validate(&self, mutating: bool) -> std::result::Result<(), InvalidIdentifier> {
        for path in [&self.config, &self.work] {
            if !path.is_absolute()
                || path.components().any(|part| {
                    matches!(
                        part,
                        std::path::Component::ParentDir | std::path::Component::CurDir
                    )
                })
            {
                return Err(InvalidIdentifier);
            }
        }
        if self.config.file_name().and_then(|name| name.to_str()) != Some("config")
            || self.work.file_name().and_then(|name| name.to_str()) != Some("work")
            || self.config.parent() != self.work.parent()
        {
            return Err(InvalidIdentifier);
        }
        if mutating
            && (self.config != Path::new("/opt/hton/config")
                || self.work != Path::new("/opt/hton/work"))
        {
            return Err(InvalidIdentifier);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallationRecord {
    pub schema_version: u32,
    pub instance_id: InstanceId,
}

pub fn context(
    operation: &str,
    mutating: bool,
) -> std::result::Result<OperationContext, InvalidIdentifier> {
    let uid = rustix::process::geteuid().as_raw();
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(|_| InvalidIdentifier)?;
    let suffix: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
    let now = SystemControl.now_ms();
    let mut scopes = BTreeSet::new();
    if mutating && uid == 0 {
        for scope in [
            "installation.bootstrap",
            "configuration.manage",
            "modules.manage",
            "certificates.manage",
            "backup.manage",
            "identities.manage",
        ] {
            scopes.insert(ScopeId::new(scope)?);
        }
    }
    Ok(OperationContext {
        operation_id: OperationId::new(format!("{operation}.{suffix}"))?,
        actor: ActorContext {
            actor_id: ActorId::new(format!("os.uid.{uid}"))?,
            source: ActorSource::Local,
            granted_scopes: scopes,
            local_approval_refs: BTreeSet::new(),
            request_id: RequestId::new(format!("cli.{suffix}"))?,
        },
        execution_mode: if mutating {
            ExecutionMode::Mutating
        } else {
            ExecutionMode::ReadOnly
        },
        deadline: UnixMillis(now.saturating_add(300_000)),
        cancellation: CancellationId::new(format!("cli.{suffix}"))?,
        event_sink: EventSinkId::new("local.stderr")?,
    })
}

#[derive(Clone)]
pub struct HostApplication {
    pub paths: HostPaths,
    pub config: Arc<SecureDir>,
    pub work: Arc<SecureDir>,
    pub journal: Arc<JournalStore>,
    pub registry: Arc<RegistryStore>,
    pub secrets: Arc<SecretStore>,
    pub control: Arc<dyn OperationControl>,
    pub installation: InstallationRecord,
}
impl HostApplication {
    pub fn open(paths: HostPaths, ctx: &OperationContext) -> Result<Self> {
        let mutating = ctx.execution_mode == ExecutionMode::Mutating;
        paths.validate(mutating).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "untrusted installation paths",
            )
        })?;
        if mutating && rustix::process::geteuid().as_raw() != 0 {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "local root or sudo authority is required",
            ));
        }
        let config = Arc::new(SecureDir::open_private(&paths.config).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "protected configuration unavailable",
            )
        })?);
        let work = Arc::new(SecureDir::open_private(&paths.work).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "protected working state unavailable",
            )
        })?);
        let bytes = work
            .read("installation.json", 16_384)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "installation identity unavailable",
                )
            })?
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "installation is not initialized",
                )
            })?;
        let installation: InstallationRecord = serde_json::from_slice(&bytes).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "installation identity corrupt; original retained",
            )
        })?;
        if installation.schema_version != 1 {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "unsupported installation identity",
            ));
        }
        let control: Arc<dyn OperationControl> = Arc::new(SystemControl);
        let journal = Arc::new(JournalStore::new(
            Arc::new(work.child("journal").map_err(|_| {
                journal::failure(ctx, ErrorCode::IoUnavailable, "journal unavailable")
            })?),
            control.clone(),
        ));
        let registry = Arc::new(RegistryStore::new(work.clone(), journal.clone()));
        let secrets = Arc::new(SecretStore::new(
            Arc::new(config.child("secrets").map_err(|_| {
                journal::failure(ctx, ErrorCode::IoUnavailable, "secret store unavailable")
            })?),
            control.clone(),
        ));
        Ok(Self {
            paths,
            config,
            work,
            journal,
            registry,
            secrets,
            control,
            installation,
        })
    }
    pub fn desired(&self, ctx: &OperationContext) -> Result<DesiredReadResult> {
        let source = config::FsConfigSource::open(&self.paths.config).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "configuration source unavailable",
            )
        })?;
        Ok(config::load(&source, &BTreeSet::from([FormatVersion(1)])))
    }
    pub fn registry(&self, ctx: &OperationContext) -> Result<RegistrySnapshot> {
        self.registry.read(&self.installation.instance_id, ctx)
    }
    pub fn runtime(&self, ctx: &OperationContext) -> Result<hton_runtime_contract::RuntimeState> {
        let expected =
            hton_runtime_contract::InstanceId::new(self.installation.instance_id.as_str())
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "invalid installation identity",
                    )
                })?;
        let runtime = self.work.child("runtime").map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "runtime directory unavailable",
            )
        })?;
        match runtime
            .read("state.json", hton_runtime_contract::MAX_WIRE_BYTES)
            .map_err(|_| {
                journal::failure(ctx, ErrorCode::IoUnavailable, "runtime state unavailable")
            })? {
            None => Ok(hton_runtime_contract::initial_state(expected)),
            Some(bytes) => {
                let state: hton_runtime_contract::RuntimeState =
                    hton_runtime_contract::decode(&bytes).map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "runtime state corrupt; original retained",
                        )
                    })?;
                if state.instance_id != expected {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "runtime instance mismatch",
                    ));
                }
                Ok(state)
            }
        }
    }
}

/// Deliberate disclosure bypasses ordinary serializable operation metadata/caches.
pub enum HostOutput {
    Metadata(serde_json::Value),
    Protected(Vec<u8>),
}

fn port_settings_writer(
    app: &HostApplication,
    ctx: &OperationContext,
) -> Result<PortSettingsWriter> {
    let catalogue = Arc::new(ModuleCatalogue::builtin().map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "trusted catalogue unavailable for VPN port allocation",
        )
    })?);
    PortSettingsWriter::new(
        &app.config,
        catalogue,
        app.journal.clone(),
        app.installation.instance_id.clone(),
        ctx,
    )
}

/// Recover one exact interrupted allocation before constructing a new candidate.
/// The caller must return partial and retry so no replacement entropy is used in
/// the same invocation that reconciles the protected transaction.
fn recover_pending_port_allocation(
    app: &HostApplication,
    ctx: &OperationContext,
) -> Result<Option<serde_json::Value>> {
    let unfinished = app.journal.unfinished(ctx)?;
    let pending = unfinished
        .iter()
        .filter(|transaction| matches!(&transaction.intent, journal::Intent::PortAllocation { .. }))
        .cloned()
        .collect::<Vec<_>>();
    if pending.is_empty() {
        return Ok(None);
    }
    if pending.len() != 1 {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "multiple unfinished VPN allocation transactions require reconciliation",
        ));
    }
    if unfinished.iter().any(|transaction| {
        matches!(
            &transaction.intent,
            journal::Intent::Lifecycle { plan }
                if plan.port_allocation_candidate.as_ref().is_some_and(|candidate| {
                    matches!(&pending[0].intent, journal::Intent::PortAllocation { candidate: allocation, .. }
                        if allocation.candidate_digest == candidate.candidate_digest)
                })
        )
    }) {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "unfinished lifecycle allocation must be resumed by its owning lifecycle plan",
        ));
    }
    let writer = port_settings_writer(app, ctx)?;
    let receipt = writer.recover(&pending[0].transaction_id, ctx)?;
    Ok(Some(serde_json::json!({
        "status": "partial",
        "recovered": "portAllocation",
        "transactionId": receipt.transaction_id,
        "retry": true,
    })))
}

fn recheck_allocation_inputs(
    app: &HostApplication,
    selection: &crate::selection::SelectionCandidate,
    candidate: &PortAllocationCandidate,
    registry_revision: &StateRevision,
    ctx: &OperationContext,
) -> Result<()> {
    let current = app.desired(ctx)?;
    let prospective = selection.read_result();
    let root = ObjectId::new("root").expect("fixed ID");
    let current_root = current.read_set.objects.get(&root).map(|d| &d.revision);
    if current_root != Some(&selection.change().expected_root_revision)
        || !current.errors.is_empty()
        || current.read_set.coherence != ReadCoherence::Coherent
        || prospective.read_set.coherence != ReadCoherence::Coherent
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "prospective configuration or root selection changed before allocation commit",
        ));
    }
    let mut current_set = current.read_set.clone();
    let mut prospective_set = prospective.read_set.clone();
    current_set.objects.remove(&root);
    prospective_set.objects.remove(&root);
    if current_set != prospective_set || candidate.read_set != prospective.read_set {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "VPN allocation read set changed after candidate preparation",
        ));
    }
    let current_registry = app.registry(ctx)?;
    if &current_registry.revision != registry_revision
        || &current_registry.revision != &candidate.registry_revision
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "VPN allocation registry revision changed",
        ));
    }
    let evidence =
        observations::reservation_evidence_for(&current_registry, app.control.as_ref(), ctx)?;
    let now = UnixMillis(app.control.now_ms());
    if !evidence.complete
        || evidence.digest != candidate.reservation_digest
        || evidence.observed_at.0 > now.0
        || evidence.valid_until.0 <= now.0
        || candidate.expires_at.0 <= now.0
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "host reservation evidence changed or expired before VPN allocation commit",
        ));
    }
    Ok(())
}

fn persist_enable_allocation(
    app: &HostApplication,
    selection: &crate::selection::SelectionCandidate,
    ctx: &OperationContext,
) -> Result<Option<PortAllocationReceipt>> {
    let prospective = selection.read_result();
    if !prospective.errors.is_empty() {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "prospective selection has configuration errors",
        ));
    }
    let desired = prospective.snapshot.as_ref().ok_or_else(|| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "prospective desired snapshot unavailable",
        )
    })?;
    let catalogue = Arc::new(ModuleCatalogue::builtin().map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "trusted catalogue unavailable for VPN port allocation",
        )
    })?);
    let normalized_report = config::normalize(desired, &catalogue);
    if normalized_report.blocking {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "prospective configuration cannot be normalized for VPN allocation",
        ));
    }
    let normalized = normalized_report.normalized_snapshot.ok_or_else(|| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "normalized prospective configuration unavailable",
        )
    })?;
    if normalized.desired.read_set != prospective.read_set {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "prospective normalized read set changed",
        ));
    }

    let now = UnixMillis(app.control.now_ms());
    let resolution = ResolutionEvidence {
        now,
        ..Default::default()
    };
    let structural_graph = catalogue.resolve(&normalized, ReleasePhase::R1, &resolution);
    if structural_graph.allocation_bindings.is_empty() {
        return Ok(None);
    }
    let registry = app.registry(ctx)?;
    let observed = observations::collect(app, ctx)?;
    if !observed.issues.is_empty() {
        return Err(journal::failure(
            ctx,
            ErrorCode::ResourceDrift,
            "registered observations are incomplete for VPN allocation planning",
        ));
    }
    let resolution = ResolutionEvidence {
        observations: observed.runtime,
        now,
        ..Default::default()
    };
    let graph = catalogue.resolve(&normalized, ReleasePhase::R1, &resolution);
    if graph.allocation_bindings.is_empty() {
        return Ok(None);
    }
    if !graph.conflicts.is_empty() {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "prospective dependency graph is not fully qualified for VPN allocation",
        ));
    }
    let evidence = observations::reservation_evidence_for(&registry, app.control.as_ref(), ctx)?;
    let allocation_now = UnixMillis(app.control.now_ms());
    if !evidence.complete
        || evidence.observed_at.0 > allocation_now.0
        || evidence.valid_until.0 <= allocation_now.0
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "complete fresh host reservation evidence is unavailable",
        ));
    }
    let expires_at = UnixMillis(
        allocation_now
            .0
            .saturating_add(30_000)
            .min(ctx.deadline.0)
            .min(evidence.valid_until.0),
    );
    if expires_at.0 <= allocation_now.0 {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanExpired,
            "VPN allocation evidence expires before commit",
        ));
    }
    let needs_entropy = graph.allocation_bindings.iter().any(|binding| {
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
    let budget = Budget::bounded_default(expires_at);
    let planned = port_allocation::plan_graph(
        &catalogue,
        &graph,
        &normalized,
        &registry,
        &evidence,
        &EntropyBytes(entropy),
        allocation_now,
        expires_at,
        &budget,
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "managed VPN port candidate could not be planned safely",
        )
    })?;
    if !planned.graph.conflicts.is_empty() {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "managed VPN port candidate conflicts with a host listener",
        ));
    }

    recheck_allocation_inputs(app, selection, &planned.candidate, &registry.revision, ctx)?;
    let needs_approval = planned.candidate.allocations.values().any(|allocation| {
        allocation
            .prior_fields
            .values()
            .any(|prior| prior.is_some_and(|prior| prior != allocation.port))
    });
    let approval = if needs_approval {
        Some(authorize_allocation(app, &planned.candidate, ctx)?)
    } else {
        None
    };
    recheck_allocation_inputs(app, selection, &planned.candidate, &registry.revision, ctx)?;
    let writer = PortSettingsWriter::new(
        &app.config,
        catalogue,
        app.journal.clone(),
        app.installation.instance_id.clone(),
        ctx,
    )?;
    let receipt = writer.commit(
        &planned.candidate,
        &planned.candidate.read_set,
        approval.as_ref(),
        ctx,
    )?;
    Ok(Some(receipt))
}

pub fn dispatch(
    operation: &str,
    paths: &HostPaths,
    args: &[String],
    _yes: bool,
    ctx: &OperationContext,
) -> Result<HostOutput> {
    if ctx.execution_mode == ExecutionMode::Mutating && paths.work.join("minimum/owner").exists() {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "minimum-profile installation requires an explicit migration before full-R1 mutations",
        ));
    }
    if args.len() > 128 || args.iter().any(|arg| arg.len() > 4096) {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "operation payload exceeds bounds",
        ));
    }
    if operation == "install.reconcile" {
        return installation::reconcile(paths, args, ctx).map(HostOutput::Metadata);
    }
    if operation == "install.selection-offer" {
        return installation::selection_offer(paths, args, ctx).map(HostOutput::Metadata);
    }
    let app = HostApplication::open(paths.clone(), ctx)?;
    if operation == "installation.uninstall" {
        return uninstall::execute(&app, args, ctx).map(HostOutput::Metadata);
    }
    if operation == "publication.switch" {
        return publication::switch(&app, args, ctx).map(HostOutput::Metadata);
    }
    if operation == "bundle.validate" {
        return bundle_validation::execute(&app, args, ctx).map(HostOutput::Metadata);
    }
    if matches!(
        operation,
        "modules.remove" | "modules.purge" | "modules.update"
    ) {
        return modules::operation(&app, operation, args, ctx).map(HostOutput::Metadata);
    }
    if operation.starts_with("certs.")
        || operation.starts_with("backup.")
        || matches!(operation, "keys.manage" | "share.export")
    {
        return crate::host_crypto::application::output(&app, operation, args, ctx);
    }
    if matches!(operation, "config.plan" | "config.stage" | "config.apply") {
        let value = match operation {
            "config.plan" => {
                serde_json::json!({"status":"preview","plan":effects::prepare(&app,ctx)?,"writes":false})
            }
            "config.stage" => {
                serde_json::json!({"status":"succeeded","planId":effects::stage(&app,ctx)?})
            }
            "config.apply" => {
                let result = match args {
                    [] => {
                        let plan = effects::prepare(&app, ctx)?;
                        effects::apply(&app, &plan, ctx)?
                    }
                    [option, plan] if option == "--plan" => effects::apply_staged(
                        &app,
                        &PlanId::new(plan).map_err(|_| {
                            journal::failure(ctx, ErrorCode::OperationDenied, "invalid plan ID")
                        })?,
                        ctx,
                    )?,
                    _ => {
                        return Err(journal::failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "invalid apply arguments",
                        ));
                    }
                };
                serde_json::to_value(result).map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "apply result encoding failed",
                    )
                })?
            }
            _ => unreachable!("fixed config operation"),
        };
        return Ok(HostOutput::Metadata(value));
    }
    if operation == "diag.observe" {
        let mut sections = BTreeSet::new();
        let mut verbose = false;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--section" => {
                    index += 1;
                    let section = args.get(index).ok_or_else(|| {
                        journal::failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "diagnostics section value is missing",
                        )
                    })?;
                    if !matches!(
                        section.as_str(),
                        "config"
                            | "docker"
                            | "ports"
                            | "services"
                            | "certs"
                            | "journal"
                            | "runtime"
                            | "tunnel"
                            | "admin"
                    ) {
                        return Err(journal::failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "unknown diagnostics section",
                        ));
                    }
                    sections.insert(section.clone());
                }
                "--verbose" => verbose = true,
                _ => {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "invalid diagnostics arguments",
                    ));
                }
            }
            index += 1;
        }
        return observations::diagnose(&app, &sections, verbose, ctx).map(HostOutput::Metadata);
    }
    if operation == "doctor.observe" {
        if !args.is_empty() {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "doctor observe does not accept operation arguments",
            ));
        }
        let mut report = observations::diagnose(&app, &BTreeSet::new(), false, ctx)?;
        report["operation"] = serde_json::json!("doctor.observe");
        return Ok(HostOutput::Metadata(report));
    }
    if operation == "identity.whoami" {
        if !args.is_empty() {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "whoami does not accept operation arguments",
            ));
        }
        return observations::whoami(&app, ctx).map(HostOutput::Metadata);
    }
    if matches!(
        operation,
        "lifecycle.up"
            | "lifecycle.down"
            | "runtime.start"
            | "runtime.rollback"
            | "services.restart"
    ) {
        return lifecycle::runtime_operation(&app, operation, args, ctx).map(HostOutput::Metadata);
    }
    if operation == "modules.add" {
        let id = match args {
            [id] => ModuleId::new(id).map_err(|_| {
                journal::failure(ctx, ErrorCode::ModuleUnknown, "invalid module identity")
            })?,
            _ => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "module add expects one ID",
                ));
            }
        };
        if ctx.execution_mode == ExecutionMode::ReadOnly {
            return Ok(HostOutput::Metadata(
                serde_json::json!({"status":"preview","moduleId":id,"writes":false,"setup":"unverified"}),
            ));
        }
        let result = setup::add_module(&app, &id, ctx)?;
        return Ok(HostOutput::Metadata(
            serde_json::json!({"status":if result.complete{"succeeded"}else{"partial"},"moduleId":result.module_id,"setupComplete":result.complete,"registryRevision":result.registry_revision,"missingInputs":result.missing_inputs,"error":result.error}),
        ));
    }
    if matches!(operation, "modules.enable" | "modules.disable") {
        let enabled = operation == "modules.enable";
        let mut save_only = false;
        let mut ids = BTreeSet::new();
        for arg in args {
            if arg == "--save-only" {
                save_only = true;
            } else {
                ids.insert(ModuleId::new(arg).map_err(|_| {
                    journal::failure(ctx, ErrorCode::ModuleUnknown, "invalid module identity")
                })?);
            }
        }
        if ids.is_empty() {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "module selection expects at least one module ID",
            ));
        }
        let desired = app.desired(ctx)?;
        let root = desired
            .documents
            .iter()
            .find(|doc| doc.object_id.as_str() == "root")
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "module selection requires coherent root config",
                )
            })?;
        let source = config::FsConfigSource::open(&app.paths.config).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "configuration source unavailable",
            )
        })?;
        let candidate = crate::selection::prepare(&source, ids, enabled, &root.revision, ctx)?;
        if ctx.execution_mode == ExecutionMode::ReadOnly {
            return Ok(HostOutput::Metadata(serde_json::json!({
                "status": "preview",
                "operation": operation,
                "writes": false,
                "enabled": enabled,
                "changed": candidate.changed(),
            })));
        }
        if enabled {
            if let Some(recovered) = recover_pending_port_allocation(&app, ctx)? {
                return Ok(HostOutput::Metadata(recovered));
            }
        }
        let allocation = if enabled {
            persist_enable_allocation(&app, &candidate, ctx)?
        } else {
            None
        };
        let commit =
            crate::selection::SelectionWriter::new(app.config.clone(), app.journal.clone())
                .commit(&candidate, &root.revision, ctx)?;
        if save_only || !commit.changed {
            return Ok(HostOutput::Metadata(serde_json::json!({
                "status": "succeeded",
                "operation": operation,
                "saved": true,
                "changed": commit.changed,
                "revision": commit.revision,
                "allocation": allocation,
            })));
        }
        let plan = effects::prepare(&app, ctx)?;
        let result = effects::apply(&app, &plan, ctx)?;
        return Ok(HostOutput::Metadata(serde_json::to_value(result).map_err(
            |_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "apply result encoding failed",
                )
            },
        )?));
    }
    Err(journal::failure(
        ctx,
        ErrorCode::ModuleIncomplete,
        "operation composition is not yet complete",
    ))
}
