//! Local crypto composition. Private payloads have a separate non-serializable response.
use super::{
    archive::AgeArchiveStore, certificates::*, identities::FileIdentityInventory,
    inventory::FsBackupInventory, restore::RuntimeRestore,
};
use crate::{
    backup::*,
    certificates::*,
    exports::*,
    host::{HostApplication, HostOutput},
    identities::*,
    journal::{self, Intent, TerminalStatus, deadline, ensure_mutating},
    secrets::{ConsumerGrant, ReplacementPolicy, SecretKind, SecretRef},
};
use hton_contracts::{config::*, operations::*};
use hton_host::{
    Budget as NativeBudget, Cancellation, HostObserver,
    crypto::{AgeRecipient, PrivateDirectory},
};
use hton_runtime_contract as wire;
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{IsTerminal, Read, Write},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
struct Cancel<'a> {
    app: &'a HostApplication,
    ctx: &'a OperationContext,
}
impl Cancellation for Cancel<'_> {
    fn cancelled(&self) -> bool {
        self.app.control.is_cancelled(&self.ctx.cancellation)
    }
}
fn fail(ctx: &OperationContext, code: ErrorCode, message: &'static str) -> Error {
    journal::failure(ctx, code, message)
}
fn budget(app: &HostApplication, ctx: &OperationContext) -> Result<NativeBudget> {
    let remaining = ctx
        .deadline
        .0
        .saturating_sub(app.control.now_ms())
        .min(300000);
    if remaining == 0 {
        return Err(fail(
            ctx,
            ErrorCode::OperationDeadline,
            "crypto operation deadline expired",
        ));
    }
    NativeBudget::crypto(
        Instant::now() + Duration::from_millis(remaining),
        16 * 1024 * 1024,
    )
    .map_err(|_| fail(ctx, ErrorCode::OperationDenied, "crypto budget invalid"))
}
fn local(ctx: &OperationContext) -> Result<()> {
    if ctx.actor.source != ActorSource::Local
        || ctx.actor.actor_id.as_str() != "os.uid.0"
        || rustix::process::geteuid().as_raw() != 0
    {
        return Err(fail(
            ctx,
            ErrorCode::OperationDenied,
            "local root or sudo authority required",
        ));
    }
    Ok(())
}
fn desired(app: &HostApplication, ctx: &OperationContext) -> Result<DesiredSnapshot> {
    let read = app.desired(ctx)?;
    if !read.errors.is_empty() {
        return Err(fail(
            ctx,
            ErrorCode::ConfigSyntax,
            "desired configuration is invalid; originals retained",
        ));
    }
    read.snapshot.ok_or_else(|| {
        fail(
            ctx,
            ErrorCode::ConfigSyntax,
            "desired configuration unavailable",
        )
    })
}
fn configured_names(
    app: &HostApplication,
    ctx: &OperationContext,
) -> Result<BTreeSet<CertificateName>> {
    let snapshot = desired(app, ctx)?;
    let mut result = BTreeSet::new();
    if let Some(values) = snapshot
        .platform
        .settings
        .get("core")
        .and_then(|core| core.get("edge"))
        .and_then(|edge| edge.get("domains"))
        .and_then(Value::as_array)
    {
        for name in values {
            let name = name
                .as_str()
                .ok_or_else(|| fail(ctx, ErrorCode::ConfigSyntax, "configured domain invalid"))?;
            result
                .insert(CertificateName::new(name).map_err(|_| {
                    fail(ctx, ErrorCode::ConfigSyntax, "configured domain invalid")
                })?);
        }
    }
    if let Some(name) = snapshot
        .platform
        .settings
        .get("core")
        .and_then(|core| core.get("certificates"))
        .and_then(|cert| cert.get("wildcardDomain"))
        .and_then(Value::as_str)
    {
        result.insert(CertificateName::new(format!("*.{name}")).map_err(|_| {
            fail(
                ctx,
                ErrorCode::ConfigSyntax,
                "configured wildcard domain invalid",
            )
        })?);
    }
    Ok(result)
}
fn trust(app: &HostApplication, ctx: &OperationContext) -> Result<hton_host::crypto::PrivateFile> {
    PrivateDirectory::open(&app.paths.config.join("trust"))
        .and_then(|dir| dir.file("production.pem", 1024 * 1024))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "protected certificate trust bundle missing; run explicit certs issue/setup",
            )
        })
}
pub fn certificate_inventory(
    app: &HostApplication,
    ctx: &OperationContext,
) -> Result<FileCertificateInventory> {
    let tools = HostObserver::new().crypto();
    let verifier = Arc::new(OpenSslVerifier::new(
        tools,
        trust(app, ctx)?,
        None,
        app.control.clone(),
    ));
    let names_app = app.clone();
    let runtime_app = app.clone();
    Ok(FileCertificateInventory::new(
        Arc::new(app.work.child("certificates").map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "certificate store unavailable",
            )
        })?),
        Arc::new(move |ctx| configured_names(&names_app, ctx)),
        Arc::new(move |ctx| {
            Ok(
                if runtime_app.runtime(ctx)?.run_intent == wire::RunIntent::Running {
                    RunIntent::Running
                } else {
                    RunIntent::Stopped
                },
            )
        }),
        verifier,
        app.control.clone(),
        app.secrets.clone(),
    ))
}
fn reconcile_setup_intents(
    app: &HostApplication,
    digest: &Sha256Digest,
    backup: bool,
    ctx: &OperationContext,
) -> Result<()> {
    for transaction in app.journal.unfinished(ctx)? {
        let matching = match &transaction.intent {
            Intent::Backup { request_digest } if backup => request_digest == digest,
            Intent::Certificates { request_digest } if !backup => request_digest == digest,
            _ => false,
        };
        if !matching
            || transaction.actor.actor_id != ctx.actor.actor_id
            || transaction.actor.source != ActorSource::Local
        {
            continue;
        }
        let mut original = ctx.clone();
        original.operation_id = transaction.operation_id;
        original.actor = transaction.actor;
        app.journal.resolve_operation(
            &transaction.transaction_id,
            TerminalStatus::Committed,
            vec![],
            None,
            &original,
        )?;
    }
    Ok(())
}
fn seed_trust(app: &HostApplication, ctx: &OperationContext) -> Result<()> {
    if trust(app, ctx).is_ok() {
        ensure_mutating(ctx, app.control.as_ref())?;
        let directory = app.config.child("trust").map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "private trust directory unavailable",
            )
        })?;
        let locked = directory
            .lock(deadline(ctx, app.control.as_ref())?)
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::OperationConflict,
                    "trust reconciliation lock unavailable",
                )
            })?;
        let bytes = locked
            .read("production.pem", 1024 * 1024)
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "public trust snapshot unavailable",
                )
            })?
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "public trust snapshot missing",
                )
            })?;
        locked
            .compare_exchange("production.pem", Some(&bytes), &bytes)
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "public trust durability uncertain",
                )
            })?;
        drop(locked);
        let digest = crate::config::digest(
            &serde_json::to_vec(&("trust-bootstrap", crate::config::digest(&bytes)))
                .expect("metadata"),
        );
        return reconcile_setup_intents(app, &digest, false, ctx);
    }
    ensure_mutating(ctx, app.control.as_ref())?;
    let source = SecureDir::open(Path::new("/etc/ssl/certs")).map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "OS public trust source unavailable",
        )
    })?;
    let bytes = source
        .read("ca-certificates.crt", 1024 * 1024)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "OS public trust source unsafe/unavailable",
            )
        })?
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::IoUnavailable,
                "OS public trust bundle missing",
            )
        })?;
    if !bytes.starts_with(b"-----BEGIN CERTIFICATE-----") {
        return Err(fail(
            ctx,
            ErrorCode::ConfigSyntax,
            "OS trust bundle is not PEM certificates",
        ));
    }
    let tx = app.journal.prepare_intent(
        Intent::Certificates {
            request_digest: crate::config::digest(
                &serde_json::to_vec(&("trust-bootstrap", crate::config::digest(&bytes)))
                    .expect("metadata"),
            ),
        },
        vec![],
        ctx,
    )?;
    let directory = app.config.create_child("trust").map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "private trust directory unavailable",
        )
    })?;
    let locked = directory
        .lock(deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationConflict,
                "trust bootstrap lock unavailable",
            )
        })?;
    let old = locked.read("production.pem", 1024 * 1024).map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "prior public trust snapshot unavailable",
        )
    })?;
    if old.is_some() {
        return Err(fail(
            ctx,
            ErrorCode::RecoveryRequired,
            "existing public trust snapshot is invalid; retained",
        ));
    }
    locked
        .compare_exchange("production.pem", None, &bytes)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "public trust snapshot publication uncertain",
            )
        })?;
    app.journal
        .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
    Ok(())
}
fn confirm(
    app: &HostApplication,
    ctx: &OperationContext,
    label: &str,
    approval: ApprovalId,
    scope: &str,
) -> Result<OperationContext> {
    local(ctx)?;
    ensure_mutating(ctx, app.control.as_ref())?;
    let token = format!(
        "CONFIRM {}",
        &crate::config::digest(approval.as_str().as_bytes()).as_str()[7..23]
    );
    let fd = rustix::fs::open(
        "/dev/tty",
        rustix::fs::OFlags::RDWR
            | rustix::fs::OFlags::NOCTTY
            | rustix::fs::OFlags::CLOEXEC
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK,
        rustix::fs::Mode::empty(),
    )
    .map_err(|_| {
        fail(
            ctx,
            ErrorCode::ApprovalRequired,
            "this operation requires exact controlling-TTY consent; --yes is insufficient",
        )
    })?;
    let mut tty = File::from(fd);
    writeln!(tty, "{label}\nType {token}:").map_err(|_| {
        fail(
            ctx,
            ErrorCode::ApprovalRequired,
            "confirmation terminal unavailable",
        )
    })?;
    tty.flush().map_err(|_| {
        fail(
            ctx,
            ErrorCode::ApprovalRequired,
            "confirmation terminal unavailable",
        )
    })?;
    let mut answer = Vec::new();
    let mut byte = [0u8; 1];
    while answer.len() < 128 {
        if app.control.now_ms() >= ctx.deadline.0 || app.control.is_cancelled(&ctx.cancellation) {
            return Err(fail(
                ctx,
                ErrorCode::OperationDeadline,
                "confirmation deadline expired",
            ));
        }
        match tty.read(&mut byte) {
            Ok(0) => break,
            Ok(_) if byte[0] == b'\n' => break,
            Ok(_) => answer.push(byte[0]),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => {
                return Err(fail(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "confirmation input unavailable",
                ));
            }
        }
    }
    if answer != token.as_bytes() {
        return Err(fail(ctx, ErrorCode::ApprovalRequired, "operation declined"));
    }
    let mut trusted = ctx.clone();
    trusted.actor.local_approval_refs.insert(approval);
    trusted
        .actor
        .granted_scopes
        .insert(ScopeId::new(scope).map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationDenied,
                "confirmation scope invalid",
            )
        })?);
    Ok(trusted)
}
fn certificate_request(
    app: &HostApplication,
    args: &[String],
    automatic: bool,
    ctx: &OperationContext,
) -> Result<CertificateIssueRequest> {
    let snapshot = desired(app, ctx)?;
    let cert = snapshot
        .platform
        .settings
        .get("core")
        .and_then(|core| core.get("certificates"))
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "DNS-01 certificate configuration missing",
            )
        })?;
    let mut names = BTreeSet::new();
    let mut force = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--domain" => {
                index += 1;
                names.insert(
                    CertificateName::new(
                        args.get(index)
                            .ok_or_else(|| {
                                fail(ctx, ErrorCode::OperationDenied, "domain argument missing")
                            })?
                            .clone(),
                    )
                    .map_err(|_| fail(ctx, ErrorCode::ConfigSyntax, "requested domain invalid"))?,
                );
            }
            "--force" => force = true,
            "--automatic" if automatic => (),
            _ => {
                return Err(fail(
                    ctx,
                    ErrorCode::OperationDenied,
                    "unsupported certificate arguments",
                ));
            }
        }
        index += 1;
    }
    let configured = configured_names(app, ctx)?;
    if names.is_empty() {
        names = configured;
    }
    if names.is_empty() {
        return Err(fail(
            ctx,
            ErrorCode::ModuleIncomplete,
            "configure own domain before certificate issuance",
        ));
    }
    let secret = |key: &str| -> Result<SecretRef> {
        let reference = cert
            .get("credentials")
            .and_then(|value| value.get(key))
            .and_then(|value| value.get("secretRef"))
            .and_then(Value::as_str)
            .and_then(crate::config::canonical_secret_id)
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "DNS credential reference invalid",
                )
            })?;
        let version = app
            .secrets
            .current_version(&reference, ctx)?
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "DNS credential must be explicitly imported into SecretStore",
                )
            })?;
        Ok(SecretRef {
            secret_id: reference,
            version,
        })
    };
    Ok(CertificateIssueRequest {
        names,
        authority: CertificateAuthority::LetsEncryptProduction,
        force,
        force_approval: None,
        automatic,
        dns_key: secret("apiKey")?,
        dns_password: secret("apiPassword")?,
    })
}
fn certificate_handle(
    app: &HostApplication,
    operation: &str,
    args: &[String],
    ctx: &OperationContext,
) -> Result<Value> {
    if matches!(operation, "certs.list" | "certs.status") {
        if !args.is_empty() {
            return Err(fail(
                ctx,
                ErrorCode::OperationDenied,
                "certificate inventory takes no arguments",
            ));
        }
        let inventory = certificate_inventory(app, ctx)?;
        let candidates = inventory.candidates(ctx)?;
        let now = app.control.now_ms();
        let values:Vec<_>=candidates.into_iter().map(|candidate|{let cert=&candidate.certificate;let remaining=cert.not_after.0.saturating_sub(now);json!({"candidateId":candidate.candidate_id,"certificate":cert,"remainingMs":remaining,"health":if !cert.chain_trusted||!cert.private_key_matches{"untrusted"}else if cert.not_before.0>now{"notYetValid"}else if remaining==0{"expired"}else if remaining<14*86400000{"expiring"}else if remaining<30*86400000{"renewalDue"}else{"healthy"},"servedState":"unverified"})}).collect();
        let unhealthy = operation == "certs.status"
            && (values.is_empty()
                || values
                    .iter()
                    .any(|value| value["health"] != "healthy" && value["health"] != "renewalDue"));
        return Ok(
            json!({"status":if unhealthy{"failed"}else{"succeeded"},"certificates":values,"servedState":"unverified"}),
        );
    }
    let automatic = operation == "certs.renew";
    let mut request = certificate_request(app, args, automatic, ctx)?;
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        return Ok(
            json!({"status":"preview","operation":operation,"names":request.names,"writes":false,"native":"unverified"}),
        );
    }
    seed_trust(app, ctx)?;
    let mut trusted = ctx.clone();
    if request.force {
        let approval = FileCertificateInventory::force_approval_id(&request).map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationDenied,
                "force request identity invalid",
            )
        })?;
        trusted = confirm(
            app,
            ctx,
            "Force native certificate issuance for the exact configured names",
            approval.clone(),
            "certificates.force",
        )?;
        request.force_approval = Some(approval);
    }
    let inventory = certificate_inventory(app, &trusted)?;
    let snapshot = desired(app, &trusted)?;
    let email = snapshot
        .platform
        .settings
        .get("core")
        .and_then(|core| core.get("certificates"))
        .and_then(|cert| cert.get("email"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "ACME account email missing",
            )
        })?
        .to_owned();
    let image = crate::host::setup::locked_image_reference("acme", &trusted)?;
    let host = HostObserver::new();
    let tools = host.crypto();
    let verifier = OpenSslVerifier::new(
        tools.clone(),
        trust(app, &trusted)?,
        None,
        app.control.clone(),
    );
    let issuer = AcmeContainerIssuer::new(
        tools,
        app.paths.work.clone(),
        image,
        email,
        app.control.clone(),
    );
    let coordinator = CertificateCoordinator {
        journal: &app.journal,
        secrets: &app.secrets,
        inventory: &inventory,
        issuer: &issuer,
        verifier: &verifier,
        dns_grant: ConsumerGrant {
            consumer: ServiceId::new("acme").expect("service"),
            scope: ScopeId::new("dns.internetbs").expect("scope"),
        },
    };
    let candidate = coordinator.issue(&request, &trusted)?;
    Ok(json!({"status":"succeeded","candidate":candidate,"delivery":"pending-admitted-apply"}))
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BackupConfig {
    schema_version: u32,
    recipients: BTreeMap<ObjectId, String>,
}
fn backup_config(app: &HostApplication, ctx: &OperationContext) -> Result<BackupConfig> {
    let bytes=app.config.read("backup.yaml",65536).map_err(|_|fail(ctx,ErrorCode::IoUnavailable,"backup configuration unavailable"))?.ok_or_else(||fail(ctx,ErrorCode::ModuleIncomplete,"backup encryption not initialized; run backup --initialize and preserve identity off-host"))?;
    let value = crate::config::decode_yaml(&bytes, crate::config::LoadLimits::default())
        .map_err(|_| fail(ctx, ErrorCode::ConfigSyntax, "backup configuration invalid"))?;
    let config: BackupConfig = serde_json::from_value(value).map_err(|_| {
        fail(
            ctx,
            ErrorCode::ConfigSyntax,
            "backup configuration schema invalid",
        )
    })?;
    if config.schema_version != 1 || config.recipients.is_empty() || config.recipients.len() > 32 {
        return Err(fail(
            ctx,
            ErrorCode::ConfigSyntax,
            "backup recipient bounds invalid",
        ));
    }
    Ok(config)
}
fn archive_store(app: &HostApplication, ctx: &OperationContext) -> Result<Arc<AgeArchiveStore>> {
    let config = backup_config(app, ctx)?;
    let recipients = config
        .recipients
        .into_iter()
        .map(|(id, value)| {
            AgeRecipient::new(value)
                .map(|recipient| (id, recipient))
                .map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "backup public recipient invalid",
                    )
                })
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let identity = PrivateDirectory::open(&app.paths.config.join("backup-identity"))
        .and_then(|directory| directory.file("identity.txt", 16384))
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "external backup decryption identity unavailable",
            )
        })?;
    Ok(Arc::new(AgeArchiveStore::new(
        Arc::new(
            app.work
                .child("backups")
                .map_err(|_| fail(ctx, ErrorCode::IoUnavailable, "backup store unavailable"))?,
        ),
        HostObserver::new().crypto(),
        recipients,
        identity,
        app.control.clone(),
        app.journal.clone(),
    )))
}
fn initialize_backup(
    app: &HostApplication,
    args: &[String],
    ctx: &OperationContext,
) -> Result<Value> {
    if !args.is_empty() {
        return Err(fail(
            ctx,
            ErrorCode::OperationDenied,
            "backup initialize takes no arguments",
        ));
    }
    local(ctx)?;
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        return Ok(
            json!({"status":"preview","writes":false,"initialization":"explicit-age-identity"}),
        );
    }
    ensure_mutating(ctx, app.control.as_ref())?;
    let tx = app.journal.prepare_intent(
        Intent::Backup {
            request_digest: crate::config::digest(b"backup.initialize.v1"),
        },
        vec![],
        ctx,
    )?;
    let directory = app.config.create_child("backup-identity").map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "private backup identity directory unavailable",
        )
    })?;
    let host = HostObserver::new();
    let tools = host.crypto();
    let cancel = Cancel { app, ctx };
    let locked = directory
        .lock(deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationConflict,
                "backup identity lock unavailable",
            )
        })?;
    let identity = match locked
        .read("identity.txt", 16384)
        .map_err(|_| fail(ctx, ErrorCode::IoUnavailable, "backup identity unavailable"))?
    {
        Some(bytes) => bytes,
        None => {
            let bytes = tools
                .generate_age_identity(budget(app, ctx)?, &cancel)
                .map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "fixed age identity generator unavailable",
                    )
                })?
                .into_bytes();
            locked
                .compare_exchange("identity.txt", None, &bytes)
                .map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "backup identity publication uncertain",
                    )
                })?;
            bytes
        }
    };
    locked
        .compare_exchange("identity.txt", Some(&identity), &identity)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "backup identity durability uncertain",
            )
        })?;
    let recipient = tools
        .derive_age_recipient(&identity, budget(app, ctx)?, &cancel)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::RecoveryRequired,
                "backup identity cannot derive a valid public recipient",
            )
        })?;
    drop(locked);
    let config = BackupConfig {
        schema_version: 1,
        recipients: BTreeMap::from([(
            ObjectId::new("local").expect("id"),
            recipient.as_str().to_owned(),
        )]),
    };
    let bytes = serde_yaml::to_string(&config).map_err(|_| {
        fail(
            ctx,
            ErrorCode::OperationConflict,
            "backup config encoding failed",
        )
    })?;
    let locked = app
        .config
        .lock(deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationConflict,
                "backup configuration lock unavailable",
            )
        })?;
    match locked.read("backup.yaml", 65536).map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "backup configuration unavailable",
        )
    })? {
        None => locked
            .compare_exchange("backup.yaml", None, bytes.as_bytes())
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "backup configuration publication uncertain",
                )
            })?,
        Some(previous) => {
            let existing = backup_config(app, ctx)?;
            if !existing
                .recipients
                .values()
                .any(|value| value == recipient.as_str())
            {
                return Err(fail(
                    ctx,
                    ErrorCode::OperationConflict,
                    "existing backup recipients differ; retained",
                ));
            }
            locked
                .compare_exchange("backup.yaml", Some(&previous), &previous)
                .map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "backup configuration durability uncertain",
                    )
                })?;
        }
    }
    app.journal
        .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
    reconcile_setup_intents(
        app,
        &crate::config::digest(b"backup.initialize.v1"),
        true,
        ctx,
    )?;
    Ok(
        json!({"status":"succeeded","identityPath":"/opt/hton/config/backup-identity/identity.txt","recipient":recipient.as_str(),"requiredAction":"Copy the protected decryption identity to secure off-host storage before relying on backups; it is never included in its own archive."}),
    )
}
fn backup_handle(
    app: &HostApplication,
    operation: &str,
    args: &[String],
    ctx: &OperationContext,
) -> Result<Value> {
    if operation == "backup.initialize" {
        return initialize_backup(app, args, ctx);
    }
    let crypto = archive_store(app, ctx)?;
    if operation == "backup.list" {
        if !args.is_empty() {
            return Err(fail(
                ctx,
                ErrorCode::OperationDenied,
                "backup list takes no arguments",
            ));
        }
        return Ok(json!({"status":"succeeded","archives":crypto.list(ctx)?}));
    }
    let mut name = None;
    let mut keep = 10u16;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--keep" => {
                index += 1;
                keep = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .filter(|value| *value > 0 && *value <= 100)
                    .ok_or_else(|| {
                        fail(
                            ctx,
                            ErrorCode::OperationDenied,
                            "backup retention must be1..100",
                        )
                    })?;
            }
            "--restore" => {
                index += 1;
                if name.is_some() {
                    return Err(fail(
                        ctx,
                        ErrorCode::OperationDenied,
                        "duplicate archive selector",
                    ));
                }
                name = Some(
                    args.get(index)
                        .ok_or_else(|| {
                            fail(
                                ctx,
                                ErrorCode::OperationDenied,
                                "restore archive ID missing",
                            )
                        })?
                        .clone(),
                );
            }
            value if !value.starts_with('-') && name.is_none() => name = Some(value.to_owned()),
            _ => {
                return Err(fail(
                    ctx,
                    ErrorCode::OperationDenied,
                    "invalid archive operation arguments",
                ));
            }
        }
        index += 1;
    }
    let archive_id = ObjectId::new(
        name.unwrap_or_else(|| format!("backup.{}", app.control.now_ms())),
    )
    .map_err(|_| {
        fail(
            ctx,
            ErrorCode::OperationDenied,
            "archive selector must be a protected logical archive ID, not an arbitrary path",
        )
    })?;
    let inventory = FsBackupInventory::new(
        app,
        HostObserver::new().crypto(),
        crypto.clone(),
        if operation == "backup.restore" {
            Some(archive_id.clone())
        } else {
            None
        },
    );
    let runtime = RuntimeRestore::new(app);
    let lock = app.work.child("locks").map_err(|_| {
        fail(
            ctx,
            ErrorCode::IoUnavailable,
            "configuration coordination lock unavailable",
        )
    })?;
    let coordinator = BackupCoordinator {
        journal: &app.journal,
        config_lock: &lock,
        instance_id: app.installation.instance_id.clone(),
        inventory: &inventory,
        crypto: crypto.as_ref(),
        runtime: &runtime,
    };
    if operation == "backup.restore" {
        let (metadata, _) = crypto.inspect(&archive_id, ctx)?;
        let approval = super::restore::approval_id(
            &archive_id,
            &metadata.manifest_digest,
            &app.installation.instance_id,
        );
        let plan = coordinator.plan_restore(archive_id, approval.clone(), ctx)?;
        if ctx.execution_mode == ExecutionMode::ReadOnly {
            return Ok(
                json!({"status":"preview","archiveId":plan.archive_id,"manifestDigest":plan.manifest_digest,"objects":plan.manifest.entries.len(),"writes":false,"resultRunIntent":"stopped"}),
            );
        }
        let trusted = confirm(
            app,
            ctx,
            "Restore this authenticated archive: stop all associated resources and replace the exact previewed configuration inventory",
            approval,
            "backup.restore",
        )?;
        let result = coordinator.restore(&plan, &trusted)?;
        return Ok(
            json!({"status":"succeeded","transactionId":result.transaction_id,"restored":result.restored,"runIntent":result.run_intent,"requiresExplicitUp":result.requires_explicit_up}),
        );
    }
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        return Ok(
            json!({"status":"preview","archiveId":archive_id,"writes":false,"kind":"fullDurable","nativeInventory":"unverified"}),
        );
    }
    let config = backup_config(app, ctx)?;
    let result = coordinator.backup(
        &BackupRequest {
            archive_id,
            kind: ArchiveKind::FullDurable,
            recipients: config.recipients.into_keys().collect(),
            keep,
        },
        ctx,
    )?;
    Ok(json!({"status":"succeeded","archive":result}))
}
struct IdentityApply<'a>(&'a HostApplication);
impl crate::apply::EffectApplyPort for IdentityApply<'_> {
    fn stage_and_apply(
        &self,
        plan: &EffectPlan,
        ctx: &OperationContext,
    ) -> Result<OperationResult> {
        crate::host::effects::apply(self.0, plan, ctx)
    }
}
fn dns_import(
    app: &HostApplication,
    args: &[String],
    ctx: &OperationContext,
) -> Result<HostOutput> {
    local(ctx)?;
    let mut key_file = None;
    let mut password_file = None;
    let mut force = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "dns" => (),
            "--import-key" => {
                index += 1;
                if key_file.is_some() {
                    return Err(fail(
                        ctx,
                        ErrorCode::OperationDenied,
                        "duplicate DNS key input",
                    ));
                }
                key_file = Some(
                    args.get(index)
                        .ok_or_else(|| {
                            fail(ctx, ErrorCode::OperationDenied, "DNS key file missing")
                        })?
                        .clone(),
                );
            }
            "--import-password" => {
                index += 1;
                if password_file.is_some() {
                    return Err(fail(
                        ctx,
                        ErrorCode::OperationDenied,
                        "duplicate DNS password input",
                    ));
                }
                password_file = Some(
                    args.get(index)
                        .ok_or_else(|| {
                            fail(ctx, ErrorCode::OperationDenied, "DNS password file missing")
                        })?
                        .clone(),
                );
            }
            "--force" => force = true,
            _ => {
                return Err(fail(
                    ctx,
                    ErrorCode::OperationDenied,
                    "DNS import accepts only private input files and explicit force",
                ));
            }
        }
        index += 1;
    }
    let key_file = key_file.ok_or_else(|| {
        fail(
            ctx,
            ErrorCode::OperationDenied,
            "both DNS credential input files are required",
        )
    })?;
    let password_file = password_file.ok_or_else(|| {
        fail(
            ctx,
            ErrorCode::OperationDenied,
            "both DNS credential input files are required",
        )
    })?;
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        return Ok(HostOutput::Metadata(
            json!({"status":"preview","kind":"dns","inputFilesRead":false,"writes":false,"disclosure":false}),
        ));
    }
    let snapshot = desired(app, ctx)?;
    let certificate = snapshot
        .platform
        .settings
        .get("core")
        .and_then(|core| core.get("certificates"))
        .ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::ModuleIncomplete,
                "configure InternetBS DNS-01 credential refs first",
            )
        })?;
    let mut imports = Vec::new();
    for (field, path) in [("apiKey", key_file), ("apiPassword", password_file)] {
        let id = certificate
            .get("credentials")
            .and_then(|value| value.get(field))
            .and_then(|value| value.get("secretRef"))
            .and_then(Value::as_str)
            .and_then(crate::config::canonical_secret_id)
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "configured DNS credential reference invalid",
                )
            })?;
        let path = Path::new(&path);
        if !path.is_absolute() {
            return Err(fail(
                ctx,
                ErrorCode::OperationDenied,
                "DNS credential files must use absolute paths",
            ));
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::OperationDenied,
                    "DNS credential input filename invalid",
                )
            })?;
        let parent = path.parent().ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::OperationDenied,
                "DNS credential input parent missing",
            )
        })?;
        let directory = SecureDir::open(parent).map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationDenied,
                "DNS credential input parent unsafe",
            )
        })?;
        let mut bytes = directory
            .read_private(name, 16384)
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::OperationDenied,
                    "DNS credential input must be owned private0600 regular file",
                )
            })?
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "DNS credential input missing",
                )
            })?;
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
        }
        if bytes.is_empty()
            || bytes.iter().any(|byte| matches!(byte, b'\n' | b'\r' | 0))
            || std::str::from_utf8(&bytes).is_err()
        {
            return Err(fail(
                ctx,
                ErrorCode::ConfigSyntax,
                "DNS credential must be one nonempty UTF-8 value without control-line injection",
            ));
        }
        let previous = app
            .secrets
            .current_version(&id, ctx)?
            .map(|version| SecretRef {
                secret_id: id.clone(),
                version,
            });
        let matches = if let Some(reference) = &previous {
            app.secrets
                .resolve(
                    reference,
                    &ConsumerGrant {
                        consumer: ServiceId::new("acme").expect("service"),
                        scope: ScopeId::new("dns.internetbs").expect("scope"),
                    },
                    ctx,
                )?
                .expose()
                == bytes.as_slice()
        } else {
            false
        };
        if previous.is_some() && !matches && !force {
            return Err(fail(
                ctx,
                ErrorCode::ApprovalRequired,
                "DNS credential differs; explicit force and exact local consent are required",
            ));
        }
        imports.push((id, bytes, previous, matches));
    }
    let mut trusted = ctx.clone();
    if imports
        .iter()
        .any(|(_, _, previous, matches)| previous.is_some() && !*matches)
    {
        let scope = imports
            .iter()
            .map(|(id, bytes, previous, _)| (id, crate::config::digest(bytes), previous))
            .collect::<Vec<_>>();
        let digest =
            crate::config::digest(&serde_json::to_vec(&scope).expect("validated metadata"));
        let approval =
            ApprovalId::new(format!("dns-import.{}", &digest.as_str()[7..])).expect("digest ID");
        trusted = confirm(
            app,
            ctx,
            "Replace the exact configured DNS-01 credentials; existing accepted certificate versions stay retained",
            approval,
            "certificates.manage",
        )?;
    }
    let request_digest = crate::config::digest(
        &serde_json::to_vec(
            &imports
                .iter()
                .map(|(id, bytes, previous, _)| (id, crate::config::digest(bytes), previous))
                .collect::<Vec<_>>(),
        )
        .expect("validated metadata"),
    );
    let tx =
        app.journal
            .prepare_intent(Intent::Certificates { request_digest }, vec![], &trusted)?;
    let mut result = Vec::new();
    for (id, bytes, previous, matches) in imports {
        let policy = match previous {
            None => ReplacementPolicy::Create,
            Some(reference) if matches => ReplacementPolicy::Reuse { reference },
            Some(previous) => ReplacementPolicy::Rotate {
                previous,
                confirmed: true,
            },
        };
        let material = crate::secrets::ProtectedMaterial::from_input(bytes).map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationDenied,
                "DNS import material bound invalid",
            )
        })?;
        let saved = app.secrets.import(
            id,
            ScopeId::new("dns.internetbs").expect("scope"),
            BTreeSet::from([ServiceId::new("acme").expect("service")]),
            policy,
            material,
            &trusted,
        )?;
        result.push(saved.reference);
    }
    app.journal
        .finish(&tx, TerminalStatus::Committed, vec![], &trusted)?;
    Ok(HostOutput::Metadata(
        json!({"status":"succeeded","kind":"dns","references":result,"materialDisclosed":false}),
    ))
}

