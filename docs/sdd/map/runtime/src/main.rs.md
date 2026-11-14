# runtime/src/main.rs

Stable ID: `runtime.executor` · Phase: `R1` · Kind: source

Independent status binding written; native execution composition remains planned. Purpose: Small trusted accepted-bundle executor

Implementation wave: `I1-C` (planned ownership, not completion).

## Features and responsibility

- VPN availability

## Methods, arguments and result

- `execute` — reads a bounded `ExecuteRequest` (with `ExecutorAction`) from stdin and returns a typed runtime result; `boot`, `guardian`, `collect --container ID`, `status` are fixed direct commands. Lifecycle (startAccepted/stopAll/trial/commit/recovery/explicit previous-compatible rollback) is driven through `ExecutorAction` inside `execute`, never as separate top-level commands.

## Structures and injected dependencies

RuntimeExecutor/RuntimeRequest/RuntimeResult; privileged one-shot dispatcher

Dependencies: admission, runtime-state store, fixed native runner, guardian, clock and independent log sink.

## Logic

- No compiler/admin dependency on accepted boot.
- Fixed owned Docker/unit operations, immutable paths and attestations.
- Management may submit proposals but cannot bypass admission.
- Read durable run intent before boot/effects; down first fences current trial and prevents guardian or explicit rollback restart.
- Explicit associated-resource lifecycle operations are bounded by registry resource identities; never infer authority from discovery.
- `RestartAccepted` independently admits the exact planned accepted bundle/epoch, fences trial/lease/maintenance, restarts only exact Owned selected services, and probes only that service set. Unknown health evidence returns Partial with retained receipts/survivors.
- Restore keeps stopped intent; down suspends periodic jobs, and only explicit up/accepted start restores autostart.

## Calls / reads

- [runtime/src/manifest.rs](manifest.rs.md) — `execute` -> `admit`.
- [runtime/src/state.rs](state.rs.md) — `execute` -> `read`.
- [runtime/src/guardian.rs](guardian.rs.md) — `execute` -> `arm`.
- [runtime/src/state.rs](state.rs.md) — `execute` -> `write`.
- [runtime/src/actions.rs](actions.rs.md) — `execute` -> `apply`.
- [runtime/src/probes.rs](probes.rs.md) — `execute` -> `verify_selected`.
- [work/registry.json](../../work/registry.json.md) — `execute` -> declarative dependency.
- [runtime-contract/src/lib.rs](../../runtime-contract/src/lib.rs.md) — `execute` -> declarative dependency.
- [logging/src/lib.rs](../../logging/src/lib.rs.md) — `execute` -> `emit`.

## Called / read by

- [scripts/runtime](../../scripts/runtime.md) — `run` -> `execute`.
- [core/src/apply.rs](../../core/src/apply.rs.md) — `apply` -> `execute`.
- [systemd/hton-data.service](../../systemd/hton-data.service.md) — `execute`.
- [tests/integration/runtime-recovery.sh](../../tests/integration/runtime-recovery.sh.md) — `verify` -> `execute`.
- [core/src/lifecycle.rs](../../core/src/lifecycle.rs.md) — `execute` -> `execute`.
- [core/src/host/lifecycle.rs](../../core/src/host/lifecycle.rs.md) — `services.restart` -> `execute`.
- [tests/unit/runtime-contracts.rs](../../tests/unit/runtime-contracts.rs.md) — `verify` -> `execute`.

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


Executable composition: `execute` (stdin `ExecuteRequest`), `boot`, `guardian`,
`collect` and `status` are wired to `HostRuntime`, which implements `NativeAction`,
`AdmissionPorts`, `ShutdownAuthority`, `GuardianSupervisor` and `ProbePort`.
`RuntimeExecutor.execute` is implemented and exported by runtime/src/lib.rs for
trusted embedding; its state/admission/action/guardian/probe responsibilities stay
under runtime.executor, not Core or the shell dispatcher. The former top-level
start/startAccepted/stopAll/trial/commit/rollback/restart commands were removed (I4):
lifecycle operations are driven through `ExecutorAction` inside `execute`, and
systemd units call `boot`/`guardian`/`collect` directly.

## I3 read-only status binding

libexec/hton-runtime status --state-root ABSOLUTE_PATH [--json] [--quiet] [--dry-run]
uses RuntimeStateStore.inspect, no Core/compiler/config/registry dependency. Output
contains durable intent/epoch/accepted/trial/lease/maintenance metadata and recorded
resource status/timestamp only; no native locators/fingerprints or raw state bytes.
Recorded resource state is historical, not fresh health. Native observations and
bundle admissibility are unverified. Absent state is reported absent with null intent,
not synthetic stopped/successful shutdown. Unsafe/unreadable/corrupt state returns 1;
usage 2. Mutation is composed through the trusted `execute` path only.
