//! R1 CLI identity enrollment. Account ownership/single-disclosure semantics are R2.
use crate::{
    apply::EffectApplyPort,
    journal::{Intent, JournalStore, ProtectedOriginal, TerminalStatus, ensure_mutating, failure},
    secrets::{ReplacementPolicy, SecretKind, SecretRef, SecretStore},
};
use hton_contracts::operations::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityIssueRequest {
    pub identity_id: ObjectId,
    pub profile_id: ObjectId,
    pub replace: bool,
    pub approval_id: Option<ApprovalId>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityRevokeRequest {
    pub identity_id: ObjectId,
    pub profile_id: ObjectId,
    pub approval_id: ApprovalId,
}
#[derive(Clone, Debug)]
pub struct QualifiedIdentityProfile {
    pub profile_id: ObjectId,
    pub service_id: ServiceId,
    pub scope: ScopeId,
    pub enabled: bool,
    pub installed: bool,
    pub qualified: bool,
    pub supports_independent_credentials: bool,
    pub credential_kind: SecretKind,
    pub externally_owned: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BindingState {
    Pending,
    Accepted,
    Revoked,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityBinding {
    pub identity_id: ObjectId,
    pub profile_id: ObjectId,
    pub service_id: ServiceId,
    pub scope: ScopeId,
    pub secret: SecretRef,
    pub state: BindingState,
    pub accepted_bundle: Option<BundleId>,
    pub transaction_id: TransactionId,
}
impl fmt::Debug for IdentityBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentityBinding")
            .field("identity_id", &self.identity_id)
            .field("profile_id", &self.profile_id)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug)]
pub struct IssuanceResult {
    pub binding: IdentityBinding,
    pub operation: OperationResult,
}
pub trait IdentityInventory: Send + Sync {
    fn run_state(&self, ctx: &OperationContext) -> Result<(RunIntent, Option<BundleId>)>;
    fn authorize_issue(
        &self,
        request: &IdentityIssueRequest,
        ctx: &OperationContext,
    ) -> Result<bool>;
    fn authorize_revoke(
        &self,
        request: &IdentityRevokeRequest,
        ctx: &OperationContext,
    ) -> Result<bool>;
    fn profile(&self, id: &ObjectId, ctx: &OperationContext) -> Result<QualifiedIdentityProfile>;
    fn binding(
        &self,
        identity: &ObjectId,
        profile: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<Option<IdentityBinding>>;
    fn by_transaction(
        &self,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<Option<IdentityBinding>>;
    fn originals(
        &self,
        identity: &ObjectId,
        profile: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<Vec<ProtectedOriginal>>;
    /// CAS-only owned identity inventory writer. It never edits an engine's live config.
    fn save_pending(
        &self,
        binding: &IdentityBinding,
        previous: Option<&IdentityBinding>,
        ctx: &OperationContext,
    ) -> Result<()>;
    /// Pure plan over current identity inventory; its exact revision is in readSet.
    fn plan_issue(&self, binding: &IdentityBinding, ctx: &OperationContext) -> Result<EffectPlan>;
    fn plan_revoke(&self, binding: &IdentityBinding, ctx: &OperationContext) -> Result<EffectPlan>;
    fn accepted_verified(
        &self,
        binding: &IdentityBinding,
        bundle: &BundleId,
        ctx: &OperationContext,
    ) -> Result<bool>;
    /// Includes active AND admissible fallback credentials and monotonic fences.
    fn revocation_verified(
        &self,
        binding: &IdentityBinding,
        ctx: &OperationContext,
    ) -> Result<bool>;
    fn finalize(
        &self,
        binding: &IdentityBinding,
        state: BindingState,
        bundle: Option<&BundleId>,
        ctx: &OperationContext,
    ) -> Result<IdentityBinding>;
}
pub struct IdentityCoordinator<'a> {
    pub journal: &'a JournalStore,
    pub inventory: &'a dyn IdentityInventory,
    pub secrets: &'a SecretStore,
    pub apply: &'a dyn EffectApplyPort,
}
impl IdentityCoordinator<'_> {
    pub fn issue(
        &self,
        request: &IdentityIssueRequest,
        ctx: &OperationContext,
    ) -> Result<IssuanceResult> {
        ensure_mutating(ctx, self.journal.control())?;
        if !self.inventory.authorize_issue(request, ctx)? {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "identity issue is outside authorized profile scope",
            ));
        }
        let intent = Intent::Identity {
            request_digest: crate::config::digest(
                &serde_json::to_vec(&("issue", request)).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "identity request encoding failed",
                    )
                })?,
            ),
        };
        if let Some(tx) = self.journal.lookup_intent(&intent, ctx)? {
            if let Some(record) = self.journal.terminal(&tx.transaction_id, ctx)? {
                let operation = record.operation_result.ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "identity operation status unavailable",
                    )
                })?;
                let binding = self
                    .inventory
                    .by_transaction(&tx.transaction_id, ctx)?
                    .ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "identity metadata missing",
                        )
                    })?;
                return Ok(IssuanceResult { binding, operation });
            }
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "interrupted identity issue requires reconciliation",
            ));
        }
        let profile = self.inventory.profile(&request.profile_id, ctx)?;
        if profile.profile_id != request.profile_id
            || !profile.installed
            || !profile.qualified
            || !profile.supports_independent_credentials
            || profile.externally_owned
            || profile.credential_kind != SecretKind::UuidV4
        {
            return Err(failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "profile does not support qualified independent R1 VLESS credentials",
            ));
        }
        let previous = self
            .inventory
            .binding(&request.identity_id, &request.profile_id, ctx)?;
        if request.replace {
            if previous.is_none()
                || !request
                    .approval_id
                    .as_ref()
                    .is_some_and(|a| ctx.actor.local_approval_refs.contains(a))
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "identity replacement requires existing binding and local approval",
                ));
            }
        } else if previous.is_some() {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "identity already exists; use explicit replacement",
            ));
        }
        let tx = self.journal.prepare_intent(
            intent,
            self.inventory
                .originals(&request.identity_id, &request.profile_id, ctx)?,
            ctx,
        )?;
        let secret_id = SecretId::new(format!(
            "identity.{}",
            &crate::config::digest(
                &serde_json::to_vec(&(&request.identity_id, &request.profile_id)).map_err(
                    |_| failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "identity key encoding failed"
                    )
                )?
            )
            .as_str()[7..]
        ))
        .map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "identity secret name invalid",
            )
        })?;
        let policy = match &previous {
            Some(old) => ReplacementPolicy::Rotate {
                previous: old.secret.clone(),
                confirmed: request.replace,
            },
            None => ReplacementPolicy::Create,
        };
        let generated = self.secrets.generate(
            secret_id,
            profile.scope.clone(),
            profile.credential_kind,
            BTreeSet::from([profile.service_id.clone()]),
            policy,
            ctx,
        )?;
        // Generated disclosure is intentionally discarded until accepted; R1 show/share
        // resolves the accepted binding explicitly, rather than leaking a pending key.
        let binding = IdentityBinding {
            identity_id: request.identity_id.clone(),
            profile_id: request.profile_id.clone(),
            service_id: profile.service_id,
            scope: profile.scope,
            secret: generated.reference,
            state: BindingState::Pending,
            accepted_bundle: None,
            transaction_id: tx.clone(),
        };
        self.inventory
            .save_pending(&binding, previous.as_ref(), ctx)?;
        if !profile.enabled {
            let (run_intent, accepted_bundle) = self.inventory.run_state(ctx)?;
            let operation = OperationResult {
                operation_id: ctx.operation_id.clone(),
                status: OperationStatus::Partial,
                transaction_id: Some(tx.clone()),
                desired_saved: true,
                pending_bundle: None,
                accepted_bundle,
                run_intent,
                member_results: vec![],
                survivors: vec![],
                issues: vec![],
            };
            self.journal
                .finish_operation(&tx, TerminalStatus::Partial, vec![], &operation, ctx)?;
            return Ok(IssuanceResult { binding, operation });
        }
        let plan = self.inventory.plan_issue(&binding, ctx)?;
        let mut operation = self.apply.stage_and_apply(&plan, ctx)?;
        operation.transaction_id = Some(tx.clone());
        let mut final_binding = binding;
        if operation.status == OperationStatus::Succeeded
            && operation.run_intent == RunIntent::Running
        {
            let accepted = operation.accepted_bundle.as_ref().ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "identity apply did not return accepted bundle",
                )
            })?;
            if !self
                .inventory
                .accepted_verified(&final_binding, accepted, ctx)?
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "identity credential was not independently verified",
                ));
            }
            final_binding = self.inventory.finalize(
                &final_binding,
                BindingState::Accepted,
                Some(accepted),
                ctx,
            )?;
        } else if operation.pending_bundle.is_some() && operation.run_intent == RunIntent::Stopped {
            operation.status = OperationStatus::Partial; // Enrollment is saved, credential is not active.
        }
        let status = terminal_status(operation.status);
        self.journal
            .finish_operation(&tx, status, vec![], &operation, ctx)?;
        Ok(IssuanceResult {
            binding: final_binding,
            operation,
        })
    }
    pub fn confirm_accepted(
        &self,
        identity: &ObjectId,
        profile: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<IdentityBinding> {
        ensure_mutating(ctx, self.journal.control())?;
        let request = IdentityIssueRequest {
            identity_id: identity.clone(),
            profile_id: profile.clone(),
            replace: false,
            approval_id: None,
        };
        if !self.inventory.authorize_issue(&request, ctx)? {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "identity acceptance is outside authorized scope",
            ));
        }
        let binding = self
            .inventory
            .binding(identity, profile, ctx)?
            .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "identity binding not found"))?;
        let (intent, accepted) = self.inventory.run_state(ctx)?;
        let accepted = accepted
            .filter(|_| intent == RunIntent::Running)
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RuntimeStopped,
                    "identity activation needs a running accepted bundle",
                )
            })?;
        if binding.state == BindingState::Revoked
            || !self.inventory.accepted_verified(&binding, &accepted, ctx)?
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "current accepted bundle does not verify this credential",
            ));
        }
        let digest = crate::config::digest(
            &serde_json::to_vec(&("confirmAccepted", identity, profile, &accepted)).map_err(
                |_| {
                    failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "identity acceptance encoding failed",
                    )
                },
            )?,
        );
        let journal_intent = Intent::Identity {
            request_digest: digest,
        };
        let existing = self.journal.lookup_intent(&journal_intent, ctx)?;
        if let Some(previous) = &existing {
            if self
                .journal
                .terminal(&previous.transaction_id, ctx)?
                .is_some_and(|record| record.status == TerminalStatus::Committed)
            {
                return self
                    .inventory
                    .binding(identity, profile, ctx)?
                    .ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "accepted identity metadata disappeared",
                        )
                    });
            }
        }
        let tx = if let Some(previous) = existing {
            previous.transaction_id
        } else {
            self.journal.prepare_intent(
                journal_intent,
                self.inventory.originals(identity, profile, ctx)?,
                ctx,
            )?
        };
        let updated =
            self.inventory
                .finalize(&binding, BindingState::Accepted, Some(&accepted), ctx)?;
        if self.inventory.run_state(ctx)? != (RunIntent::Running, Some(accepted.clone()))
            || !self.inventory.accepted_verified(&updated, &accepted, ctx)?
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "accepted credential binding changed during metadata publication",
            ));
        }
        let result = OperationResult {
            operation_id: ctx.operation_id.clone(),
            status: OperationStatus::Succeeded,
            transaction_id: Some(tx.clone()),
            desired_saved: false,
            pending_bundle: None,
            accepted_bundle: Some(accepted),
            run_intent: intent,
            member_results: vec![],
            survivors: vec![],
            issues: vec![],
        };
        if self
            .journal
            .terminal(&tx, ctx)?
            .is_some_and(|record| record.status == TerminalStatus::RecoveryRequired)
        {
            self.journal.resolve_operation(
                &tx,
                TerminalStatus::Committed,
                vec![],
                Some(&result),
                ctx,
            )?;
        } else {
            self.journal
                .finish_operation(&tx, TerminalStatus::Committed, vec![], &result, ctx)?;
        }
        Ok(updated)
    }
    pub fn revoke(
        &self,
        request: &IdentityRevokeRequest,
        ctx: &OperationContext,
    ) -> Result<OperationResult> {
        ensure_mutating(ctx, self.journal.control())?;
        if !ctx.actor.local_approval_refs.contains(&request.approval_id)
            || !self.inventory.authorize_revoke(request, ctx)?
        {
            return Err(failure(
                ctx,
                ErrorCode::ApprovalRequired,
                "identity revocation requires exact authorized approval",
            ));
        }
        let intent = Intent::Identity {
            request_digest: crate::config::digest(
                &serde_json::to_vec(&("revoke", request)).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "revocation request encoding failed",
                    )
                })?,
            ),
        };
        if let Some(tx) = self.journal.lookup_intent(&intent, ctx)? {
            return self
                .journal
                .terminal(&tx.transaction_id, ctx)?
                .and_then(|r| r.operation_result)
                .ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "interrupted revocation requires reconciliation",
                    )
                });
        }
        let binding = self
            .inventory
            .binding(&request.identity_id, &request.profile_id, ctx)?
            .ok_or_else(|| failure(ctx, ErrorCode::ModuleUnknown, "identity binding not found"))?;
        if binding.state != BindingState::Accepted {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "only an accepted identity can be revoked",
            ));
        }
        let tx = self.journal.prepare_intent(
            intent,
            self.inventory
                .originals(&request.identity_id, &request.profile_id, ctx)?,
            ctx,
        )?;
        let effect = self.inventory.plan_revoke(&binding, ctx)?;
        let mut result = self.apply.stage_and_apply(&effect, ctx)?;
        result.transaction_id = Some(tx.clone());
        if result.status == OperationStatus::Succeeded {
            if !self.inventory.revocation_verified(&binding, ctx)? {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "active and fallback revocation fences are not verified",
                ));
            }
            self.inventory.finalize(
                &binding,
                BindingState::Revoked,
                result.accepted_bundle.as_ref(),
                ctx,
            )?;
        }
        self.journal
            .finish_operation(&tx, terminal_status(result.status), vec![], &result, ctx)?;
        Ok(result)
    }
}
fn terminal_status(status: OperationStatus) -> TerminalStatus {
    match status {
        OperationStatus::Succeeded => TerminalStatus::Committed,
        OperationStatus::Partial => TerminalStatus::Partial,
        OperationStatus::Failed => TerminalStatus::Failed,
        OperationStatus::RecoveryRequired => TerminalStatus::RecoveryRequired,
    }
}
