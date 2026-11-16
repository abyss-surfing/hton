//! Bounded typed IPC to the independent runtime and protected authority publication.
use super::*;
use crate::journal::{Intent, deadline, ensure_mutating};
use hton_runtime_contract as wire;
use std::{
    num::{NonZeroU16, NonZeroU32},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

pub fn execute(
    app: &HostApplication,
    action: wire::ExecutorAction,
    ctx: &OperationContext,
) -> Result<wire::RuntimeResult> {
    if !matches!(action, wire::ExecutorAction::Observe) {
        ensure_mutating(ctx, app.control.as_ref())?;
    }
    if app.control.now_ms() >= ctx.deadline.0 || app.control.is_cancelled(&ctx.cancellation) {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDeadline,
            "runtime request budget expired or cancelled",
        ));
    }
    let before = app.runtime(ctx)?;
    let context = wire::OperationContext {
        operation_id: wire::OperationId::new(ctx.operation_id.as_str()).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "operation identity invalid",
            )
        })?,
        caller: wire::CallerId::new(ctx.actor.actor_id.as_str()).map_err(|_| {
            journal::failure(ctx, ErrorCode::OperationDenied, "actor identity invalid")
        })?,
        budget: wire::Budget {
            deadline_ms: ctx.deadline.0,
            max_concurrency: NonZeroU16::new(4).expect("constant"),
            max_bytes: NonZeroU32::new(4 * 1024 * 1024).expect("constant"),
            max_observations: NonZeroU16::new(1024).expect("constant"),
        },
        cancellation_id: None,
    };
    let request = wire::ExecuteRequest {
        contract_version: wire::ContractVersion::V1,
        instance_id: before.instance_id,
        expected_run_epoch: before.run_epoch,
        action,
        context,
    };
    let bytes = serde_json::to_vec(&request).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "runtime request encoding failed",
        )
    })?;
    let remaining = ctx
        .deadline
        .0
        .saturating_sub(app.control.now_ms())
        .min(300_000);
    let budget = hton_host::Budget::new(
        Instant::now() + Duration::from_millis(remaining),
        wire::MAX_WIRE_BYTES,
    )
    .map_err(|_| journal::failure(ctx, ErrorCode::OperationDeadline, "runtime budget invalid"))?;
    let tools = hton_host::HostObserver::new();
    let response = tools
        .runtime_client()
        .execute(&bytes, budget, &AtomicBool::new(false))
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "runtime transport failed; inspect durable state before retry",
            )
        })?;
    if let Ok(result) = wire::decode::<wire::RuntimeResult>(&response) {
        if result.operation_id != request.context.operation_id {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "runtime result identity mismatch",
            ));
        }
        return Ok(result);
    }
    if let Ok(error) = wire::decode::<wire::RuntimeError>(&response) {
        let code = match error.code {
            wire::ErrorCode::Stopped => ErrorCode::OperationConflict,
            wire::ErrorCode::StaleInput => ErrorCode::PlanStale,
            wire::ErrorCode::ApprovalRequired => ErrorCode::ApprovalRequired,
            wire::ErrorCode::MissingArtifact => ErrorCode::ModuleIncomplete,
            wire::ErrorCode::MissingImage => ErrorCode::ModuleIncomplete,
            wire::ErrorCode::DeadlineExceeded => ErrorCode::OperationDeadline,
            _ => ErrorCode::RecoveryRequired,
        };
        return Err(journal::failure(
            ctx,
            code,
            "independent runtime rejected execution; inspect local recovery metadata",
        ));
    }
    Err(journal::failure(
        ctx,
        ErrorCode::RecoveryRequired,
        "runtime result is malformed; original state retained",
    ))
}

