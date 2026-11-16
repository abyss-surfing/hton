//! Independent local runtime entry. Status has no native mutation authority.
use hton_runtime::{
    Clock,
    host::{HostRuntime, SystemClock},
    state::RuntimeStateStore,
};
use hton_runtime_contract::*;
use hton_storage::SecureDir;
use serde_json::{Value, json};
use std::{
    io::{self, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

struct Request {
    root: PathBuf,
    json: bool,
    quiet: bool,
}
fn request(args: &[std::ffi::OsString]) -> Result<Request, ()> {
    if args.len() > 12 || args.iter().any(|value| value.len() > 4096) {
        return Err(());
    }
    let args: Vec<_> = args
        .iter()
        .map(|value| value.to_str().ok_or(()))
        .collect::<Result<_, _>>()?;
    let mut root = None;
    let mut json = false;
    let mut quiet = false;
    let mut index = 1;
    while index < args.len() {
        match args[index] {
            "--state-root" if root.is_none() => {
                index += 1;
                let path = PathBuf::from(args.get(index).ok_or(())?);
                if !path.is_absolute() {
                    return Err(());
                }
                root = Some(path);
            }
            "--json" if !json => json = true,
            "--quiet" if !quiet => quiet = true,
            "--dry-run" => (),
            _ => return Err(()),
        }
        index += 1;
    }
    Ok(Request {
        root: root.ok_or(())?,
        json,
        quiet,
    })
}
fn inspect(root: &std::path::Path) -> Result<Option<RuntimeState>, ErrorCode> {
    match SecureDir::open(root) {
        Ok(directory) => RuntimeStateStore::inspect(&directory),
        Err(hton_storage::Error::NotFound) => Ok(None),
        Err(_) => Err(ErrorCode::Io),
    }
}
fn run(request: Request) -> Result<bool, ()> {
    let result = inspect(&request.root);
    let state = result.as_ref().ok().and_then(Option::as_ref);
    let status = match &result {
        Ok(Some(_)) => "unverified",
        Ok(None) => "absent",
        Err(_) => "failed",
    };
    let resources: Vec<_> = state
        .into_iter()
        .flat_map(|state| state.resources.as_slice())
        .map(|receipt| {
            json!({
                "resourceId": receipt.resource.resource_id,
                "recordedStatus": receipt.status,
                "recordedRestartPolicy": receipt.restart_policy,
                "recordedEffectiveBundle": receipt.effective_bundle,
                "observedAtMs": receipt.timestamp_ms,
                "recordedRunEpoch": receipt.run_epoch,
            })
        })
        .collect();
    let lease = state.and_then(|state| state.lease.as_ref()).map(|lease| {
        json!({
            "leaseId": lease.lease_id, "candidate": lease.candidate, "runEpoch": lease.run_epoch,
            "deadlineMs": lease.deadline_ms, "recordedState": lease.state,
        })
    });
    let output = json!({
        "apiVersion": 1, "operation": "runtime.status", "status": status,
        "instanceId": state.map(|state| &state.instance_id),
        "revision": state.map(|state| state.revision),
        "runIntent": state.map(|state| state.run_intent),
        "runEpoch": state.map(|state| state.run_epoch),
        "accepted": state.and_then(|state| state.accepted.as_ref()),
        "previousCompatible": state.and_then(|state| state.previous_compatible.as_ref()),
        "trial": state.and_then(|state| state.trial.as_ref()),
        "lease": lease,
        "maintenance": state.and_then(|state| state.maintenance.as_ref()),
        "recordedResources": resources,
        "errorCode": result.as_ref().err(),
        "unverified": ["native-observations", "bundle-admissibility"],
    });
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    if request.json {
        serde_json::to_writer(&mut writer, &output).map_err(|_| ())?;
        writeln!(writer).map_err(|_| ())?;
    } else {
        if !request.quiet || result.is_err() {
            writeln!(writer, "runtime status: {status}").map_err(|_| ())?;
        }
        for key in [
            "instanceId",
            "revision",
            "runIntent",
            "runEpoch",
            "accepted",
            "previousCompatible",
            "trial",
            "lease",
            "maintenance",
            "errorCode",
        ] {
            if output[key] != Value::Null {
                writeln!(writer, "{key}: {}", output[key]).map_err(|_| ())?;
            }
        }
        for receipt in resources {
            writeln!(writer, "recorded resource: {receipt}").map_err(|_| ())?;
        }
        writeln!(
            writer,
            "unverified: native-observations, bundle-admissibility"
        )
        .map_err(|_| ())?;
    }
    writer.flush().map_err(|_| ())?;
    Ok(result.is_ok())
}
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).take(13).collect();
    match args.first().and_then(|argument| argument.to_str()) {
        Some("execute" | "boot" | "guardian" | "collect") => return host_entry(&args),
        Some("status") => (),
        _ => {
            eprintln!("hton-runtime: invalid operation");
            return ExitCode::from(2);
        }
    }
    let request = match request(&args) {
        Ok(request) => request,
        Err(()) => {
            eprintln!("hton-runtime: invalid arguments");
            return ExitCode::from(2);
        }
    };
    match run(request) {
        Ok(true) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstanceRecord {
    schema_version: u32,
    instance_id: InstanceId,
}
fn host_entry(args: &[std::ffi::OsString]) -> ExitCode {
    match host_run(args) {
        Ok(code) => code,
        Err(code) => {
            let _ = writeln!(io::stdout(), "{}", json!({"code":code,"status":"failed"}));
            ExitCode::FAILURE
        }
    }
}
fn host_run(args: &[std::ffi::OsString]) -> Result<ExitCode, ErrorCode> {
    if rustix::process::geteuid().as_raw() != 0 {
        return Err(ErrorCode::ApprovalRequired);
    }
    if args.len() > 12 || args.iter().any(|arg| arg.len() > 4096) {
        return Err(ErrorCode::InvalidManifest);
    }
    let args: Vec<_> = args
        .iter()
        .map(|arg| arg.to_str().ok_or(ErrorCode::InvalidManifest))
        .collect::<Result<_, _>>()?;
    let command = *args.first().ok_or(ErrorCode::InvalidManifest)?;
    let mut state = None;
    let mut bundles = None;
    let mut authorities = None;
    let mut container = None;
    let mut index = 1;
    while index < args.len() {
        let key = args[index];
        index += 1;
        let value = *args.get(index).ok_or(ErrorCode::InvalidManifest)?;
        match key {
            "--state-root" if state.is_none() => state = Some(PathBuf::from(value)),
            "--bundle-root" if bundles.is_none() => bundles = Some(PathBuf::from(value)),
            "--authority-root" if authorities.is_none() => authorities = Some(PathBuf::from(value)),
            "--container" if command == "collect" && container.is_none() => {
                container = Some(value.to_owned())
            }
            _ => return Err(ErrorCode::InvalidManifest),
        }
        index += 1;
    }
    let state = state.ok_or(ErrorCode::InvalidManifest)?;
    let bundles = bundles.ok_or(ErrorCode::InvalidManifest)?;
    let authorities = authorities.ok_or(ErrorCode::InvalidManifest)?;
    if !state.is_absolute() || !bundles.is_absolute() || !authorities.is_absolute() {
        return Err(ErrorCode::InvalidManifest);
    }
    let directory = SecureDir::open_private(&state).map_err(|_| ErrorCode::Io)?;
    let instance = match RuntimeStateStore::inspect(&directory)? {
        Some(state) => state.instance_id,
        None => {
            let bytes = directory
                .read("instance.json", 64 * 1024)
                .map_err(|_| ErrorCode::Io)?
                .ok_or(ErrorCode::MissingArtifact)?;
            let record: InstanceRecord = decode(&bytes).map_err(|_| ErrorCode::InvalidManifest)?;
            if record.schema_version != 1 {
                return Err(ErrorCode::UnsupportedVersion);
            }
            record.instance_id
        }
    };
    let host = HostRuntime::open(&state, &bundles, &authorities, instance)?;
    let logs = host.logging()?;
    if command == "execute" {
        let mut bytes = Vec::new();
        io::stdin()
            .take((MAX_WIRE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| ErrorCode::Io)?;
        let request: ExecuteRequest = decode(&bytes).map_err(|_| ErrorCode::InvalidManifest)?;
        let result = host.execute(&request, &logs);
        match result {
            Ok(result) => {
                let success = result.status == ResultStatus::Succeeded;
                serde_json::to_writer(io::stdout().lock(), &result).map_err(|_| ErrorCode::Io)?;
                writeln!(io::stdout()).map_err(|_| ErrorCode::Io)?;
                return Ok(if success {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                });
            }
            Err(error) => {
                serde_json::to_writer(io::stdout().lock(), &error).map_err(|_| ErrorCode::Io)?;
                writeln!(io::stdout()).map_err(|_| ErrorCode::Io)?;
                return Ok(ExitCode::FAILURE);
            }
        }
    }
    let context = || OperationContext {
        operation_id: OperationId::new(format!("runtime-{}-{}", command, std::process::id()))
            .expect("OS operation ID"),
        caller: CallerId::new(format!("os.uid.{}", rustix::process::geteuid().as_raw()))
            .expect("OS caller ID"),
        budget: Budget {
            deadline_ms: SystemClock.now_ms().saturating_add(120_000),
            max_concurrency: std::num::NonZeroU16::new(4).expect("fixed budget"),
            max_bytes: std::num::NonZeroU32::new(MAX_WIRE_BYTES as u32).expect("fixed budget"),
            max_observations: std::num::NonZeroU16::new(1024).expect("fixed budget"),
        },
        cancellation_id: None,
    };
    match command {
        "boot" => {
            let result = host.boot(&context(), &logs).map_err(|e| e.code)?;
            serde_json::to_writer(io::stdout().lock(), &result).map_err(|_| ErrorCode::Io)?;
            writeln!(io::stdout()).map_err(|_| ErrorCode::Io)?;
            Ok(ExitCode::SUCCESS)
        }
        "guardian" => loop {
            let ctx = context();
            if let Err(error) = host.monitor_once(&ctx, &logs) {
                let _ = logs.emit(
                    hton_logging::Component::Guardian,
                    hton_logging::EventContext::default(),
                    hton_logging::Severity::Error,
                    hton_logging::EventCode::RecoveryRequired,
                    hton_logging::SafeFields {
                        reason: Some(hton_logging::FailureReason::RecoveryRequired),
                        ..Default::default()
                    },
                );
                if error.code == ErrorCode::Io {
                    return Err(error.code);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        },
        "collect" => host
            .collect(
                &container.ok_or(ErrorCode::InvalidManifest)?,
                &context(),
                &logs,
            )
            .map(|_| ExitCode::SUCCESS)
            .map_err(|e| e.code),
        _ => Err(ErrorCode::InvalidManifest),
    }
}
