//! Pure, allowlisted first-publication input candidate construction.
use crate::{config, host::installation, journal};
use hton_contracts::{
    config::{
        DesiredReadResult, HostEndpoint, InstallConfigCandidate, InstallConfigChange,
        InstallConfigField, InstallConfigValue, InstallInput, ReadCoherence, Transport,
    },
    operations::{ErrorCode, OperationContext, Result, Sha256Digest},
};
use serde::Serialize;
use serde_json::Value;
use std::net::{IpAddr, Ipv4Addr};

/// Prepare an exact, digest-bound change to the saved site domain and public edge.
/// This function reads only the supplied snapshot and performs no I/O or writes.
pub fn prepare_input_candidate(
    input: &InstallInput,
    desired: &DesiredReadResult,
    ctx: &OperationContext,
) -> Result<InstallConfigCandidate> {
    if !installation::valid_site_domain(&input.site_domain)
        || input.public_edge.transport != Transport::Tcp
        || input.public_edge.port != 443
        || input.public_edge.address != IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "installation input must contain a valid domain and public IPv4 TCP/443 endpoint",
        ));
    }

    let Some(snapshot) = desired.snapshot.as_ref() else {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "configuration snapshot is unavailable for installation input review",
        ));
    };
    if desired
        .errors
        .iter()
        .any(|issue| issue.severity == hton_contracts::config::Severity::Error)
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSyntax,
            "configuration diagnostics prevent installation input planning",
        ));
    }
    if desired.read_set.coherence != ReadCoherence::Coherent
        || snapshot.read_set != desired.read_set
        || desired
            .read_set
            .absent_objects
            .iter()
            .any(|id| desired.read_set.objects.contains_key(id))
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "installation input requires one coherent configuration read set",
        ));
    }
    if input.certificate_email.is_some() != input.provenance.certificate_email.is_some()
        || input
            .certificate_email
            .as_deref()
            .is_some_and(|email| !valid_certificate_email(email))
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::OperationDenied,
            "certificate account email is missing provenance or has invalid syntax",
        ));
    }

    let core = snapshot
        .platform
        .settings
        .get("core")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "saved core publication settings are unavailable",
            )
        })?;
    let edge = core.get("edge").and_then(Value::as_object).ok_or_else(|| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "saved core edge settings are unavailable",
        )
    })?;

    let saved_domains = edge
        .get("domains")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "saved site domain list is invalid",
            )
        })?;
    if saved_domains.len() > 1 {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "installation input cannot replace a multi-domain publication candidate",
        ));
    }
    let previous_domain = match saved_domains.first() {
        None => None,
        Some(domain) => {
            let domain = domain
                .as_str()
                .ok_or_else(|| {
                    journal::failure(
                        ctx,
                        ErrorCode::ConfigSemanticConflict,
                        "saved site domain value is invalid",
                    )
                })?
                .to_owned();
            if !installation::valid_site_domain(&domain) {
                return Err(journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "saved site domain is invalid and must be repaired before installation",
                ));
            }
            Some(InstallConfigValue::SiteDomain(domain))
        }
    };
    let proposed_domain = InstallConfigValue::SiteDomain(input.site_domain.clone());

    let listen = edge
        .get("listen")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "saved public listener settings are unavailable",
            )
        })?;
    let previous_edge = saved_edge_endpoint(listen, ctx)?;
    let proposed_edge = InstallConfigValue::PublicEdge(input.public_edge.clone());

    let certificates = core.get("certificates");
    let previous_certificate_email = match certificates {
        Some(value) => Some(existing_certificate_email(value, ctx)?),
        None => None,
    };
    let proposed_certificate_email = match input.certificate_email.as_deref() {
        Some(email) => Some(email),
        None => previous_certificate_email
            .as_deref()
            .filter(|email| valid_certificate_email(email)),
    };
    let Some(proposed_certificate_email) = proposed_certificate_email else {
        return Err(journal::failure(
            ctx,
            ErrorCode::ModuleIncomplete,
            "a valid certificate account email is required for DNS-01 publication",
        ));
    };
    let mut changes = Vec::with_capacity(3);
    if previous_domain.as_ref() != Some(&proposed_domain) {
        changes.push(InstallConfigChange {
            field: InstallConfigField::SiteDomain,
            previous: previous_domain,
            proposed: proposed_domain,
        });
    }
    if previous_edge.as_ref() != Some(&proposed_edge) {
        changes.push(InstallConfigChange {
            field: InstallConfigField::PublicEdge,
            previous: previous_edge,
            proposed: proposed_edge,
        });
    }
    let proposed_certificate_email =
        InstallConfigValue::CertificateEmail(proposed_certificate_email.to_owned());
    if previous_certificate_email
        .as_ref()
        .map(|email| InstallConfigValue::CertificateEmail(email.clone()))
        .as_ref()
        != Some(&proposed_certificate_email)
    {
        changes.push(InstallConfigChange {
            field: InstallConfigField::CertificateEmail,
            previous: previous_certificate_email.map(InstallConfigValue::CertificateEmail),
            proposed: proposed_certificate_email,
        });
    }
    if changes.is_empty() {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "installation input does not change the saved publication settings",
        ));
    }

    let candidate_digest = digest_candidate(&changes, &desired.read_set).map_err(|_| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "installation input candidate digest could not be computed",
        )
    })?;
    Ok(InstallConfigCandidate {
        changes,
        read_set: desired.read_set.clone(),
        candidate_digest,
    })
}

