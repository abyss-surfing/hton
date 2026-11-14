# work/registry.json

Stable ID: `state.registry` · Phase: `R1` · Kind: runtime-artifact

Planned file binding; no implemented functionality is claimed. Purpose: Durable installed/associated composition and setup checkpoints

Implementation wave: `I1-B` (planned ownership, not completion).

## Features and responsibility

- state

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Generated Git-ignored working artifact, never shipped secret data.
- Stable module IDs, ownership, exact resource identities, setup checkpoints, pinned images and original restart policies.
- Does not duplicate desired enabled flags; unknown/changed external identities require reconciliation.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [runtime/src/main.rs](../runtime/src/main.rs.md) — `execute` -> declarative dependency.
- [core/src/registry.rs](../core/src/registry.rs.md) — `read` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
