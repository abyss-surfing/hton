# adapters/engines/xray.rs

Stable ID: `adapter.engine-xray` · Phase: `R1` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Trusted xray settings/check/render integration

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
- [tests/unit/adapters-contracts.rs](../../tests/unit/adapters-contracts.rs.md) — `verify` -> `check`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

I2 binding: XrayRenderer implements EngineRenderer and ModuleCheck. Deterministic
native JSON contains private WS or XHTTP VLESS inbound, scoped UUID clients,
explicit no stream TLS (Caddy terminates own TLS), private Unbound TCP-local DNS,
IPv4 direct egress and explicit private destination denial. No public SOCKS/API.
Transport-specific paths are canonicalized from supplied protected path material.
Native validation and real clients remain separate, never fabricated by serialization.
Primary schemas: https://xtls.github.io/en/config/transports/websocket.html,
https://xtls.github.io/en/config/transports/xhttp.html and
https://github.com/XTLS/Xray-core/blob/main/infra/conf/transport_internet.go.

Native request access logs remain off; warning/error console output requires the
independent supervised redaction/rotation collector before persistent storage.
No public API/SOCKS/metrics listener is rendered. Direct destinations in private,
loopback, link-local, multicast/reserved and IPv6 ranges are denied; TCP-local DNS
requests bypass user routing so the private Unbound dependency still works.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
