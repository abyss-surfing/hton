//! Short-lived local CLI transport. No native or mutation authority is composed here.
use hton_contracts::{
    config::CheckIssue,
    operations::{
        ActorContext, ActorId, ActorSource, CancellationId, ErrorCode, EventSinkId, ExecutionMode,
        FormatVersion, InstanceId, OperationContext, OperationId, RequestId, StateRevision,
        UnixMillis,
    },
};
use hton_core::{catalog::ModuleCatalogue, config, lifecycle, registry::RegistryStore};
use hton_storage::SecureDir;
use serde::Serialize;
use std::{
    collections::BTreeSet,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckOutput {
    api_version: u32,
    operation: &'static str,
    status: &'static str,
    validation_scope: &'static str,
    blocking: bool,
    issues: Vec<CheckIssue>,
    unverified: [&'static str; 4],
}

struct Request {
    operation: &'static str,
    root: PathBuf,
    work: Option<PathBuf>,
    json: bool,
    quiet: bool,
    dry_run: bool,
    yes: bool,
    arguments: Vec<String>,
}
fn request() -> Result<Request, ()> {
    let args: Vec<_> = std::env::args_os().skip(1).take(129).collect();
    if args.len() > 128 || args.iter().any(|arg| arg.len() > 4096) {
        return Err(());
    }
    let args: Vec<_> = args
        .iter()
        .map(|arg| arg.to_str().ok_or(()))
        .collect::<Result<_, _>>()?;
    const OPERATIONS: &[&str] = &[
        "config.check",
        "modules.list",
        "install.reconcile",
        "install.selection-offer",
        "config.plan",
        "config.stage",
        "config.apply",
        "modules.add",
        "modules.enable",
        "modules.disable",
        "modules.remove",
        "modules.purge",
        "modules.update",
        "lifecycle.up",
        "lifecycle.down",
        "diag.observe",
        "keys.manage",
        "share.export",
        "certs.issue",
        "certs.status",
        "certs.list",
        "certs.renew",
        "services.restart",
        "publication.switch",
        "foreign.restore",
        "backup.create",
        "backup.initialize",
        "backup.list",
        "backup.restore",
        "doctor.observe",
        "doctor.fix",
        "doctor.watch",
        "installation.uninstall",
        "identity.whoami",
        "bundle.validate",
        "runtime.start",
        "runtime.rollback",
    ];
    let operation = *OPERATIONS
        .iter()
        .find(|operation| Some(**operation) == args.first().copied())
        .ok_or(())?;
    let mut root = None;
    let mut work = None;
    let mut json = false;
    let mut quiet = false;
    let mut index = 1;
    let mut dry_run = false;
    let mut yes = false;
    let mut arguments = Vec::new();
    while index < args.len() {
        match args[index] {
            "--config-root" if root.is_none() => {
                index += 1;
                let value = args.get(index).ok_or(())?;
                let path = PathBuf::from(value);
                if !path.is_absolute() {
                    return Err(());
                }
                root = Some(path);
            }
            "--work-root" if work.is_none() => {
                index += 1;
                let path = PathBuf::from(args.get(index).ok_or(())?);
                if !path.is_absolute() {
                    return Err(());
                }
                work = Some(path);
            }
            "--json" if !json => json = true,
            "--quiet" if !quiet => quiet = true,
            "--dry-run" => dry_run = true,
            "--yes" => yes = true,
            "--" => {
                arguments = args[index + 1..]
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect();
                break;
            }
            _ => return Err(()),
        }
        index += 1;
    }
    if operation != "config.check" && work.is_none() {
        return Err(());
    }
    Ok(Request {
        operation,
        work,
        root: root.ok_or(())?,
        json,
        quiet,
        dry_run,
        yes,
        arguments,
    })
}
fn check(request: Request) -> Result<u8, ()> {
    let mut issues = Vec::new();
    let mut checked = false;
    match config::FsConfigSource::open(&request.root) {
        Ok(source) => {
            let desired = config::load(&source, &BTreeSet::from([FormatVersion(1)]));
            issues.extend(desired.errors);
            if let Some(snapshot) = desired.snapshot {
                match ModuleCatalogue::builtin() {
                    Ok(catalogue) => {
                        let report = config::normalize(&snapshot, &catalogue);
                        issues.extend(report.issues);
                        checked = !report.blocking;
                    }
                    Err(issue) => issues.push(issue),
                }
            }
        }
        Err(_) => issues.push(config::issue(
            ErrorCode::IoUnavailable,
            None,
            None,
            "Configuration root is unavailable or unsafe",
        )),
    }
    let blocking = !checked
        || issues
            .iter()
            .any(|issue| issue.severity == hton_contracts::config::Severity::Error);
    let output = CheckOutput {
        api_version: 1,
        operation: "config.check",
        status: if blocking { "failed" } else { "unverified" },
        validation_scope: "syntax-and-normalization",
        blocking,
        issues,
        unverified: [
            "installed-registry",
            "module-semantics",
            "secret-versions",
            "native-observations",
        ],
    };
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    if request.json {
        serde_json::to_writer(&mut writer, &output).map_err(|_| ())?;
        writeln!(writer).map_err(|_| ())?;
    } else {
        if !request.quiet || blocking {
            writeln!(
                writer,
                "config check: {} (syntax-and-normalization)",
                output.status
            )
            .map_err(|_| ())?;
        }
        for issue in &output.issues {
            writeln!(writer, "{:?}: {}", issue.code, issue.message).map_err(|_| ())?;
        }
        // Never hide missing validation evidence behind quiet output.
        writeln!(writer, "unverified: {}", output.unverified.join(", ")).map_err(|_| ())?;
    }
    writer.flush().map_err(|_| ())?;
    Ok(u8::from(blocking))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InventoryOutput {
    api_version: u32,
    operation: &'static str,
    status: &'static str,
    registry_status: &'static str,
    instance_id: Option<InstanceId>,
    revision: Option<StateRevision>,
    desired_status: &'static str,
    modules: Vec<lifecycle::ModuleInventoryEntry>,
    issues: Vec<CheckIssue>,
    unverified: [&'static str; 1],
}
fn local_read_context() -> OperationContext {
    let uid = rustix::process::geteuid().as_raw();
    let pid = std::process::id();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    OperationContext {
        operation_id: OperationId::new(format!("read.{uid}.{pid}")).expect("local IDs"),
        actor: ActorContext {
            actor_id: ActorId::new(format!("os.uid.{uid}")).expect("local IDs"),
            source: ActorSource::Local,
            granted_scopes: BTreeSet::new(),
            local_approval_refs: BTreeSet::new(),
            request_id: RequestId::new(format!("read.{uid}.{pid}")).expect("local IDs"),
        },
        execution_mode: ExecutionMode::ReadOnly,
        deadline: UnixMillis(now.saturating_add(30_000)),
        cancellation: CancellationId::new("local.read").expect("static ID"),
        event_sink: EventSinkId::new("local.stderr").expect("static ID"),
    }
}
fn inventory(request: Request) -> Result<u8, ()> {
    let ctx = local_read_context();
    let mut issues = Vec::new();
    let mut desired = None;
    match config::FsConfigSource::open(&request.root) {
        Ok(source) => {
            let read = config::load(&source, &BTreeSet::from([FormatVersion(1)]));
            issues.extend(read.errors);
            desired = read.snapshot;
        }
        Err(_) => issues.push(config::issue(
            ErrorCode::IoUnavailable,
            None,
            None,
            "Desired configuration is unavailable or unsafe",
        )),
    }
    let mut registry = None;
    let registry_status = match SecureDir::open(request.work.as_ref().ok_or(())?) {
        Ok(directory) => match RegistryStore::inspect(&directory, &ctx) {
            Ok(Some(snapshot)) => {
                registry = Some(snapshot);
                "available"
            }
            Ok(None) => "absent",
            Err(error) => {
                issues.push(config::issue(
                    error.code,
                    None,
                    None,
                    "Registry is unavailable or corrupt; originals retained",
                ));
                "unavailable"
            }
        },
        Err(hton_storage::Error::NotFound) => "absent",
        Err(_) => {
            issues.push(config::issue(
                ErrorCode::IoUnavailable,
                None,
                None,
                "Registry directory is unavailable or unsafe",
            ));
            "unavailable"
        }
    };
    if registry
        .as_ref()
        .is_some_and(|snapshot| !snapshot.diagnostics.is_empty())
    {
        issues.push(config::issue(
            ErrorCode::RecoveryRequired,
            None,
            None,
            "Registry retains diagnostics requiring reconciliation",
        ));
    }
    let catalogue = match ModuleCatalogue::builtin() {
        Ok(catalogue) => catalogue,
        Err(issue) => {
            issues.push(issue);
            return write_inventory(
                request,
                InventoryOutput {
                    api_version: 1,
                    operation: "modules.list",
                    status: "failed",
                    registry_status,
                    instance_id: registry
                        .as_ref()
                        .map(|snapshot| snapshot.instance_id.clone()),
                    revision: registry.as_ref().map(|snapshot| snapshot.revision),
                    desired_status: if desired.is_some() {
                        "available"
                    } else {
                        "unavailable"
                    },
                    modules: Vec::new(),
                    issues,
                    unverified: ["native-state"],
                },
            );
        }
    };
    let registry_view = match registry.as_ref() {
        Some(snapshot) => lifecycle::RegistryInventory::Available(snapshot),
        None if registry_status == "absent" => lifecycle::RegistryInventory::Absent,
        None => lifecycle::RegistryInventory::Unavailable,
    };
    let modules = lifecycle::list(&catalogue, desired.as_ref(), registry_view);
    let status = if issues
        .iter()
        .any(|issue| issue.severity == hton_contracts::config::Severity::Error)
    {
        "failed"
    } else {
        "unverified"
    };
    write_inventory(
        request,
        InventoryOutput {
            api_version: 1,
            operation: "modules.list",
            status,
            registry_status,
            instance_id: registry
                .as_ref()
                .map(|snapshot| snapshot.instance_id.clone()),
            revision: registry.as_ref().map(|snapshot| snapshot.revision),
            desired_status: if desired.is_some() {
                "available"
            } else {
                "unavailable"
            },
            modules,
            issues,
            unverified: ["native-state"],
        },
    )
}
fn write_inventory(request: Request, output: InventoryOutput) -> Result<u8, ()> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    if request.json {
        serde_json::to_writer(&mut writer, &output).map_err(|_| ())?;
        writeln!(writer).map_err(|_| ())?;
    } else {
        if !request.quiet {
            writeln!(
                writer,
                "modules list: {} registry={} desired={}",
                output.status, output.registry_status, output.desired_status
            )
            .map_err(|_| ())?;
        }
        for entry in &output.modules {
            writeln!(
                writer,
                "{} registered={:?} membership={:?} setup={:?} enabled={:?} native=unverified",
                entry.module_id,
                entry.registered,
                entry.membership,
                entry.setup_state,
                entry.desired_enabled
            )
            .map_err(|_| ())?;
        }
        for issue in &output.issues {
            writeln!(writer, "{:?}: {}", issue.code, issue.message).map_err(|_| ())?;
        }
        writeln!(writer, "unverified: native-state").map_err(|_| ())?;
    }
    writer.flush().map_err(|_| ())?;
    Ok(u8::from(output.status == "failed"))
}
fn run(request: Request) -> Result<u8, ()> {
    match request.operation {
        "config.check" => check(request),
        "modules.list" => inventory(request),
        _ => host_operation(request),
    }
}
fn host_operation(request: Request) -> Result<u8, ()> {
    let paths = hton_core::host::HostPaths {
        config: request.root,
        work: request.work.ok_or(())?,
    };
    let mutating = !request.dry_run
        && !matches!(
            request.operation,
            "config.plan"
                | "install.selection-offer"
                | "diag.observe"
                | "certs.status"
                | "certs.list"
                | "backup.list"
                | "doctor.observe"
                | "identity.whoami"
        );
    let ctx = hton_core::host::context(request.operation, mutating).map_err(|_| ())?;
    let mut arguments = request.arguments.clone();
    if request.operation == "share.export"
        && request.json
        && !arguments.iter().any(|arg| arg == "--format")
    {
        arguments.extend(["--format".to_owned(), "json".to_owned()]);
    }
    let result =
        hton_core::host::dispatch(request.operation, &paths, &arguments, request.yes, &ctx);
    let (output, failure_code) = match result {
        Ok(hton_core::host::HostOutput::Metadata(value)) => (value, 1),
        Ok(hton_core::host::HostOutput::Protected(bytes)) => {
            let stdout = io::stdout();
            let mut writer = stdout.lock();
            writer.write_all(&bytes).map_err(|_| ())?;
            writer.flush().map_err(|_| ())?;
            return Ok(0);
        }
        Err(error) => {
            let failure_code = if error.code == ErrorCode::ApprovalRequired {
                4
            } else {
                1
            };
            (
                serde_json::json!({"apiVersion":1,"operation":request.operation,"status":"failed","error":error}),
                failure_code,
            )
        }
    };
    let success = !matches!(
        output.get("status").and_then(serde_json::Value::as_str),
        Some("failed" | "recoveryRequired" | "recovery_required" | "partial")
    );
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    if request.json {
        serde_json::to_writer(&mut writer, &output).map_err(|_| ())?;
    } else if matches!(
        request.operation,
        "diag.observe" | "doctor.observe" | "identity.whoami"
    ) {
        render_read_only_report(&mut writer, request.operation, &output, request.quiet)?;
    } else {
        serde_json::to_writer_pretty(&mut writer, &output).map_err(|_| ())?;
    }
    writeln!(writer).map_err(|_| ())?;
    writer.flush().map_err(|_| ())?;
    Ok(if success { 0 } else { failure_code })
}

fn render_read_only_report(
    writer: &mut impl Write,
    operation: &str,
    output: &serde_json::Value,
    quiet: bool,
) -> Result<(), ()> {
    let status = output
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unverified");
    if !quiet || status == "failed" {
        writeln!(writer, "{}: {status}", operation).map_err(|_| ())?;
    }
    if let Some(message) = output
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(serde_json::Value::as_str)
    {
        writeln!(writer, "error: {message}").map_err(|_| ())?;
    }
    match operation {
        "diag.observe" | "doctor.observe" => {
            if let Some(sections) = output
                .get("observedSections")
                .and_then(serde_json::Value::as_array)
            {
                writeln!(
                    writer,
                    "observed sections: {}",
                    sections
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .map_err(|_| ())?;
            }
            if let Some(counts) = output.get("counts").and_then(serde_json::Value::as_object) {
                for (key, value) in counts {
                    if let Some(count) = value.as_u64() {
                        writeln!(writer, "{key}: {count}").map_err(|_| ())?;
                    }
                }
            }
            if let Some(sections) = output
                .get("unsupportedSections")
                .and_then(serde_json::Value::as_array)
            {
                let names: Vec<_> = sections
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect();
                if !names.is_empty() {
                    writeln!(writer, "unverified sections: {}", names.join(", "))
                        .map_err(|_| ())?;
                }
            }
            if let Some(fields) = output
                .get("unverified")
                .and_then(serde_json::Value::as_array)
            {
                let names: Vec<_> = fields
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect();
                if !names.is_empty() {
                    writeln!(writer, "unverified: {}", names.join(", ")).map_err(|_| ())?;
                }
            }
        }
        "identity.whoami" => {
            if let Some(instance) = output.get("instanceId").and_then(serde_json::Value::as_str) {
                writeln!(writer, "HTON instance: {instance}").map_err(|_| ())?;
            }
            match output.get("externalIpv4") {
                Some(serde_json::Value::String(address)) => {
                    writeln!(writer, "external IPv4: {address}").map_err(|_| ())?
                }
                _ => writeln!(writer, "external IPv4: unverified").map_err(|_| ())?,
            }
            if let Some(domains) = output.get("domains").and_then(serde_json::Value::as_array) {
                for domain in domains {
                    if let Some(name) = domain.get("queryName").and_then(serde_json::Value::as_str)
                    {
                        let addresses = domain
                            .get("addresses")
                            .and_then(serde_json::Value::as_array)
                            .map(|values| {
                                values
                                    .iter()
                                    .filter_map(serde_json::Value::as_str)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .filter(|value| !value.is_empty())
                            .unwrap_or_else(|| "unverified".to_owned());
                        writeln!(writer, "DNS A {name}: {addresses}").map_err(|_| ())?;
                    }
                }
            }
            if let Some(fields) = output.get("unprobed").and_then(serde_json::Value::as_array) {
                let names: Vec<_> = fields
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect();
                writeln!(writer, "unprobed: {}", names.join(", ")).map_err(|_| ())?;
            }
        }
        _ => return Err(()),
    }
    Ok(())
}

fn main() -> ExitCode {
    let request = match request() {
        Ok(request) => request,
        Err(()) => {
            eprintln!("hton-core: invalid operation or arguments");
            return ExitCode::from(2);
        }
    };
    match run(request) {
        Ok(code) => ExitCode::from(code),
        Err(()) => ExitCode::FAILURE,
    }
}
