//! Own DNS-01 certificate preparation. Delivery is a separate admitted apply.
use crate::journal::{Intent, JournalStore, TerminalStatus, ensure_mutating, failure};
use crate::secrets::{ConsumerGrant, ProtectedMaterial, SecretRef, SecretStore};
use hton_contracts::operations::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CertificateName(String);
impl CertificateName {
    pub fn new(value: impl Into<String>) -> std::result::Result<Self, InvalidIdentifier> {
        let name = value.into();
        let host = name.strip_prefix("*.").unwrap_or(&name);
        if name.len() > 253
            || host.is_empty()
            || host.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
        {
            return Err(InvalidIdentifier);
        }
        Ok(Self(name))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn is_wildcard(&self) -> bool {
        self.0.starts_with("*.")
    }
    pub fn host(&self) -> &str {
        self.0.strip_prefix("*.").unwrap_or(&self.0)
    }
    pub fn covers(&self, requested: &CertificateName) -> bool {
        if self == requested {
            return true;
        }
        if !self.is_wildcard() || requested.is_wildcard() {
            return false;
        }
        requested
            .as_str()
            .strip_suffix(self.host())
            .is_some_and(|prefix| {
                prefix.ends_with('.')
                    && !prefix[..prefix.len() - 1].is_empty()
                    && !prefix[..prefix.len() - 1].contains('.')
            })
    }
}
impl TryFrom<String> for CertificateName {
    type Error = InvalidIdentifier;
    fn try_from(v: String) -> std::result::Result<Self, Self::Error> {
        Self::new(v)
    }
}
impl From<CertificateName> for String {
    fn from(v: CertificateName) -> Self {
        v.0
    }
}
impl fmt::Debug for CertificateName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CertificateName").field(&self.0).finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CertificateAuthority {
    LetsEncryptProduction,
    LetsEncryptStaging,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CertificateIssueRequest {
    pub names: BTreeSet<CertificateName>,
    pub authority: CertificateAuthority,
    pub force: bool,
    pub force_approval: Option<ApprovalId>,
    pub automatic: bool,
    pub dns_key: SecretRef,
    pub dns_password: SecretRef,
}
/// Private keys cannot flow through Debug, normal operation metadata or log fields.
pub struct CertificateMaterial {
    chain: Vec<u8>,
    key: Vec<u8>,
}
impl CertificateMaterial {
    pub fn new(chain: Vec<u8>, key: Vec<u8>) -> std::result::Result<Self, InvalidIdentifier> {
        if chain.is_empty() || key.is_empty() || chain.len() > 262_144 || key.len() > 65_536 {
            return Err(InvalidIdentifier);
        }
        Ok(Self { chain, key })
    }
    pub fn chain(&self) -> &[u8] {
        &self.chain
    }
    pub fn key(&self) -> &[u8] {
        &self.key
    }
}
impl fmt::Debug for CertificateMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CertificateMaterial([redacted])")
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerifiedCertificate {
    pub names: BTreeSet<CertificateName>,
    pub authority: CertificateAuthority,
    pub fingerprint: Sha256Digest,
    pub chain_trusted: bool,
    pub private_key_matches: bool,
    pub not_before: UnixMillis,
    pub not_after: UnixMillis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CertificateCandidate {
    pub candidate_id: ObjectId,
    pub certificate: VerifiedCertificate,
    pub reused: bool,
    pub transaction_id: Option<TransactionId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateHealth {
    Healthy,
    RenewalDue,
    Expiring,
    Expired,
    NotYetValid,
    Untrusted,
}
#[derive(Clone, Debug)]
pub struct CertificateStatus {
    pub certificate: VerifiedCertificate,
    pub health: CertificateHealth,
    pub remaining_ms: u64,
}

pub struct DnsCredentials {
    pub api_key: ProtectedMaterial,
    pub password: ProtectedMaterial,
}
impl fmt::Debug for DnsCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DnsCredentials([redacted])")
    }
}
/// Host binding uses pinned short-lived acme.sh, explicit LE endpoint and DNS-01
/// dns_internetbs only; no Docker socket, service start/reload or shell hook execution.
pub trait AcmeIssuer: Send + Sync {
    fn issue(
        &self,
        request: &CertificateIssueRequest,
        credentials: &DnsCredentials,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<CertificateMaterial>;
}
pub trait X509Verifier: Send + Sync {
    fn verify(
        &self,
        material: &CertificateMaterial,
        authority: CertificateAuthority,
        ctx: &OperationContext,
    ) -> Result<VerifiedCertificate>;
}
pub trait CertificateInventory: Send + Sync {
    fn configured_names(&self, ctx: &OperationContext) -> Result<BTreeSet<CertificateName>>;
    fn run_intent(&self, ctx: &OperationContext) -> Result<RunIntent>;
    fn reusable(
        &self,
        names: &BTreeSet<CertificateName>,
        authority: CertificateAuthority,
        ctx: &OperationContext,
    ) -> Result<Option<(ObjectId, CertificateMaterial)>>;
    fn force_allowed(
        &self,
        approval: &ApprovalId,
        request: &CertificateIssueRequest,
        ctx: &OperationContext,
    ) -> Result<bool>;
    fn saved(
        &self,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<Option<CertificateCandidate>>;
    fn save_candidate(
        &self,
        transaction: &TransactionId,
        material: &CertificateMaterial,
        verified: &VerifiedCertificate,
        ctx: &OperationContext,
    ) -> Result<CertificateCandidate>;
}
pub struct CertificateCoordinator<'a> {
    pub journal: &'a JournalStore,
    pub secrets: &'a SecretStore,
    pub inventory: &'a dyn CertificateInventory,
    pub issuer: &'a dyn AcmeIssuer,
    pub verifier: &'a dyn X509Verifier,
    pub dns_grant: ConsumerGrant,
}
impl CertificateCoordinator<'_> {
    pub fn issue(
        &self,
        request: &CertificateIssueRequest,
        ctx: &OperationContext,
    ) -> Result<CertificateCandidate> {
        ensure_mutating(ctx, self.journal.control())?;
        if request.names.is_empty()
            || request.names.len() > 100
            || !request
                .names
                .is_subset(&self.inventory.configured_names(ctx)?)
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "certificate names are outside configured own domains",
            ));
        }
        let request_digest = crate::config::digest(&serde_json::to_vec(request).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "certificate request cannot be encoded",
            )
        })?);
        let intent = Intent::Certificates { request_digest };
        if let Some(tx) = self.journal.lookup_intent(&intent, ctx)? {
            let candidate = self
                .inventory
                .saved(&tx.transaction_id, ctx)?
                .ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "interrupted issuer transaction requires explicit reconciliation",
                    )
                })?;
            if !self.usable(&candidate.certificate, request)
                || candidate.transaction_id.as_ref() != Some(&tx.transaction_id)
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "retained certificate candidate is no longer qualified",
                ));
            }
            match self.journal.terminal(&tx.transaction_id, ctx)? {
                Some(record) if record.status == TerminalStatus::Committed => return Ok(candidate),
                None => {
                    self.journal.finish(
                        &tx.transaction_id,
                        TerminalStatus::Committed,
                        vec![],
                        ctx,
                    )?;
                    return Ok(candidate);
                }
                _ => {
                    return Err(failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "certificate transaction requires explicit reconciliation",
                    ));
                }
            }
        }
        if request.automatic && self.inventory.run_intent(ctx)? == RunIntent::Stopped {
            return Err(failure(
                ctx,
                ErrorCode::RuntimeStopped,
                "automatic certificate jobs are suspended while stopped",
            ));
        }
        if request.force {
            let allowed = request
                .force_approval
                .as_ref()
                .is_some_and(|a| ctx.actor.local_approval_refs.contains(a));
            if !allowed
                || !self.inventory.force_allowed(
                    request.force_approval.as_ref().ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::ApprovalRequired,
                            "forced issue requires local approval",
                        )
                    })?,
                    request,
                    ctx,
                )?
            {
                return Err(failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "forced certificate issuance is not approved",
                ));
            }
        }
        if !request.force {
            if let Some((id, material)) =
                self.inventory
                    .reusable(&request.names, request.authority, ctx)?
            {
                let verified = self.verifier.verify(&material, request.authority, ctx)?;
                if self.usable(&verified, request)
                    && verified.not_after.0
                        > self
                            .journal
                            .control()
                            .now_ms()
                            .saturating_add(30 * 24 * 60 * 60 * 1000)
                {
                    return Ok(CertificateCandidate {
                        candidate_id: id,
                        certificate: verified,
                        reused: true,
                        transaction_id: None,
                    });
                }
            }
        }
        let tx = self.journal.prepare_intent(intent, vec![], ctx)?;
        let credentials = DnsCredentials {
            api_key: self
                .secrets
                .resolve(&request.dns_key, &self.dns_grant, ctx)?,
            password: self
                .secrets
                .resolve(&request.dns_password, &self.dns_grant, ctx)?,
        };
        let prepared = (|| {
            let material = self.issuer.issue(request, &credentials, &tx, ctx)?;
            ensure_mutating(ctx, self.journal.control())?;
            let verified = self.verifier.verify(&material, request.authority, ctx)?;
            if !self.usable(&verified, request) {
                return Err(failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "certificate SAN chain key or validity qualification failed",
                ));
            }
            let candidate = self
                .inventory
                .save_candidate(&tx, &material, &verified, ctx)?;
            if candidate.certificate != verified || candidate.transaction_id.as_ref() != Some(&tx) {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "saved certificate candidate identity mismatch",
                ));
            }
            self.journal
                .finish(&tx, TerminalStatus::Committed, vec![], ctx)?;
            Ok(candidate)
        })();
        if prepared.is_err() {
            let _ = self
                .journal
                .finish(&tx, TerminalStatus::RecoveryRequired, vec![], ctx);
        }
        prepared
    }
    fn usable(&self, c: &VerifiedCertificate, request: &CertificateIssueRequest) -> bool {
        c.authority == request.authority
            && request
                .names
                .iter()
                .all(|requested| c.names.iter().any(|san| san.covers(requested)))
            && c.chain_trusted
            && c.private_key_matches
            && c.not_before.0 <= self.journal.control().now_ms()
            && c.not_after.0 > self.journal.control().now_ms()
    }
    pub fn status(&self, certificate: VerifiedCertificate) -> CertificateStatus {
        let now = self.journal.control().now_ms();
        let remaining = certificate.not_after.0.saturating_sub(now);
        let health = if !certificate.chain_trusted || !certificate.private_key_matches {
            CertificateHealth::Untrusted
        } else if certificate.not_before.0 > now {
            CertificateHealth::NotYetValid
        } else if remaining == 0 {
            CertificateHealth::Expired
        } else if remaining < 14 * 24 * 60 * 60 * 1000 {
            CertificateHealth::Expiring
        } else if remaining < 30 * 24 * 60 * 60 * 1000 {
            CertificateHealth::RenewalDue
        } else {
            CertificateHealth::Healthy
        };
        CertificateStatus {
            certificate,
            health,
            remaining_ms: remaining,
        }
    }
}
