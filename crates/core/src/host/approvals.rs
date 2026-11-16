//! Protected local grants; ordinary --yes is never public/foreign consent.
use super::*;
use crate::journal::{Intent, TerminalStatus, deadline, ensure_mutating};
use rustix::fs::{Mode, OFlags};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Write},
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OwnedPolicy {
    schema_version: u32,
    instance_id: InstanceId,
    privileges: BTreeSet<Privilege>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PublicConsent {
    schema_version: u32,
    instance_id: InstanceId,
    grant: ApprovalEnvelope,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AllocationConsent {
    schema_version: u32,
    instance_id: InstanceId,
    operation_id: OperationId,
    candidate_digest: Sha256Digest,
    scope_digest: Sha256Digest,
    issuer: ActorId,
    approval_id: ApprovalId,
    expires_at: UnixMillis,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstallInputConsent {
    schema_version: u32,
    instance_id: InstanceId,
    operation_id: OperationId,
    candidate_digest: Sha256Digest,
    scope_digest: Sha256Digest,
    issuer: ActorId,
    approval_id: ApprovalId,
    expires_at: UnixMillis,
}

/// Crate-private capability minted only after exact local TTY review. Callers
/// cannot synthesize it from a serialized ApprovalId or AllocationReview.
#[derive(Clone, Debug)]
pub(crate) struct AllocationApproval {
    candidate_digest: Sha256Digest,
    scope_digest: Sha256Digest,
    approval_id: ApprovalId,
    expires_at: UnixMillis,
    instance_id: InstanceId,
    issuer: ActorId,
    operation_id: OperationId,
}
impl AllocationApproval {
    pub(crate) fn covers(
        &self,
        candidate: &PortAllocationCandidate,
        instance_id: &InstanceId,
        ctx: &OperationContext,
        now: UnixMillis,
    ) -> bool {
        self.candidate_digest == candidate.candidate_digest
            && self.instance_id == *instance_id
            && self.issuer == ctx.actor.actor_id
            && self.operation_id == ctx.operation_id
            && allocation_scope(
                instance_id,
                &ctx.actor.actor_id,
                &ctx.operation_id,
                &candidate.candidate_digest,
                candidate,
            )
            .is_ok_and(|scope| self.scope_digest == scope)
            && self.expires_at.0 > now.0
            && self.approval_id.as_str()
                == format!("allocation.{}", &self.scope_digest.as_str()[7..39])
    }
}

/// Opaque local capability for one exact site/edge input candidate. It is
/// reconstructed only by verifying the protected consent record.
#[derive(Clone, Debug)]
pub(crate) struct InstallInputApproval {
    candidate_digest: Sha256Digest,
    scope_digest: Sha256Digest,
    approval_id: ApprovalId,
    expires_at: UnixMillis,
    instance_id: InstanceId,
    issuer: ActorId,
    operation_id: OperationId,
}
impl InstallInputApproval {
    pub(crate) fn covers(
        &self,
        candidate: &InstallConfigCandidate,
        instance_id: &InstanceId,
        ctx: &OperationContext,
        now: UnixMillis,
    ) -> bool {
        self.candidate_digest == candidate.candidate_digest
            && self.instance_id == *instance_id
            && self.issuer == ctx.actor.actor_id
            && self.operation_id == ctx.operation_id
            && install_input_scope(
                instance_id,
                &ctx.actor.actor_id,
                &ctx.operation_id,
                &candidate.candidate_digest,
                candidate,
            )
            .is_ok_and(|scope| self.scope_digest == scope)
            && self.expires_at.0 > now.0
            && self.approval_id.as_str()
                == format!("install-input.{}", &self.scope_digest.as_str()[7..39])
    }
}

/// Request exact local review after the caller has completed its read-only
/// conflict preflight. `--yes` is not consulted and cannot create this grant.
pub(crate) fn authorize_install_input(
    app: &HostApplication,
    candidate: &InstallConfigCandidate,
    ctx: &OperationContext,
) -> Result<InstallInputReview> {
    ensure_local_root(ctx)?;
    ensure_mutating(ctx, app.control.as_ref())?;
    validate_install_input_candidate(candidate, ctx)?;
    let now = UnixMillis(app.control.now_ms());
    let expires_at = UnixMillis(now.0.saturating_add(300_000).min(ctx.deadline.0));
    if expires_at.0 <= now.0 {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDeadline,
            "install input approval deadline expired",
        ));
    }
    let scope_digest = install_input_scope(
        &app.installation.instance_id,
        &ctx.actor.actor_id,
        &ctx.operation_id,
        &candidate.candidate_digest,
        candidate,
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "install input approval scope encoding failed",
        )
    })?;
    let approval_id = ApprovalId::new(format!("install-input.{}", &scope_digest.as_str()[7..39]))
        .expect("digest-derived install input approval ID");
    let record_name = format!("install-input-{}.json", &scope_digest.as_str()[7..39]);
    let approvals = app.work.child("approvals").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "install input approval inventory unavailable",
        )
    })?;

    if let Some(bytes) = approvals.read(&record_name, 16_384).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "install input approval record unavailable",
        )
    })? {
        let stored: InstallInputConsent = serde_json::from_slice(&bytes).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "install input approval record corrupt; retained",
            )
        })?;
        verify_install_input_record(
            &stored,
            app,
            candidate,
            &scope_digest,
            &approval_id,
            &ctx.operation_id,
            &ctx.actor.actor_id,
            now,
            None,
            ctx,
        )?;
        let tx = app.journal.prepare_intent(
            Intent::Approval {
                scope_digest: scope_digest.clone(),
            },
            vec![],
            ctx,
        )?;
        app.journal
            .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
        return Ok(InstallInputReview {
            candidate_digest: candidate.candidate_digest.clone(),
            approval_id,
            instance_id: app.installation.instance_id.clone(),
            operation_id: ctx.operation_id.clone(),
        });
    }

    confirm_install_input_on_tty(candidate, &scope_digest, ctx, app)?;
    let encoded = serde_json::to_vec(&InstallInputConsent {
        schema_version: 1,
        instance_id: app.installation.instance_id.clone(),
        operation_id: ctx.operation_id.clone(),
        candidate_digest: candidate.candidate_digest.clone(),
        scope_digest: scope_digest.clone(),
        issuer: ctx.actor.actor_id.clone(),
        approval_id: approval_id.clone(),
        expires_at,
    })
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "install input approval encoding failed",
        )
    })?;
    let tx = app.journal.prepare_intent(
        Intent::Approval {
            scope_digest: scope_digest.clone(),
        },
        vec![],
        ctx,
    )?;
    let lock = approvals
        .lock(deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "install input approval lock unavailable",
            )
        })?;
    lock.compare_exchange(&record_name, None, &encoded)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "install input approval publication uncertain; reconcile before retry",
            )
        })?;
    app.journal
        .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
    Ok(InstallInputReview {
        candidate_digest: candidate.candidate_digest.clone(),
        approval_id,
        instance_id: app.installation.instance_id.clone(),
        operation_id: ctx.operation_id.clone(),
    })
}

