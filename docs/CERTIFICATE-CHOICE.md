# Certificate validation and issuer choice

Research checkpoint: 2026-11-03. Authority: OR-19/OR-20/OR-21. This document separates
owner goals, verified technical facts and engineering recommendations. No live
issuer migration or certificate issuance has been performed for this research.

## Independent choices

An issuer signs the certificate: Let's Encrypt, ZeroSSL, Google Public CA, etc.
A validation challenge demonstrates domain control. HTTP-01 and DNS-01 do not
select stronger or weaker site TLS encryption. Both support unattended renewal
with an appropriate maintained ACME client.

HTTP-01 serves a challenge at the domain on public TCP/80. Domain routing and
port availability must remain suitable at future renewals. It needs no DNS API
key. It cannot obtain a wildcard certificate. DNS-01 publishes a TXT record under
_acme-challenge. It supports wildcards and does not require inbound HTTP/80;
unattended operation normally requires a suitable DNS API or delegated challenge
zone. Manual TXT changes on every renewal do not meet the owner's unattended goal.
Source: [Let's Encrypt challenge types](https://letsencrypt.org/docs/challenge-types/).

Public DNS/IP agreement is a useful preflight, not CA authorization or proof that
public HTTP/80 reaches the challenge handler. DNS caches, AAAA records, NAT,
firewalls, proxy/CDN routing and CAA restrictions still affect issuance. Keep DNS
instructions provider-neutral. Scope public identity observations to explicit
setup; actual CA challenge requests are ordinary certificate lifecycle traffic.

## Issuer alternatives

- Let's Encrypt is a suitable default primary for the public product. A
  normal automated ACME account does not need a DNS-provider API account for
  HTTP-01. Its public validation requirements remain independent of client choice.
- ZeroSSL provides free automated ACME certificates and requires External Account
  Binding (EAB). Those are CA account credentials, not DNS-provider credentials.
  Use the ACME service rather than confusing it with the separate commercial
  REST API. Source: [ZeroSSL ACME](https://zerossl.com/documentation/acme).
- Google Public CA supports automated publicly trusted issuance. ACME account
  registration requires EAB and the Google Cloud setup described in its official
  guide. Source: [Google Public CA tutorial](https://docs.cloud.google.com/certificate-manager/docs/public-ca-tutorial?hl=en).

There is no established evidence that replacing Let's Encrypt alone improves
confidentiality or browser UX for the supported fleet. Issuer failover can improve
availability during issuer-specific outages; it does not fix wrong DNS, blocked
ports or restrictive CAA. Alternative issuers introduce account/configuration and
chain compatibility considerations, so do not enable one silently.

## Recommended engineering direction, not completed implementation

Select validation by domain/publication policy, not by the number of domains or
VPS. HTTP-01 is appropriate where every validator can reach the correct challenge
handler on TCP/80; it can serve many independent domains and nodes. DNS-01 is
appropriate for wildcards, absent inbound HTTP, or shared names without coordinated
HTTP challenge routing. Neither method is globally preferred for every topology.
Keep Let's Encrypt as a configurable default CA and DNS providers pluggable.
Implement OR-19 provider-neutral preflight/manual help using the configured
publication topology; example.com is an instance example only.
Use one maintained certificate lifecycle owner, persisted private account/key
state, renewal before expiry, retries, served-chain/hostname verification and an
expiry health signal. Reuse valid material and retain it on failed renewals.

Caddy supports native automatic issuance, renewal, retry and issuer fallback:
[Caddy automatic HTTPS](https://caddyserver.com/docs/automatic-https). Its loaded-
certificate mode in the current HTON profile does not activate that lifecycle.
The existing profile uses acme.sh plus a timer. A switch to native Caddy management
would require replacing that issuer ownership, synchronizing storage and rollback
contracts, and acceptance; never run two independent renewers on the same keys.
Google likewise recommends maintained fully automated ACME clients:
[Google ACME best practices](https://pki.goog/updates/september2025-acme-best-practices.html).

Current implementation work D115 adds HTTP-01 and explicit publication preflight
to the minimum profile, with DNS-01/InternetBS as an optional adapter. Acceptance
is tracked in IMPLEMENTATION-STATUS.md. Fleet coordination, further DNS adapters
and alternative-issuer failover remain implementation work. This research must not be read as their acceptance result.

## Fleet-specific reliability constraints

Independent VPS/domain deployments keep node identity, configuration, accounts,
private keys and certificate storage separate. There is no Git-shipped personal
domain/email or automatically shared private key.

When the same FQDN resolves to several nodes, HTTP-01 requests can arrive at any
front end. Route the challenge consistently or use an appropriate DNS-01 policy.
Do not require DNS A records to equal each node's outbound IP: a load balancer,
proxy, NAT or round-robin pool can be the intended publication. Preflight compares
observations to configured entry points and reports mismatches/unverified fields.

For concurrent DNS-01 requests, preserve other TXT values and remove only the
owned challenge token. Prefer least-privilege DNS accounts or challenge-zone
delegation. A shared certificate/distribution service is optional and would require
one issuance owner, coordinated locking and authenticated secret distribution.

Renewal scheduling must account for CA limits shared across registered domains,
reuse accepted certificates, stagger requests, retry with backoff and expose
per-domain/per-node expiry health. A reserve issuer is an explicit account/policy
choice and must be compatible with CAA. It is not a fix for failed publication.

Current gaps include the minimum profile's first-domain-only selection, hardcoded
InternetBS path, public bootstrap instance values and missing fleet challenge
coordination. These are implementation limitations, not accepted product limits.
