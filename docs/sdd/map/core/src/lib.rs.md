# core/src/lib.rs

Stable ID: `core.library` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Shared domain API for helper CLI and controller

Implementation wave: `I3-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- structure

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- A replaceable implementation/build binding; preserve logical contracts.
- No live credentials, runtime arbitrary code downloads or hidden configuration authority.

## Calls / reads

- [contracts/src/operations.rs](../../contracts/src/operations.rs.md) — declarative dependency.
- [contracts/src/config.rs](../../contracts/src/config.rs.md) — declarative dependency.
- [core/src/port_allocation.rs](port_allocation.rs.md) — pure allocation policy reexport; settings persistence/recovery is not yet integrated.

## Called / read by

- [core/src/main.rs](main.rs.md) — `run` -> declarative dependency.
- [controller/src/api.rs](../../controller/src/api.rs.md) — `handle` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I1 composition binding

The library exports the implemented config/catalog/plan/journal/lock/registry/
secrets owners as they are written in I1; no CLI entry, apply or engine stub is
created to satisfy the future I3 composition. No implicit global singletons:
callers construct owners with explicit storage/clock/observation/authorization
ports. Source crate depends on data contracts, independent filesystem/logging
infrastructure, bounded YAML scanning/decoding, offline schema validation,
cryptographic hashes and OS randomness. R2/web packages remain excluded.

## I2 composition binding

Export render/apply/lifecycle/setup/selection/certificates/backup/identities/exports/
checks as their sources are written. Trusted adapters remain independently owned
source modules compiled into the Core crate with explicit path bindings; render
and lifecycle depend on narrow injected ports, not global adapter instances.
Native process drivers still belong to the I3 installed composition. Core imports
runtime-contract DTO/canonical serialization, not the runtime executor implementation.
The planned `port_allocation` module is exported here as the single policy owner for
VPN backend allocation and narrow per-service settings commit, not as a `core.plan`
helper or a generic config writer.
