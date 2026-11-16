//! Byte-preserving TCP edge. No ALPN-based protocol guessing or own TLS termination.
use crate::render::{EngineContext, EngineRenderer, RenderedArtifact};
use hton_contracts::{config::*, operations::*};
use hton_runtime_contract::Engine;
use serde_json::Value;
use std::collections::BTreeSet;

pub struct HaproxyRenderer;
impl EngineRenderer for HaproxyRenderer {
    fn service_id(&self) -> ServiceId {
        ServiceId::new("haproxy").expect("trusted ID")
    }
    fn engine(&self) -> Engine {
        Engine::Haproxy
    }
    fn render(&self, context: &EngineContext<'_, '_>) -> Result<Vec<RenderedArtifact>> {
        let edge = context.endpoint("haproxy")?;
        let caddy = context.endpoint("caddy")?;
        let mut allowed: BTreeSet<String> =
            context.inputs.snapshot.desired.platform.settings["core"]["edge"]["domains"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_ascii_lowercase)
                .collect();
        let mut routes = String::new();
        let mut backends = String::new();
        let mut raw: Vec<_> = context
            .inputs
            .graph
            .route_claims
            .iter()
            .filter_map(|claim| match &claim.matcher {
                RouteMatch::Sni { hostname } => Some((hostname.to_ascii_lowercase(), claim)),
                _ => None,
            })
            .collect();
        raw.sort_by(|left, right| left.0.cmp(&right.0));
        for (index, (hostname, claim)) in raw.iter().enumerate() {
            if !crate::catalog::valid_hostname(hostname, false) || !allowed.insert(hostname.clone())
            {
                return Err(context.error(
                    ErrorCode::ConfigSemanticConflict,
                    "Exact foreign SNI conflicts with another route or own domain",
                ));
            }
            let backend = context
                .inputs
                .network
                .foreign_backends
                .get(&claim.target_service)
                .ok_or_else(|| {
                    context.error(
                        ErrorCode::ModuleIncomplete,
                        "Exact foreign SNI has no qualified native passthrough endpoint",
                    )
                })?;
            if !backend.server_names.contains(hostname) {
                return Err(context.error(
                    ErrorCode::ResourceDrift,
                    "Passthrough SNI is not in exact native serverNames qualification",
                ));
            }
            routes.push_str(&format!(
                "    use_backend foreign_{index} if {{ req.ssl_sni -i {hostname} }}\n"
            ));
            backends.push_str(&format!(
                "\nbackend foreign_{index}\n    server native {} check inter 5s fall 3 rise 2\n",
                backend.endpoint
            ));
        }
        if allowed
            .iter()
            .any(|domain| !crate::catalog::valid_hostname(domain, false))
        {
            return Err(context.error(
                ErrorCode::ConfigSemanticConflict,
                "TLS routing names are invalid",
            ));
        }
        let config = if allowed.is_empty() {
            // Unpublished no-certificate bootstrap: only its qualified private endpoint, all traffic closed.
            format!(
                "global\n    maxconn 32\n    nbthread 1\ndefaults\n    mode tcp\n    timeout connect 5s\n    timeout client 5s\n    timeout server 5s\nfrontend unpublished\n    bind {edge}\n    tcp-request content reject\n"
            )
        } else {
            include_str!("../../../platform/etc/haproxy/haproxy.cfg.template")
                .replace("{{LISTEN}}", &edge.to_string())
                .replace(
                    "{{ALLOWED_SNI}}",
                    &allowed.into_iter().collect::<Vec<_>>().join(" "),
                )
                .replace("{{RAW_ROUTES}}", &routes)
                .replace("{{CADDY_ENDPOINT}}", &caddy.to_string())
                .replace("{{RAW_BACKENDS}}", &backends)
        };
        Ok(vec![context.artifact(
            "haproxy.cfg",
            config.into_bytes(),
            Vec::new(),
        )?])
    }
}
