# web/admin/src/lib/api.ts

Stable ID: `web.api` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Typed requests/metadata query adapter

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `request` — arguments: operation, typed payload, semantic preconditions; result: ApiResponse or operation reference.

## Structures and injected dependencies

TypedManagementClient; transport/error mapping

Dependencies: authenticated request transport, bounded payloads, semantic preconditions.

## Logic

- Generated types/schema, bounded errors; no credential response in query cache.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [admin/src/api.rs](../../../../admin/src/api.rs.md) — `request` -> `route`.

## Called / read by

- [web/admin/src/pages/Login.tsx](../pages/Login.tsx.md) — `render` -> `request`.
- [web/admin/src/app/App.tsx](../app/App.tsx.md) — `renderShell` -> `request`.
- [web/admin/src/lib/draft.ts](draft.ts.md) — `edit` -> `request`.
- [web/admin/src/pages/Overview.tsx](../pages/Overview.tsx.md) — `render` -> `request`.
- [web/admin/src/pages/Features.tsx](../pages/Features.tsx.md) — `toggleDraft` -> `request`.
- [web/admin/src/components/PlanReview.tsx](../components/PlanReview.tsx.md) — `apply` -> `request`.
- [web/admin/src/pages/Devices.tsx](../pages/Devices.tsx.md) — `onboard` -> `request`.
- [web/admin/src/pages/Operations.tsx](../pages/Operations.tsx.md) — `render` -> `request`.
- [web/admin/src/pages/Users.tsx](../pages/Users.tsx.md) — `render` -> `request`.
- [web/admin/src/components/CredentialCreate.tsx](../components/CredentialCreate.tsx.md) — `create` -> `request`.
- [web/admin/src/pages/Publications.tsx](../pages/Publications.tsx.md) — `render` -> `request`.
- [web/admin/src/pages/Connectivity.tsx](../pages/Connectivity.tsx.md) — `render` -> `request`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
