# admin/src/main.rs

Stable ID: `admin.entry` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Unprivileged web task/bootstrap entry

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- web

## Methods, arguments and result

- `serve` — arguments: internal bind, settings; result: HTTP service.

## Structures and injected dependencies

Typed request/result functions defined by methods below; state stays in the owning domain record

Dependencies: explicit ports from INTERFACES; no construction of hidden global config/engine authority.

## Logic

- Static frontend and authenticated API only.
- Optional Telegram independent; no Docker socket.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [admin/src/auth.rs](auth.rs.md) — `serve` -> `authenticate`.
- [admin/src/api.rs](api.rs.md) — `serve` -> `route`.
- [admin/src/events.rs](events.rs.md) — `serve` -> `stream`.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
