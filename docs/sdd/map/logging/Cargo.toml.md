# logging/Cargo.toml

Stable ID: `build.logging-manifest` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Independent bounded logging infrastructure package

Implementation wave: `I0-B` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- build

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- No Core/admin/compiler dependencies; SafeFields/redaction/sink/rotation contract shared by management and runtime.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/Cargo.toml](../core/Cargo.toml.md) — declarative dependency.
- [runtime/Cargo.toml](../runtime/Cargo.toml.md) — declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Concrete implementation binding (2026-10-31)

Concrete R1 dependency binding: serde/serde_json plus hton-storage only; no
hton-contracts, Core, runtime-contract, configurator or API dependency.
