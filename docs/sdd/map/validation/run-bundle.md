# validation/run-bundle

Stable ID: `validation.bundle` · Phase: `R1` · Kind: source

Source wrapper delegates to the fixed Core coordinator. Purpose: final-output Docker qualification.

Implementation wave: `I3-B`; source written, build and final acceptance pending.

## Features and responsibility

- Docker tests

## Methods, arguments and result

- `testBundle` — arguments: protected exact `BundleId`, canonical candidate/artifact/image digests, fixed test mapping, bounded Budget and `OperationContext`; result: protected `BundleValidationAttestation` with per-scenario evidence and exact cleanup receipt, or typed failure/recovery-required.

## Structures and injected dependencies

Typed request/result functions defined by methods below; state stays in the owning domain record

Dependencies: protected BundleStore reader, trusted candidate digest/hash verifier, fixed pinned-image native validator and isolated semantic-test Docker owner, operation journal/cleanup recovery. No hidden global config/engine authority.

## Logic

- Native validation then actual pinned server/client containers.
- Verify exact candidate and staged artifact hashes and immutable cached image digests before effects.
- Generate private test mappings from candidate listener claims; reject public host binds, foreign mounts, client Compose, arbitrary images or commands.
- Journal exact created IDs before start; cleanup only this operation's containers/network/project. Uncertain cleanup blocks attestation and requires recovery.
- Bind attestation to exact BundleId, candidate/artifact/image digests, runner version and results; report unverified external/device claims.

## Calls / reads

- [core/src/host/bundle_validation.rs](../core/src/host/bundle_validation.rs.md) — fixed Core qualification owner.
- [host-support/src/validation.rs](../host-support/src/validation.rs.md) — fixed scoped Docker driver.

Topology is generated from exact candidate endpoints by the fixed Rust owner;
there is no caller-provided or separately executed Compose template.

## Called / read by

- [scripts/validate](../scripts/validate.md) — `run` -> `testBundle`.
- [core/src/apply.rs](../core/src/apply.rs.md) — `apply` -> consumes only fresh exact successful attestation; normal apply does not implicitly run semantic tests.
- [.github/workflows/ci.yml](../.github/workflows/ci.yml.md) — `testBundle`.
- [tests/integration/protocol-matrix.sh](../tests/integration/protocol-matrix.sh.md) — `verify` -> `testBundle`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
