# web/admin/src/pages/Publications.tsx

Stable ID: `web.publications` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Publication owner/site/agent UX

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `render` — arguments: authorized publications; result: site routes.

## Structures and injected dependencies

Typed view state and render/event functions; no domain policy owner

Dependencies: typed management API client, metadata/events, permission-filtered view props.

## Logic

- Core route reservations, access modes, one-time agent binding.
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
