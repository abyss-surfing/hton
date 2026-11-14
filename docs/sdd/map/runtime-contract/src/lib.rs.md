# runtime-contract/src/lib.rs

Stable ID: `runtime-contract.wire` · Phase: `R1` · Kind: contract

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Independently buildable runtime bundle/state/lease wire contract

Implementation wave: `I0-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- runtime-contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Bounded versioned DTOs from INTERFACES section 6 only; no compiler/configurator/admin dependency.
- Run intent/epoch and exact resource identity are part of admission and recovery; no shell commands.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [runtime/src/main.rs](../../runtime/src/main.rs.md) — `execute` -> declarative dependency.
- [runtime/src/manifest.rs](../../runtime/src/manifest.rs.md) — `admit` -> declarative dependency.
- [runtime/src/state.rs](../../runtime/src/state.rs.md) — `read` -> declarative dependency.
- [runtime/src/guardian.rs](../../runtime/src/guardian.rs.md) — `monitor` -> declarative dependency.
- [runtime/src/probes.rs](../../runtime/src/probes.rs.md) — `verify` -> declarative dependency.
- [runtime/src/actions.rs](../../runtime/src/actions.rs.md) — `apply` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Concrete I0 Rust binding (2026-10-31)

`hton-runtime-contract` is a data-only crate depending on serde/serde_json, with no
management imports. Version 1 uses camelCase fields and tagged snake_case variants;
unknown fields and unsupported versions are rejected. Logical IDs, SHA-256 digests,
image pins, safe relative bundle artifact locations and bounded collections validate
at construction and deserialization. The outer JSON decoder enforces a 4 MiB input
limit; collections cap at 4096 members and strings cap at their declared size.
Numeric revision/epoch/timestamps are unsigned integers (timestamps Unix milliseconds).

CandidateBundle carries artifacts, services, claims, scoped secrets, fixed actions,
required native evidence and optional staging evidence. AdmissionProof is evidence,
never authority: runtime must authenticate its origin, re-observe fingerprints and
validate its approvals. There is deliberately no deserializable `AdmittedBundle`;
its private constructor and protected handles belong to runtime manifest admission.
Wire errors expose fixed codes and structured IDs only, avoiding backend stderr or
arbitrary secret-bearing error strings. These DTO checks enforce structural bounds;
the independent runtime still owns semantic/cryptographic/resource checks.

State includes durable stopped/running intent, monotonically increasing epoch and
revision, accepted/previous/trial references, lease/fallback, credential fences and
resource receipts. Typed requests cannot supply shell commands, Docker argv, absolute
paths or arbitrary environment. Rollback explicitly distinguishes accepted bundle
from pre-install closed listeners. Action receipts and aggregate results expose
partial/recovery-required outcomes. Probe evidence binds expected artifact/backend
and auth/route policy; an open port does not establish qualification.

`ExecutorAction::RollbackAccepted { bundle_id }` names only the durable
previous-compatible target. `StateTransition::BeginRollback` CAS-checks that target
against the committed lease fallback, current accepted ID, previous-compatible ID,
run intent and exact epoch, then enters `LeaseState::RollingBack`. This action carries
no client-generated manifest digest or inverse list; runtime resolves those from
protected authority. A successful explicit restore clears the consumed one-step
previous-compatible pointer. RollingBack is active for stop fencing and admission.

The 4 MiB decoder bounds input bytes, not total heap use. Deadline, action timeout,
concurrency, artifact size and observation byte/count upper limits are trusted
runtime admission/operation policy (wire numbers are not an approval to spend that
budget); zero concurrency/count/bytes/timeouts are rejected structurally. The runtime
must narrow supplied values before allocating, reading artifacts, spawning native
processes or probing. StateSchemaVersion and ContractVersion are separate types.
Runtime logical IDs intentionally do not import management IDs: Core performs an
explicit checked value conversion at its runtime port; there is no implicit cast
or management dependency in the runtime wire package.

I1 action fingerprint binding: before/after include the relevant mutable native
settings/status; identityDigest identifies the concrete native object independently.
For Owned Recreate, after binds an admitted launch specification and expected status,
not an unknowable future Docker ID. Only the runtime's trusted native port may return
the newly created exact identity after independently verifying that specification;
the protected ResourceReceipt becomes its durable binding. Associated recreation
is prohibited. Ordinary Start/Stop cannot adopt replacement identities.

## I2 serialization completion

`hash(bytes)` and `candidate_digest(CandidateBundle)` own canonical SHA-256 wire
encoding, previously local to runtime manifest. They exclude manifestDigest and
its attached nativeEvidence/staging attestations from the self-fingerprint; the
protected manifest file has its own exact-byte digest for admission proof. Object
keys sort recursively and ordered arrays remain ordered. Both compiler and runtime
import this one implementation; no management/business/effect dependency is added.
Engine::Unbound explicitly represents the private native recursive DNS service.


## I2 retirement and restore reservations

CandidateBundle.retired_services holds RetiredService {service_id,module_id,
resource,restart_owner_resources}: exact owned resources retired by this change,
without active native artifacts/images. Active and retired IDs/resources are
separate. Retirement forwards only Stop/DisableAutostart; declared inverses may
Start/EnableAutostart under admitted fallback authority. Accepted cold boot and
semantic probes iterate active services only, never resurrect retired services.

RuntimeState.maintenance is an optional durable MaintenanceReservation
{reservation_id,operation_id,run_epoch}. ReserveMaintenance is allowed only from
stopped, verified stopped/disabled receipts, no active trial and no reservation;
it advances epoch and blocks all StartIntent/ArmTrial/accepted startup/promotion.
ReleaseMaintenance requires exact reservation and operation, stopped state and
matching epoch, after trusted restore verification and durable terminal publication.
It never expires automatically. Down remains permitted and preserves the reservation.

I3 protected host authority wire: NativeLocator binds full concrete container/unit
or scoped service artifact identity. AuthorizedResource binds runtime identity,
structural fingerprint, service, dependency shutdown graph and approved restart
policy. RuntimeAuthority binds instance/revision/epoch/bundle ID+digest, exact
resource inventory, approvals, independent native/staging evidence, fallback/
secret/claims grants. Protected per-bundle records live under runtime/authorities;
current authority record supports shutdown. Deserializing these proposals grants
no native authority: host independently validates literals, ownership, exact launch,
evidence and current fences. No raw secret material or free-form command/path field.

initial_state(instance) is the canonical pure initial stopped DTO construction,
shared by runtime.state.initial and Core's absent-state planning snapshot. It
creates no record and makes no claim that associated native resources are stopped.

`ExecutorAction::RestartAccepted { bundle_id, expected_run_epoch, service_ids }`
requires an exact accepted bundle, planned epoch and nonempty catalogue service list;
Core resolves omitted CLI selectors from the saved enabled optional/data-service set.
Runtime rejects empty selection/stale bundle or epoch, re-admits the bundle and requires Running
intent/current epoch and no trial, lease or maintenance, rejects missing or
Associated services, and emits fixed Restart actions for exact bundle identities.
Unknown semantic probe evidence for selected services returns Partial while
preserving action receipts and survivor IDs; unrelated accepted members are not probed.

`UninstallOwned` carries a bounded exact resource-identity set, never locators or
native arguments. It is distinct from lifecycle stop and admitted activation.
`ActionOutcome::Removed` records confirmed exact native absence. The independent
executor accepts only protected shutdown-authority Owned containers while durable
intent is stopped; native removal requires its own protected before-effect record.
`RecordOwnedRemovals` retires only exact Owned stopped receipts and preserves the
stopped epoch and retained accepted/fallback artifact references.
