//! Validated logging projection, independent of runtime's YAML-free consumption.
use super::*;
use crate::journal::{deadline, ensure_mutating};
use serde_json::json;

pub fn project(
    app: &HostApplication,
    transaction: &TransactionId,
    ctx: &OperationContext,
) -> Result<Arc<hton_logging::LogSink>> {
    ensure_mutating(ctx, app.control.as_ref())?;
    let tx = app.journal.read(transaction, ctx)?;
    if tx.operation_id != ctx.operation_id || tx.actor != ctx.actor {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "logging projection requires exact operation journal",
        ));
    }
    let desired = app.desired(ctx)?;
    let snapshot = desired.snapshot.ok_or_else(|| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "logging settings require valid desired configuration",
        )
    })?;
    let logging = snapshot.platform.settings.get("logging");
    let directory = logging
        .and_then(|value| value.get("directory"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("/var/logs/hton");
    let path = Path::new(directory);
    if !path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "logging directory must be an absolute protected path",
        ));
    }
    let max_bytes = logging
        .and_then(|value| value.get("maxBytes"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(67_108_864);
    let days = logging
        .and_then(|value| value.get("retentionDays"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(7);
    let level = logging
        .and_then(|value| value.get("level"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("info");
    if max_bytes < 4096 || max_bytes > 1_073_741_824 || !(1..=7).contains(&days) {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "logging policy exceeds bounded limits",
        ));
    }
    let severity = match level {
        "error" => hton_logging::Severity::Error,
        "warn" => hton_logging::Severity::Warning,
        "info" => hton_logging::Severity::Information,
        "debug" => hton_logging::Severity::Debug,
        _ => {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "logging level is unsupported",
            ));
        }
    };
    let policy = hton_logging::LoggingPolicy::bounded(severity, max_bytes, days).map_err(|_| {
        journal::failure(ctx, ErrorCode::ConfigSyntax, "logging ring policy invalid")
    })?;
    let projection =
        json!({"directory":directory,"maxBytes":max_bytes,"retentionDays":days,"level":level});
    let bytes = serde_json::to_vec(&projection).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "logging projection encoding failed",
        )
    })?;
    let runtime = app.work.child("runtime").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "runtime inventory unavailable",
        )
    })?;
    let locked = runtime
        .lock(deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "logging projection lock unavailable",
            )
        })?;
    let old = locked.read("logging.json", 16384).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "prior logging projection unavailable",
        )
    })?;
    locked
        .compare_exchange("logging.json", old.as_deref(), &bytes)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "logging projection durability uncertain",
            )
        })?;
    let logs = SecureDir::create(path).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "protected log directory unavailable",
        )
    })?;
    hton_logging::LogSink::configure(policy, logs)
        .map(Arc::new)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "bounded log sink unavailable",
            )
        })
}