/// Verify a serialized review against its protected local record and mint the
/// opaque capability consumed by the narrowly scoped config writer.
pub(crate) fn verify_install_input(
    app: &HostApplication,
    candidate: &InstallConfigCandidate,
    review: &InstallInputReview,
    ctx: &OperationContext,
) -> Result<InstallInputApproval> {
    ensure_local_root(ctx)?;
    ensure_mutating(ctx, app.control.as_ref())?;
    validate_install_input_candidate(candidate, ctx)?;
    let scope_digest = install_input_scope(
        &app.installation.instance_id,
        &ctx.actor.actor_id,
        &ctx.operation_id,
        &candidate.candidate_digest,
        candidate,
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "install input approval scope encoding failed",
        )
    })?;
    let expected_id = ApprovalId::new(format!("install-input.{}", &scope_digest.as_str()[7..39]))
        .expect("digest-derived install input approval ID");
    if review.candidate_digest != candidate.candidate_digest
        || review.instance_id != app.installation.instance_id
        || review.operation_id != ctx.operation_id
        || review.approval_id != expected_id
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "install input review does not cover this candidate and operation",
        ));
    }
    let record_name = format!("install-input-{}.json", &scope_digest.as_str()[7..39]);
    let approvals = app.work.child("approvals").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "install input approval inventory unavailable",
        )
    })?;
    let bytes = approvals
        .read(&record_name, 16_384)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "install input approval record unavailable",
            )
        })?
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ApprovalRequired,
                "protected install input approval is missing",
            )
        })?;
    let stored: InstallInputConsent = serde_json::from_slice(&bytes).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "install input approval record corrupt; retained",
        )
    })?;
    let now = UnixMillis(app.control.now_ms());
    verify_install_input_record(
        &stored,
        app,
        candidate,
        &scope_digest,
        &expected_id,
        &ctx.operation_id,
        &ctx.actor.actor_id,
        now,
        None,
        ctx,
    )?;
    Ok(InstallInputApproval {
        candidate_digest: candidate.candidate_digest.clone(),
        scope_digest,
        approval_id: expected_id,
        expires_at: stored.expires_at,
        instance_id: app.installation.instance_id.clone(),
        issuer: ctx.actor.actor_id.clone(),
        operation_id: ctx.operation_id.clone(),
    })
}

