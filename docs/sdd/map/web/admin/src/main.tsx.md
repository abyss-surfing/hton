# web/admin/src/main.tsx

Stable ID: `web.entry` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Static React application entry

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `mount` — arguments: documentRoot; result: application.

## Structures and injected dependencies

Typed view state and render/event functions; no domain policy owner

Dependencies: typed management API client, metadata/events, permission-filtered view props.

## Logic

- No Node SSR/runtime or remote CDN dependency.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [web/admin/src/app/App.tsx](app/App.tsx.md) — `mount` -> `renderShell`.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
