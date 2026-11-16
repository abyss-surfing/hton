//! Explicit R1 owner disclosure from accepted bindings, with exact transport serializers.
use crate::{
    certificates::CertificateName,
    journal::{OperationControl, failure},
    secrets::{ConsumerGrant, SecretRef, SecretStore},
};
use hton_contracts::operations::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fmt, num::NonZeroU16};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XHttpMode {
    Auto,
    PacketUp,
    StreamUp,
    StreamOne,
}
impl XHttpMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::PacketUp => "packet-up",
            Self::StreamUp => "stream-up",
            Self::StreamOne => "stream-one",
        }
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "transport",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum VlessTransport {
    WebSocket {
        path: String,
        host: CertificateName,
    },
    XHttp {
        path: String,
        host: CertificateName,
        mode: XHttpMode,
    },
}
impl VlessTransport {
    fn path(&self) -> &str {
        match self {
            Self::WebSocket { path, .. } | Self::XHttp { path, .. } => path,
        }
    }
    fn host(&self) -> &CertificateName {
        match self {
            Self::WebSocket { host, .. } | Self::XHttp { host, .. } => host,
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::WebSocket { .. } => "ws",
            Self::XHttp { .. } => "xhttp",
        }
    }
}
impl fmt::Debug for VlessTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VlessTransport")
            .field("kind", &self.kind())
            .field("path", &"[redacted]")
            .finish()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    VlessUri,
    XrayJson,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptedCredentialBinding {
    pub identity_id: ObjectId,
    pub profile_id: ObjectId,
    pub accepted_bundle: BundleId,
    pub service_id: ServiceId,
    pub secret: SecretRef,
    pub scope: ScopeId,
    pub enabled: bool,
    pub revoked: bool,
    pub credential_fence: u64,
    pub endpoint: CertificateName,
    pub port: NonZeroU16,
    pub server_name: CertificateName,
    pub transport: VlessTransport,
    pub display_name: String,
}
impl fmt::Debug for AcceptedCredentialBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AcceptedCredentialBinding")
            .field("identity_id", &self.identity_id)
            .field("profile_id", &self.profile_id)
            .field("accepted_bundle", &self.accepted_bundle)
            .finish_non_exhaustive()
    }
}
pub struct ProtectedExport(Vec<u8>);
impl ProtectedExport {
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Debug for ProtectedExport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProtectedExport([redacted])")
    }
}
pub trait ExportInventory: Send + Sync {
    fn accepted(
        &self,
        identity: &ObjectId,
        profile: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<AcceptedCredentialBinding>;
    fn current_accepted(&self, ctx: &OperationContext) -> Result<(BundleId, u64)>;
    fn disclosure_allowed(
        &self,
        binding: &AcceptedCredentialBinding,
        ctx: &OperationContext,
    ) -> Result<bool>;
    /// Client/version capability qualification is separate for WS and XHTTP.
    fn client_qualified(
        &self,
        client: &ObjectId,
        transport: &VlessTransport,
        format: &ExportFormat,
        ctx: &OperationContext,
    ) -> Result<bool>;
}
pub struct ExportCoordinator<'a> {
    pub inventory: &'a dyn ExportInventory,
    pub secrets: &'a SecretStore,
    pub control: &'a dyn OperationControl,
}
impl ExportCoordinator<'_> {
    pub fn export_profile(
        &self,
        identity: &ObjectId,
        profile: &ObjectId,
        client: &ObjectId,
        format: ExportFormat,
        ctx: &OperationContext,
    ) -> Result<ProtectedExport> {
        if self.control.is_cancelled(&ctx.cancellation) || self.control.now_ms() >= ctx.deadline.0 {
            return Err(failure(
                ctx,
                ErrorCode::OperationDeadline,
                "profile export deadline or cancellation reached",
            ));
        }
        let binding = self.inventory.accepted(identity, profile, ctx)?;
        let (accepted, fence) = self.inventory.current_accepted(ctx)?;
        if &binding.identity_id != identity
            || &binding.profile_id != profile
            || binding.accepted_bundle != accepted
            || binding.credential_fence < fence
            || !binding.enabled
            || binding.revoked
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "profile is not an active accepted credential binding",
            ));
        }
        if !self.inventory.disclosure_allowed(&binding, ctx)?
            || !self
                .inventory
                .client_qualified(client, &binding.transport, &format, ctx)?
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "credential disclosure or exact client transport is not qualified",
            ));
        }
        validate_binding(&binding, ctx)?;
        let material = self.secrets.resolve(
            &binding.secret,
            &ConsumerGrant {
                consumer: binding.service_id.clone(),
                scope: binding.scope.clone(),
            },
            ctx,
        )?;
        let uuid = std::str::from_utf8(material.expose()).map_err(|_| {
            failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "credential is not a qualified VLESS UUID",
            )
        })?;
        if !valid_uuid(uuid) {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "credential is not a qualified VLESS UUID",
            ));
        }
        let bytes = match format {
            ExportFormat::VlessUri => {
                let mut query = format!(
                    "encryption=none&security=tls&type={}&sni={}&host={}&path={}",
                    binding.transport.kind(),
                    percent(binding.server_name.as_str()),
                    percent(binding.transport.host().as_str()),
                    percent(binding.transport.path())
                );
                match &binding.transport {
                    VlessTransport::WebSocket { .. } => query.push_str("&alpn=http%2F1.1"),
                    VlessTransport::XHttp { mode, .. } => {
                        query.push_str(&format!("&mode={}&alpn=h2%2Chttp%2F1.1", mode.as_str()))
                    }
                }
                format!(
                    "vless://{}@{}:{}?{}#{}",
                    uuid,
                    binding.endpoint.as_str(),
                    binding.port,
                    query,
                    percent(&binding.display_name)
                )
                .into_bytes()
            }
            ExportFormat::XrayJson => {
                let mut stream = json!({"network":binding.transport.kind(),"security":"tls","tlsSettings":{"serverName":binding.server_name.as_str(),"allowInsecure":false}});
                match &binding.transport {
                    VlessTransport::WebSocket { path, host } => {
                        stream["tlsSettings"]["alpn"] = json!(["http/1.1"]);
                        stream["wsSettings"] = json!({"path":path,"host":host.as_str()});
                    }
                    VlessTransport::XHttp { path, host, mode } => {
                        stream["tlsSettings"]["alpn"] = json!(["h2", "http/1.1"]);
                        stream["xhttpSettings"] =
                            json!({"path":path,"host":host.as_str(),"mode":mode.as_str()});
                    }
                }
                // Current native Xray dialect, independently pinned by client qualification.
                serde_json::to_vec_pretty(&json!({"outbounds":[{"tag":"hton","protocol":"vless","settings":{"address":binding.endpoint.as_str(),"port":binding.port.get(),"id":uuid,"encryption":"none"},"streamSettings":stream}]})).map_err(|_|failure(ctx,ErrorCode::IoUnavailable,"profile serialization failed"))?
            }
        };
        if self.control.now_ms() >= ctx.deadline.0 || self.control.is_cancelled(&ctx.cancellation) {
            return Err(failure(
                ctx,
                ErrorCode::OperationDeadline,
                "profile export expired before disclosure",
            ));
        }
        // Recheck accepted/fence after resolution; no fallback can re-export a revoked key.
        if self.inventory.current_accepted(ctx)? != (accepted, fence) {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "accepted credential policy changed during export",
            ));
        }
        Ok(ProtectedExport(bytes))
    }
}
fn validate_binding(binding: &AcceptedCredentialBinding, ctx: &OperationContext) -> Result<()> {
    let path = binding.transport.path();
    if binding.endpoint.is_wildcard()
        || binding.server_name.is_wildcard()
        || binding.transport.host().is_wildcard()
        || binding.display_name.len() > 256
        || !path.starts_with('/')
        || path.len() > 1024
        || path
            .bytes()
            .any(|b| b < 0x21 || b > 0x7e || matches!(b, b'?' | b'#' | b'\\'))
    {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "accepted client endpoint or route is not serializable",
        ));
    }
    Ok(())
}
fn valid_uuid(uuid: &str) -> bool {
    uuid.len() == 36
        && uuid.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn percent(value: &str) -> String {
    let mut encoded = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{b:02X}"));
        }
    }
    encoded
}
