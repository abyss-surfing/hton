//! Concrete native certificate qualification, DNS-01 issuance and immutable candidate storage.
use crate::{
    certificates::*,
    config::digest,
    journal::{OperationControl, deadline, ensure_mutating, failure},
    secrets::{ProtectedMaterial, ReplacementPolicy, SecretRef, SecretStore},
};
use hton_contracts::operations::*;
use hton_host::{
    Budget as NativeBudget, Cancellation,
    crypto::{CryptoTools, PrivateDirectory, PrivateFile},
    docker::ImageReference,
};
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

struct Cancel<'a> {
    control: &'a dyn OperationControl,
    ctx: &'a OperationContext,
}
impl Cancellation for Cancel<'_> {
    fn cancelled(&self) -> bool {
        self.control.is_cancelled(&self.ctx.cancellation)
    }
}
fn budget(control: &dyn OperationControl, ctx: &OperationContext) -> Result<NativeBudget> {
    if control.is_cancelled(&ctx.cancellation) {
        return Err(failure(
            ctx,
            ErrorCode::OperationCancelled,
            "certificate operation cancelled",
        ));
    }
    let remaining = ctx.deadline.0.saturating_sub(control.now_ms()).min(300_000);
    if remaining == 0 {
        return Err(failure(
            ctx,
            ErrorCode::OperationDeadline,
            "certificate deadline reached",
        ));
    }
    NativeBudget::crypto(
        std::time::Instant::now() + std::time::Duration::from_millis(remaining),
        16 * 1024 * 1024,
    )
    .map_err(|_| {
        failure(
            ctx,
            ErrorCode::OperationDeadline,
            "certificate native budget invalid",
        )
    })
}
fn publish(
    dir: &SecureDir,
    name: &str,
    bytes: &[u8],
    control: &dyn OperationControl,
    ctx: &OperationContext,
) -> Result<()> {
    let locked = dir.lock(deadline(ctx, control)?).map_err(|_| {
        failure(
            ctx,
            ErrorCode::IoUnavailable,
            "certificate publication lock unavailable",
        )
    })?;
    let existing = locked.read(name, 2 * 1024 * 1024).map_err(|_| {
        failure(
            ctx,
            ErrorCode::IoUnavailable,
            "certificate publication read failed",
        )
    })?;
    if existing.as_deref().is_some_and(|old| old != bytes) {
        return Err(failure(
            ctx,
            ErrorCode::OperationConflict,
            "immutable certificate object conflict",
        ));
    }
    locked
        .compare_exchange(name, existing.as_deref(), bytes)
        .map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "certificate publication durability uncertain",
            )
        })
}
fn encode<T: Serialize>(value: &T, ctx: &OperationContext) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| {
        failure(
            ctx,
            ErrorCode::OperationConflict,
            "certificate record encoding failed",
        )
    })
}

