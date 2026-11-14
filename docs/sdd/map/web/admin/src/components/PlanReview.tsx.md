# web/admin/src/components/PlanReview.tsx

Stable ID: `web.plan` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Preview, apply and recovery progress

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- UX

## Methods, arguments and result

- `apply` — arguments: planId, expectedInputs; result: OperationView.

## Structures and injected dependencies

PlanView/OperationView; presentation of exact prepared effect plan

Dependencies: typed API client, protected planId, metadata events; no local planning rules.

## Logic

- No optimistic ready status; explain scope/interruption/local approvals.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [web/admin/src/lib/api.ts](../lib/api.ts.md) — `apply` -> `request`.
- [web/admin/src/lib/events.ts](../lib/events.ts.md) — `apply` -> `connect`.

## Called / read by

- [web/admin/src/pages/Configuration.tsx](../pages/Configuration.tsx.md) — `render` -> `apply`.
- [web/admin/tests/workflows.spec.ts](../../tests/workflows.spec.ts.md) — `verify` -> `apply`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
