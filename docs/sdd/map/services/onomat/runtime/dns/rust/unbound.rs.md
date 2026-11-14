# services/onomat/runtime/dns/rust/unbound.rs

Stable ID: `adapter.engine-unbound` · Phase: `R1` · Kind: source
Implementation wave: `I2-A`.

Owned by Onomat; HTON includes the renderer from this path. Owns pure Unbound
native serialization through injected EngineRenderer. Inputs:
explicit private IPv4 endpoint/project subnet, prepared DNSSEC trust-anchor bytes
with exact digest, and enabled shared-engine consumers. Outputs protected config
and trust-anchor artifacts; no DNS socket, download, issuer, state or process effect.
Recursion is private to the project, DNSSEC validated, caches/threads bounded,
IPv6 disabled for R1. Native unbound-checkconf and real DNS evidence are separate.
Caller core.render, DTO owner contracts; upstream engine provides DNS semantics.
Primary syntax: https://www.nlnetlabs.nl/documentation/unbound/unbound.conf/.

R1 serialization consumes a prepared pinned/qualified root trust-anchor artifact,
never fetches a key during render or accepted boot. Startup must have that artifact
available; missing material blocks qualification. Query/reply logs are disabled;
native error console output goes through the independent redacted log collector.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
