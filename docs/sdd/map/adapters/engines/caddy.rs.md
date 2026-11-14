# adapters/engines/caddy.rs

Stable ID: `adapter.engine-caddy` · Phase: `R1` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Trusted caddy settings/check/render integration

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

I2 binding: CaddyRenderer implements EngineRenderer; Caddyfile serialization uses
safe validated tokens, own-domain qualified supplied certificate/key artifacts,
H1/H2 only, auto HTTPS issuance disabled and admin off. The ordinary site serves
the exact bounded bytes read from `/var/www/index.html` as a separate immutable
`site-index.html` artifact. Its fixed fallback route rewrites requests to this file;
client paths cannot address sibling Caddyfile or certificate artifacts. No source
path or host mount enters Caddy, and source symlinks cannot reach private HTON
configuration. The Caddyfile binds the content digest so changed source content
changes the admitted bundle. VLESS WS exact path or
XHTTP slash descendants proxy privately to Xray with streaming flush; remaining
requests reach the ordinary-site file server. No management hostname or process is
required.
Primary syntax: https://caddyserver.com/docs/caddyfile/options,
https://caddyserver.com/docs/caddyfile/directives/reverse_proxy.

Correction: graceful native reload requires admin Unix socket
`unix//run/caddy-admin.sock` in private container /run; no published TCP admin port.
`persist_config off` and `auto_https off` prevent an alternate writer/issuer.
Runtime logs go to filtered stdout (request/error/header/URI fields removed), then
through the independent runtime redaction/rotation collector. Access logs remain
off. WS upstream explicitly uses HTTP/1.1; XHTTP allows h2c/1.1 with immediate
streaming flush. A path's slash descendants apply only to XHTTP.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
