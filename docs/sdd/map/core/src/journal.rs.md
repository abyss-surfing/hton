# core/src/journal.rs

Foreign remap and restore coordinators use separate fixed `ForeignRemap` and
`ForeignRestore` intents binding canonical request digests before effects.
Protected step-level snapshot/transition custody remains under the foreign owner;
central terminal records cannot replace exact native reconciliation evidence.

Stable ID: `core.journal` · Phase: `R1` · Kind: source

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Durable intent and compensating recovery

Implementation wave: `I1-B` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- recovery

## Methods, arguments and result

- `prepare` — arguments: EffectPlan, protected originals, OperationContext; result: TransactionId with durable intent.
- `finish` — arguments: transactionId, typed status, effect receipts; result: DurableRecord.
- `recover` — arguments: unfinished transactions, observed effects, run intent, approval scope; result: RecoveryResult or recovery-required.
- `stagePlan` — arguments: EffectPlan, expiry, OperationContext; result: Protected PlanId bound to exact inputs.
- `readPlan` — arguments: PlanId, ActorContext; result: StoredPlan or expired/notFound/forbidden.

## Structures and injected dependencies

JournalStore/Transaction/StoredPlan/EffectReceipt/ProtectedOriginal; durable intent and recovery inventory

Dependencies: fsync-capable protected storage, clock, current observed effects, safe log sink.

## Logic

- Flush before effects; reconcile effect-before-record crashes.
- Never overwrite drift or purge incomplete originals.
- Protected originals bind both byte digest and whether the artifact existed; legacy journal records decode as present, while new absent-file creates can recover without confusing absence with empty bytes.
- Read-only plan preview does not persist; explicit stagePlan owns protected plan storage/expiry/idempotent lookup.

## Calls / reads

- Runtime state/observations are supplied to reconciliation; journal does not import/read runtime state.
- [logging/src/lib.rs](../../logging/src/lib.rs.md) — `prepare` -> `emit`.

## Called / read by

- [scripts/config](../../scripts/config.md) — `run` -> `stagePlan`.
- [scripts/remap-undo](../../scripts/remap-undo.md) — `run` -> `recover`.
- [core/src/apply.rs](apply.rs.md) — `apply` -> `prepare`.
- [core/src/apply.rs](apply.rs.md) — `apply` -> `finish`.
- [core/src/apply.rs](apply.rs.md) — `apply` -> `readPlan`.
- [configurator/src/migration.rs](../../configurator/src/migration.rs.md) — `migrate` -> `prepare`.
- [adapters/foreign](../../adapters/foreign.rs.md) — `planRemap` -> `prepare`.
- [core/src/registry.rs](registry.rs.md) — `transition` -> `prepare_intent`.
- [core/src/lifecycle.rs](lifecycle.rs.md) — `execute` -> `prepare`.
- [core/src/lifecycle.rs](lifecycle.rs.md) — `execute` -> `stagePlan`.
- [configurator/src/changes.rs](../../configurator/src/changes.rs.md) — `commit` -> `prepare`.
- [core/src/selection.rs](selection.rs.md) — `prepare` -> `prepare`.
- [tests/unit/journal-contracts.rs](../../tests/unit/journal-contracts.rs.md) — `verify` -> `recover`.
- [core/src/port_allocation.rs](port_allocation.rs.md) — `commit/recover` -> durable allocation intent/receipt.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Concrete implementation binding (2026-10-31)

Concrete R1 binding: `JournalStore` receives a protected hton-storage Directory and
clock/cancellation ports. `prepare_intent` persists an immutable intent plus protected
originals before effects; `prepare` accepts EffectPlan. Completion is a separate
create-only record. `record_step` is immutable and idempotent by step identity.
`stage_plan/read_plan` persist exact actor-bound expiring plans; read never creates
storage. Recovery only classifies supplied observations as before/after/drift/unknown
and returns required reconciliation; it cannot execute or replay native effects.
Unknown/drift retains originals. All mutating methods reject readOnly/deadline/
cancellation; journal entries contain fixed typed actions and artifact references.

## Implemented ownership details

