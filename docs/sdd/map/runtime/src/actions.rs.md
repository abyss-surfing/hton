# runtime/src/actions.rs

Stable ID: `runtime.actions` · Phase: `R1` · Kind: source

I1 implementation binding; host integration and product acceptance remain pending. Purpose: Fixed admitted native Docker/runtime actions

Implementation wave: `I1-C` (library implementation present; final acceptance pending).

## Features and responsibility

- safety

## Methods, arguments and result

- `apply` — arguments: AdmittedBundle, fixed actions, run intent/epoch, OperationContext; result: NativeEffectReceipts.
- `restore_fallback` — arguments: compatible fallback, current run intent/epoch, OperationContext; result: CompensationResult without revival after down.

## Structures and injected dependencies

FixedNativeRunner/NativeReceipt; admitted native operations

Dependencies: pinned Docker/systemd process ports, fixed action allowlist, bounded timeout/output, resource identity checks.

## Logic

- No compiler/web import, shell strings or arbitrary Docker methods.
- Short-held runtime lock and durable lease fencing serialize native effects.
- Bound command timeouts; guardian can recover without a management-held lock.
- Apply fixed stop/autostart operations to exact registered associated resources with timeouts; continue after member failures.
- Restart only the explicit nonempty service selection from the current admitted accepted bundle; reject Associated/missing services, enforce exact epoch/identity under the effect gate, and record each receipt before aggregate health reporting.
- Recheck trial/active lease, accepted bundle and epoch under the effect gate; `guardian.arm` uses the same gate while admitting a trial.
- Restart checks post-action Running status and admitted launch identity; semantic probe gaps remain Partial and do not erase successful action receipts.
- Structural launch drift blocks Restart; it cannot silently become Recreate or artifact installation.
- Never change foreign config/cert ownership implicitly; restore original restart policy only for explicit detach/undo.
- Restore keeps stopped intent; down suspends periodic jobs, and only explicit up/accepted start restores autostart.

## Calls / reads

- [runtime-contract/src/lib.rs](../../runtime-contract/src/lib.rs.md) — `apply` -> declarative dependency.

## Called / read by

- [runtime/src/main.rs](main.rs.md) — `execute` -> `apply`.
- [runtime/src/guardian.rs](guardian.rs.md) — `monitor` -> `restore`.

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

I1 action reconciliation: each candidate forward action declares distinct exact
before/after fingerprints and one inverse on the same logical resource/service.
Fingerprints cover relevant settings/status; native identity_digest identifies the
concrete native object separately. Forward/inverse graphs cannot overlap or share
an inverse. Admission rejects missing inverses, unsupported topology and associated
resource recreation. ActionExecutor observes exact identity and before fingerprint
under the effect lock, performs the bounded fixed action, independently checks its
receipt/after/deadline, then durably records the actual resource receipt.

Compensation orders forwards topologically and reverses that order. Observed-before
means no effect and is skipped; observed-after is eligible for its declared inverse;
unexpected fingerprints require recovery instead of blind undo. Non-cleanup inverses
use fully admitted fallback artifacts, never failed candidate artifacts. A later stop
permits Stop/DisableAutostart/RestoreArtifacts only. Native matches_admitted verifies
accepted native configuration before idempotent start; already running healthy
services and enabled restart policies return Unchanged. Failed dependencies prevent
their dependents from starting, while independent services are still attempted.

Owned Recreate independently observes the old exact native identity before execution.
The approved after fingerprint binds the independently derived admitted launch spec
and expected status. Changed native identity is accepted only for Owned Recreate,
with the same logical resource ID/kind/ownership, exact after fingerprint and
receipt.effectiveBundle equal to the admitted target. The native port verifies the
created object's launch descriptor, retains its concrete identity in the receipt,
and fails before effects when that fingerprint cannot be derived. Other actions
cannot silently adopt a different native identity. Subsequent actions prefer that
protected actual binding. Associated resources cannot use Recreate.

DirectoryEffectGate uses a separate SecureDir flock with a bounded monotonic wait.
No config lock is held. Every stop/autostart attempt and final observation has its
own narrowed deadline; expired/cancelled attempts become explicit failures without
native invocation. Final stopped/unknown observations are durably retained; only
verified stopped resources with disabled restart owners count as off. Log events
include safe operation/service/resource correlation and closed failure reasons.

`uninstall_owned` is a separate full-maintenance operation. It does not reuse
StopAll as scoped removal. It validates exact Owned container identities against
protected shutdown authority under the effect gate, requires stopped intent and
no active trial/maintenance, and delegates only bounded stopped-owned deletion.
Successful exact absence yields a Removed action receipt and retires that runtime
resource receipt; failures retain survivor identities and continue other targets.
