# Cargo.toml

Stable ID: `build.workspace` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Current structured-helper workspace binding

Implementation wave: `I0-B` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- build

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Dependency separation keeps trusted runtime executor independent of compiler/web.
- R1 build excludes R2 account/publication/agent implementation dependencies; architecture boundaries are checked before release.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Concrete I0 build binding (D79)

The initial workspace contains only implemented `hton-contracts` and
`hton-runtime-contract` libraries. Add Core/runtime/logging members when their
owned source packages exist; do not create empty implementations to satisfy a
future manifest. Use Rust edition 2024, serde derive and serde_json with the exact
resolved dependency graph committed in Cargo.lock. Runtime wire has no dependency
on management contracts. Later management/R2 packages stay outside this closure.
`cargo check --workspace --locked` is a compilation check, not U0/F0 acceptance.
Toolchains may be provisioned in a temporary build directory; this is not a
runtime dependency or installation on the managed VPS.

## I1 execution closure

As the owned sources are implemented, workspace members expand to Core, runtime,
logging and fs-support (package hton-storage). Pure mechanics, DTOs and domains
remain separate crates; runtime still excludes management contracts/Core. The
initial two-member I0 closure above describes its earlier state, not the final
R1 build. No empty future crate is created simply to make Cargo accept a manifest.

I3 adds independent hton-host (host-support) mechanics/typed observations. No domain dependency is introduced into runtime; compilation is deferred by D81.

D81 overrides older intermediate compile permission: package graph and source may
be edited now, but no cargo check/build until the full selected plans are ready.