fn install_input_scope(
    instance_id: &InstanceId,
    issuer: &ActorId,
    operation_id: &OperationId,
    candidate_digest: &Sha256Digest,
    candidate: &InstallConfigCandidate,
) -> std::result::Result<Sha256Digest, serde_json::Error> {
    serde_json::to_vec(&(
        "install-input-approval-v1",
        instance_id,
        issuer,
        operation_id,
        candidate_digest,
        candidate,
    ))
    .map(|bytes| config::digest(&bytes))
}

fn recompute_install_input_digest(
    candidate: &InstallConfigCandidate,
) -> std::result::Result<Sha256Digest, serde_json::Error> {
    serde_json::to_vec(&(
        "install-config-candidate-v1",
        &candidate.changes,
        &candidate.read_set,
    ))
    .map(|bytes| config::digest(&bytes))
}

pub(crate) fn validate_install_input_candidate(
    candidate: &InstallConfigCandidate,
    ctx: &OperationContext,
) -> Result<()> {
    if candidate.changes.is_empty()
        || candidate.changes.len() > 3
        || candidate.read_set.coherence != ReadCoherence::Coherent
        || candidate
            .read_set
            .absent_objects
            .iter()
            .any(|id| candidate.read_set.objects.contains_key(id))
        || recompute_install_input_digest(candidate).ok().as_ref()
            != Some(&candidate.candidate_digest)
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "install input candidate is malformed, stale or has an invalid digest",
        ));
    }
    let mut site_seen = false;
    let mut edge_seen = false;
    let mut certificate_email_seen = false;
    for change in &candidate.changes {
        let valid = match (&change.field, &change.proposed, &change.previous) {
            (
                InstallConfigField::SiteDomain,
                InstallConfigValue::SiteDomain(proposed),
                previous,
            ) => {
                let valid_domain = super::installation::valid_site_domain(proposed);
                let valid_previous = previous
                    .as_ref()
                    .is_none_or(|value| matches!(value, InstallConfigValue::SiteDomain(_)));
                let unique = !site_seen;
                site_seen = true;
                valid_domain && valid_previous && unique
            }
            (
                InstallConfigField::PublicEdge,
                InstallConfigValue::PublicEdge(proposed),
                previous,
            ) => {
                let valid_endpoint = proposed.transport == Transport::Tcp
                    && proposed.address == std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
                    && proposed.port == 443;
                let valid_previous = previous
                    .as_ref()
                    .is_some_and(|value| matches!(value, InstallConfigValue::PublicEdge(_)));
                let unique = !edge_seen;
                edge_seen = true;
                valid_endpoint && valid_previous && unique
            }
            (
                InstallConfigField::CertificateEmail,
                InstallConfigValue::CertificateEmail(proposed),
                previous,
            ) => {
                let valid_proposed = super::install_input::valid_certificate_email(proposed);
                let valid_previous = previous.as_ref().is_none_or(|value| {
                    matches!(value, InstallConfigValue::CertificateEmail(email) if email.len() <= 320 && super::install_input::valid_certificate_email(email))
                });
                let unique = !certificate_email_seen;
                certificate_email_seen = true;
                valid_proposed && valid_previous && unique
            }
            _ => false,
        };
        if !valid {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "install input candidate contains invalid or duplicate allowlisted values",
            ));
        }
        if change.previous.as_ref() == Some(&change.proposed) {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "install input candidate contains an unchanged value",
            ));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn verify_install_input_record(
    stored: &InstallInputConsent,
    app: &HostApplication,
    candidate: &InstallConfigCandidate,
    scope_digest: &Sha256Digest,
    approval_id: &ApprovalId,
    operation_id: &OperationId,
    issuer: &ActorId,
    now: UnixMillis,
    expected_expiry: Option<UnixMillis>,
    ctx: &OperationContext,
) -> Result<()> {
    if stored.schema_version != 1
        || stored.instance_id != app.installation.instance_id
        || stored.operation_id != *operation_id
        || stored.candidate_digest != candidate.candidate_digest
        || stored.scope_digest != *scope_digest
        || stored.issuer != *issuer
        || stored.approval_id != *approval_id
        || stored.expires_at.0 <= now.0
        || stored.expires_at.0 > ctx.deadline.0
        || expected_expiry.is_some_and(|expiry| stored.expires_at != expiry)
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "protected install input approval is stale or mismatched",
        ));
    }
    Ok(())
}

