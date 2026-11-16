# schemas/registry-v1.schema.json

Stable ID: `schema.registry` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Registry durable shape contract

Implementation wave: `I1-B` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- schema

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Versioned registry stores composition facts and checkpoints, not duplicate desired enablement.
- Exact resource identity/policy fields are validated semantically by the registry and runtime.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/registry.rs](../core/src/registry.rs.md) — `read` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Concrete implementation binding (2026-10-31)

Concrete R1 registry wire shape mirrors RegistryDocument: schemaVersion=1 plus
instanceId/revision/entries/diagnostics/recoveryOriginals. Entry setupState is a
tagged state record, resources and checkpoints are arrays, membership is distinct
from setup state, and adoption/original-policy fields match management contracts.
RegistryStore supplies semantic identity/approval/resource uniqueness checks.

I2 receipt schema includes required bounded stateFingerprints, separating mutable
native state/config hashes from structural ResourceIdentity fingerprints.
