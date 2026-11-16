# runtime/src/state.rs

Stable ID: `runtime.state` · Phase: `R1` · Kind: source

I1 implementation binding; host integration and product acceptance remain pending. Purpose: Accepted runtime IDs and durable lease/fences

Implementation wave: `I1-C` (library implementation present; final acceptance pending).

## Features and responsibility

- runtime

## Methods, arguments and result

- `read` — arguments: projectId; result: RuntimeState.
- `write` — arguments: expectedState, transition; result: durable RuntimeState.

## Structures and injected dependencies

RuntimeStateStore/RuntimeState/RunIntent/RunEpoch; durable compare-and-set repository

Dependencies: protected durable work path, atomic record publisher, clock; no target YAML/parser.

## Logic

- Trial active != lastKnownGood.
- BeginRollback compares current accepted and previous-compatible IDs with the committed lease fallback, exact resource/run epoch and Running intent before CAS to RollingBack.
- Keep previous compatible bundle and required images/secret versions.
- Atomic transitions; no unconditional overwrite by failed UI.
- Persist durable run intent running/stopped and monotonic lifecycle epoch under work/runtime, not /run.
- Stopped intent takes precedence over boot/trial/rollback; accept compare-and-set transitions only.
- Restore keeps stopped intent; down suspends periodic jobs, and only explicit up/accepted start restores autostart.

## Calls / reads

- [runtime-contract/src/lib.rs](../../runtime-contract/src/lib.rs.md) — `read` -> declarative dependency.

## Called / read by

- [scripts/runtime](../../scripts/runtime.md) — `run` -> `read`.
- [core/src/checks.rs](../../core/src/checks.rs.md) — `observe` -> `read`.
- [runtime/src/main.rs](main.rs.md) — `execute` -> `read`.
- [runtime/src/main.rs](main.rs.md) — `execute` -> `write`.
- [runtime/src/manifest.rs](manifest.rs.md) — `admit` -> `read`.
- [runtime/src/guardian.rs](guardian.rs.md) — `monitor` -> `read`.
- [runtime/src/guardian.rs](guardian.rs.md) — `monitor` -> `write`.
- [runtime/src/guardian.rs](guardian.rs.md) — `arm` -> `read`.
- [runtime/src/guardian.rs](guardian.rs.md) — `arm` -> `write`.

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

RuntimeStateStore owns a private SecureDir state.json, parses bounded versioned
records, and publishes compare-and-set transitions under a bounded exclusive flock.
Initial missing state means stopped revision/epoch zero; reading it writes nothing.
A successful write fsyncs temporary bytes and parent directory through hton-storage.
Uncertain I/O publication returns recovery-required and is never treated as success
by rereading equal bytes. Stop advances epoch before effects; lease epoch stays old.
Short effect and state locks use separate directories to avoid lock inversion.


I2 restore maintenance: durable reservation blocks start/activation through the
single state owner. Reservation follows confirmed shutdown and requires stopped
intent with no active lease; if concurrent up wins CAS, reserve refuses rather
than restoring into a running server. No automatic expiry can expose partial
restores. Stop increments epoch while retaining reservation. Only the same trusted
operation may release it after complete restore verification and terminal status.

I3 status binding: RuntimeStateStore.inspect(directory) reads an existing state.json
as Option<RuntimeState> without inventing an initial instance, creating a lock or
requiring an execution context. Bounded decoding and state invariants share the same
private decoder as read/write. Missing record is None; unreadable/corrupt is typed
ErrorCode. Instance matching remains mandatory in normal state repository calls.

initial delegates canonical initial DTO construction to runtime-contract.initial_state;
state transition/admission/persistence authority remains exclusively in this owner.

Explicit rollback is a separate `RollingBack` lease state. StopIntent remains
permitted and advances the epoch; compensation may restore artifacts but cannot
start after that fence. FinishCompensation marks the lease terminal and consumes the
one-step previous-compatible pointer only for explicit rollback.

RecordOwnedRemovals requires stopped intent, no active trial/maintenance, unique
Owned container identities and exact matching stopped/disabled current receipts.
It retires those receipts only; already-retired identities are idempotent after
independent native removal verification. Epoch, accepted/fallback artifact
references and credential fences remain preserved.
