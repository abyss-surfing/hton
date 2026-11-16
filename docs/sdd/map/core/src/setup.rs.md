# core/src/setup.rs

Stable ID: `core.setup` · Phase: `R1` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Idempotent setup/resource preparation with checkpoints

Implementation wave: `I2-B` (planned ownership, not completion).

## Features and responsibility

- core

## Methods, arguments and result

- `prepare` — arguments: moduleId, RegistryEntry?, artifactInventory, suppliedInputs; result: SetupPlan with missing actions.
- `resume` — arguments: SetupPlan, OperationContext; result: SetupResult incomplete/complete with durable receipts.

## Structures and injected dependencies

SetupCoordinator/SetupPlan/StepReceipt; resumable resource preparation

Dependencies: trusted setup capability ports, protected seed writer, registry, journal/clock/log sink.

## Logic

- Execute only declared trusted setup capabilities; incomplete state survives interruption.
- Seed missing editable files only; do not overwrite customization or run an arbitrary instruction as shell.
- Newly installed module stays disabled; setup does not become an implicit updater.
- Use explicit injected storage/clock/adapter ports and typed results in INTERFACES; do not duplicate another owner's business rule.

## Calls / reads

- [core/src/registry.rs](registry.rs.md) — `prepare` -> `read`.
- [core/src/registry.rs](registry.rs.md) — `resume` -> `transition`.
- [core/src/catalog.rs](catalog.rs.md) — `prepare` -> `describe`.
- [logging/src/lib.rs](../../logging/src/lib.rs.md) — `prepare` -> `emit`.
- [adapters/resources.rs](../../adapters/resources.rs.md) — `prepare` -> `prepare`.
- [adapters/resources.rs](../../adapters/resources.rs.md) — `resume` -> `executeStep`.

## Called / read by

- [core/src/lifecycle.rs](lifecycle.rs.md) — `plan` -> `prepare`.
- [core/src/lifecycle.rs](lifecycle.rs.md) — `execute` -> `resume`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I2 concrete binding

`SetupCoordinator` owns registry, journal and trusted per-module SetupBackend
capabilities. `prepare` consumes RegistrySnapshot, read-only artifact/settings
inventory and a ModuleSetup provider; returns bounded acyclic SetupPlan only.
`resume` rechecks exact plan/registry inputs, persists typed Setup intent and a new
disabled/incomplete owned registry entry before external steps, then reconciles
each checkpoint through SetupBackend.observe_step before bounded execute_step.
Verified receipts are durable before subsequent effects. Unknown/uncertain effects
block replay and remain incomplete. Registry SetupProgress only appends verified
resources/versions backed by fresh stopped observations; Complete requires every
required step verified and all missing/operator actions resolved. New modules are
disabled via the selection owner; repeat add never disables an existing member.
Associated modules are registered only after exact qualified association evidence.

SetupBackend extends trusted ModuleSetup with observe_step and entry/precondition
metadata; ResourcePort implements actual fixed acquisition/seed checks in I3.
No setup path starts listeners or updates an existing pinned image version.