fn confirm_install_input_on_tty(
    candidate: &InstallConfigCandidate,
    scope: &Sha256Digest,
    ctx: &OperationContext,
    app: &HostApplication,
) -> Result<()> {
    let fd = rustix::fs::open(
        "/dev/tty",
        OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "site, public-edge and certificate-account changes require exact local TTY approval",
        )
    })?;
    let mut tty = File::from(fd);
    for change in &candidate.changes {
        let previous = change
            .previous
            .as_ref()
            .map(install_input_value_text)
            .unwrap_or_else(|| "<unset>".to_owned());
        let proposed = install_input_value_text(&change.proposed);
        writeln!(
            tty,
            "Change {} from {} to {}",
            install_input_field_name(change.field),
            previous,
            proposed
        )
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ApprovalRequired,
                "install input approval prompt unavailable",
            )
        })?;
    }
    let expected = format!("INSTALL {}", &scope.as_str()[7..23]);
    writeln!(
        tty,
        "Type {expected} to approve these exact installation inputs:"
    )
    .and_then(|_| tty.flush())
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "install input approval prompt unavailable",
        )
    })?;
    read_exact_confirmation(&mut tty, &expected, ctx, app.control.as_ref())?;
    Ok(())
}

fn install_input_field_name(field: InstallConfigField) -> &'static str {
    match field {
        InstallConfigField::SiteDomain => "siteDomain",
        InstallConfigField::PublicEdge => "publicEdge",
        InstallConfigField::CertificateEmail => "certificateEmail",
    }
}

fn install_input_value_text(value: &InstallConfigValue) -> String {
    match value {
        InstallConfigValue::SiteDomain(domain) => domain.clone(),
        InstallConfigValue::PublicEdge(endpoint) => format!(
            "{}://{}:{}",
            transport_name(endpoint.transport),
            endpoint.address,
            endpoint.port
        ),
        InstallConfigValue::CertificateEmail(email) => email.clone(),
    }
}

/// Persisted authorization for one exact allocation candidate. The scope digest
/// includes the complete candidate, including ports, field/transport mappings,
/// prior values, reservations, and read-set revisions.
pub(crate) fn authorize_allocation(
    app: &HostApplication,
    candidate: &PortAllocationCandidate,
    ctx: &OperationContext,
) -> Result<AllocationApproval> {
    ensure_local_root(ctx)?;
    ensure_mutating(ctx, app.control.as_ref())?;
    let now = UnixMillis(app.control.now_ms());
    validate_allocation_review(candidate, now, ctx)?;

    let scope_digest = allocation_scope(
        &app.installation.instance_id,
        &ctx.actor.actor_id,
        &ctx.operation_id,
        &candidate.candidate_digest,
        candidate,
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "port allocation approval scope encoding failed",
        )
    })?;
    let approval_id = ApprovalId::new(format!("allocation.{}", &scope_digest.as_str()[7..39]))
        .expect("digest-derived approval ID");
    let expires_at = candidate.expires_at;
    let record_name = format!("allocation-{}.json", &scope_digest.as_str()[7..39]);
    let approvals = app.work.child("approvals").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "allocation approval inventory unavailable",
        )
    })?;

    if let Some(bytes) = approvals.read(&record_name, 65_536).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "allocation approval record unavailable",
        )
    })? {
        let stored: AllocationConsent = serde_json::from_slice(&bytes).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "allocation approval record corrupt; retained",
            )
        })?;
        if stored.schema_version != 1
            || stored.instance_id != app.installation.instance_id
            || stored.operation_id != ctx.operation_id
            || stored.candidate_digest != candidate.candidate_digest
            || stored.scope_digest != scope_digest
            || stored.issuer != ctx.actor.actor_id
            || stored.approval_id != approval_id
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "allocation approval record does not match this exact candidate",
            ));
        }
        if stored.expires_at.0 <= now.0 || stored.expires_at != expires_at {
            return Err(journal::failure(
                ctx,
                ErrorCode::ApprovalRequired,
                "allocation approval expired; create a fresh candidate",
            ));
        }
        let tx = app.journal.prepare_intent(
            Intent::Approval {
                scope_digest: scope_digest.clone(),
            },
            vec![],
            ctx,
        )?;
        app.journal
            .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
        return Ok(AllocationApproval {
            candidate_digest: candidate.candidate_digest.clone(),
            scope_digest,
            approval_id,
            expires_at,
            instance_id: app.installation.instance_id.clone(),
            issuer: ctx.actor.actor_id.clone(),
            operation_id: ctx.operation_id.clone(),
        });
    }

    confirm_allocation_on_tty(candidate, &scope_digest, ctx, app.control.as_ref())?;
    let encoded = serde_json::to_vec(&AllocationConsent {
        schema_version: 1,
        instance_id: app.installation.instance_id.clone(),
        operation_id: ctx.operation_id.clone(),
        candidate_digest: candidate.candidate_digest.clone(),
        scope_digest: scope_digest.clone(),
        issuer: ctx.actor.actor_id.clone(),
        approval_id: approval_id.clone(),
        expires_at,
    })
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "allocation approval encoding failed",
        )
    })?;
    let tx = app.journal.prepare_intent(
        Intent::Approval {
            scope_digest: scope_digest.clone(),
        },
        vec![],
        ctx,
    )?;
    let lock = approvals
        .lock(deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "allocation approval lock unavailable",
            )
        })?;
    lock.compare_exchange(&record_name, None, &encoded)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "allocation approval publication uncertain; reconcile before retry",
            )
        })?;
    app.journal
        .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
    Ok(AllocationApproval {
        candidate_digest: candidate.candidate_digest.clone(),
        scope_digest,
        approval_id,
        expires_at,
        instance_id: app.installation.instance_id.clone(),
        issuer: ctx.actor.actor_id.clone(),
        operation_id: ctx.operation_id.clone(),
    })
}

