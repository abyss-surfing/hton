# runtime/src/probes.rs

Stable ID: `runtime.probes` · Phase: `R1` · Kind: source

I1 implementation binding; host integration and product acceptance remain pending. Purpose: Independent required-route checks

Implementation wave: `I1-C` (library implementation present; final acceptance pending).

## Features and responsibility

- acceptance

## Methods, arguments and result

- `verify` — arguments: bundle, checkBudget; result: verified evidence.

## Structures and injected dependencies

RuntimeVerification/Evidence; post-activation identity/policy checks

Dependencies: admitted bundle claims, bounded protocol/native observation ports, clock.

## Logic

- Distinguish candidate from still-running prior backend.
- Check actual auth/protocol where required, not only a listener.
- `verify_selected` accepts a nonempty exact service set for scoped restart checks; unsupported probe kinds remain blocking, and unrelated accepted members do not affect that result.

## Calls / reads

- [runtime-contract/src/lib.rs](../../runtime-contract/src/lib.rs.md) — `verify` -> declarative dependency.

## Called / read by

- [runtime/src/guardian.rs](guardian.rs.md) — `monitor` -> `verify`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


## I1-C concrete binding

Implemented as an independent `hton-runtime` library; binary composition follows I3.
All mutable host authority enters through injected trusted ports. Wire proposals
never manufacture AdmittedBundle, which has private fields and no Deserialize.
Native operations and observation ports enforce supplied absolute deadlines; the
coordinator narrows per-action deadlines and rechecks durable epoch under the
short effect lock. Port failures remain typed failures, never synthetic success.
No Core, management contract, YAML, network download or shell expression dependency.

I3 recreation-aware probe identity compares the actual admitted resource map,
including independently verified native replacement receipts; the immutable original
Service.resource is not falsely reported as the newly observed concrete ID.
Unchanged service generations qualify by canonical native+launch revision and exact
files/launch, retaining honest historical effective bundle instead of forced restarts.
