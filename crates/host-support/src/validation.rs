//! Fixed-command Docker sandboxes for explicit candidate qualification.
use crate::{
    Budget, Cancellation, Error, Result,
    docker::{DockerObserver, ImageReference},
    process::{Runner, Tool},
};
use serde_json::Value;
use std::{net::Ipv4Addr, sync::Arc};

pub struct ValidationDocker {
    runner: Arc<Runner>,
    scope: String,
}
#[derive(Clone, Copy)]
pub enum Engine {
    Haproxy,
    Caddy,
    Xray,
    Unbound,
}
#[derive(Clone, Copy)]
pub enum Role {
    Native,
    Server,
    Client,
    RejectClient,
    SocksRequest,
    DnsTcp,
    DnsUdp,
}
#[derive(Clone, Debug)]
pub struct SandboxResource {
    pub id: String,
    pub network: bool,
}
impl ValidationDocker {
    pub fn new(scope: &str) -> Result<Self> {
        if scope.len() != 32
            || !scope
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            runner: Arc::new(Runner::new()),
            scope: scope.into(),
        })
    }
    fn name(&self, suffix: &str) -> String {
        format!("hton-validation-{}-{suffix}", self.scope)
    }
    fn prefix(&self) -> String {
        format!(
            "10.{}.{}",
            u8::from_str_radix(&self.scope[..2], 16).expect("validated scope"),
            u8::from_str_radix(&self.scope[2..4], 16).expect("validated scope")
        )
    }
    fn label(&self) -> String {
        format!("io.hton.validation={}", self.scope)
    }
    fn command(
        &self,
        args: &[String],
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Vec<u8>> {
        self.runner.run(
            Tool::Docker,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
            budget,
            cancel,
        )
    }
    pub fn check_image(
        &self,
        image: &ImageReference,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        DockerObserver::new(self.runner.clone())
            .inspect_image(image, budget, cancel)
            .map(|_| ())
    }
    pub fn network(&self, budget: Budget, cancel: &dyn Cancellation) -> Result<SandboxResource> {
        let bytes = self.command(
            &[
                "network".into(),
                "create".into(),
                "--driver".into(),
                "bridge".into(),
                "--internal".into(),
                "--subnet".into(),
                format!("{}.0/24", self.prefix()),
                "--label".into(),
                self.label(),
                self.name("network"),
            ],
            budget,
            cancel,
        )?;
        resource(&bytes, true)
    }
    /// No arbitrary command or host mount can enter this interface.
    pub fn create(
        &self,
        service: &str,
        image: &ImageReference,
        engine: Engine,
        role: Role,
        network: Option<&SandboxResource>,
        site: Option<(&str, u16, Ipv4Addr)>,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<SandboxResource> {
        if !matches!(
            service,
            "haproxy" | "caddy" | "xray" | "unbound" | "xray-client"
        ) {
            return Err(Error::InvalidInput);
        }
        let role_name = match role {
            Role::Native => "native",
            Role::Server => "server",
            Role::Client => "client",
            Role::RejectClient => "reject-client",
            Role::SocksRequest => "socks-client",
            Role::DnsTcp => "dns-tcp",
            Role::DnsUdp => "dns-udp",
        };
        if matches!(role, Role::Native) != network.is_none() || network.is_some_and(|r| !r.network)
        {
            return Err(Error::InvalidInput);
        }
        self.check_image(image, budget, cancel)?;
        if let Some(net) = network {
            self.verify(net, budget, cancel)?;
        }
        let mount = format!(
            "type=bind,src=/opt/hton/work/validation/{}/{service},dst=/etc/hton/current,readonly",
            self.scope
        );
        let mut args: Vec<String> = vec!["container", "create", "--pull", "never", "--name"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        args.extend([
            self.name(&format!(
                "{service}-{role_name}{}",
                site.map_or_else(String::new, |(_, port, ip)| format!(
                    "-{port}-{}",
                    ip.octets()[3]
                ))
            )),
            "--label".into(),
            self.label(),
            "--restart".into(),
            "no".into(),
            "--network".into(),
            network.map_or_else(|| "none".into(), |r| r.id.clone()),
            "--user".into(),
            "0:0".into(),
            "--read-only".into(),
            "--cap-drop".into(),
            "ALL".into(),
            "--security-opt".into(),
            "no-new-privileges:true".into(),
            "--memory".into(),
            "128m".into(),
            "--cpus".into(),
            "1.0".into(),
            "--pids-limit".into(),
            "64".into(),
            "--log-driver".into(),
            "none".into(),
        ]);
        if matches!(role, Role::Native | Role::Server) {
            args.extend(["--mount".into(), mount]);
        }
        for mount in [
            "/tmp:rw,noexec,nosuid,size=16m",
            "/run:rw,noexec,nosuid,size=4m",
            "/data:rw,noexec,nosuid,size=16m",
            "/config:rw,noexec,nosuid,size=4m",
            "/acme.sh:rw,noexec,nosuid,size=1m",
        ] {
            args.extend(["--tmpfs".into(), mount.into()]);
        }
        if network.is_some() {
            let index = match role {
                Role::RejectClient => 102,
                Role::SocksRequest => 103,
                Role::DnsTcp => 104,
                Role::DnsUdp => 105,
                Role::Client => {
                    if site.is_some_and(|(_, _, ip)| ip.octets()[3] == 10) {
                        101
                    } else {
                        100
                    }
                }
                _ => match service {
                    "haproxy" => 10,
                    "caddy" => 11,
                    "xray" => 12,
                    "unbound" => 13,
                    "xray-client" => 14,
                    _ => return Err(Error::InvalidInput),
                },
            };
            args.extend(["--ip".into(), format!("{}.{index}", self.prefix())]);
        }
        let (entry, command): (&str, Vec<String>) = match role {
            Role::Client | Role::RejectClient => {
                if !matches!(engine, Engine::Caddy) {
                    return Err(Error::InvalidInput);
                }
                let (requested_host, port, address) = site.ok_or(Error::InvalidInput)?;
                let host = if matches!(role, Role::RejectClient) {
                    "hton-validation.invalid"
                } else {
                    requested_host
                };
                if port == 0
                    || address.is_unspecified()
                    || address.is_multicast()
                    || host.len() > 253
                    || !host.split('.').all(|p| {
                        !p.is_empty()
                            && p.len() <= 63
                            && p.bytes()
                                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                    })
                {
                    return Err(Error::InvalidInput);
                }
                args.extend(["--add-host".into(), format!("{host}:{address}")]);
                for env in [
                    "http_proxy=",
                    "https_proxy=",
                    "HTTP_PROXY=",
                    "HTTPS_PROXY=",
                    "no_proxy=*",
                ] {
                    args.extend(["--env".into(), env.into()]);
                }
                let mut command = vec![
                    "--silent".into(),
                    "--show-error".into(),
                    "--fail".into(),
                    "--max-time".into(),
                    "10".into(),
                    "--connect-timeout".into(),
                    "5".into(),
                    "--noproxy".into(),
                    "*".into(),
                    "--proto".into(),
                    "=https".into(),
                    "--resolve".into(),
                    format!("{host}:{port}:{address}"),
                    "--write-out".into(),
                    "%{http_code}".into(),
                    format!("https://{host}:{port}/"),
                ];
                if matches!(role, Role::RejectClient) {
                    command.insert(0, "--insecure".into());
                }
                ("/usr/bin/curl", command)
            }
            Role::SocksRequest => {
                let (host, port, proxy) = site.ok_or(Error::InvalidInput)?;
                let target = host.parse::<Ipv4Addr>().map_err(|_| Error::InvalidInput)?;
                if target.is_unspecified() || port == 0 || proxy.is_unspecified() {
                    return Err(Error::InvalidInput);
                }
                (
                    "/usr/bin/curl",
                    vec![
                        "--silent".into(),
                        "--show-error".into(),
                        "--fail".into(),
                        "--max-time".into(),
                        "15".into(),
                        "--socks5-hostname".into(),
                        format!("{proxy}:1080"),
                        "--noproxy".into(),
                        "".into(),
                        "--proto".into(),
                        "=http".into(),
                        "--write-out".into(),
                        "%{http_code}".into(),
                        format!("http://{target}:{port}/"),
                    ],
                )
            }
            Role::DnsTcp | Role::DnsUdp => {
                let (_, port, address) = site.ok_or(Error::InvalidInput)?;
                if port == 0 || address.is_unspecified() {
                    return Err(Error::InvalidInput);
                }
                let mut args = vec![
                    format!("@{address}"),
                    "-p".into(),
                    port.to_string(),
                    "hton-validation.invalid".into(),
                    "A".into(),
                    "+short".into(),
                    "+time=2".into(),
                    "+tries=1".into(),
                ];
                if matches!(role, Role::DnsTcp) {
                    args.push("+tcp".into());
                }
                ("/usr/bin/dig", args)
            }
            Role::Native => match engine {
                Engine::Haproxy => (
                    "haproxy",
                    vec!["-c", "-f", "/etc/hton/current/haproxy.cfg"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                ),
                Engine::Caddy => (
                    "caddy",
                    vec![
                        "validate",
                        "--config",
                        "/etc/hton/current/Caddyfile",
                        "--adapter",
                        "caddyfile",
                    ]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                ),
                Engine::Xray => (
                    "xray",
                    vec!["run", "-test", "-config", "/etc/hton/current/xray.json"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                ),
                Engine::Unbound => (
                    "/usr/local/unbound/unbound.d/sbin/unbound-checkconf",
                    vec!["/etc/hton/current/unbound.conf".into()],
                ),
            },
            Role::Server => match engine {
                Engine::Haproxy => (
                    "haproxy",
                    vec!["-W", "-db", "-f", "/etc/hton/current/haproxy.cfg"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                ),
                Engine::Caddy => (
                    "caddy",
                    vec![
                        "run",
                        "--config",
                        "/etc/hton/current/Caddyfile",
                        "--adapter",
                        "caddyfile",
                    ]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                ),
                Engine::Xray => (
                    "xray",
                    vec!["run", "-config", "/etc/hton/current/xray.json"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                ),
                Engine::Unbound => (
                    "/usr/local/unbound/unbound.d/sbin/unbound",
                    vec![
                        "-d".into(),
                        "-c".into(),
                        "/etc/hton/current/unbound.conf".into(),
                    ],
                ),
            },
        };
        args.extend(["--entrypoint".into(), entry.into(), image.as_str().into()]);
        args.extend(command);
        resource(&self.command(&args, budget, cancel)?, false)
    }
    pub fn start(
        &self,
        r: &SandboxResource,
        attach: bool,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Vec<u8>> {
        if r.network {
            return Err(Error::InvalidInput);
        }
        self.verify(r, budget, cancel)?;
        let mut args = vec!["container".into(), "start".into()];
        if attach {
            args.push("--attach".into());
        }
        args.push(r.id.clone());
        let output = self.command(&args, budget, cancel)?;
        if attach {
            let facts = self.inspect(r, budget, cancel)?;
            if facts.pointer("/State/Running").and_then(Value::as_bool) != Some(false)
                || facts.pointer("/State/ExitCode").and_then(Value::as_i64) != Some(0)
            {
                return Err(Error::CommandFailed);
            }
        }
        Ok(output)
    }
    pub fn request_rejected(
        &self,
        r: &SandboxResource,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<()> {
        self.verify(r, budget, cancel)?;
        if r.network {
            return Err(Error::InvalidInput);
        }
        let result = self.runner.run_capture(
            Tool::Docker,
            &["container", "start", "--attach", r.id.as_str()],
            &[],
            budget,
            cancel,
        )?;
        let facts = self.inspect(r, budget, cancel)?;
        if result.success
            || result.stdout != b"000"
            || facts.pointer("/State/Running").and_then(Value::as_bool) != Some(false)
            || facts
                .pointer("/State/ExitCode")
                .and_then(Value::as_i64)
                .is_none_or(|code| !matches!(code, 35 | 52 | 56))
        {
            return Err(Error::CommandFailed);
        }
        Ok(())
    }
    pub fn address(
        &self,
        r: &SandboxResource,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Ipv4Addr> {
        let value = self.inspect(r, budget, cancel)?;
        let networks = value
            .pointer("/NetworkSettings/Networks")
            .and_then(Value::as_object)
            .ok_or(Error::InvalidResponse)?;
        if networks.len() != 1 {
            return Err(Error::IdentityMismatch);
        }
        let address = networks
            .values()
            .next()
            .and_then(|v| {
                v.get("IPAddress")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .or_else(|| v.pointer("/IPAMConfig/IPv4Address").and_then(Value::as_str))
            })
            .ok_or(Error::InvalidResponse)?
            .parse::<Ipv4Addr>()
            .map_err(|_| Error::InvalidResponse)?;
        if address.is_unspecified() || address.is_multicast() {
            return Err(Error::InvalidResponse);
        }
        Ok(address)
    }
    fn inspect(
        &self,
        r: &SandboxResource,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Value> {
        if r.id.len() != 64
            || !r
                .id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(Error::InvalidInput);
        }
        let bytes = self.command(
            &[
                if r.network { "network" } else { "container" }.into(),
                "inspect".into(),
                r.id.clone(),
            ],
            budget,
            cancel,
        )?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)?;
        let array = value.as_array().ok_or(Error::InvalidResponse)?;
        if array.len() != 1 {
            return Err(Error::InvalidResponse);
        }
        let facts = &array[0];
        if facts.get("Id").and_then(Value::as_str) != Some(r.id.as_str())
            || facts
                .pointer(if r.network {
                    "/Labels/io.hton.validation"
                } else {
                    "/Config/Labels/io.hton.validation"
                })
                .and_then(Value::as_str)
                != Some(self.scope.as_str())
            || !facts
                .get("Name")
                .and_then(Value::as_str)
                .is_some_and(|s| s.trim_start_matches('/').starts_with(&self.name("")))
        {
            return Err(Error::IdentityMismatch);
        }
        Ok(facts.clone())
    }
    fn verify(&self, r: &SandboxResource, budget: Budget, cancel: &dyn Cancellation) -> Result<()> {
        self.inspect(r, budget, cancel).map(|_| ())
    }
    /// Exact label inventory also recovers a create that crashed before receipt.
    pub fn resources(
        &self,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Vec<SandboxResource>> {
        let mut found = Vec::new();
        for network in [false, true] {
            let mut args = vec![
                if network { "network" } else { "container" }.into(),
                "ls".into(),
            ];
            if !network {
                args.push("--all".into());
            }
            args.extend([
                "--no-trunc".into(),
                "--quiet".into(),
                "--filter".into(),
                format!("label={}", self.label()),
            ]);
            let bytes = self.command(&args, budget, cancel)?;
            for line in std::str::from_utf8(&bytes)
                .map_err(|_| Error::InvalidResponse)?
                .lines()
            {
                if found.len() >= 32 {
                    return Err(Error::OutputLimit);
                }
                let r = resource(line.as_bytes(), network)?;
                self.verify(&r, budget, cancel)?;
                found.push(r);
            }
        }
        Ok(found)
    }
    pub fn cleanup(
        &self,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Vec<SandboxResource>> {
        let resources = self.resources(budget, cancel)?;
        for r in &resources {
            self.verify(r, budget, cancel)?;
            let mut args = vec![
                if r.network { "network" } else { "container" }.into(),
                "rm".into(),
            ];
            if !r.network {
                args.push("--force".into());
                args.push("--volumes".into());
            }
            args.push(r.id.clone());
            self.command(&args, budget, cancel)?;
        }
        if !self.resources(budget, cancel)?.is_empty() {
            return Err(Error::IdentityMismatch);
        }
        Ok(resources)
    }
}
fn resource(bytes: &[u8], network: bool) -> Result<SandboxResource> {
    let id = std::str::from_utf8(bytes)
        .map_err(|_| Error::InvalidResponse)?
        .trim();
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(Error::InvalidResponse);
    }
    Ok(SandboxResource {
        id: id.into(),
        network,
    })
}
