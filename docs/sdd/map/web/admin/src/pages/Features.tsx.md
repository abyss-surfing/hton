# web/admin/src/pages/Features.tsx

Stable ID: `web.features` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Searchable module table and batch toggles

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `toggleDraft` — arguments: moduleId, enabled; result: draft.
- `render` — arguments: catalogue, observed; result: feature view.

## Structures and injected dependencies

Typed view state and render/event functions; no domain policy owner

Dependencies: typed management API client, metadata/events, permission-filtered view props.

## Logic

- Core locked; explain dependencies and unqualified blocks.
- Render mandatory core as protected; retain disabled settings; show dependency explanation and availability independently of desired enabled and health.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [web/admin/src/lib/draft.ts](../lib/draft.ts.md) — `toggleDraft` -> `edit`.
- [web/admin/src/lib/api.ts](../lib/api.ts.md) — `toggleDraft` -> `request`.

## Called / read by

- [web/admin/src/app/App.tsx](../app/App.tsx.md) — `renderShell` -> `render`.
- [web/admin/tests/workflows.spec.ts](../../tests/workflows.spec.ts.md) — `verify` -> `toggleDraft`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
