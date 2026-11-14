//! Fixed native probe clients, never a general HTTP/command execution API.
use crate::{
    Budget, Cancellation, Error, Result,
    crypto::PrivateDirectory,
    docker::{ContainerId, ImageReference},
    process::{Runner, Tool},
};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
static CLIENT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[derive(Clone)]
pub struct ProbeTools {
    runner: Arc<Runner>,
}
pub struct HttpReply {
    pub status: u16,
    pub body: Vec<u8>,
}
impl std::fmt::Debug for HttpReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpReply")
            .field("status", &self.status)
            .field("body", &"[protected]")
            .finish()
    }
}
fn hostname(value: &str) -> bool {
    value.len() <= 253
        && !value.is_empty()
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}
impl ProbeTools {
    pub(crate) fn new(runner: Arc<Runner>) -> Self {
        Self { runner }
    }
    pub fn http(
        &self,
        endpoint: SocketAddrV4,
        tls_hostname: Option<&str>,
        socks: Option<SocketAddrV4>,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<HttpReply> {
        if endpoint.ip().is_unspecified()
            || endpoint.ip().is_multicast()
            || endpoint.ip().is_broadcast()
            || endpoint.port() == 0
            || tls_hostname.is_some_and(|name| !hostname(name))
        {
            return Err(Error::InvalidInput);
        }
        let mut args = vec![
            "--disable",
            "--noproxy",
            "*",
            "--ipv4",
            "--silent",
            "--show-error",
            "--max-time",
            "5",
            "--connect-timeout",
            "2",
            "--proto",
            "=http,https",
            "--max-redirs",
            "0",
            "--request",
            "GET",
            "--write-out",
            "\n%{http_code}",
        ];
        let url = match tls_hostname {
            Some(name) => format!("https://{name}:{}/", endpoint.port()),
            None => format!("http://{endpoint}/"),
        };
        let resolve =
            tls_hostname.map(|name| format!("{name}:{}:{}", endpoint.port(), endpoint.ip()));
        if let Some(resolve) = resolve.as_ref() {
            args.extend_from_slice(&["--resolve", resolve]);
        }
        let proxy = socks.map(|endpoint| endpoint.to_string());
        // Explicit SOCKS is intentional; noproxy must not bypass it.
        if let Some(proxy) = proxy.as_ref() {
            args.retain(|arg| *arg != "*");
            let at = args
                .iter()
                .position(|arg| *arg == "--noproxy")
                .ok_or(Error::InvalidInput)?;
            args.remove(at);
            args.extend_from_slice(&["--noproxy", "", "--socks5", proxy]);
        }
        args.extend_from_slice(&["--url", &url]);
        let bytes = self.runner.run(Tool::Curl, &args, budget, cancel)?;
        let split = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .ok_or(Error::InvalidResponse)?;
        let code = std::str::from_utf8(&bytes[split + 1..])
            .map_err(|_| Error::InvalidResponse)?
            .parse::<u16>()
            .map_err(|_| Error::InvalidResponse)?;
        if !(100..=599).contains(&code) {
            return Err(Error::InvalidResponse);
        }
        budget.check(cancel)?;
        Ok(HttpReply {
            status: code,
            body: bytes[..split].to_vec(),
        })
    }
    pub fn dnssec_root(
        &self,
        address: Ipv4Addr,
        port: u16,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<bool> {
        if address.is_unspecified() || address.is_multicast() || port == 0 {
            return Err(Error::InvalidInput);
        }
        let server = format!("@{address}");
        let port = port.to_string();
        let bytes = self.runner.run(
            Tool::Dig,
            &[
                &server,
                "-p",
                &port,
                ".",
                "SOA",
                "+dnssec",
                "+time=2",
                "+tries=1",
                "+noall",
                "+comments",
                "+answer",
            ],
            budget,
            cancel,
        )?;
        let text = std::str::from_utf8(&bytes).map_err(|_| Error::InvalidResponse)?;
        let status = text
            .lines()
            .any(|line| line.starts_with(";; ->>HEADER<<-") && line.contains("status: NOERROR,"));
        let authenticated = text
            .lines()
            .find(|line| line.starts_with(";; flags:"))
            .and_then(|line| line.split(';').nth(2))
            .is_some_and(|flags| flags.split_whitespace().any(|flag| flag == "ad"));
        let soa = text
            .lines()
            .filter(|line| !line.starts_with(';'))
            .any(|line| {
                let parts: Vec<_> = line.split_whitespace().collect();
                parts.len() >= 7 && parts[0] == "." && parts[2] == "IN" && parts[3] == "SOA"
            });
        budget.check(cancel)?;
        Ok(status && authenticated && soa)
    }
    fn docker(&self, args: &[&str], budget: Budget, cancel: &dyn Cancellation) -> Result<Vec<u8>> {
        let mut full = vec![
            "--host",
            "unix:///var/run/docker.sock",
            "--config",
            "/opt/hton/work/docker-client",
        ];
        full.extend_from_slice(args);
        self.runner.run(Tool::Docker, &full, budget, cancel)
    }
    pub fn create_xray_client(
        &self,
        image: &ImageReference,
        namespace: &ContainerId,
        config: &PrivateDirectory,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<ContainerId> {
        let root = std::path::Path::new("/opt/hton/work/runtime/probes");
        let relative = config
            .path()
            .strip_prefix(root)
            .map_err(|_| Error::InvalidInput)?;
        if relative.components().count() != 1
            || relative.to_str().is_none_or(|value| {
                value.is_empty()
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            })
        {
            return Err(Error::InvalidInput);
        }
        let _protected = config.file("client.json", 1024 * 1024)?;
        let mount = format!(
            "type=bind,source={},target=/probe,readonly",
            config.path().to_str().ok_or(Error::InvalidInput)?
        );
        // No remote image resolution or scripts: the selected image must already exist.
        let facts = crate::docker::DockerObserver::new(self.runner.clone())
            .inspect_image(image, budget, cancel)?;
        if facts.os != "linux" {
            return Err(Error::InvalidResponse);
        }
        let network = format!("container:{}", namespace.as_str());
        let name = format!(
            "hton-probe-{}-{}",
            std::process::id(),
            CLIENT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let bytes = self.docker(
            &[
                "create",
                "--name",
                &name,
                "--pull",
                "never",
                "--network",
                &network,
                "--restart",
                "no",
                "--read-only",
                "--cap-drop",
                "ALL",
                "--security-opt",
                "no-new-privileges:true",
                "--log-driver",
                "none",
                "--memory",
                "64m",
                "--pids-limit",
                "32",
                "--cpus",
                "0.5",
                "--tmpfs",
                "/tmp:rw,noexec,nosuid,size=4m",
                "--mount",
                &mount,
                "--label",
                "io.hton.probe=true",
                "--entrypoint",
                "xray",
                image.as_str(),
                "run",
                "-config",
                "/probe/client.json",
            ],
            budget,
            cancel,
        )?;
        ContainerId::new(
            std::str::from_utf8(&bytes)
                .map_err(|_| Error::InvalidResponse)?
                .trim(),
        )
    }
    pub fn start_xray_client(
        &self,
        id: &ContainerId,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        self.docker(&["start", id.as_str()], budget, cancel)
            .map(|_| ())
    }
    pub fn remove_xray_client(
        &self,
        id: &ContainerId,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        let facts = crate::docker::DockerObserver::new(self.runner.clone())
            .inspect_container(id, budget, cancel)?;
        if facts
            .config
            .labels
            .as_ref()
            .and_then(|labels| labels.get("io.hton.probe"))
            .map(String::as_str)
            != Some("true")
            || !facts.name.starts_with("/hton-probe-")
            || facts.host_config.restart_policy.name != "no"
            || facts.host_config.log_config.r#type != "none"
        {
            return Err(Error::IdentityMismatch);
        }
        self.docker(&["rm", "--force", id.as_str()], budget, cancel)
            .map(|_| ())
    }
}
