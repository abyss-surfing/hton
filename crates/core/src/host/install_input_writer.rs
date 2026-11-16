//! Journalled, allowlisted writer for first-publication site inputs.
use super::{HostApplication, InstallInputApproval, approvals};
use crate::{
    config::{self, LoadLimits},
    journal::{self, Intent, ProtectedOriginal, TerminalStatus},
};
use hton_contracts::{
    config::{
        InstallConfigCandidate, InstallConfigDocumentReceipt, InstallConfigField,
        InstallConfigReceipt, InstallConfigValue, InstallConfigWriteState, ReadSet,
    },
    operations::{ArtifactId, ErrorCode, OperationContext, Result, TransactionId},
};
use serde_json::{Map, Value};

/// Writes only the known root configuration document. Callers must finish the
/// complete read-only install preflight before obtaining `approval`.
pub struct InstallInputWriter<'a> {
    app: &'a HostApplication,
}

impl<'a> InstallInputWriter<'a> {
    pub fn new(app: &'a HostApplication) -> Self {
        Self { app }
    }

    pub fn commit(
        &self,
        candidate: &InstallConfigCandidate,
        expected_read_set: &ReadSet,
        approval: &InstallInputApproval,
        ctx: &OperationContext,
    ) -> Result<InstallConfigReceipt> {
        self.commit_inner(candidate, expected_read_set, Some(approval), false, ctx)
    }

    /// Install-only path for exact typed values explicitly entered at the CLI.
    /// The caller must revalidate the candidate/read set before reaching this
    /// writer; no generic input authority is widened.
    pub(crate) fn commit_for_install(
        &self,
        candidate: &InstallConfigCandidate,
        expected_read_set: &ReadSet,
        ctx: &OperationContext,
    ) -> Result<InstallConfigReceipt> {
        self.commit_inner(candidate, expected_read_set, None, true, ctx)
    }