/// Mint a one-operation allocation capability before first-install stores exist.
/// The grant lives only in memory; durable approval/journal effects begin after
/// this exact TTY challenge has succeeded and the installer creates its stores.
pub(crate) fn authorize_first_install_allocation(
    instance_id: &InstanceId,
    candidate: &PortAllocationCandidate,
    ctx: &OperationContext,
    control: &dyn OperationControl,
) -> Result<AllocationApproval> {
    ensure_local_root(ctx)?;
    ensure_mutating(ctx, control)?;
    let now = UnixMillis(control.now_ms());
    validate_allocation_review(candidate, now, ctx)?;

    let scope_digest = allocation_scope(
        instance_id,
        &ctx.actor.actor_id,
        &ctx.operation_id,
        &candidate.candidate_digest,
        candidate,
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "port allocation approval scope encoding failed",
        )
    })?;
    confirm_allocation_on_tty(candidate, &scope_digest, ctx, control)?;
    let approval_id = ApprovalId::new(format!("allocation.{}", &scope_digest.as_str()[7..39]))
        .expect("digest-derived approval ID");
    Ok(AllocationApproval {
        candidate_digest: candidate.candidate_digest.clone(),
        scope_digest,
        approval_id,
        expires_at: candidate.expires_at.min(ctx.deadline),
        instance_id: instance_id.clone(),
        issuer: ctx.actor.actor_id.clone(),
        operation_id: ctx.operation_id.clone(),
    })
}

fn validate_allocation_review(
    candidate: &PortAllocationCandidate,
    now: UnixMillis,
    ctx: &OperationContext,
) -> Result<()> {
    if candidate.expires_at.0 <= now.0 || candidate.allocations.is_empty() {
        return Err(journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "port allocation candidate is expired or empty",
        ));
    }
    if candidate.allocations.len() > 64
        || candidate
            .allocations
            .values()
            .any(|allocation| allocation.fields.is_empty() || allocation.fields.len() > 16)
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "port allocation candidate exceeds review bounds",
        ));
    }
    Ok(())
}

fn allocation_scope(
    instance_id: &InstanceId,
    issuer: &ActorId,
    operation_id: &OperationId,
    candidate_digest: &Sha256Digest,
    candidate: &PortAllocationCandidate,
) -> std::result::Result<Sha256Digest, serde_json::Error> {
    serde_json::to_vec(&(
        "allocation-approval-v1",
        instance_id,
        issuer,
        operation_id,
        candidate_digest,
        candidate,
    ))
    .map(|bytes| config::digest(&bytes))
}

