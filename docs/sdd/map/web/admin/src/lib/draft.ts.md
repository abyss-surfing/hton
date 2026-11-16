# web/admin/src/lib/draft.ts

Stable ID: `web.draft` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Shared schema-driven draft/revision state

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `edit` — arguments: DraftState, ParameterChanges, baselines; result: ProtectedDraftState.
- `preview` — arguments: draftId, semantic preconditions; result: PlanView or retained ConflictDraft.

## Structures and injected dependencies

ParameterDraft/Baseline/ConflictView; client-only prepared edits

Dependencies: typed API client, form metadata; no persisted reusable secret or replacement file payload.

## Logic

- Form and YAML share one normalized draft; stale manual edits require rebase.
- UI forms and YAML use the same schema and server normalization; presentation hints never change defaults or validation. Batch toggle edits into one revision-bound plan.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [web/admin/src/lib/api.ts](api.ts.md) — `edit` -> `request`.

## Called / read by

- [web/admin/src/pages/Features.tsx](../pages/Features.tsx.md) — `toggleDraft` -> `edit`.
- [web/admin/src/pages/Configuration.tsx](../pages/Configuration.tsx.md) — `render` -> `edit`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
