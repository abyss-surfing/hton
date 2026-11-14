//! Explicit public binding cutover through the shared admitted apply owner.
use super::{HostApplication, effects};
use crate::journal;
use hton_contracts::operations::*;
use serde_json::{Value, json};

pub fn switch(app: &HostApplication, args: &[String], ctx: &OperationContext) -> Result<Value> {
    let mut validate_only = false;
    let mut port = None;
    for argument in args {
        if argument == "--validate-only" && !validate_only {
            validate_only = true;
        } else if port.is_none() && !argument.starts_with('-') {
            port = Some(
                argument
                    .parse::<u16>()
                    .ok()
                    .filter(|port| *port != 0)
                    .ok_or_else(|| {
                        journal::failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "public cutover port must be in 1-65535",
                        )
                    })?,
            );
        } else {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "public cutover accepts one port and --validate-only",
            ));
        }
    }
    let preview = validate_only || ctx.execution_mode == ExecutionMode::ReadOnly;
    // Reject an incorrect target assertion before preparation can request local
    // consent. The later graph check and root revision comparison close drift.
    let desired = app.desired(ctx)?;
    let saved_port = desired
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.platform.settings.get("core"))
        .and_then(|core| core.get("edge"))
        .and_then(|edge| edge.get("listen"))
        .and_then(|listen| listen.get("port"))
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port != 0)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "public cutover requires a valid saved edge port",
            )
        })?;
    if port.is_some_and(|port| port != saved_port) {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "requested port differs from saved edge configuration; edit desired YAML before cutover",
        ));
    }
    let mut prepare_ctx = ctx.clone();
    if preview {
        prepare_ctx.execution_mode = ExecutionMode::ReadOnly;
    }
    let (plan, target_port) = effects::prepare_publication(app, port, &prepare_ctx)?;
    let root = ObjectId::new("root").expect("fixed object ID");
    if plan.read_set.objects.get(&root) != desired.read_set.objects.get(&root) {
        return Err(journal::failure(
            ctx,
            ErrorCode::PlanStale,
            "desired edge configuration changed during public cutover preparation",
        ));
    }
    if preview {
        return Ok(json!({"status":"preview", "operation":"publication.switch",
            "port":target_port, "writes":false, "plan":plan,
            "servedState":"unverified", "externalDns":"unverified"}));
    }
    let result = effects::apply(app, &plan, ctx)?;
    Ok(
        json!({"status":result.status, "operation":"publication.switch",
        "port":target_port, "operationResult":result,
        "servedState":"unverified", "externalDns":"unverified"}),
    )
}