fn confirm_allocation_on_tty(
    candidate: &PortAllocationCandidate,
    scope: &Sha256Digest,
    ctx: &OperationContext,
    control: &dyn OperationControl,
) -> Result<()> {
    let fd = rustix::fs::open(
        "/dev/tty",
        OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "port allocation requires exact local TTY approval; --yes cannot replace it",
        )
    })?;
    let mut tty = File::from(fd);
    for (service_id, allocation) in &candidate.allocations {
        let transports = allocation
            .fields
            .iter()
            .map(|field| format!("{}:{}", field.path, transport_name(field.transport)))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            tty,
            "Allocate VPN service {} on port {} ({})",
            service_id, allocation.port, transports
        )
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ApprovalRequired,
                "port allocation approval prompt unavailable",
            )
        })?;
    }
    let expected = format!("PORTS {}", &scope.as_str()[7..23]);
    writeln!(
        tty,
        "Type {expected} to approve this exact allocation candidate:"
    )
    .and_then(|_| tty.flush())
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "port allocation approval prompt unavailable",
        )
    })?;
    read_exact_confirmation(&mut tty, &expected, ctx, control)?;
    Ok(())
}

fn transport_name(transport: Transport) -> &'static str {
    match transport {
        Transport::Tcp => "tcp",
        Transport::Udp => "udp",
    }
}

fn ensure_local_root(ctx: &OperationContext) -> Result<()> {
    if ctx.actor.source != ActorSource::Local
        || rustix::process::geteuid().as_raw() != 0
        || ctx.actor.actor_id.as_str() != "os.uid.0"
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "local root or sudo authority required",
        ));
    }
    Ok(())
}
pub struct Authorized {
    pub context: OperationContext,
    pub approvals: Vec<ApprovalEnvelope>,
    pub verified_ids: BTreeSet<ApprovalId>,
}

pub fn initialize(
    app: &HostApplication,
    transaction: &TransactionId,
    ctx: &OperationContext,
) -> Result<()> {
    ensure_mutating(ctx, app.control.as_ref())?;
    let tx = app.journal.read(transaction, ctx)?;
    if tx.actor != ctx.actor || tx.operation_id != ctx.operation_id {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "policy seed requires exact installation journal",
        ));
    }
    let policy = OwnedPolicy {
        schema_version: 1,
        instance_id: app.installation.instance_id.clone(),
        privileges: BTreeSet::from([
            Privilege::DockerLifecycle,
            Privilege::ManageSystemd,
            Privilege::WriteProtectedState,
        ]),
    };
    let bytes = serde_json::to_vec(&policy).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "local policy encoding failed",
        )
    })?;
    super::installation::seed(
        &app.work,
        "local-policy.json",
        &bytes,
        ctx,
        app.control.as_ref(),
    )?;
    app.work.create_child("approvals").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "private approval inventory unavailable",
        )
    })?;
    Ok(())
}

