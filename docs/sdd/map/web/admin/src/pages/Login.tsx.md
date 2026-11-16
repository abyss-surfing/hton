# web/admin/src/pages/Login.tsx

Stable ID: `web.login` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Local bootstrap and authenticated owner login UI

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX
- auth

## Methods, arguments and result

- `render` — arguments: auth state, local bootstrap context; result: login form.

## Structures and injected dependencies

Typed view state and render/event functions; no domain policy owner

Dependencies: typed management API client, metadata/events, permission-filtered view props.

## Logic

- No public unauthenticated setup or raw secret caching.
- Explicit expired/session/error states and keyboard access.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [web/admin/src/lib/api.ts](../lib/api.ts.md) — `render` -> `request`.

## Called / read by

- [web/admin/src/app/App.tsx](../app/App.tsx.md) — `renderShell` -> `render`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