pub struct OpenSslVerifier {
    tools: CryptoTools,
    production: PrivateFile,
    staging: Option<PrivateFile>,
    control: Arc<dyn OperationControl>,
}
impl OpenSslVerifier {
    pub fn new(
        tools: CryptoTools,
        production: PrivateFile,
        staging: Option<PrivateFile>,
        control: Arc<dyn OperationControl>,
    ) -> Self {
        Self {
            tools,
            production,
            staging,
            control,
        }
    }
}
impl X509Verifier for OpenSslVerifier {
    fn verify(
        &self,
        material: &CertificateMaterial,
        authority: CertificateAuthority,
        ctx: &OperationContext,
    ) -> Result<VerifiedCertificate> {
        let cancel = Cancel {
            control: self.control.as_ref(),
            ctx,
        };
        let native_budget = budget(self.control.as_ref(), ctx)?;
        let trust = match authority {
            CertificateAuthority::LetsEncryptProduction => &self.production,
            CertificateAuthority::LetsEncryptStaging => self.staging.as_ref().ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleIncomplete,
                    "explicit staging trust is not configured",
                )
            })?,
        };
        self.tools
            .verify_chain(
                material.chain(),
                trust,
                self.control.now_ms() / 1000,
                native_budget,
                &cancel,
            )
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "certificate trust/validity verification failed",
                )
            })?;
        let certificate_key = self
            .tools
            .certificate_public_key(material.chain(), native_budget, &cancel)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "certificate public key extraction failed",
                )
            })?;
        let private_key = self
            .tools
            .private_public_key(material.key(), native_budget, &cancel)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "private key qualification failed",
                )
            })?;
        if certificate_key.expose() != private_key.expose() {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "certificate and private key do not match",
            ));
        }
        let der = self
            .tools
            .certificate_der(material.chain(), native_budget, &cancel)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "certificate DER extraction failed",
                )
            })?;
        let metadata = self
            .tools
            .certificate_metadata(material.chain(), native_budget, &cancel)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "certificate SAN/time extraction failed",
                )
            })?;
        let text = std::str::from_utf8(metadata.expose())
            .map_err(|_| failure(ctx, ErrorCode::ConfigSyntax, "certificate metadata invalid"))?;
        let mut before = None;
        let mut after = None;
        let mut names = BTreeSet::new();
        let mut san = false;
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("notBefore=") {
                if before
                    .replace(parse_time(value).ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::ConfigSyntax,
                            "certificate start time invalid",
                        )
                    })?)
                    .is_some()
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "duplicate certificate validity",
                    ));
                }
            } else if let Some(value) = line.strip_prefix("notAfter=") {
                if after
                    .replace(parse_time(value).ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::ConfigSyntax,
                            "certificate expiry time invalid",
                        )
                    })?)
                    .is_some()
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "duplicate certificate validity",
                    ));
                }
            } else if line.trim() == "X509v3 Subject Alternative Name:"
                || line.trim() == "X509v3 Subject Alternative Name: critical"
            {
                if san {
                    return Err(failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "duplicate certificate SAN extension",
                    ));
                }
                san = true;
            } else if san {
                for name in line.trim().split(',') {
                    let dns = name.trim().strip_prefix("DNS:").ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::ModuleUnsupported,
                            "certificate has an unsupported SAN type",
                        )
                    })?;
                    let name = CertificateName::new(dns).map_err(|_| {
                        failure(ctx, ErrorCode::ConfigSyntax, "certificate DNS SAN invalid")
                    })?;
                    if !names.insert(name) {
                        return Err(failure(
                            ctx,
                            ErrorCode::ConfigSyntax,
                            "duplicate certificate SAN",
                        ));
                    }
                }
            } else {
                return Err(failure(
                    ctx,
                    ErrorCode::ConfigSyntax,
                    "unexpected certificate metadata",
                ));
            }
        }
        let not_before = before.ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "certificate start time absent",
            )
        })?;
        let not_after = after.ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "certificate expiry time absent",
            )
        })?;
        let now = self.control.now_ms();
        if names.is_empty()
            || names.len() > 100
            || not_before > now
            || not_after <= now
            || not_before >= not_after
        {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "certificate SAN/validity is unusable",
            ));
        }
        Ok(VerifiedCertificate {
            names,
            authority,
            fingerprint: digest(der.expose()),
            chain_trusted: true,
            private_key_matches: true,
            not_before: UnixMillis(not_before),
            not_after: UnixMillis(not_after),
        })
    }
}
/// Parse OpenSSL's fixed C-locale ASN.1 time display, with calendar validation.
fn parse_time(value: &str) -> Option<u64> {
    let parts: Vec<_> = value.split_whitespace().collect();
    if parts.len() != 5 || parts[4] != "GMT" {
        return None;
    }
    let month = match parts[0] {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let day: i64 = parts[1].parse().ok()?;
    let year: i64 = parts[3].parse().ok()?;
    if !(1970..=9999).contains(&year) {
        return None;
    }
    let clock: Vec<_> = parts[2].split(':').collect();
    if clock.len() != 3 {
        return None;
    }
    let hour: u64 = clock[0].parse().ok()?;
    let minute: u64 = clock[1].parse().ok()?;
    let second: u64 = clock[2].parse().ok()?;
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day < 1 || day > max {
        return None;
    }
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = year / 400;
    let yoe = year - era * 400;
    let shifted = month + if month > 2 { -3 } else { 9 };
    let days =
        era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * shifted + 2) / 5 + day - 1 - 719468;
    let seconds = u64::try_from(days)
        .ok()?
        .checked_mul(86400)?
        .checked_add(hour * 3600 + minute * 60 + second)?;
    seconds.checked_mul(1000)
}