pub fn authorize(
    app: &HostApplication,
    graph: &DependencyGraph,
    ctx: &OperationContext,
) -> Result<Authorized> {
    ensure_local_root(ctx)?;
    let bytes = app
        .work
        .read("local-policy.json", 65536)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "local owned-resource policy unavailable",
            )
        })?
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ApprovalRequired,
                "local owned-resource policy has not been initialized",
            )
        })?;
    let policy: OwnedPolicy = serde_json::from_slice(&bytes).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "local policy corrupt; retained",
        )
    })?;
    let allowed = BTreeSet::from([
        Privilege::DockerLifecycle,
        Privilege::ManageSystemd,
        Privilege::WriteProtectedState,
    ]);
    if policy.schema_version != 1
        || policy.instance_id != app.installation.instance_id
        || !policy.privileges.is_subset(&allowed)
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "local policy exceeds owned ordinary authority",
        ));
    }
    let registry = app.registry(ctx)?;
    let resources: BTreeSet<_> = registry
        .entries
        .values()
        .filter(|entry| {
            entry.ownership == Ownership::Owned && entry.membership == MembershipState::Installed
        })
        .flat_map(|entry| {
            entry
                .resources
                .iter()
                .map(|resource| resource.resource_id.clone())
        })
        .collect();
    let digest = config::digest(&serde_json::to_vec(&(&policy, &resources)).map_err(|_| {
        journal::failure(ctx, ErrorCode::IoUnavailable, "local grant encoding failed")
    })?);
    let own = ApprovalEnvelope {
        approval_id: ApprovalId::new(format!("owned.{}", &digest.as_str()[7..39]))
            .expect("digest ID"),
        issuer: ctx.actor.actor_id.clone(),
        version: FormatVersion(1),
        listener_scopes: vec![],
        privileges: policy.privileges,
        resources: resources.clone(),
        evidence: vec![],
        expires_at: None,
    };
    let mut approvals = vec![own];
    let listeners: Vec<_> = graph
        .listener_claims
        .iter()
        .filter(|claim| claim.public)
        .cloned()
        .collect();
    if !listeners.is_empty() {
        let scope = config::digest(
            &serde_json::to_vec(&(&app.installation.instance_id, &listeners, &resources)).map_err(
                |_| {
                    journal::failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "public consent scope encoding failed",
                    )
                },
            )?,
        );
        let name = format!("public-{}.json", &scope.as_str()[7..39]);
        let directory = app.work.child("approvals").map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::IoUnavailable,
                "approval inventory unavailable",
            )
        })?;
        let old = directory.read(&name, 262144).map_err(|_| {
            journal::failure(ctx, ErrorCode::IoUnavailable, "public consent unavailable")
        })?;
        let consent = old
            .as_ref()
            .and_then(|bytes| serde_json::from_slice::<PublicConsent>(bytes).ok());
        let valid = consent.as_ref().is_some_and(|consent| {
            consent.schema_version == 1
                && consent.instance_id == app.installation.instance_id
                && consent.grant.issuer == ctx.actor.actor_id
                && consent.grant.listener_scopes == listeners
                && consent.grant.resources == resources
                && consent.grant.privileges == BTreeSet::from([Privilege::BindPublicListener])
                && consent
                    .grant
                    .expires_at
                    .is_some_and(|expires| expires.0 > app.control.now_ms())
        });
        let grant = if valid {
            consent.expect("validated consent").grant
        } else {
            if old.is_some() && consent.is_none() {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "public consent corrupt; original retained",
                ));
            }
            ensure_mutating(ctx, app.control.as_ref()).map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "public listener requires explicit local consent during stage/apply",
                )
            })?;
            let expected = format!("PUBLIC {}", &scope.as_str()[7..23]);
            let fd = rustix::fs::open(
                "/dev/tty",
                OFlags::RDWR
                    | OFlags::NOCTTY
                    | OFlags::CLOEXEC
                    | OFlags::NOFOLLOW
                    | OFlags::NONBLOCK,
                Mode::empty(),
            )
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "public consent requires a controlling TTY; --yes cannot replace it",
                )
            })?;
            let mut tty = File::from(fd);
            for listener in &listeners {
                writeln!(
                    tty,
                    "Authorize public {:?} {}:{} for {}",
                    listener.transport, listener.address, listener.port, listener.service_id
                )
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::ApprovalRequired,
                        "public consent prompt unavailable",
                    )
                })?;
            }
            writeln!(
                tty,
                "Type {expected} to authorize this exact scope for five minutes:"
            )
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "public consent prompt unavailable",
                )
            })?;
            tty.flush().map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "public consent prompt unavailable",
                )
            })?;
            read_exact_confirmation(&mut tty, &expected, ctx, app.control.as_ref())?;
            let grant = ApprovalEnvelope {
                approval_id: ApprovalId::new(format!("public.{}", &scope.as_str()[7..39]))
                    .expect("digest ID"),
                issuer: ctx.actor.actor_id.clone(),
                version: FormatVersion(1),
                listener_scopes: listeners,
                privileges: BTreeSet::from([Privilege::BindPublicListener]),
                resources,
                evidence: vec![],
                expires_at: Some(UnixMillis(app.control.now_ms().saturating_add(300_000))),
            };
            let bytes = serde_json::to_vec(&PublicConsent {
                schema_version: 1,
                instance_id: app.installation.instance_id.clone(),
                grant: grant.clone(),
            })
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "public consent encoding failed",
                )
            })?;
            let consent_tx = app.journal.prepare_intent(
                crate::journal::Intent::Approval {
                    scope_digest: scope.clone(),
                },
                vec![],
                ctx,
            )?;
            let locked = directory
                .lock(deadline(ctx, app.control.as_ref())?)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "public consent lock unavailable",
                    )
                })?;
            locked
                .compare_exchange(&name, old.as_deref(), &bytes)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "public consent publication uncertain",
                    )
                })?;
            let state = app.runtime(ctx)?;
            let accepted = state
                .accepted
                .as_ref()
                .map(|bundle| BundleId::new(bundle.as_str()))
                .transpose()
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "accepted identity invalid",
                    )
                })?;
            let outcome = OperationResult {
                operation_id: ctx.operation_id.clone(),
                status: OperationStatus::Succeeded,
                transaction_id: Some(consent_tx.clone()),
                desired_saved: false,
                pending_bundle: None,
                accepted_bundle: accepted,
                run_intent: match state.run_intent {
                    hton_runtime_contract::RunIntent::Running => RunIntent::Running,
                    hton_runtime_contract::RunIntent::Stopped => RunIntent::Stopped,
                },
                member_results: vec![],
                survivors: vec![],
                issues: vec![],
            };
            app.journal.finish_operation(
                &consent_tx,
                crate::journal::TerminalStatus::Committed,
                vec![],
                &outcome,
                ctx,
            )?;
            grant
        };
        approvals.push(grant);
    }
    let verified_ids = approvals
        .iter()
        .map(|approval| approval.approval_id.clone())
        .collect::<BTreeSet<_>>();
    let mut context = ctx.clone();
    context
        .actor
        .local_approval_refs
        .extend(verified_ids.iter().cloned());
    Ok(Authorized {
        context,
        approvals,
        verified_ids,
    })
}

