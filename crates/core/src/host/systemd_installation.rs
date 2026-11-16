//! Fixed platform-unit delivery; service lifecycle remains owned by runtime.
use super::HostApplication;
use crate::{
    config,
    journal::{self, Intent, TerminalStatus},
};
use hton_contracts::operations::*;
use hton_host::{Budget, Cancellation, HostObserver};
use hton_storage::SecureDir;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const UNITS: [(&str, &[u8]); 5] = [
    (
        "hton-data.service",
        include_bytes!("../../../../platform/systemd/hton-data.service"),
    ),
    (
        "hton-guardian.service",
        include_bytes!("../../../../platform/systemd/hton-guardian.service"),
    ),
    (
        "hton-acme.service",
        include_bytes!("../../../../platform/systemd/hton-acme.service"),
    ),
    (
        "hton-acme.timer",
        include_bytes!("../../../../platform/systemd/hton-acme.timer"),
    ),
    (
        "hton-native-log@.service",
        include_bytes!("../../../../platform/systemd/hton-native-log@.service"),
    ),
];

struct Cancel<'a>(&'a HostApplication, &'a OperationContext);
impl Cancellation for Cancel<'_> {
    fn cancelled(&self) -> bool {
        self.0.control.is_cancelled(&self.1.cancellation)
    }
}

fn link_path(name: &str) -> PathBuf {
    Path::new("/etc/systemd/system").join(name)
}
fn template_path(name: &str) -> PathBuf {
    Path::new("/opt/hton/work/systemd").join(name)
}

/// A foreign unit is never replaced, including a regular file with equal bytes.
fn check_links(require_present: bool, ctx: &OperationContext) -> Result<()> {
    let parent = Path::new("/etc/systemd/system");
    let info = std::fs::symlink_metadata(parent).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "system unit directory unavailable",
        )
    })?;
    use std::os::unix::fs::MetadataExt;
    if !info.is_dir() || info.uid() != 0 || info.mode() & 0o022 != 0 {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "system unit directory is unsafe",
        ));
    }
    for (name, _) in UNITS {
        match std::fs::symlink_metadata(link_path(name)) {
            Ok(info) if info.file_type().is_symlink() && info.uid() == 0 => {
                if std::fs::read_link(link_path(name)).ok().as_ref() != Some(&template_path(name)) {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "platform unit path is owned by another installation",
                    ));
                }
            }
            Ok(_) => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ResourceDrift,
                    "platform unit path is occupied; original retained",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !require_present => {}
            Err(_) => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "platform unit link unavailable",
                ));
            }
        }
    }
    Ok(())
}

fn check_templates(
    directory: &SecureDir,
    require_present: bool,
    ctx: &OperationContext,
) -> Result<()> {
    for (name, expected) in UNITS {
        let bytes = directory.read(name, 65_536).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ResourceDrift,
                "protected platform template unreadable",
            )
        })?;
        if bytes.as_deref().is_some_and(|bytes| bytes != expected) {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "platform template changed; explicit upgrade required",
            ));
        }
        if require_present && bytes.is_none() {
            return Err(journal::failure(
                ctx,
                ErrorCode::ResourceDrift,
                "platform template disappeared",
            ));
        }
    }
    Ok(())
}

/// Read-only ownership check before full uninstall can stop any data service.
pub(crate) fn preflight_uninstall(app: &HostApplication, ctx: &OperationContext) -> Result<()> {
    check_links(false, ctx)?;
    match app.work.child("systemd") {
        Ok(directory) => check_templates(&directory, true, ctx),
        Err(hton_storage::Error::NotFound) => {
            for (name, _) in UNITS {
                match std::fs::symlink_metadata(link_path(name)) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    _ => {
                        return Err(journal::failure(
                            ctx,
                            ErrorCode::ResourceDrift,
                            "platform unit ownership inventory is missing or inaccessible",
                        ));
                    }
                }
            }
            Ok(())
        }
        Err(_) => Err(journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "platform template inventory unavailable",
        )),
    }
}

