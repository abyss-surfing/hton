//! Config-independent local lifecycle composition; no native state mutation rules.
use super::*;
use crate::{
    catalog::ModuleCatalogue,
    lifecycle::{self, LifecycleSnapshot, PreparedOperation},
};
use hton_runtime_contract as wire;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn runtime_operation(
    app: &HostApplication,
    operation: &str,
    args: &[String],
    ctx: &OperationContext,
) -> Result<Value> {
    let state = app.runtime(ctx)?;
    let registry = app.registry(ctx)?;
    let catalogue = ModuleCatalogue::builtin().map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "trusted catalogue unavailable",
        )
    })?;
    let desired = if operation == "services.restart" {
        Some(app.desired(ctx)?)
    } else {
        None
    };
    let request = match operation {
        "lifecycle.down" if args.is_empty() => LifecycleRequest::Down,
        // Whole-server `up` deliberately reuses the explicit accepted-start path:
        // it starts the saved composition without compiling desired YAML or pulling
        // images. Candidate changes belong to config apply.
        "lifecycle.up" | "runtime.start" => {
            let selected = match args {
                [] => state.accepted.as_ref().map(|id| id.as_str()),
                [option, id] if option == "--bundle" => Some(id.as_str()),
                _ => {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "invalid accepted startup arguments",
                    ));
                }
            };
            let id = selected.ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "no accepted bundle is available",
                )
            })?;
            LifecycleRequest::StartAccepted {
                bundle_id: BundleId::new(id).map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "invalid accepted bundle identity",
                    )
                })?,
                approval_refs: BTreeSet::new(),
            }
        }
        "runtime.rollback" => {
            let target = match args {
                [] => state.previous_compatible.as_ref().map(|id| id.as_str()),
                [option, id] if option == "--bundle" => Some(id.as_str()),
                _ => {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "rollback accepts at most one bundle ID",
                    ));
                }
            }
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "no previous-compatible bundle is available",
                )
            })?;
            LifecycleRequest::RollbackAccepted {
                bundle_id: BundleId::new(target).map_err(|_| {
                    journal::failure(ctx, ErrorCode::OperationDenied, "invalid bundle identity")
                })?,
            }
        }
        "services.restart" => {
            let mut ids = Vec::new();
            let mut wait_seconds = 10;
            let mut wait_seen = false;
            let mut index = 0;
            while index < args.len() {
                if args[index] == "--wait" {
                    if wait_seen {
                        return Err(journal::failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "duplicate restart wait bound",
                        ));
                    }
                    wait_seen = true;
                    index += 1;
                    let wait = args.get(index).and_then(|value| value.parse::<u16>().ok());
                    if !wait.is_some_and(|value| (1..=300).contains(&value)) {
                        return Err(journal::failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "invalid restart wait bound",
                        ));
                    }
                    wait_seconds = wait.expect("validated wait bound");
                } else if !args[index].starts_with('-') {
                    ids.push(wire::ServiceId::new(args[index].as_str()).map_err(|_| {
                        journal::failure(ctx, ErrorCode::OperationDenied, "invalid service ID")
                    })?);
                } else {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "invalid restart arguments",
                    ));
                }
                index += 1;
            }
            let bundle_id = state.accepted.clone().ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "no accepted bundle is available",
                )
            })?;
            LifecycleRequest::RestartServices {
                service_ids: ids,
                wait_seconds,
                bundle_id,
            }
        }
        _ => {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "invalid config-independent lifecycle operation",
            ));
        }
    };
    let snapshot = LifecycleSnapshot {
        desired,
        registry: registry.clone(),
        runtime: state,
        observations: RuntimeObservation {
            resources: BTreeMap::new(),
        },
    };
    if ctx.execution_mode == ExecutionMode::ReadOnly {
        let plan = lifecycle::plan(
            request,
            &snapshot,
            &PreparedOperation::default(),
            &catalogue,
            ctx,
        )?;
        return Ok(json!({"status":"preview","operation":operation,"plan":plan,"writes":false}));
    }
    let result = lifecycle::execute_runtime_request(
        request,
        &snapshot,
        &catalogue,
        &app.journal,
        |plan, journal_receipts| {
            let action = match &plan.request {
                LifecycleRequest::Down => wire::ExecutorAction::StopAll,
                LifecycleRequest::StartAccepted { bundle_id, .. } => {
                    wire::ExecutorAction::StartAccepted {
                        bundle_id: wire::BundleId::new(bundle_id.as_str()).map_err(|_| {
                            journal::failure(
                                ctx,
                                ErrorCode::OperationDenied,
                                "invalid accepted identity",
                            )
                        })?,
                    }
                }
                LifecycleRequest::RollbackAccepted { bundle_id } => {
                    wire::ExecutorAction::RollbackAccepted {
                        bundle_id: wire::BundleId::new(bundle_id.as_str()).map_err(|_| {
                            journal::failure(
                                ctx,
                                ErrorCode::OperationDenied,
                                "invalid rollback identity",
                            )
                        })?,
                    }
                }
                LifecycleRequest::RestartServices {
                    service_ids,
                    bundle_id,
                    ..
                } => wire::ExecutorAction::RestartAccepted {
                    bundle_id: wire::BundleId::new(bundle_id.as_str()).map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "invalid accepted bundle identity",
                        )
                    })?,
                    expected_run_epoch: plan.expected_inputs.run_epoch.0,
                    service_ids: wire::Items::new(service_ids.clone()).map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "invalid restart service selection",
                        )
                    })?,
                },
                _ => {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "lifecycle action escaped fixed request",
                    ));
                }
            };
            if let LifecycleRequest::RestartServices { bundle_id, .. } = &plan.request {
                let current_runtime = app.runtime(ctx)?;
                let current_registry = app.registry(ctx)?;
                let current_desired = app.desired(ctx)?;
                if current_runtime.run_epoch != plan.expected_inputs.run_epoch.0
                    || current_runtime.revision != plan.expected_inputs.runtime_revision.0
                    || current_runtime.accepted.as_ref().map(|id| id.as_str())
                        != Some(bundle_id.as_str())
                    || current_registry.revision != plan.expected_inputs.registry_revision
                    || current_desired.read_set != plan.expected_inputs.read_set
                    || current_desired.snapshot.is_none()
                    || !current_desired.errors.is_empty()
                {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::PlanStale,
                        "restart selection inputs changed before Runtime dispatch",
                    ));
                }
            }
            let mut action_ctx = ctx.clone();
            if let LifecycleRequest::RestartServices { wait_seconds, .. } = &plan.request {
                let wait_deadline = app
                    .control
                    .now_ms()
                    .saturating_add(u64::from(*wait_seconds) * 1_000);
                action_ctx.deadline.0 = action_ctx.deadline.0.min(wait_deadline);
            }
            let native = super::runtime::execute(app, action, &action_ctx)?;
            if matches!(&plan.request, LifecycleRequest::RestartServices { .. }) {
                for receipt in native.receipts.as_slice() {
                    let (wire::ActionOutcome::Completed { receipt: resource }
                    | wire::ActionOutcome::Unchanged { receipt: resource }) = &receipt.outcome
                    else {
                        continue;
                    };
                    let resource_id = ResourceId::new(resource.resource.resource_id.as_str())
                        .map_err(|_| {
                            journal::failure(
                                ctx,
                                ErrorCode::RecoveryRequired,
                                "Runtime receipt resource ID is invalid",
                            )
                        })?;
                    let fingerprint = hton_contracts::config::Sha256Digest::new(
                        resource.resource.fingerprint.as_str(),
                    )
                    .map_err(|_| {
                        journal::failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "Runtime receipt fingerprint is invalid",
                        )
                    })?;
                    journal_receipts.push(StepReceipt {
                        step_id: StepId::new(receipt.action_id.as_str()).map_err(|_| {
                            journal::failure(
                                ctx,
                                ErrorCode::RecoveryRequired,
                                "Runtime action receipt ID is invalid",
                            )
                        })?,
                        idempotency_key: RequestId::new(ctx.operation_id.as_str()).map_err(
                            |_| {
                                journal::failure(
                                    ctx,
                                    ErrorCode::RecoveryRequired,
                                    "operation ID cannot bind Runtime receipt",
                                )
                            },
                        )?,
                        status: StepStatus::Verified,
                        recorded_at: UnixMillis(resource.timestamp_ms),
                        resources: vec![],
                        state_fingerprints: BTreeMap::from([(resource_id, fingerprint)]),
                        retained_artifacts: vec![],
                        error: None,
                    });
                }
            }
            let mut result = super::runtime::map_result(&native, ctx)?;
            if let LifecycleRequest::RestartServices { service_ids, .. } = &plan.request {
                for service_id in service_ids {
                    let owner = registry
                        .entries
                        .iter()
                        .find_map(|(module_id, entry)| {
                            (module_id.as_str() != "core"
                                && entry.ownership == Ownership::Owned
                                && entry.membership == MembershipState::Installed
                                && entry.setup_state == SetupState::Complete)
                                .then(|| {
                                    entry
                                        .resources
                                        .iter()
                                        .find(|resource| {
                                            resource.ownership == Ownership::Owned
                                                && resource.service_id.as_str()
                                                    == service_id.as_str()
                                        })
                                        .map(|resource| (module_id, resource))
                                })
                                .flatten()
                        })
                        .ok_or_else(|| {
                            journal::failure(ctx, ErrorCode::PlanStale, "restart owner disappeared")
                        })?;
                    result.member_results.push(MemberResult {
                        module_id: owner.0.clone(),
                        status: if result.status == OperationStatus::Succeeded {
                            OperationStatus::Succeeded
                        } else {
                            OperationStatus::Partial
                        },
                        resources: vec![owner.1.resource_id.clone()],
                        error: None,
                    });
                }
            } else {
                for (id, entry) in &registry.entries {
                    let resources: Vec<_> = entry
                        .resources
                        .iter()
                        .map(|resource| resource.resource_id.clone())
                        .collect();
                    let failed = native.errors.as_slice().iter().any(|error| {
                        error.resource_id.as_ref().is_some_and(|resource| {
                            resources
                                .iter()
                                .any(|known| known.as_str() == resource.as_str())
                        })
                    });
                    result.member_results.push(MemberResult {
                        module_id: id.clone(),
                        status: if failed {
                            OperationStatus::Partial
                        } else if result.status == OperationStatus::Succeeded {
                            OperationStatus::Succeeded
                        } else {
                            result.status
                        },
                        resources,
                        error: None,
                    });
                }
            }
            Ok(result)
        },
        ctx,
    )?;
    serde_json::to_value(result).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::IoUnavailable,
            "lifecycle result encoding failed",
        )
    })
}