/// This capability cannot be deserialized or manufactured by CLI input.
pub(crate) struct PurgeApproval {
    digest: Sha256Digest,
}
impl PurgeApproval {
    pub(crate) fn digest(&self) -> &Sha256Digest {
        &self.digest
    }
}

pub(crate) fn authorize_module_purge(
    app: &HostApplication,
    module: &ModuleId,
    artifact: &ArtifactId,
    scope: &Sha256Digest,
    ctx: &OperationContext,
) -> Result<PurgeApproval> {
    ensure_local_root(ctx)?;
    ensure_mutating(ctx, app.control.as_ref())?;
    let fd = rustix::fs::open(
        "/dev/tty",
        OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "module purge requires exact local TTY confirmation",
        )
    })?;
    let mut tty = File::from(fd);
    let expected = format!("PURGE {}", &scope.as_str()[7..23]);
    writeln!(tty, "Delete retained configuration artifact {artifact} for removed module {module}. Shared engines and credentials are preserved.\nType {expected} to approve this exact deletion:")
        .and_then(|_| tty.flush()).map_err(|_| journal::failure(ctx, ErrorCode::ApprovalRequired,
            "module purge confirmation prompt unavailable"))?;
    read_exact_confirmation(&mut tty, &expected, ctx, app.control.as_ref())?;
    Ok(PurgeApproval {
        digest: scope.clone(),
    })
}

/// I/O only: domain owners retain approval scope, lifetime and persistence rules.
pub(crate) fn confirm_local_scope(
    app: &HostApplication,
    ctx: &OperationContext,
    prompt: &str,
    expected: &str,
) -> Result<()> {
    ensure_local_root(ctx)?;
    ensure_mutating(ctx, app.control.as_ref())?;
    if prompt.len() > 16_384 || prompt.contains('\0') {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "local confirmation prompt exceeds bounds",
        ));
    }
    let fd = rustix::fs::open(
        "/dev/tty",
        OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "exact local confirmation requires a controlling TTY; --yes cannot replace it",
        )
    })?;
    let mut tty = File::from(fd);
    writeln!(
        tty,
        "{prompt}\nType {expected} to approve this exact scope:"
    )
    .and_then(|_| tty.flush())
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ApprovalRequired,
            "local confirmation prompt unavailable",
        )
    })?;
    read_exact_confirmation(&mut tty, expected, ctx, app.control.as_ref())
}

fn read_exact_confirmation(
    tty: &mut File,
    expected: &str,
    ctx: &OperationContext,
    control: &dyn OperationControl,
) -> Result<()> {
    if expected.is_empty()
        || expected.len() >= 128
        || !expected.bytes().all(|byte| (b' '..=b'~').contains(&byte))
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "local confirmation scope is invalid",
        ));
    }
    let mut answer = Vec::new();
    let mut byte = [0u8; 1];
    while answer.len() < 128 {
        ensure_mutating(ctx, control)?;
        match tty.read(&mut byte) {
            Ok(0) => break,
            Ok(_) if byte[0] == b'\n' => {
                if answer == expected.as_bytes() {
                    return Ok(());
                }
                break;
            }
            Ok(_) => answer.push(byte[0]),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "local confirmation input unavailable",
                ));
            }
        }
    }
    Err(journal::failure(
        ctx,
        ErrorCode::ApprovalRequired,
        "exact local confirmation declined",
    ))
}
