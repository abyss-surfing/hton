# scripts/validate

Stable ID: `cmd.validate` · Phase: `R1` · Kind: source

Shell/Core request contract bound; dispatcher calls the concrete protected Core runner. Purpose: Test final rendered bundle in isolated Docker

Implementation wave: `I3-A` (planned ownership, not completion).

## Features and responsibility

- CLI
- safety

## Methods, arguments and result

- `run` — arguments: exact BundleId, --json; result: exit code and redacted `BundleValidationAttestation` summary or structured error.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- Explicit mutating qualification command, never part of dry-run or ordinary apply.
- Core resolves a protected candidate and binds the result to exact artifacts/images and generated private test mappings.
- Show only bounded scenario outcomes, cleanup/recovery status and explicit external/device unverified fields.

## Calls / reads

- [core/src/host/bundle_validation.rs](../core/src/host/bundle_validation.rs.md) — fixed `execute`; validation/run-bundle is another wrapper of this same operation.
- [lib/hton](../lib/hton.md) — `run` -> `invokeHelper`.

## Called / read by

- [bin/hton](../bin/hton.md) — `dispatch` -> `run`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
