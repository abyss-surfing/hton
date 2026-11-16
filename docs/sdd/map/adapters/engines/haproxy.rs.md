# adapters/engines/haproxy.rs

Stable ID: `adapter.engine-haproxy` · Phase: `R1` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Trusted haproxy settings/check/render integration

Implementation wave: `I2-A` (planned ownership, not completion).

## Features and responsibility

- adapter

## Methods, arguments and result

- `check` — arguments: typed settings, DependencyGraph, supplied evidence; result: CheckReport.
- `render` — arguments: normalized settings, consumers, scoped secret bindings; result: NativeArtifacts and manifest claims.

## Structures and injected dependencies

Typed engine settings / NativeArtifacts; pure ModuleCheck and ModuleRender capability implementations

Dependencies: canonical defaults, dependency consumers, scoped secret bindings, safe native serializer.

## Logic

- Pure format-specific rule/serialization ownership, no active writes or Docker socket.
- Use upstream engine, typed serializers and canonical defaults; no alternative feature registry.
- Native execution belongs to runtime fixed actions; full client qualification is a final functional task.

## Calls / reads

- [contracts/src/config.rs](../../contracts/src/config.rs.md) — `check` -> declarative dependency.

## Called / read by

- [core/src/render.rs](../../core/src/render.rs.md) — `compile` -> `render`.
- [core/src/config.rs](../../core/src/config.rs.md) — `validate` -> `check` through trusted ModuleCheck port.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

I2 binding: HaproxyRenderer implements EngineRenderer. Bounded ClientHello inspection,
exact qualified REALITY SNI raw branches before own-domain Caddy TLS passthrough;
no ALPN heuristics, TLS termination or PROXY header injection into foreign traffic.
Unknown/absent SNI, malformed/non-TLS and timeout reject explicitly. Backends are
numeric qualified private endpoints, eliminating optional management DNS dependency.
Native-target/public-edge topology checks prevent the admitted known routing loop.
Primary syntax: https://docs.haproxy.org/3.2/configuration.html.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