    fn commit_inner(
        &self,
        candidate: &InstallConfigCandidate,
        expected_read_set: &ReadSet,
        approval: Option<&InstallInputApproval>,
        direct_install_consent: bool,
        ctx: &OperationContext,
    ) -> Result<InstallConfigReceipt> {
        journal::ensure_mutating(ctx, self.app.control.as_ref())?;
        approvals::validate_install_input_candidate(candidate, ctx)?;
        if &candidate.read_set != expected_read_set {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "install input read set changed",
            ));
        }
        let now = hton_contracts::operations::UnixMillis(self.app.control.now_ms());
        match approval {
            Some(approval)
                if !approval.covers(candidate, &self.app.installation.instance_id, ctx, now) =>
            {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "install input approval does not cover this candidate",
                ));
            }
            None if !direct_install_consent => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ApprovalRequired,
                    "install input approval is required",
                ));
            }
            _ => {}
        }
        let root_id = hton_contracts::operations::ObjectId::new("root").expect("fixed object ID");
        let root_revision = expected_read_set.objects.get(&root_id).ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "saved root configuration revision is missing",
            )
        })?;
        let root = self
            .app
            .config
            .lock(journal::deadline(ctx, self.app.control.as_ref())?)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "root configuration lock unavailable",
                )
            })?;
        let original = root
            .read("hton.yaml", LoadLimits::default().file_bytes)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "root configuration read failed",
                )
            })?
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "root configuration disappeared",
                )
            })?;
        if config::content_revision(root_id.clone(), &original) != *root_revision {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "root configuration changed after preflight",
            ));
        }
        let value = config::decode_yaml(&original, LoadLimits::default()).map_err(|_| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "root configuration is invalid; retained",
            )
        })?;
        let value = apply_changes(value, candidate, ctx)?;
        let replacement = serde_yaml::to_string(&value)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "updated root configuration encoding failed",
                )
            })?
            .into_bytes();
        if replacement == original {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "install input candidate has no serialized effect",
            ));
        }
        let tx = self.app.journal.prepare_intent(
            Intent::InstallConfig {
                candidate: candidate.clone(),
            },
            vec![ProtectedOriginal {
                artifact_id: ArtifactId::new("config.root").expect("fixed artifact ID"),
                digest: config::digest(&original),
                was_present: true,
                bytes: original.clone(),
            }],
            ctx,
        )?;
        journal::ensure_mutating(ctx, self.app.control.as_ref())?;
        if root
            .compare_exchange("hton.yaml", Some(&original), &replacement)
            .is_err()
        {
            let _ = self
                .app
                .journal
                .finish(&tx, TerminalStatus::RecoveryRequired, vec![], ctx);
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "root config CAS uncertain; exact recovery required",
            ));
        }
        self.app
            .journal
            .finish(&tx, TerminalStatus::Committed, vec![], ctx)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "root config was written but journal completion is uncertain",
                )
            })?;
        Ok(InstallConfigReceipt {
            transaction_id: tx,
            documents: vec![InstallConfigDocumentReceipt {
                object_id: root_id.clone(),
                before: Some(root_revision.clone()),
                after: config::content_revision(root_id, &replacement),
            }],
            state: InstallConfigWriteState::Committed,
        })
    }

    /// Reconcile only the exact before/after bytes bound by the journal intent.
    pub fn recover(
        &self,
        transaction_id: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<InstallConfigReceipt> {
        journal::ensure_mutating(ctx, self.app.control.as_ref())?;
        let tx = self.app.journal.read(transaction_id, ctx)?;
        let Intent::InstallConfig { candidate } = &tx.intent else {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationDenied,
                "journal transaction is not an install input write",
            ));
        };
        approvals::validate_install_input_candidate(candidate, ctx)?;
        if self
            .app
            .journal
            .terminal(transaction_id, ctx)?
            .is_some_and(|record| record.status != TerminalStatus::RecoveryRequired)
        {
            return Err(journal::failure(
                ctx,
                ErrorCode::OperationConflict,
                "install input recovery requires an unfinished or recovery-required transaction",
            ));
        }
        let original_record = tx
            .originals
            .iter()
            .find(|original| original.artifact_id.as_str() == "config.root")
            .filter(|original| original.was_present)
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "protected root original missing",
                )
            })?;
        if config::digest(&original_record.bytes) != original_record.digest {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "protected root original digest mismatch",
            ));
        }
        let root_id = hton_contracts::operations::ObjectId::new("root").expect("fixed object ID");
        let before = candidate
            .read_set
            .objects
            .get(&root_id)
            .cloned()
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "journaled root revision missing",
                )
            })?;
        if config::content_revision(root_id.clone(), &original_record.bytes) != before {
            return Err(journal::failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "journaled root original does not match candidate revision",
            ));
        }
        let value =
            config::decode_yaml(&original_record.bytes, LoadLimits::default()).map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "protected root original cannot be decoded",
                )
            })?;
        let replacement = serde_yaml::to_string(&apply_changes(value, candidate, ctx)?)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "journaled root replacement cannot be encoded",
                )
            })?
            .into_bytes();
        let root = self
            .app
            .config
            .lock(journal::deadline(ctx, self.app.control.as_ref())?)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "root config recovery lock unavailable",
                )
            })?;
        let current = root
            .read("hton.yaml", LoadLimits::default().file_bytes)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "root config recovery read failed",
                )
            })?;
        match current.as_deref() {
            Some(bytes) if bytes == original_record.bytes => root
                .compare_exchange("hton.yaml", Some(bytes), &replacement)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "root config recovery CAS failed",
                    )
                })?,
            Some(bytes) if bytes == replacement => root
                .compare_exchange("hton.yaml", Some(bytes), &replacement)
                .map_err(|_| {
                    journal::failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "root config durability remains uncertain",
                    )
                })?,
            _ => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "root config is neither expected old nor exact new; retained for repair",
                ));
            }
        }
        self.app
            .journal
            .finish(transaction_id, TerminalStatus::Committed, vec![], ctx)
            .map_err(|_| {
                journal::failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "root config recovered but journal completion is uncertain",
                )
            })?;
        Ok(InstallConfigReceipt {
            transaction_id: transaction_id.clone(),
            documents: vec![InstallConfigDocumentReceipt {
                object_id: root_id.clone(),
                before: Some(before),
                after: config::content_revision(root_id, &replacement),
            }],
            state: InstallConfigWriteState::Committed,
        })
    }
}