/// Validate certificate metadata that already exists without normalizing or
/// replacing its provider and SecretRef values. Existing metadata is reusable
/// only when its account email is also valid; repair belongs to config editing.
pub(super) fn existing_certificate_email(
    certificates: &Value,
    ctx: &OperationContext,
) -> Result<String> {
    let Some(certificates) = certificates.as_object() else {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "existing certificate configuration is malformed",
        ));
    };
    if certificates.get("ca").and_then(Value::as_str) != Some("letsencrypt")
        || certificates.get("challenge").and_then(Value::as_str) != Some("dns-01")
        || certificates.get("provider").and_then(Value::as_str) != Some("internetbs")
    {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "existing certificate issuer settings are incomplete or unsupported",
        ));
    }
    let credentials = certificates
        .get("credentials")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "existing InternetBS credential references are missing",
            )
        })?;
    for key in ["apiKey", "apiPassword"] {
        let reference = credentials
            .get(key)
            .and_then(Value::as_object)
            .and_then(|credential| credential.get("secretRef"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                journal::failure(
                    ctx,
                    ErrorCode::ConfigSemanticConflict,
                    "existing InternetBS credential references are missing",
                )
            })?;
        if crate::config::canonical_secret_id(reference).is_none() {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "existing InternetBS credential reference is invalid",
            ));
        }
    }
    match certificates.get("email") {
        Some(Value::String(email)) if email.len() <= 320 && valid_certificate_email(email) => {
            Ok(email.clone())
        }
        _ => Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "existing certificate account email is missing or invalid",
        )),
    }
}

pub(crate) fn valid_certificate_email(value: &str) -> bool {
    if value.is_empty() || value.len() > 254 || !value.is_ascii() {
        return false;
    }
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !domain.contains('@')
        && !local.is_empty()
        && local.len() <= 64
        && local
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(valid_email_local_byte))
        && domain.contains('.')
        && super::installation::valid_site_domain(domain)
}

fn valid_email_local_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'%' | b'+' | b'-')
}

fn saved_edge_endpoint(
    listen: &serde_json::Map<String, Value>,
    ctx: &OperationContext,
) -> Result<Option<InstallConfigValue>> {
    let transport = match listen.get("transport").and_then(Value::as_str) {
        Some("tcp") => Transport::Tcp,
        Some("udp") => Transport::Udp,
        _ => {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "saved listener transport is invalid",
            ));
        }
    };
    let port = listen
        .get("port")
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port != 0)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "saved listener port is invalid",
            )
        })?;
    let families = listen
        .get("addressFamilies")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "saved listener address families are invalid",
            )
        })?;
    if families.len() != 1 || families[0].as_str() != Some("ipv4") {
        return Err(journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "installation input cannot rewrite a non-IPv4 saved listener",
        ));
    }
    let scope = listen.get("scope").and_then(Value::as_str).ok_or_else(|| {
        journal::failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "saved listener scope is missing",
        )
    })?;
    let address = match scope {
        "public" => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        "host-loopback" => IpAddr::V4(Ipv4Addr::LOCALHOST),
        _ => {
            return Err(journal::failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "installation input cannot rewrite a non-host edge listener",
            ));
        }
    };
    Ok(Some(InstallConfigValue::PublicEdge(HostEndpoint {
        transport,
        address,
        port,
    })))
}

fn digest_candidate<T: Serialize>(
    changes: &T,
    read_set: &hton_contracts::config::ReadSet,
) -> std::result::Result<Sha256Digest, serde_json::Error> {
    let bytes = serde_json::to_vec(&("install-config-candidate-v1", changes, read_set))?;
    Ok(config::digest(&bytes))
}
