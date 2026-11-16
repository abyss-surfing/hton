# web/admin/src/components/CredentialCreate.tsx

Stable ID: `web.credentials` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Single-disclosure creation drawer

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `create` — arguments: profiles, editable date-default name; optional family only with far-future capability; result: ephemeral reveal.

## Structures and injected dependencies

Typed view state and render/event functions; no domain policy owner

Dependencies: typed management API client, metadata/events, permission-filtered view props.

## Logic

- Defaults from config; cache/storage/log-free secret response.
- Close/refresh destroys reveal; lost response -> revoke/recreate.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [web/admin/src/lib/api.ts](../lib/api.ts.md) — `create` -> `request`.

## Called / read by

- [web/admin/src/pages/Users.tsx](../pages/Users.tsx.md) — `render` -> `create` (R2 only).
- [web/admin/src/pages/Publications.tsx](../pages/Publications.tsx.md) — `render` -> `create` (R2 only).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
