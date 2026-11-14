# runtime/Cargo.toml

Stable ID: `build.runtime-manifest` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Minimal trusted runtime package, independent of web/compiler

Implementation wave: `I0-B` (ownership assignment; completion evidence is in the status ledger).

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
- Dependency closure excludes core compiler, admin, controller and R2 packages; shared wire types must remain data-only.

## Calls / reads

- [runtime-contract/Cargo.toml](../runtime-contract/Cargo.toml.md) — declarative dependency.
- [logging/Cargo.toml](../logging/Cargo.toml.md) — declarative dependency.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
