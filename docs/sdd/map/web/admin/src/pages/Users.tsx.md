# web/admin/src/pages/Users.tsx

Stable ID: `web.users` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Own dashboard/admin metadata hierarchy

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `render` — arguments: actor, users, profiles, credentials; result: authorized table.

## Structures and injected dependencies

Typed view state and render/event functions; no domain policy owner

Dependencies: typed management API client, metadata/events, permission-filtered view props.

## Logic

- Users see own IDs; admin sees all metadata but no historical secret.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [web/admin/src/lib/api.ts](../lib/api.ts.md) — `render` -> `request`.
- [web/admin/src/components/CredentialCreate.tsx](../components/CredentialCreate.tsx.md) — `render` -> `create` (R2 only).

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
