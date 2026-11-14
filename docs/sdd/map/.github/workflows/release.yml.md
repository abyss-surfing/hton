# .github/workflows/release.yml

Stable ID: `ci.release` · Phase: `R1` · Kind: configuration

I3-B build binding written; execution/acceptance evidence is separate. Purpose: Pinned artifact/image release and provenance

Implementation wave: `I3-B` (planned ownership, not completion).

## Features and responsibility

- build

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- No generated false digest; signed/verified artifacts before installation.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


## I3-B concrete binding (2026-11-01)

Manual workflow first requires the D81 completed-plan marker. It invokes the
single tooling/package-release owner with exact external Rust 1.99.1/musl tools,
locked dependencies and pinned official actions. Candidate archives/checksums and
per-file provenance upload as workflow artifacts only; no publication/deployment.
Unsigned candidates are not authenticated release or acceptance evidence.
