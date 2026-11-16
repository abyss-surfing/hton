# web/admin/src/lib/events.ts

Stable ID: `web.events` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: One client SSE stream

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `connect` — arguments: session, cursor; result: metadata invalidation.

## Structures and injected dependencies

Typed view state and render/event functions; no domain policy owner

Dependencies: typed management API client, metadata/events, permission-filtered view props.

## Logic

- Reconnect/resync with backoff; no token bytes or per-card loops.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [admin/src/events.rs](../../../../admin/src/events.rs.md) — `connect` -> `stream`.

## Called / read by

- [web/admin/src/pages/Overview.tsx](../pages/Overview.tsx.md) — `render` -> `connect`.
- [web/admin/src/components/PlanReview.tsx](../components/PlanReview.tsx.md) — `apply` -> `connect`.
- [web/admin/src/pages/Operations.tsx](../pages/Operations.tsx.md) — `render` -> `connect`.
- [web/admin/src/pages/Connectivity.tsx](../pages/Connectivity.tsx.md) — `render` -> `connect`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