pub struct AcmeContainerIssuer {
    tools: CryptoTools,
    work_path: PathBuf,
    image: ImageReference,
    email: String,
    control: Arc<dyn OperationControl>,
}
impl AcmeContainerIssuer {
    pub fn new(
        tools: CryptoTools,
        work_path: PathBuf,
        image: ImageReference,
        email: String,
        control: Arc<dyn OperationControl>,
    ) -> Self {
        Self {
            tools,
            work_path,
            image,
            email,
            control,
        }
    }
}
impl AcmeIssuer for AcmeContainerIssuer {
    fn issue(
        &self,
        request: &CertificateIssueRequest,
        credentials: &DnsCredentials,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<CertificateMaterial> {
        ensure_mutating(ctx, self.control.as_ref())?;
        let root = SecureDir::open_private(&self.work_path).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "protected ACME work unavailable",
            )
        })?;
        let _lock = root
            .lock(deadline(ctx, self.control.as_ref())?)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "another ACME operation is active",
                )
            })?;
        let request_hash = digest(&encode(request, ctx)?);
        let name = format!(
            "r-{}",
            &digest(transaction.as_str().as_bytes()).as_str()[7..]
        );
        let directory = root.create_child(&name).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "ACME transaction work unavailable",
            )
        })?;
        if directory
            .read("begin.json", 65536)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "ACME checkpoint unavailable"))?
            .is_some()
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "interrupted ACME job requires reconciliation",
            ));
        }
        publish(
            &directory,
            "begin.json",
            &encode(&(1u32, transaction, request_hash), ctx)?,
            self.control.as_ref(),
            ctx,
        )?;
        let env = format!(
            "INTERNETBS_API_KEY={}\nINTERNETBS_API_PASSWORD={}\n",
            credential(credentials.api_key.expose(), ctx)?,
            credential(credentials.password.expose(), ctx)?
        );
        publish(
            &directory,
            "dns.env",
            env.as_bytes(),
            self.control.as_ref(),
            ctx,
        )?;
        directory.create_child("output").map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "ACME output directory unavailable",
            )
        })?;
        let private = PrivateDirectory::open(&self.work_path.join(&name)).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "ACME work capability invalid",
            )
        })?;
        let envfile = private.file("dns.env", 65536).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "ACME credential file unavailable",
            )
        })?;
        let mut names = request
            .names
            .iter()
            .map(|name| name.as_str().to_owned())
            .collect::<Vec<_>>();
        names.sort_by_key(|name| name.starts_with("*."));
        if names.first().is_none_or(|name| name.starts_with("*.")) {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "wildcard issuance requires its explicit base domain",
            ));
        }
        let shared = PrivateDirectory::open(&self.work_path).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "ACME shared state capability invalid",
            )
        })?;
        let container = format!(
            "hton-acme-{}",
            &digest(transaction.as_str().as_bytes()).as_str()[7..]
        );
        let cancel = Cancel {
            control: self.control.as_ref(),
            ctx,
        };
        let record_job = |id: &hton_host::docker::ContainerId| -> hton_host::Result<()> {
            let bytes = serde_json::to_vec(&(1u32, transaction, id.as_str(), self.image.as_str()))
                .map_err(|_| hton_host::Error::InvalidInput)?;
            publish(&directory, "job.json", &bytes, self.control.as_ref(), ctx)
                .map_err(|_| hton_host::Error::Io)
        };
        self.tools
            .issue_acme(
                &self.image,
                &shared,
                &envfile,
                &name,
                &names,
                request.authority == CertificateAuthority::LetsEncryptStaging,
                &self.email,
                &container,
                &record_job,
                budget(self.control.as_ref(), ctx)?,
                &cancel,
            )
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "ACME native issuance failed or uncertain; checkpoint retained",
                )
            })?;
        let output = directory
            .child("output")
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "ACME output unavailable"))?;
        let chain = output
            .read("fullchain.pem", 262144)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "ACME chain unavailable"))?
            .ok_or_else(|| failure(ctx, ErrorCode::RecoveryRequired, "ACME chain missing"))?;
        let key = output
            .read("key.pem", 65536)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "ACME private key unavailable",
                )
            })?
            .ok_or_else(|| failure(ctx, ErrorCode::RecoveryRequired, "ACME private key missing"))?;
        let material = CertificateMaterial::new(chain, key)
            .map_err(|_| failure(ctx, ErrorCode::ConfigSyntax, "ACME material bounds invalid"))?;
        publish(
            &directory,
            "issued.json",
            &encode(
                &(
                    1u32,
                    transaction,
                    digest(material.chain()),
                    digest(material.key()),
                ),
                ctx,
            )?,
            self.control.as_ref(),
            ctx,
        )?;
        Ok(material)
    }
}

