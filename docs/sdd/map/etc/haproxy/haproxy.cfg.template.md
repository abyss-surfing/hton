# etc/haproxy/haproxy.cfg.template

Stable ID: `template.haproxy` · Phase: `R1` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: TCP/SNI and optional DoT template

Implementation wave: `I2-A` (planned ownership, not completion).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Exact raw routes before default Caddy; no no-ALPN identity rule.
- Absent/stopped/unresolvable admin backend cannot break startup/reload; test route-local failure independently.
- No automatic protocol recognition from missing ALPN; R1 IPv4.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/render.rs](../../core/src/render.rs.md) — `compile` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

I2 serializer-owned slots LISTEN/ALLOWED_SNI/RAW_ROUTES/CADDY_ENDPOINT/RAW_BACKENDS
accept only validated numeric endpoints and concrete DNS names. Bounded inspection
rejects missing/unknown SNI and timeout/non-TLS; recognized own-domain ClientHello
passes unchanged to Caddy, including no-ALPN. ECH is not recognized as a protocol:
unrecognized outer SNI rejects; recognized outer names reach ordinary native TLS,
which has no HTON ECH provisioning. Final clients must qualify these outcomes.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
