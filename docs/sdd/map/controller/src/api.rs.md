# controller/src/api.rs

Stable ID: `control.api` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Typed authorization and operation dispatch

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- API

## Methods, arguments and result

- `handle` — arguments: version, requestId, actor, typed payload; result: redacted response.

## Structures and injected dependencies

ApiRequest/ApiResponse/ActorContext; typed dispatch and authorization

Dependencies: token/peer authenticator, Core operations, configurator port, durable operation store, bounded events.

## Logic

- Reject unknown operations/fields, arbitrary paths/Docker calls.
- Scoped key result separate from metadata/config responses.
- R2 no replay of consumed secret bytes.
- Expose catalogue schemas/defaults and availability through bounded typed requests; delegate normalization/planning to core, never duplicate their rules.
- Return desired enabled, pending generation, accepted generation, observed health and blocked reasons independently.
- Optional full management module; CLI never calls it as a requirement.
- Atomic parameter edits delegate to configurator; immutable runtime plans still bind full candidate inputs.
- Use ActorContext/token scopes and typed operations; authorization cannot broaden locally granted foreign/privilege scope.

## Calls / reads

- [core/src/config.rs](../../core/src/config.rs.md) — `handle` -> `load`.
- [core/src/plan.rs](../../core/src/plan.rs.md) — `handle` -> `plan`.
- [core/src/apply.rs](../../core/src/apply.rs.md) — `handle` -> `apply`.
- [core/src/identities.rs](../../core/src/identities.rs.md) — `handle` -> `issue`.
- [core/src/exports.rs](../../core/src/exports.rs.md) — `handle` -> `exportProfile`.
- [core/src/checks.rs](../../core/src/checks.rs.md) — `handle` -> `observe`.
- [core/src/lib.rs](../../core/src/lib.rs.md) — `handle` -> declarative dependency.
- [configurator/src/main.rs](../../configurator/src/main.rs.md) — `handle` -> `dispatch`.

## Called / read by

- [controller/src/main.rs](main.rs.md) — `serve` -> `handle`.
- [admin/src/api.rs](../../admin/src/api.rs.md) — `route` -> `handle`.
- [admin/src/telegram.rs](../../admin/src/telegram.rs.md) — `poll` -> `handle`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