pub fn publish_authority(
    app: &HostApplication,
    authority: &wire::RuntimeAuthority,
    transaction: &TransactionId,
    ctx: &OperationContext,
) -> Result<()> {
    ensure_mutating(ctx, app.control.as_ref())?;
    let tx = app.journal.read(transaction, ctx)?;
    let plan = match &tx.intent {
        Intent::Effects { plan } | Intent::PrepareBundle { plan } => plan,
        _ => {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "authority requires an effects journal intent",
            ));
        }
    };
    let state = app.runtime(ctx)?;
    let accepted = state
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
    let runtime_revision = crate::plan::runtime_binding_revision(
        StateRevision(state.revision),
        match state.run_intent {
            wire::RunIntent::Running => RunIntent::Running,
            wire::RunIntent::Stopped => RunIntent::Stopped,
        },
        RunEpoch(state.run_epoch),
        &accepted,
    )
    .map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "runtime binding encoding failed",
        )
    })?;
    if tx.operation_id != ctx.operation_id
        || tx.actor != ctx.actor
        || authority.instance_id.as_str() != app.installation.instance_id.as_str()
        || authority.run_epoch != state.run_epoch
        || !plan
            .read_set
            .objects
            .get(&ObjectId::new("state.runtime").expect("fixed ID"))
            .is_some_and(|dependency| dependency.revision == runtime_revision)
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "authority publication fence changed",
        ));
    }
    let bundles = app.work.child("bundles").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "bundle inventory unavailable",
        )
    })?;
    let bundle = bundles.child(authority.bundle_id.as_str()).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ModuleIncomplete,
            "compiled bundle unavailable",
        )
    })?;
    let bytes = bundle
        .read("manifest.json", wire::MAX_WIRE_BYTES)
        .map_err(|_| {
            journal::failure(ctx, ErrorCode::IoUnavailable, "bundle manifest unavailable")
        })?
        .ok_or_else(|| {
            journal::failure(ctx, ErrorCode::ModuleIncomplete, "bundle manifest missing")
        })?;
    let candidate: wire::CandidateBundle = wire::decode(&bytes).map_err(|_| {
        journal::failure(ctx, ErrorCode::RecoveryRequired, "bundle manifest corrupt")
    })?;
    if candidate.bundle_id != authority.bundle_id
        || candidate.instance_id != authority.instance_id
        || candidate.manifest_digest != authority.bundle_digest
        || wire::candidate_digest(&candidate).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "candidate fingerprint unavailable",
            )
        })? != authority.bundle_digest
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "authority is not bound to exact compiled bundle",
        ));
    }
    let bytes = serde_json::to_vec(authority).map_err(|_| {
        journal::failure(ctx, ErrorCode::IoUnavailable, "authority encoding failed")
    })?;
    if bytes.len() > wire::MAX_WIRE_BYTES {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "authority exceeds bounded wire size",
        ));
    }
    let runtime = app.work.child("runtime").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "runtime inventory unavailable",
        )
    })?;
    let dir = runtime.child("authorities").map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "authority inventory unavailable",
        )
    })?;
    let locked = dir
        .lock(deadline(ctx, app.control.as_ref())?)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "authority publication lock unavailable",
            )
        })?;
    let name = format!("{}.json", authority.bundle_id.as_str());
    let old = locked.read(&name, wire::MAX_WIRE_BYTES).map_err(|_| {
        journal::failure(ctx, ErrorCode::IoUnavailable, "prior authority unavailable")
    })?;
    if old.as_ref().is_some_and(|old| old != &bytes) {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationConflict,
            "immutable bundle authority already differs",
        ));
    }
    locked
        .compare_exchange(&name, old.as_deref(), &bytes)
        .map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "authority publication durability uncertain",
            )
        })?;
    Ok(())
}

pub fn map_result(result: &wire::RuntimeResult, ctx: &OperationContext) -> Result<OperationResult> {
    if result.operation_id.as_str() != ctx.operation_id.as_str() {
        return Err(journal::failure(
            ctx,
            ErrorCode::RecoveryRequired,
            "runtime result operation mismatch",
        ));
    }
    Ok(OperationResult {
        operation_id: ctx.operation_id.clone(),
        status: match result.status {
            wire::ResultStatus::Succeeded => OperationStatus::Succeeded,
            wire::ResultStatus::Partial => OperationStatus::Partial,
            wire::ResultStatus::Failed => OperationStatus::Failed,
            wire::ResultStatus::RecoveryRequired => OperationStatus::RecoveryRequired,
        },
        transaction_id: None,
        desired_saved: false,
        pending_bundle: None,
        accepted_bundle: result
            .accepted
            .as_ref()
            .map(|id| BundleId::new(id.as_str()))
            .transpose()
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "accepted bundle identity invalid",
                )
            })?,
        run_intent: match result.run_intent {
            wire::RunIntent::Running => RunIntent::Running,
            wire::RunIntent::Stopped => RunIntent::Stopped,
        },
        member_results: vec![],
        survivors: result
            .survivors
            .as_slice()
            .iter()
            .map(|id| ResourceId::new(id.as_str()))
            .collect::<std::result::Result<_, _>>()
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "survivor identity invalid",
                )
            })?,
        issues: result
            .errors
            .as_slice()
            .iter()
            .map(|_| {
                config::issue(
                    ErrorCode::RecoveryRequired,
                    None,
                    None,
                    "Native member failed or needs reconciliation; inspect runtime receipts",
                )
            })
            .collect(),
    })
}