type Names = Arc<dyn Fn(&OperationContext) -> Result<BTreeSet<CertificateName>> + Send + Sync>;
type Intent = Arc<dyn Fn(&OperationContext) -> Result<RunIntent> + Send + Sync>;
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CandidateRecord {
    schema_version: u32,
    candidate: CertificateCandidate,
    chain: Vec<u8>,
    key: Vec<u8>,
    chain_digest: Sha256Digest,
    key_digest: Sha256Digest,
    prepared: PreparedCertificate,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedCertificate {
    pub candidate: CertificateCandidate,
    pub chain: SecretRef,
    pub key: SecretRef,
    pub scope: ScopeId,
}
pub struct FileCertificateInventory {
    directory: Arc<SecureDir>,
    configured: Names,
    runtime: Intent,
    verifier: Arc<dyn X509Verifier>,
    control: Arc<dyn OperationControl>,
    secrets: Arc<SecretStore>,
}
impl FileCertificateInventory {
    pub fn new(
        directory: Arc<SecureDir>,
        configured: Names,
        runtime: Intent,
        verifier: Arc<dyn X509Verifier>,
        control: Arc<dyn OperationControl>,
        secrets: Arc<SecretStore>,
    ) -> Self {
        Self {
            directory,
            configured,
            runtime,
            verifier,
            control,
            secrets,
        }
    }
    fn candidate_name(id: &ObjectId) -> String {
        format!("c-{}.json", &digest(id.as_str().as_bytes()).as_str()[7..])
    }
    fn tx_name(tx: &TransactionId) -> String {
        format!("t-{}.json", &digest(tx.as_str().as_bytes()).as_str()[7..])
    }
    pub fn force_approval_id(
        request: &CertificateIssueRequest,
    ) -> std::result::Result<ApprovalId, InvalidIdentifier> {
        let bytes = serde_json::to_vec(&(
            &request.names,
            request.authority,
            &request.dns_key,
            &request.dns_password,
            request.automatic,
        ))
        .map_err(|_| InvalidIdentifier)?;
        ApprovalId::new(format!("cert-force.{}", &digest(&bytes).as_str()[7..]))
    }
    fn read_record(&self, name: &str, ctx: &OperationContext) -> Result<Option<CandidateRecord>> {
        let Some(bytes) = self.directory.read(name, 2 * 1024 * 1024).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "certificate record unavailable",
            )
        })?
        else {
            return Ok(None);
        };
        let record: CandidateRecord = serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "certificate record corrupt",
            )
        })?;
        if record.schema_version != 1
            || digest(&record.chain) != record.chain_digest
            || digest(&record.key) != record.key_digest
            || name != Self::candidate_name(&record.candidate.candidate_id)
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "certificate record fingerprint/identity mismatch",
            ));
        }
        Ok(Some(record))
    }
    fn material(record: &CandidateRecord, ctx: &OperationContext) -> Result<CertificateMaterial> {
        CertificateMaterial::new(record.chain.clone(), record.key.clone()).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "certificate record material bounds invalid",
            )
        })
    }
    pub fn prepared(&self, ctx: &OperationContext) -> Result<Option<PreparedCertificate>> {
        let Some(bytes) = self.directory.read("prepared.json", 65536).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "prepared certificate unavailable",
            )
        })?
        else {
            return Ok(None);
        };
        let prepared: PreparedCertificate = serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "prepared certificate pointer corrupt",
            )
        })?;
        let record = self
            .read_record(&Self::candidate_name(&prepared.candidate.candidate_id), ctx)?
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "prepared certificate bytes missing",
                )
            })?;
        if record.prepared.chain != prepared.chain
            || record.prepared.key != prepared.key
            || record.candidate != prepared.candidate
            || prepared.scope != ScopeId::new("site.tls").expect("fixed scope")
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "prepared certificate binding mismatch",
            ));
        }
        let material = Self::material(&record, ctx)?;
        if self
            .verifier
            .verify(&material, record.candidate.certificate.authority, ctx)?
            != record.candidate.certificate
        {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "prepared certificate no longer qualified",
            ));
        }
        Ok(Some(prepared))
    }
    pub fn candidates(&self, ctx: &OperationContext) -> Result<Vec<CertificateCandidate>> {
        let mut candidates = Vec::new();
        for name in self.directory.list(4096).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "certificate inventory unavailable",
            )
        })? {
            if !name.starts_with("c-") {
                continue;
            }
            let record = self.read_record(&name, ctx)?.ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "certificate inventory changed",
                )
            })?;
            candidates.push(record.candidate);
        }
        candidates.sort_by(|a, b| b.certificate.not_after.cmp(&a.certificate.not_after));
        Ok(candidates)
    }
    pub fn material_for(
        &self,
        id: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<CertificateMaterial> {
        let record = self
            .read_record(&Self::candidate_name(id), ctx)?
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleUnknown,
                    "certificate candidate absent",
                )
            })?;
        Self::material(&record, ctx)
    }
}
impl CertificateInventory for FileCertificateInventory {
    fn configured_names(&self, ctx: &OperationContext) -> Result<BTreeSet<CertificateName>> {
        (self.configured)(ctx)
    }
    fn run_intent(&self, ctx: &OperationContext) -> Result<RunIntent> {
        (self.runtime)(ctx)
    }
    fn reusable(
        &self,
        names: &BTreeSet<CertificateName>,
        authority: CertificateAuthority,
        ctx: &OperationContext,
    ) -> Result<Option<(ObjectId, CertificateMaterial)>> {
        for candidate in self.candidates(ctx)? {
            if candidate.certificate.authority != authority
                || !names.iter().all(|requested| {
                    candidate
                        .certificate
                        .names
                        .iter()
                        .any(|name| name.covers(requested))
                })
            {
                continue;
            }
            let material = self.material_for(&candidate.candidate_id, ctx)?;
            if let Ok(verified) = self.verifier.verify(&material, authority, ctx) {
                if verified == candidate.certificate {
                    return Ok(Some((candidate.candidate_id, material)));
                }
            }
        }
        Ok(None)
    }
    fn force_allowed(
        &self,
        approval: &ApprovalId,
        request: &CertificateIssueRequest,
        ctx: &OperationContext,
    ) -> Result<bool> {
        Ok(ctx.actor.source == ActorSource::Local
            && ctx
                .actor
                .granted_scopes
                .contains(&ScopeId::new("certificates.force").expect("fixed scope"))
            && ctx.actor.local_approval_refs.contains(approval)
            && Self::force_approval_id(request).as_ref() == Ok(approval))
    }
    fn saved(
        &self,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<Option<CertificateCandidate>> {
        let Some(bytes) = self
            .directory
            .read(&Self::tx_name(transaction), 65536)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "certificate transaction pointer unavailable",
                )
            })?
        else {
            return Ok(None);
        };
        let candidate: CertificateCandidate = serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "certificate transaction pointer corrupt",
            )
        })?;
        if candidate.transaction_id.as_ref() != Some(transaction) {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "certificate transaction identity mismatch",
            ));
        }
        let material = self.material_for(&candidate.candidate_id, ctx)?;
        if self
            .verifier
            .verify(&material, candidate.certificate.authority, ctx)?
            != candidate.certificate
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "saved certificate qualification changed",
            ));
        }
        Ok(Some(candidate))
    }
    fn save_candidate(
        &self,
        transaction: &TransactionId,
        material: &CertificateMaterial,
        verified: &VerifiedCertificate,
        ctx: &OperationContext,
    ) -> Result<CertificateCandidate> {
        ensure_mutating(ctx, self.control.as_ref())?;
        if self.verifier.verify(material, verified.authority, ctx)? != *verified {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "certificate candidate was not independently qualified",
            ));
        }
        let id = ObjectId::new(format!(
            "cert.{}",
            &digest(&encode(
                &(
                    transaction,
                    &verified.fingerprint,
                    digest(material.chain()),
                    digest(material.key())
                ),
                ctx
            )?)
            .as_str()[7..]
        ))
        .map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "certificate candidate ID invalid",
            )
        })?;
        let candidate = CertificateCandidate {
            candidate_id: id,
            certificate: verified.clone(),
            reused: false,
            transaction_id: Some(transaction.clone()),
        };
        let scope = ScopeId::new("site.tls").expect("fixed scope");
        let consumers = BTreeSet::from([ServiceId::new("caddy").expect("fixed service")]);
        let suffix = &candidate.candidate_id.as_str()[5..];
        let chain = self
            .secrets
            .import(
                SecretId::new(format!("certificate.{suffix}.chain")).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "certificate secret ID invalid",
                    )
                })?,
                scope.clone(),
                consumers.clone(),
                ReplacementPolicy::Create,
                ProtectedMaterial::from_input(material.chain().to_vec()).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "certificate chain bounds invalid",
                    )
                })?,
                ctx,
            )?
            .reference;
        let key = self
            .secrets
            .import(
                SecretId::new(format!("certificate.{suffix}.key")).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "certificate secret ID invalid",
                    )
                })?,
                scope.clone(),
                consumers,
                ReplacementPolicy::Create,
                ProtectedMaterial::from_input(material.key().to_vec()).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "certificate key bounds invalid",
                    )
                })?,
                ctx,
            )?
            .reference;
        let prepared = PreparedCertificate {
            candidate: candidate.clone(),
            chain,
            key,
            scope,
        };
        let record = CandidateRecord {
            schema_version: 1,
            candidate: candidate.clone(),
            chain: material.chain().to_vec(),
            key: material.key().to_vec(),
            chain_digest: digest(material.chain()),
            key_digest: digest(material.key()),
            prepared: prepared.clone(),
        };
        publish(
            &self.directory,
            &Self::candidate_name(&candidate.candidate_id),
            &encode(&record, ctx)?,
            self.control.as_ref(),
            ctx,
        )?;
        publish(
            &self.directory,
            &Self::tx_name(transaction),
            &encode(&candidate, ctx)?,
            self.control.as_ref(),
            ctx,
        )?;
        let bytes = encode(&prepared, ctx)?;
        let locked = self
            .directory
            .lock(deadline(ctx, self.control.as_ref())?)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "prepared certificate lock unavailable",
                )
            })?;
        let previous = locked.read("prepared.json", 65536).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "prepared certificate baseline unavailable",
            )
        })?;
        locked
            .compare_exchange("prepared.json", previous.as_deref(), &bytes)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "prepared certificate pointer durability uncertain",
                )
            })?;
        Ok(candidate)
    }
}

fn credential<'a>(bytes: &'a [u8], ctx: &OperationContext) -> Result<&'a str> {
    let value = std::str::from_utf8(bytes).map_err(|_| {
        failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "DNS credential encoding invalid",
        )
    })?;
    if value.is_empty()
        || value.len() > 16384
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
    {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "DNS credential is not a single envfile value",
        ));
    }
    Ok(value)
}
