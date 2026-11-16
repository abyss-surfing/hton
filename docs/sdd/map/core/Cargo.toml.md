# core/Cargo.toml

Stable ID: `build.core-manifest` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Current structured-helper library/CLI package dependencies

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

## Calls / reads

- [contracts/Cargo.toml](../contracts/Cargo.toml.md) — declarative dependency.
- [logging/Cargo.toml](../logging/Cargo.toml.md) — declarative dependency.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I1 concrete dependencies

Package `hton-core` uses workspace hton-contracts, hton-storage, hton-logging,
serde/serde_json, sha2, getrandom, serde_yaml, yaml-rust2 (scanner only) and
jsonschema with default features disabled. External schema fetching is not
permitted. Scanner rejects aliases/anchors/tags before YAML decoding. No API/web/
configurator or runtime compiler dependency is introduced. Exact versions are
recorded in Cargo.lock; target Rust edition remains 2024.


I3-A: Cargo's core/src/main.rs target builds the hton-core helper binary alongside
the library. Release packaging installs that externally built artifact as
/opt/hton/libexec/hton-core. No host toolchain is called by shell invokeHelper.

I3-A local inventory additionally uses workspace rustix.process for effective UID
metadata; it adds no native mutation or remote authentication authority.
