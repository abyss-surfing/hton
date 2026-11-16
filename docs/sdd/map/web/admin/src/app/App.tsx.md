# web/admin/src/app/App.tsx

Stable ID: `web.shell` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Authenticated role-aware navigation shell

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `renderShell` — arguments: session, route; result: layout.

## Structures and injected dependencies

Typed view state and render/event functions; no domain policy owner

Dependencies: typed management API client, metadata/events, permission-filtered view props.

## Logic

- Core owner routes first; future accounts/publications phase-gated.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [web/admin/src/lib/api.ts](../lib/api.ts.md) — `renderShell` -> `request`.
- [web/admin/src/pages/Overview.tsx](../pages/Overview.tsx.md) — `renderShell` -> `render`.
- [web/admin/src/pages/Features.tsx](../pages/Features.tsx.md) — `renderShell` -> `render`.
- [web/admin/src/pages/Configuration.tsx](../pages/Configuration.tsx.md) — `renderShell` -> `render`.
- [web/admin/src/pages/Devices.tsx](../pages/Devices.tsx.md) — `renderShell` -> `onboard`.
- [web/admin/src/pages/Operations.tsx](../pages/Operations.tsx.md) — `renderShell` -> `render`.
- [web/admin/src/styles/theme.css](../styles/theme.css.md) — `renderShell` -> declarative dependency.
- [web/admin/src/pages/Login.tsx](../pages/Login.tsx.md) — `renderShell` -> `render`.

## Called / read by

- [web/admin/src/main.tsx](../main.tsx.md) — `mount` -> `renderShell`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