fn apply_changes(
    mut root: Value,
    candidate: &InstallConfigCandidate,
    ctx: &OperationContext,
) -> Result<Value> {
    let core = root
        .as_object_mut()
        .and_then(|root| root.get_mut("core"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "root core settings are invalid",
            )
        })?;
    for change in &candidate.changes {
        match (change.field, &change.proposed) {
            (InstallConfigField::SiteDomain, InstallConfigValue::SiteDomain(domain)) => {
                let edge = core
                    .get_mut("edge")
                    .and_then(Value::as_object_mut)
                    .ok_or_else(|| {
                        journal::failure(
                            ctx,
                            ErrorCode::ConfigSemanticConflict,
                            "root edge settings are invalid",
                        )
                    })?;
                let domains = edge
                    .get_mut("domains")
                    .and_then(Value::as_array_mut)
                    .ok_or_else(|| {
                        journal::failure(
                            ctx,
                            ErrorCode::ConfigSemanticConflict,
                            "root domain list is invalid",
                        )
                    })?;
                match change.previous.as_ref() {
                    None if domains.is_empty() => domains.push(Value::String(domain.clone())),
                    Some(InstallConfigValue::SiteDomain(previous))
                        if domains.as_slice() == [Value::String(previous.clone())] =>
                    {
                        domains[0] = Value::String(domain.clone())
                    }
                    _ => {
                        return Err(journal::failure(
                            ctx,
                            ErrorCode::ConfigReadConflict,
                            "saved domain no longer matches approved previous value",
                        ));
                    }
                }
            }
            (InstallConfigField::PublicEdge, InstallConfigValue::PublicEdge(endpoint)) => {
                let edge = core
                    .get_mut("edge")
                    .and_then(Value::as_object_mut)
                    .ok_or_else(|| {
                        journal::failure(
                            ctx,
                            ErrorCode::ConfigSemanticConflict,
                            "root edge settings are invalid",
                        )
                    })?;
                let listen = edge
                    .get_mut("listen")
                    .and_then(Value::as_object_mut)
                    .ok_or_else(|| {
                        journal::failure(
                            ctx,
                            ErrorCode::ConfigSemanticConflict,
                            "root listener settings are invalid",
                        )
                    })?;
                let current = endpoint_from_json(listen).ok_or_else(|| {
                    journal::failure(
                        ctx,
                        ErrorCode::ConfigSemanticConflict,
                        "root listener endpoint is invalid",
                    )
                })?;
                if change.previous.as_ref()
                    != Some(&InstallConfigValue::PublicEdge(current.clone()))
                {
                    return Err(journal::failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "saved listener no longer matches approved previous value",
                    ));
                }
                listen.insert("transport".into(), Value::String("tcp".into()));
                listen.insert("port".into(), Value::Number(443.into()));
                listen.insert("scope".into(), Value::String("public".into()));
                listen.insert(
                    "addressFamilies".into(),
                    Value::Array(vec![Value::String("ipv4".into())]),
                );
            }
            (InstallConfigField::CertificateEmail, InstallConfigValue::CertificateEmail(email)) => {
                apply_certificate_email(core, change, email, ctx)?
            }
            _ => {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "install input candidate contains an unsupported field",
                ));
            }
        }
    }
    Ok(root)
}

fn apply_certificate_email(
    core: &mut Map<String, Value>,
    change: &hton_contracts::config::InstallConfigChange,
    email: &str,
    ctx: &OperationContext,
) -> Result<()> {
    if !super::install_input::valid_certificate_email(email) {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "certificate account email is invalid",
        ));
    }
    if !core.contains_key("certificates") {
        if change.previous.is_some() {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "certificate configuration disappeared after preflight",
            ));
        }
        core.insert(
            "certificates".into(),
            serde_json::json!({
                "ca": "letsencrypt",
                "challenge": "dns-01",
                "provider": "internetbs",
                "email": email,
                "credentials": {
                    "apiKey": { "secretRef": "dns/internetbs/api-key" },
                    "apiPassword": { "secretRef": "dns/internetbs/api-password" }
                }
            }),
        );
    } else {
        let certificates = core.get_mut("certificates").ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "certificate configuration disappeared during update",
            )
        })?;
        let previous_email = super::install_input::existing_certificate_email(certificates, ctx)?;
        let previous_value = Some(InstallConfigValue::CertificateEmail(previous_email));
        if change.previous != previous_value {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "saved certificate email no longer matches the reviewed value",
            ));
        }
        certificates
            .as_object_mut()
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "existing certificate metadata is malformed",
                )
            })?
            .insert("email".into(), Value::String(email.to_owned()));
    }
    Ok(())
}

fn endpoint_from_json(listen: &Map<String, Value>) -> Option<hton_contracts::config::HostEndpoint> {
    use hton_contracts::config::{HostEndpoint, Transport};
    let transport = match listen.get("transport")?.as_str()? {
        "tcp" => Transport::Tcp,
        "udp" => Transport::Udp,
        _ => return None,
    };
    let port = u16::try_from(listen.get("port")?.as_u64()?).ok()?;
    let families = listen.get("addressFamilies")?.as_array()?;
    if families.len() != 1 || families[0].as_str()? != "ipv4" {
        return None;
    }
    let scope = listen.get("scope")?.as_str()?;
    let address = match scope {
        "public" => std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
        "host-loopback" => std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
        _ => return None,
    };
    Some(HostEndpoint {
        address,
        transport,
        port,
    })
}
