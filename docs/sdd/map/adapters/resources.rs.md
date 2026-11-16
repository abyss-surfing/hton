# adapters/resources.rs

Stable ID: `adapter.resources` · Phase: `R1` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Declared resource acquisition and seed-only setup effects

Implementation wave: `I2-B` (planned ownership, not completion).

## Features and responsibility

- adapter

## Methods, arguments and result

- `prepare` — arguments: ModuleDescriptor, inventory, suppliedInputs; result: SetupStep list with exact resources and missing inputs.
- `executeStep` — arguments: approved SetupStep, OperationContext; result: Durable StepReceipt or structured incomplete/error.

## Structures and injected dependencies

SetupStep/StepReceipt; fixed image/template resource operations

Dependencies: pinned artifact fetch port, seed-only config writer, resource observations, safe log sink.

## Logic

- Fixed image fetch/template seed operations, protected config handles and trusted descriptors only.
- Never overwrite existing edited files; no arbitrary image/shell supplied by config/API.
- Reconcile existing artifacts before retry; resources may be obtained while server stopped without starting listeners.

## Calls / reads

- [logging/src/lib.rs](../logging/src/lib.rs.md) — `executeStep` -> `emit`.

## Called / read by

- [core/src/setup.rs](../core/src/setup.rs.md) — `prepare` -> `prepare`.
- [core/src/setup.rs](../core/src/setup.rs.md) — `resume` -> `executeStep`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I2 concrete binding

`ResourceAdapter` implements ModuleSetup plus SetupBackend over an immutable trusted
ResourceRecipe. Recipe binds module ID, exact ServiceId->ImageDigest pins, known
config ObjectId->template ArtifactId, stopped owned resource IDs, and compatibility
capability. `ResourcePort` exposes typed observe/obtain-image/seed-missing/prepare-
stopped-resource/compatibility/operator-action operations only. No shell, endpoint,
image name, path or executable comes from user YAML. Each step has deterministic
idempotency key/dependency/timeout; observe reconciles existing content/resources
before execution, seed never overwrites an existing customization. Unknown effects
return Uncertain and must be reconciled, not re-executed. Native host binding I3
must enforce exact pins, grants, stopped state and context deadlines.