fn keys(app: &HostApplication, args: &[String], ctx: &OperationContext) -> Result<HostOutput> {
    local(ctx)?;
    if args.iter().any(|arg| arg == "dns") {
        return dns_import(app, args, ctx);
    }
    let mut identity = ObjectId::new("default").expect("id");
    let mut show = false;
    let mut force = false;
    let mut kinds = BTreeSet::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--device" => {
                index += 1;
                identity = ObjectId::new(args.get(index).ok_or_else(|| {
                    fail(ctx, ErrorCode::OperationDenied, "identity selector missing")
                })?)
                .map_err(|_| fail(ctx, ErrorCode::OperationDenied, "identity selector invalid"))?;
            }
            "--show" => show = true,
            "--force" => force = true,
            "all" => {
                kinds.insert("uuid");
                kinds.insert("vpn-path");
            }
            "uuid" => {
                kinds.insert("uuid");
            }
            "vpn-path" => {
                kinds.insert("vpn-path");
            }
            _ => {
                return Err(fail(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "requested credential kind has no qualified R1 profile",
                ));
            }
        }
        index += 1;
    }
    if kinds.is_empty() {
        kinds.extend(["uuid", "vpn-path"]);
    }
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        return Ok(HostOutput::Metadata(
            json!({"status":"preview","identity":identity,"kinds":kinds,"writes":false,"disclosure":false}),
        ));
    }
    let inventory = FileIdentityInventory::open(app);
    let mut metadata = Vec::new();
    let mut disclosure = BTreeMap::new();
    if kinds.contains("vpn-path") {
        let id = SecretId::new("vless.path").expect("id");
        let previous = app
            .secrets
            .current_version(&id, ctx)?
            .map(|version| SecretRef {
                secret_id: id.clone(),
                version,
            });
        let mut trusted = ctx.clone();
        let policy = match previous.as_ref() {
            Some(reference) if force => {
                let approval = ApprovalId::new(format!(
                    "path-rotate.{}",
                    &crate::config::digest(&serde_json::to_vec(reference).expect("metadata"))
                        .as_str()[7..]
                ))
                .expect("id");
                trusted = confirm(
                    app,
                    ctx,
                    "Replace the pending VLESS path token; accepted bundles keep explicit previous grants until safe apply",
                    approval,
                    "identities.rotate",
                )?;
                ReplacementPolicy::Rotate {
                    previous: reference.clone(),
                    confirmed: true,
                }
            }
            Some(reference) => ReplacementPolicy::Reuse {
                reference: reference.clone(),
            },
            None => ReplacementPolicy::Create,
        };
        let generated = app.secrets.generate(
            id,
            ScopeId::new("module.vless").expect("scope"),
            SecretKind::RandomToken,
            BTreeSet::from([
                ServiceId::new("xray").expect("service"),
                ServiceId::new("caddy").expect("service"),
            ]),
            policy,
            &trusted,
        )?;
        metadata.push(json!({"kind":"vpn-path","reference":generated.reference,"created":generated.created,"delivery":"pending-safe-apply"}));
        if show {
            let material = app.secrets.resolve(
                &generated.reference,
                &ConsumerGrant {
                    consumer: ServiceId::new("xray").expect("service"),
                    scope: ScopeId::new("module.vless").expect("scope"),
                },
                &trusted,
            )?;
            disclosure.insert(
                "vpn-path",
                String::from_utf8(material.expose().to_vec()).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "path credential encoding invalid",
                    )
                })?,
            );
        }
    }
    if kinds.contains("uuid") {
        let profile = ObjectId::new("vless").expect("profile");
        let existing = inventory.binding(&identity, &profile, ctx)?;
        let binding = if existing.is_some() && !force {
            existing.expect("checked")
        } else {
            if let Some(previous) = existing
                .as_ref()
                .filter(|previous| previous.state == BindingState::Accepted)
            {
                // Refuse an incompatible revocation before allocating a new secret or
                // replacing pending identity metadata. The effects owner decides safety.
                crate::host::effects::prepare_identity(app, previous, false, ctx)?;
            }
            let mut trusted = ctx.clone();
            let approval = if force {
                let approval = FileIdentityInventory::revocation_approval_id(&identity, &profile);
                trusted = confirm(
                    app,
                    ctx,
                    "Replace this identity credential through the guarded accepted/fallback policy",
                    approval.clone(),
                    "identities.rotate",
                )?;
                Some(approval)
            } else {
                None
            };
            let apply = IdentityApply(app);
            let coordinator = IdentityCoordinator {
                journal: &app.journal,
                inventory: &inventory,
                secrets: &app.secrets,
                apply: &apply,
            };
            coordinator
                .issue(
                    &IdentityIssueRequest {
                        identity_id: identity.clone(),
                        profile_id: profile,
                        replace: force,
                        approval_id: approval,
                    },
                    &trusted,
                )?
                .binding
        };
        metadata.push(json!({"kind":"uuid","identity":binding.identity_id,"profile":binding.profile_id,"reference":binding.secret,"state":binding.state}));
        if show {
            if binding.state != BindingState::Accepted {
                return Err(fail(
                    ctx,
                    ErrorCode::OperationDenied,
                    "pending credentials are not disclosed as accepted profiles; explicitly enable/up first",
                ));
            }
            let material = app.secrets.resolve(
                &binding.secret,
                &ConsumerGrant {
                    consumer: binding.service_id.clone(),
                    scope: binding.scope.clone(),
                },
                ctx,
            )?;
            disclosure.insert(
                "uuid",
                String::from_utf8(material.expose().to_vec()).map_err(|_| {
                    fail(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "credential encoding invalid",
                    )
                })?,
            );
        }
    }
    if show {
        return Ok(HostOutput::Protected(
            serde_json::to_vec(&disclosure).map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::OperationConflict,
                    "disclosure encoding failed",
                )
            })?,
        ));
    }
    Ok(HostOutput::Metadata(
        json!({"status":if metadata.iter().any(|value|value.get("state")==Some(&json!("pending"))){"partial"}else{"succeeded"},"credentials":metadata,"materialDisclosed":false}),
    ))
}
fn share(app: &HostApplication, args: &[String], ctx: &OperationContext) -> Result<HostOutput> {
    local(ctx)?;
    let mut identity = ObjectId::new("default").expect("id");
    let mut selected = false;
    let mut client = ObjectId::new("native").expect("client");
    let mut format = "text";
    let mut output = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--client" => {
                index += 1;
                client = ObjectId::new(args.get(index).ok_or_else(|| {
                    fail(ctx, ErrorCode::OperationDenied, "client selector missing")
                })?)
                .map_err(|_| fail(ctx, ErrorCode::OperationDenied, "client selector invalid"))?;
            }
            "--format" => {
                index += 1;
                format = args
                    .get(index)
                    .ok_or_else(|| fail(ctx, ErrorCode::OperationDenied, "format missing"))?
                    .as_str();
            }
            "--kind" => {
                index += 1;
                if args.get(index).map(String::as_str) != Some("vless") {
                    return Err(fail(
                        ctx,
                        ErrorCode::ModuleUnsupported,
                        "export kind is not a qualified R1 profile",
                    ));
                }
            }
            "--out" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| {
                            fail(ctx, ErrorCode::OperationDenied, "output selector missing")
                        })?
                        .clone(),
                );
            }
            "--target" | "--router" | "--ssh" | "--extra" | "--no-dot" => {
                return Err(fail(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "device recipe is not qualified by the accepted native client profile",
                ));
            }
            value if !value.starts_with('-') && !selected => {
                identity = ObjectId::new(value).map_err(|_| {
                    fail(ctx, ErrorCode::OperationDenied, "identity selector invalid")
                })?;
                selected = true;
            }
            _ => {
                return Err(fail(
                    ctx,
                    ErrorCode::OperationDenied,
                    "invalid export arguments",
                ));
            }
        }
        index += 1;
    }
    let format = match format {
        "json" => ExportFormat::XrayJson,
        "text" => ExportFormat::VlessUri,
        _ => {
            return Err(fail(
                ctx,
                ErrorCode::ModuleUnsupported,
                "requested export format is not qualified",
            ));
        }
    };
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        return Ok(HostOutput::Metadata(
            json!({"status":"preview","identity":identity,"client":client,"format":format,"writes":false,"disclosure":false}),
        ));
    }
    let mut trusted = ctx.clone();
    trusted
        .actor
        .granted_scopes
        .insert(ScopeId::new("identities.disclose").expect("scope"));
    let inventory = FileIdentityInventory::open(app);
    let coordinator = ExportCoordinator {
        inventory: &inventory,
        secrets: &app.secrets,
        control: app.control.as_ref(),
    };
    let bytes = coordinator
        .export_profile(
            &identity,
            &ObjectId::new("vless").expect("profile"),
            &client,
            format,
            &trusted,
        )?
        .expose()
        .to_vec();
    if let Some(path) = output {
        let path = Path::new(&path);
        let parent = path.parent().ok_or_else(|| {
            fail(
                ctx,
                ErrorCode::OperationDenied,
                "export output parent missing",
            )
        })?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                fail(
                    ctx,
                    ErrorCode::OperationDenied,
                    "export output filename invalid",
                )
            })?;
        let directory = SecureDir::open_private(parent).map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationDenied,
                "export output requires an existing private0700 directory",
            )
        })?;
        let locked = directory
            .lock(deadline(ctx, app.control.as_ref())?)
            .map_err(|_| {
                fail(
                    ctx,
                    ErrorCode::OperationConflict,
                    "export output lock unavailable",
                )
            })?;
        locked.compare_exchange(name, None, &bytes).map_err(|_| {
            fail(
                ctx,
                ErrorCode::OperationConflict,
                "export output already exists or publication uncertain",
            )
        })?;
        if !std::io::stdout().is_terminal() {
            return Ok(HostOutput::Metadata(
                json!({"status":"succeeded","protectedOutputWritten":true,"materialOnStdout":false}),
            ));
        }
    }
    Ok(HostOutput::Protected(bytes))
}
pub fn output(
    app: &HostApplication,
    operation: &str,
    args: &[String],
    ctx: &OperationContext,
) -> Result<HostOutput> {
    match operation {
        "keys.manage" => keys(app, args, ctx),
        "share.export" => share(app, args, ctx),
        _ => handle(app, operation, args, ctx).map(HostOutput::Metadata),
    }
}
pub fn handle(
    app: &HostApplication,
    operation: &str,
    args: &[String],
    ctx: &OperationContext,
) -> Result<Value> {
    match operation {
        "certs.issue" | "certs.renew" | "certs.status" | "certs.list" => {
            certificate_handle(app, operation, args, ctx)
        }
        "backup.initialize" | "backup.create" | "backup.list" | "backup.restore" => {
            backup_handle(app, operation, args, ctx)
        }
        _ => Err(fail(
            ctx,
            ErrorCode::OperationDenied,
            "unknown crypto operation",
        )),
    }
}
