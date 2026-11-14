//! Native Xray VLESS serializer; transport/authentication remain upstream responsibilities.
use crate::{
    config::issue,
    render::{EngineContext, EngineRenderer, RenderedArtifact},
};
use hton_contracts::{config::*, operations::*};
use hton_runtime_contract::Engine;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub struct XrayRenderer;
impl ModuleCheck for XrayRenderer {
    fn check(
        &self,
        settings: &ModuleSettings,
        _graph: &DependencyGraph,
        _observations: &RuntimeObservation,
    ) -> Vec<CheckIssue> {
        let valid = settings.schema_version == FormatVersion(1)
            && matches!(
                settings.settings.get("transport").and_then(Value::as_str),
                Some("ws" | "xhttp")
            )
            && settings
                .settings
                .get("path")
                .and_then(|path| path.get("secretRef"))
                .and_then(Value::as_str)
                .and_then(crate::config::canonical_secret_id)
                .is_some();
        if valid {
            Vec::new()
        } else {
            vec![issue(
                ErrorCode::ConfigSemanticConflict,
                None,
                ModuleId::new("vless").ok(),
                "VLESS requires a supported transport and a valid protected path reference",
            )]
        }
    }
}
impl EngineRenderer for XrayRenderer {
    fn service_id(&self) -> ServiceId {
        ServiceId::new("xray").expect("trusted ID")
    }
    fn engine(&self) -> Engine {
        Engine::Xray
    }
    fn render(&self, context: &EngineContext<'_, '_>) -> Result<Vec<RenderedArtifact>> {
        if context.consumers.iter().any(|id| id.as_str() != "vless") {
            return Err(context.error(
                ErrorCode::ModuleUnsupported,
                "Xray renderer has an unsupported protocol consumer",
            ));
        }
        let endpoint = context.endpoint("xray")?;
        let dns = context.endpoint("unbound")?;
        let (path, path_reference) = context.vless_path()?;
        let settings = &context.inputs.snapshot.desired.module_settings
            [&ModuleId::new("vless").expect("trusted ID")];
        let transport = settings
            .settings
            .get("transport")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                context.error(
                    ErrorCode::ModuleIncomplete,
                    "VLESS transport is not normalized",
                )
            })?;
        let mut secrets = vec![path_reference];
        let mut clients = Vec::new();
        let mut seen = BTreeSet::new();
        for credential in context
            .inputs
            .credentials
            .iter()
            .filter(|binding| binding.module_id.as_str() == "vless")
        {
            let uuid =
                std::str::from_utf8(context.secret(&credential.reference)?).map_err(|_| {
                    context.error(
                        ErrorCode::ConfigSemanticConflict,
                        "VLESS credential has invalid encoding",
                    )
                })?;
            if uuid.len() != 36
                || !uuid.bytes().enumerate().all(|(index, byte)| {
                    if [8, 13, 18, 23].contains(&index) {
                        byte == b'-'
                    } else {
                        byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()
                    }
                })
                || !seen.insert(uuid.to_owned())
            {
                return Err(context.error(
                    ErrorCode::ConfigSemanticConflict,
                    "VLESS credentials must be distinct canonical UUIDs",
                ));
            }
            clients.push(json!({"id":uuid,"email":credential.identity_id.as_str(),"level":0}));
            secrets.push(credential.reference.clone());
        }
        if clients.is_empty() {
            return Err(context.error(
                ErrorCode::ModuleIncomplete,
                "Enabled VLESS has no provisioned credential binding",
            ));
        }
        clients.sort_by(|left, right| left["email"].as_str().cmp(&right["email"].as_str()));
        let stream = match transport {
            "ws" => {
                json!({"network":"ws","security":"none","wsSettings":{"path":path,"acceptProxyProtocol":false}})
            }
            "xhttp" => {
                json!({"network":"xhttp","security":"none","xhttpSettings":{"path":path,"mode":"auto"}})
            }
            _ => {
                return Err(context.error(
                    ErrorCode::ModuleUnsupported,
                    "VLESS transport is not implemented by the native serializer",
                ));
            }
        };
        let mut config: Value = serde_json::from_str(include_str!(
            "../../../platform/etc/xray/config.json.template"
        ))
        .map_err(|_| {
            context.error(
                ErrorCode::ConfigSemanticConflict,
                "Trusted Xray template is invalid",
            )
        })?;
        config["dns"]["servers"] = json!([format!("tcp+local://{dns}")]);
        config["inbounds"] = json!([{"tag":"vless","listen":endpoint.ip().to_string(),"port":endpoint.port(),"protocol":"vless","settings":{"clients":clients,"decryption":"none"},"streamSettings":stream,"sniffing":{"enabled":false}}]);
        let bytes = serde_json::to_vec_pretty(&config).map_err(|_| {
            context.error(
                ErrorCode::ConfigSemanticConflict,
                "Native Xray configuration cannot be encoded",
            )
        })?;
        Ok(vec![context.artifact("xray.json", bytes, secrets)?])
    }
}
