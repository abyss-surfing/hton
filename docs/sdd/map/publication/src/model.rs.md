# publication/src/model.rs

Stable ID: `publication.model` · Phase: `R2` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: User-owned local-site routes

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- reverse proxy

## Methods, arguments and result

- `planPublication` — arguments: actor, hostOrPath, target, mode; result: publication plan.

## Structures and injected dependencies

Publication/HostPathClaim/AgentBinding; publication ownership planner

Dependencies: account grants, reserved-route inventory, allowed private targets.

## Logic

- Reserved admin/raw SNI protected; no arbitrary private target claims.
- Subdomain preferred, root explicit and path compatibility tested.

## Calls / reads

- [accounts/src/model.rs](../../accounts/src/model.rs.md) — `planPublication` -> `authorize` (R2 only).
- [core/src/plan.rs](../../core/src/plan.rs.md) — `planPublication` -> `plan`.

## Called / read by

- [admin/src/api.rs](../../admin/src/api.rs.md) — `route` -> `planPublication` (R2 only).
- [publication/src/adapter.rs](adapter.rs.md) — `configure` -> `planPublication` (R2 only).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
