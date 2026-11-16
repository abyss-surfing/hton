# etc/caddy/Caddyfile.template

Stable ID: `template.caddy` · Phase: `R1` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: Own HTTPS/HTTP/site/Naive routes

Implementation wave: `I2-A` (planned ownership, not completion).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- One TLS terminator for HTTP, protected path boundaries, supplied certs.
- Site and VPN route closure is independent of optional admin DNS/backend availability.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/render.rs](../../core/src/render.rs.md) — `compile` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

I2 serializer-owned SITE_ADDRESSES/BIND_ADDRESS/TLS/VPN_ROUTES/SITE_INDEX_DIGEST
slots. Native admin is private Unix socket for graceful reload; auto issuance/
persistent config off, H3 disabled, no admin/web branch. Ordinary site uses an explicit `route` block: VLESS handling runs first; its
fallback rewrites every remaining request to `/site-index.html`, sets root to
`/etc/hton/current`, then invokes `file_server`. The HTML is the exact separate
immutable bundle artifact, with no source webroot or host mount. The fixed rewrite
prevents request paths from addressing sibling Caddyfile/certificate artifacts.
Changed source content is picked up on the next admitted bundle. Native console logging is
filtered and requires supervised runtime redaction/rotation before persistence.
