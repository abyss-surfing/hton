//! Fixed HTTPS observation services; no arbitrary URL or listener authority.
use crate::{
    Budget, Cancellation, Error, Result,
    process::{Runner, Tool},
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    net::Ipv4Addr,
    sync::Arc,
};
#[derive(Clone)]
pub struct NetworkObserver {
    runner: Arc<Runner>,
}
#[derive(Deserialize)]
struct IpResponse {
    ip: Ipv4Addr,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DnsResponse {
    status: u16,
    #[serde(rename = "TC")]
    truncated: bool,
    question: Vec<Question>,
    #[serde(default)]
    answer: Vec<Answer>,
}
#[derive(Deserialize)]
struct Question {
    name: String,
    #[serde(rename = "type")]
    kind: u16,
}
#[derive(Deserialize)]
struct Answer {
    name: String,
    #[serde(rename = "type")]
    kind: u16,
    data: String,
    #[serde(rename = "TTL")]
    ttl: u32,
}
#[derive(Clone, Debug)]
pub struct DnsIpv4 {
    pub addresses: BTreeSet<Ipv4Addr>,
    pub ttl_seconds: u32,
}
fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
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
fn public(ip: Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();
    !ip.is_private()
        && !ip.is_loopback()
        && !ip.is_link_local()
        && !ip.is_documentation()
        && !ip.is_broadcast()
        && !ip.is_multicast()
        && !ip.is_unspecified()
        && a != 0
        && a < 224
        && !(a == 100 && (64..=127).contains(&b))
        && !(a == 198 && (18..=19).contains(&b))
        && !(a == 192 && b == 0)
}
impl NetworkObserver {
    pub(crate) fn new(runner: Arc<Runner>) -> Self {
        Self { runner }
    }
    fn get(
        &self,
        url: &str,
        accept: &str,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Vec<u8>> {
        self.runner.run(
            Tool::Curl,
            &[
                "--disable",
                "--ipv4",
                "--silent",
                "--show-error",
                "--fail",
                "--connect-timeout",
                "3",
                "--max-time",
                "8",
                "--proto",
                "=https",
                "--tlsv1.2",
                "--noproxy",
                "*",
                "--header",
                accept,
                "--url",
                url,
            ],
            budget,
            cancel,
        )
    }
    pub fn public_ipv4(&self, budget: Budget, cancel: &dyn Cancellation) -> Result<Ipv4Addr> {
        let bytes = self.get(
            "https://api.ipify.org?format=json",
            "Accept: application/json",
            budget,
            cancel,
        )?;
        let response: IpResponse =
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)?;
        if !public(response.ip) {
            return Err(Error::InvalidResponse);
        }
        budget.check(cancel)?;
        Ok(response.ip)
    }
    pub fn dns_ipv4(
        &self,
        domain: &str,
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<DnsIpv4> {
        if !name(domain) {
            return Err(Error::InvalidInput);
        }
        let url = format!("https://cloudflare-dns.com/dns-query?name={domain}&type=A&cd=false");
        let bytes = self.get(&url, "Accept: application/dns-json", budget, cancel)?;
        let response: DnsResponse =
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)?;
        if response.status != 0
            || response.truncated
            || response.question.len() != 1
            || response.question[0].kind != 1
            || response.question[0]
                .name
                .trim_end_matches('.')
                .to_ascii_lowercase()
                != domain
            || response.answer.len() > 128
        {
            return Err(Error::InvalidResponse);
        }
        let mut aliases = BTreeMap::new();
        let mut records = BTreeMap::<String, (BTreeSet<Ipv4Addr>, u32)>::new();
        for answer in response.answer {
            let owner = answer.name.trim_end_matches('.').to_ascii_lowercase();
            if !name(&owner) {
                return Err(Error::InvalidResponse);
            }
            match answer.kind {
                1 => {
                    let ip = answer
                        .data
                        .parse::<Ipv4Addr>()
                        .map_err(|_| Error::InvalidResponse)?;
                    let entry = records
                        .entry(owner)
                        .or_insert((BTreeSet::new(), answer.ttl));
                    entry.0.insert(ip);
                    entry.1 = entry.1.min(answer.ttl);
                }
                5 => {
                    let target = answer.data.trim_end_matches('.').to_ascii_lowercase();
                    if !name(&target) || aliases.insert(owner, (target, answer.ttl)).is_some() {
                        return Err(Error::InvalidResponse);
                    }
                }
                _ => (),
            }
        }
        let mut current = domain.to_owned();
        let mut seen = BTreeSet::new();
        let mut ttl = u32::MAX;
        for _ in 0..16 {
            if !seen.insert(current.clone()) {
                return Err(Error::InvalidResponse);
            }
            if let Some((addresses, record_ttl)) = records.remove(&current) {
                if aliases.contains_key(&current) || addresses.is_empty() {
                    return Err(Error::InvalidResponse);
                }
                budget.check(cancel)?;
                return Ok(DnsIpv4 {
                    addresses,
                    ttl_seconds: ttl.min(record_ttl),
                });
            }
            let (next, alias_ttl) = aliases.remove(&current).ok_or(Error::InvalidResponse)?;
            ttl = ttl.min(alias_ttl);
            current = next;
        }
        Err(Error::InvalidResponse)
    }
}
