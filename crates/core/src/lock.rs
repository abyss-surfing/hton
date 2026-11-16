//! Bounded RAII mutation lock; release before waiting for runtime trial.
use crate::journal::{OperationControl, deadline, ensure_mutating, failure};
use hton_contracts::operations::*;
use hton_storage::{LockedDir, SecureDir};
pub type LockGuard = LockedDir;
pub fn acquire(
    directory: &SecureDir,
    _project_id: &InstanceId,
    ctx: &OperationContext,
    control: &dyn OperationControl,
) -> Result<LockGuard> {
    ensure_mutating(ctx, control)?;
    directory.lock(deadline(ctx, control)?).map_err(|_| {
        failure(
            ctx,
            ErrorCode::OperationConflict,
            "project mutation lock unavailable before deadline",
        )
    })
}
