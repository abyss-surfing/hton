# admin/src/events.rs

Stable ID: `admin.events` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Authenticated SSE forwarding

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- reactivity

## Methods, arguments and result

- `stream` — arguments: actor, lastEventId; result: redacted SSE.

## Structures and injected dependencies

Typed request/result functions defined by methods below; state stays in the owning domain record

Dependencies: explicit ports from INTERFACES; no construction of hidden global config/engine authority.

## Logic

- Bound connections/time/backoff; no per-card polling on server.
- Resync metadata after missing events.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [controller/src/events.rs](../../controller/src/events.rs.md) — `stream` -> `subscribe`.

## Called / read by

- [admin/src/main.rs](main.rs.md) — `serve` -> `stream`.
- [web/admin/src/lib/events.ts](../../web/admin/src/lib/events.ts.md) — `connect` -> `stream`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
