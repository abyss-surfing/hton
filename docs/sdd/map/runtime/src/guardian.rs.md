# runtime/src/guardian.rs

Stable ID: `runtime.guardian` · Phase: `R1` · Kind: source

I1 implementation binding; host integration and product acceptance remain pending. Purpose: Independent activation lease and recovery

Implementation wave: `I1-C` (library implementation present; final acceptance pending).

## Features and responsibility

- recovery

## Methods, arguments and result

- `arm` — arguments: admitted candidate, fallback target, deadline, runEpoch; result: durable LeaseId.
- `monitor` — arguments: LeaseId, state/clock/resource ports; result: commit observation, compensation, stopped-with-artifacts or recovery-required.
- `rollback_accepted` — exact previous-compatible BundleId plus current durable state; result: inverse-action restore after revalidating protected fallback.

## Structures and injected dependencies

RuntimeGuardian/Lease; independent trial supervisor

Dependencies: durable state store, monotonic clock/deadline, fixed compensation runner, verifier, log sink.

## Logic

- Runs independently of web/compiler lifecycle.
- Verify candidate routes, timeout into compatible rollback.
- No steady-state infinite restart due to transient ISP outage.
- A later stopped epoch forbids revival by an older trial lease; restore artifacts without starting traffic if stopped.
- `arm` takes the same short runtime effect gate used by fixed actions while it CAS-writes the durable trial lease, serializing lease admission with service restart.
- Restore keeps stopped intent; down suspends periodic jobs, and only explicit up/accepted start restores autostart.

## Calls / reads

- [runtime/src/probes.rs](probes.rs.md) — `monitor` -> `verify`.
- [runtime/src/state.rs](state.rs.md) — `monitor` -> `read`.
- [runtime/src/state.rs](state.rs.md) — `monitor` -> `write`.
- [runtime/src/manifest.rs](manifest.rs.md) — `monitor` -> `admit`.
- [runtime/src/actions.rs](actions.rs.md) — `monitor` -> `restore_fallback` and `compensate`.
- [runtime/src/actions.rs](actions.rs.md) — `arm` -> shared effect gate.
- [runtime-contract/src/lib.rs](../../runtime-contract/src/lib.rs.md) — `monitor` -> declarative dependency.
- [logging/src/lib.rs](../../logging/src/lib.rs.md) — `monitor` -> `emit`.
- [runtime/src/state.rs](state.rs.md) — `arm` -> `read`.
- [runtime/src/state.rs](state.rs.md) — `arm` -> `write`.

## Called / read by

- [runtime/src/main.rs](main.rs.md) — `execute` -> `arm`.
- [systemd/hton-guardian.service](../../systemd/hton-guardian.service.md) — `monitor`.
- [tests/integration/runtime-recovery.sh](../../tests/integration/runtime-recovery.sh.md) — `verify` -> `monitor`.

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

GuardianSupervisor.ensure_monitoring(instance, durableLease, context) must obtain an
acknowledgement from an independently supervised worker before activation. Failure
leaves the durable Armed lease and prevents native effects. The host/systemd worker
binding belongs to I3; an in-process management task does not satisfy this port.
Monitor only commits Verifying trials after semantic probes pass. Lease expiry or a
later stopped epoch compensates; stopped recovery restores artifacts without starts.
Explicit rollback is separate from lease-expiry recovery: it is accepted only for the
current committed lease and its exact previous-compatible fallback. Re-admit the
fallback before effects and before terminal acceptance; expired certificates or
changed credential fences leave recovery-required. A concurrent down wins by epoch.
