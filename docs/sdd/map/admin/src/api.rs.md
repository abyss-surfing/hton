# admin/src/api.rs

Stable ID: `admin.api` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Same-origin frontend API adapter

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- web

## Methods, arguments and result

- `route` — arguments: authenticated request; result: bounded response.

## Structures and injected dependencies

AuthenticatedHttpRequest; same-origin request adapter

Dependencies: web session/CSRF checker, typed management API client; no direct Core/config/Docker writes.

## Logic

- Web cannot write config directly or fabricate local approval.
- Translate typed controller results; do not expose raw store/runtime files.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [admin/src/auth.rs](auth.rs.md) — `route` -> `authenticate`.
- [controller/src/api.rs](../../controller/src/api.rs.md) — `route` -> `handle`.
- [accounts/src/issuance.rs](../../accounts/src/issuance.rs.md) — `route` -> `create` (R2 only).
- [accounts/src/model.rs](../../accounts/src/model.rs.md) — `route` -> `authorize` (R2 only).
- [publication/src/model.rs](../../publication/src/model.rs.md) — `route` -> `planPublication` (R2 only).

## Called / read by

- [admin/src/main.rs](main.rs.md) — `serve` -> `route`.
- [web/admin/src/lib/api.ts](../../web/admin/src/lib/api.ts.md) — `request` -> `route`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