Implemented API: JournalStore::new(Arc<SecureDir>, Arc<dyn OperationControl>),
with_log_sink, prepare/prepare_intent, read, record_step, finish, stage_plan,
read_plan, unfinished, bounded `history`, and recover. `unfinished` provides a read-only inventory and
retains recoveryRequired terminal transactions for explicit reconciliation. IDs are SHA-256 of operation plus typed intent, allowing
multiple distinct transactions per operation. Pre-existing matching records are
re-published through durable CAS before returning mutation permission; only Conflict
enters idempotency comparison, and I/O uncertainty always blocks effects. Committed
Effects require every planned action's trusted Verified receipt and resource/post
fingerprint. Recovery is conservative observation-only, including stop intent fences.
SystemControl is a CLI clock without a cancellation registry; service entries inject
a control implementing their trusted cancellation source. Logging failure is ignored
for durability decisions; filesystem work is owned only by hton-storage SecureDir.

## I2 typed workflow journal

Intent adds Lifecycle{plan}, Setup{plan}, Selection{expected_revision,
replacement_digest,module_ids,enabled}, Update{plan} plus Certificates/Backup/
Restore/Identity{request_digest}. Digests bind full domain-owned protected requests;
raw paths, shell and secret values are never executable journal fields.
`Intent::PortAllocation { candidate, instance_id }` additionally stores the exact immutable
service-to-port candidate, installation identity, prior settings values, coherent
read set and reservation digest before any settings CAS. Allocation recovery consumes this journaled value
and never draws new entropy or substitutes a replacement candidate.
`history(ctx)` returns at most the bounded journal inventory with every transaction
and terminal identity-validated; owners use it for exact committed provenance, never
to infer from filenames. `terminal(id,ctx) -> Option<DurableRecord>` reads typed durable status without writes.
`finish_operation(id,status,receipts,OperationResult,ctx)` additionally stores redacted
metadata result. Existing `finish` remains for state-store transactions. Planned
forward receipts are checked for Effects/Setup/Update/Lifecycle; zero-step maintenance
intents are closed only by the trusted owning coordinator after its domain checks.
A visible but unconfirmed fsync is never idempotent success.

`lookup_intent(&Intent,ctx) -> Option<Transaction>` is read-only and shares the
journal-owned transaction ID derivation. Retry coordinators inspect its terminal
result before recapturing originals or checking already-consumed preconditions.
Known final partial/failed workflows have explicit Partial/Failed terminal status;
only uncertain/recovery-required outcomes remain pending for reconciliation.


`read_plan_metadata(id,actor,ctx)` authenticates the protected stored plan without
TTL admission; completed retries read durable status first. Any new effects still
require nonexpired read_plan/fresh input checks. TerminalStatus.Deferred specifically
records qualified pending Effects while stopped, with no native receipts; it is
successfully prepared metadata, not invented activation success.

I3 Intent.Installation binds the protected bootstrap request digest before any
config/installation seed publication. Its records share existing immutable intent,
originals, terminal OperationResult and explicit recovery; no separate ad hoc cache.
`Intent::InstallConfig { candidate }` binds the exact allowlisted site-domain and
public-edge input before root-config CAS; protected originals distinguish an
existing root document from absence for observation-based recovery.

`Intent::ModulePurge` binds one allowlisted module, original registry revision,
exact removed entry digest, retained config content revision and digest-bound local
approval. ProtectedOriginal retains exact config bytes before deletion; the host
membership owner reconciles exact-old/absent states and refuses drift/re-add.

`Intent::BundleValidation { bundle_id, candidate_digest, scope }` binds an explicit
sandbox qualification before resource creation. The fixed operation scope permits
exact-label recovery across effect-before-receipt crashes. Returned immutable IDs
are retained as private `ResourceId` keys in step fingerprints before start. Only
confirmed Docker and copied-artifact cleanup permits committed terminal status.

InstallationUninstall retains the exact original registry and retention flags
before durable shutdown. UninstallNative binds the exact stopped runtime resource
identities before removal, permitting native absence/state retirement to resume
without selecting new resources. Nested registry transitions retain exact deletion
receipts; all code/config/secrets/recovery files remain protected by default.