/// Called by the journaled full uninstall coordinator after Runtime shutdown and
/// exact native-resource removal. Protected template bytes remain recoverable.
pub(crate) fn uninstall(app: &HostApplication, ctx: &OperationContext) -> Result<()> {
    journal::ensure_mutating(ctx, app.control.as_ref())?;
    app.paths.validate(true).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "unit removal requires fixed installation paths",
        )
    })?;
    if rustix::process::geteuid().as_raw() != 0 {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "unit removal requires local root",
        ));
    }
    preflight_uninstall(app, ctx)?;
    let state = app.runtime(ctx)?;
    if state.run_intent != hton_runtime_contract::RunIntent::Stopped || state.trial.is_some() {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationConflict,
            "whole-server stopped state is required before unit removal",
        ));
    }
    let directory = match app.work.child("systemd") {
        Ok(directory) => directory,
        Err(hton_storage::Error::NotFound) => {
            for (name, _) in UNITS {
                if std::fs::symlink_metadata(link_path(name)).is_ok() {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::ResourceDrift,
                        "platform unit ownership record is missing",
                    ));
                }
            }
            return Ok(());
        }
        Err(_) => {
            return Err(journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "platform template inventory unavailable",
            ));
        }
    };
    let _lock = directory
        .lock(journal::deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "platform unit maintenance is already running",
            )
        })?;
    check_templates(&directory, true, ctx)?;
    check_links(false, ctx)?;
    let budget = Budget::new(
        Instant::now()
            + Duration::from_millis(
                ctx.deadline
                    .0
                    .saturating_sub(app.control.now_ms())
                    .min(30_000),
            ),
        65_536,
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::OperationDeadline,
            "unit removal budget expired",
        )
    })?;
    let control = HostObserver::new().systemd_control();
    control
        .stop_platform_units(budget, &Cancel(app, ctx))
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "platform unit shutdown incomplete; unit links retained",
            )
        })?;
    for (name, _) in UNITS {
        journal::ensure_mutating(ctx, app.control.as_ref())?;
        check_links(false, ctx)?;
        match std::fs::remove_file(link_path(name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "platform link removal incomplete; repeat uninstall",
                ));
            }
        }
    }
    std::fs::File::open("/etc/systemd/system")
        .and_then(|directory| directory.sync_all())
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "platform link removal durability uncertain",
            )
        })?;
    control
        .reload_manager(budget, &Cancel(app, ctx))
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "system manager reload incomplete; repeat uninstall",
            )
        })?;
    Ok(())
}

pub fn install(app: &HostApplication, ctx: &OperationContext) -> Result<()> {
    journal::ensure_mutating(ctx, app.control.as_ref())?;
    let desired = app.desired(ctx)?;
    let enabled = desired
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.platform.settings.get("installation"))
        .and_then(|settings| settings.get("systemd"))
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "systemd installation setting unavailable",
            )
        })?;
    if !enabled {
        return Ok(());
    }
    app.paths.validate(true).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "platform units require fixed installation paths",
        )
    })?;
    if rustix::process::geteuid().as_raw() != 0 {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "platform unit delivery requires local root",
        ));
    }
    check_links(false, ctx)?;
    let directory = app.work.create_child("systemd").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "protected platform template directory unavailable",
        )
    })?;
    let locked = directory
        .lock(journal::deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "platform unit delivery is already running",
            )
        })?;
    check_templates(&directory, false, ctx)?;
    let identity = UNITS
        .iter()
        .map(|(name, bytes)| (*name, config::digest(bytes)))
        .collect::<Vec<_>>();
    let request_digest = config::digest(
        &serde_json::to_vec(&(
            "platform-systemd-install-v1",
            &app.installation.instance_id,
            identity,
        ))
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "platform unit identity encoding failed",
            )
        })?,
    );
    let pending = app.journal.unfinished(ctx)?.into_iter().filter(|transaction|
        matches!(&transaction.intent, Intent::Installation { request_digest: digest } if digest == &request_digest)
    ).collect::<Vec<_>>();
    if pending.len() > 1 {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "multiple platform unit deliveries require reconciliation",
        ));
    }
    let tx = match pending.first() {
        Some(transaction) => transaction.transaction_id.clone(),
        None => app
            .journal
            .prepare_intent(Intent::Installation { request_digest }, vec![], ctx)?,
    };
    for (name, bytes) in UNITS {
        journal::ensure_mutating(ctx, app.control.as_ref())?;
        let old = locked.read(name, 65_536).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "platform template unavailable during delivery",
            )
        })?;
        if old.is_none() {
            locked.compare_exchange(name, None, bytes).map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "platform template delivery interrupted; repeat install",
                )
            })?;
        } else if old.as_deref() != Some(bytes) {
            return Err(journal::failure(
                ctx,
                ErrorCode::ResourceDrift,
                "platform template changed during delivery",
            ));
        }
    }
    check_links(false, ctx)?;
    let budget = Budget::new(
        Instant::now()
            + Duration::from_millis(
                ctx.deadline
                    .0
                    .saturating_sub(app.control.now_ms())
                    .min(30_000),
            ),
        65_536,
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::OperationDeadline,
            "platform delivery budget expired",
        )
    })?;
    HostObserver::new()
        .systemd_control()
        .link_platform_units(budget, &Cancel(app, ctx))
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "platform unit linking incomplete; repeat install",
            )
        })?;
    check_links(true, ctx)?;
    check_templates(&directory, true, ctx)?;
    let mut completion_ctx = ctx.clone();
    if let Some(transaction) = pending.first() {
        completion_ctx.operation_id = transaction.operation_id.clone();
    }
    app.journal
        .finish(&tx, TerminalStatus::Committed, vec![], &completion_ctx)?;
    Ok(())
}
