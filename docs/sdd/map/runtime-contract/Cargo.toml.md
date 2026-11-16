# runtime-contract/Cargo.toml

Stable ID: `build.runtime-contract-manifest` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Independent pure runtime wire package

Implementation wave: `I0-B` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- build

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Build independently without management contract package or compiler.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [runtime/Cargo.toml](../runtime/Cargo.toml.md) — declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Concrete I0 binding

Library package `hton-runtime-contract`, Rust edition 2024, workspace serde (derive)
and serde_json dependencies only; no host, Docker, framework or management-service dependencies.

## I2 dependency completion

Add sha2 for the canonical wire fingerprint/serialization helper; this is pure
encoding, not business policy, validation authority or host I/O. The I0 dependency
paragraph records its earlier state. Runtime remains independently buildable.
