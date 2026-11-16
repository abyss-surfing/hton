//! Own-domain HTTPS and the ordinary site, independent of management processes.
use crate::render::{EngineContext, EngineRenderer, RenderedArtifact};
use hton_contracts::operations::*;
use hton_runtime_contract::Engine;
use serde_json::Value;

pub struct CaddyRenderer;
impl EngineRenderer for CaddyRenderer {
    fn service_id(&self) -> ServiceId {
        ServiceId::new("caddy").expect("trusted ID")
    }
    fn engine(&self) -> Engine {
        Engine::Caddy
    }
    fn render(&self, context: &EngineContext<'_, '_>) -> Result<Vec<RenderedArtifact>> {
        if context
            .consumers
            .iter()
            .any(|id| !matches!(id.as_str(), "core" | "vless"))
        {
            return Err(context.error(
                ErrorCode::ModuleUnsupported,
                "Caddy renderer has an unsupported optional consumer",
            ));
        }
        let endpoint = context.endpoint("caddy")?;
        let domains: Vec<_> =
            context.inputs.snapshot.desired.platform.settings["core"]["edge"]["domains"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_ascii_lowercase)
                .collect();
        if domains
            .iter()
            .any(|domain| !crate::catalog::valid_hostname(domain, false))
        {
            return Err(context.error(
                ErrorCode::ConfigSemanticConflict,
                "Own HTTPS hostnames are invalid",
            ));
        }
        let mut artifacts = Vec::new();
        let mut secrets = Vec::new();
        let (addresses, tls) = if domains.is_empty() {
            (format!("http://{endpoint}"), String::new())
        } else {
            let certificate = context.inputs.certificate.ok_or_else(|| {
                context.error(
                    ErrorCode::ModuleIncomplete,
                    "Own-domain HTTPS certificate material is not qualified",
                )
            })?;
            if certificate.service_id != *context.service_id
                || certificate.valid_until.0 <= context.inputs.now.0
                || domains.iter().any(|domain| !certificate.covers(domain))
            {
                return Err(context.error(
                    ErrorCode::ModuleIncomplete,
                    "Qualified certificate does not cover the own-domain HTTPS consumer",
                ));
            }
            let chain = context.secret(&certificate.chain)?;
            let key = context.secret(&certificate.key)?;
            if chain.is_empty() || key.is_empty() {
                return Err(context.error(
                    ErrorCode::ModuleIncomplete,
                    "Qualified certificate bytes are missing",
                ));
            }
            artifacts.push(context.artifact(
                "fullchain.pem",
                chain.to_vec(),
                vec![certificate.chain.clone()],
            )?);
            artifacts.push(context.artifact(
                "key.pem",
                key.to_vec(),
                vec![certificate.key.clone()],
            )?);
            secrets.extend([certificate.chain.clone(), certificate.key.clone()]);
            (
                domains
                    .iter()
                    .map(|domain| format!("https://{domain}:{}", endpoint.port()))
                    .collect::<Vec<_>>()
                    .join(", "),
                "    tls /etc/hton/current/fullchain.pem /etc/hton/current/key.pem".into(),
            )
        };
        let mut routes = String::new();
        if context
            .inputs
            .graph
            .enabled_modules
            .iter()
            .any(|id| id.as_str() == "vless")
        {
            let (path, reference) = context.vless_path()?;
            secrets.push(reference);
            let xray = context.endpoint("xray")?;
            let transport = context.inputs.snapshot.desired.module_settings
                [&ModuleId::new("vless").expect("trusted ID")]
                .settings
                .get("transport")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    context.error(
                        ErrorCode::ModuleIncomplete,
                        "VLESS transport is not normalized",
                    )
                })?;
            let matcher = match transport {
                "ws" => path.clone(),
                "xhttp" => format!("{path} {path}/*"),
                _ => {
                    return Err(context.error(
                        ErrorCode::ModuleUnsupported,
                        "Caddy VLESS transport is unsupported",
                    ));
                }
            };
            let versions = if transport == "xhttp" {
                "h2c 1.1"
            } else {
                "1.1"
            };
            routes = format!(
                "        @vless path {matcher}\n        handle @vless {{\n            reverse_proxy {xray} {{\n                flush_interval -1\n                transport http {{\n                    versions {versions}\n                }}\n            }}\n        }}"
            );
        }
        artifacts.push(context.artifact(
            "site-index.html",
            context.inputs.site_body.to_vec(),
            Vec::new(),
        )?);
        let config = include_str!("../../../platform/etc/caddy/Caddyfile.template")
            .replace("{{SITE_ADDRESSES}}", &addresses)
            .replace("{{BIND_ADDRESS}}", &endpoint.ip().to_string())
            .replace("{{TLS}}", &tls)
            .replace("{{VPN_ROUTES}}", &routes)
            .replace(
                "{{SITE_INDEX_DIGEST}}",
                context.inputs.site_index_digest.as_str(),
            );
        artifacts.push(context.artifact("Caddyfile", config.into_bytes(), secrets)?);
        Ok(artifacts)
    }
}
