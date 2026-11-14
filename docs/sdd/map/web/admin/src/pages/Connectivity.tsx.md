# web/admin/src/pages/Connectivity.tsx

Stable ID: `web.connectivity` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Home-vantage why/failover/IPv6 view

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `render` — arguments: local evidence, policy; result: path explanation.

## Structures and injected dependencies

Typed view state and render/event functions; no domain policy owner

Dependencies: typed management API client, metadata/events, permission-filtered view props.

## Logic

- Distinguish IPv6 proxy/TUN/L3 and exit vs transport family.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [web/admin/src/lib/api.ts](../lib/api.ts.md) — `render` -> `request`.
- [web/admin/src/lib/events.ts](../lib/events.ts.md) — `render` -> `connect`.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
