# core/src/registry.rs

`uninstalled_owned_projection(entry)` is the shared pure retained-membership
projection used after independently verified native removal and by uninstall
resume checks. It grants no deletion authority. `recover_uninstall` accepts only
the exact nested Registry::UninstallOwned intent and protected original registry;
it reconciles exact-before/exact-after revisions and refuses all other states.

Stable ID: `core.registry` · Phase: `R1` · Kind: source

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Registry invariants, explicit adoption and resumable setup metadata

Implementation wave: `I1-B` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- core

## Methods, arguments and result

- `read` — arguments: instanceId; result: RegistrySnapshot with revision or diagnostics.
- `transition` — arguments: expectedRevision, RegistryChange, OperationContext; result: DurableRegistryResult.

## Structures and injected dependencies

RegistryStore/RegistrySnapshot/RegistryChange; installed/adoption state repository

Dependencies: protected working directory, compare-and-set publication, journal, clock.

## Logic

- Explicit adoption only; discovery does not transfer authority.
- Compare revision and exact resource fingerprints before effects; protect original foreign policies.
- Persist incomplete add checkpoints; re-add resumes without overwriting retained data.
- Source code, editable config and runtime working state have distinct access policies.
- Use explicit injected storage/clock/adapter ports and typed results in INTERFACES; do not duplicate another owner's business rule.

## Calls / reads

- [work/registry.json](../../work/registry.json.md) — `read` -> declarative dependency.
- [core/src/journal.rs](journal.rs.md) — `transition` -> `prepare_intent` before publication; read never journals.
- [schemas/registry-v1.schema.json](../../schemas/registry-v1.schema.json.md) — `read` -> declarative dependency.
- [logging/src/lib.rs](../../logging/src/lib.rs.md) — `transition` -> `emit`.

## Called / read by

- [core/src/lifecycle.rs](lifecycle.rs.md) — `plan` -> `read`.
- [core/src/lifecycle.rs](lifecycle.rs.md) — `execute` -> `transition`.
- [core/src/setup.rs](setup.rs.md) — `prepare` -> `read`.
- [core/src/setup.rs](setup.rs.md) — `resume` -> `transition`.
- [tests/unit/registry-contracts.rs](../../tests/unit/registry-contracts.rs.md) — `verify` -> `transition`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Concrete implementation binding (2026-10-31)

Concrete R1 binding: a versioned RegistryDocument wraps the shared RegistrySnapshot.
`read` is read-only and refuses unsupported/corrupt bytes without replacing them.
`transition` locks, compares expected revision and resource fingerprints, validates
the typed change, persists immutable journal intent/protected original, then CAS
publishes and records terminal receipt. Install cannot overwrite an existing entry;
Adopt requires explicit matching local approval and original restart policy for each
foreign resource. CleanupRequired retains survivors; Remove retains data; Purge
requires separate supplied no-live/no-shared/no-recovery evidence. Partial setup
checkpoints remain immutable/idempotent. Desired enabled state never enters registry.

## Implemented ownership details

Implemented transition preconditions are a trusted RegistryPreconditions value:
exact fresh observations, resources without shared consumers, modules without needed
recovery originals. Purge additionally requires removedRetained, owned and stopped.
Foreign adoption checks matching local approval refs and observed original restart
policy. RegistryDocument adds schemaVersion without duplicating RegistrySnapshot DTOs.
Registry lock precedes journal lock; injection must use distinct protected directories.
Durable CAS failure retains immutable intent/original; terminal failure reports
reconciliation instead of claiming rollback. Install can resume matching incomplete
facts but cannot replace ownership/identities/installed versions silently.

## I2 setup progress

SetupProgress appends resources/versions/checkpoints to an Installed/Incomplete
owned entry. Existing resources and versions are immutable in this transition;
separate Update owns replacement. A Verified checkpoint plus fresh exact stopped
observations backs newly registered native resources; retained artifact IDs back
new image versions. Only the trusted SetupCoordinator asserts Complete after all
declared steps/operator inputs are verified. Uncertain or failed setup remains
incomplete; adoption never uses placeholder owned installation records.


RebindOwned{module_id,previous,replacement,receipt} replaces only an installed owned
resource's exact native locator/structural fingerprint after verified recreation
and fresh exact observation. Logical module/resource/service IDs and locator kind
stay fixed, image-version update remains separate. No discovered foreign resource
may enter via rebinding. Protected previous registry bytes remain journal originals.

## I3-A zero-write inventory read

RegistryStore.inspect(directory, readOnlyContext) reads/validates existing registry
through the same bounded decoder/invariant owner without constructing JournalStore.
Missing file returns None; unsafe/unreadable/corrupt is an error with originals
untouched. Inventory reads do not assert an expected instance: returned instanceId
is metadata only. Mutating read/transition retain exact expected-instance checks.
The caller cannot use inspect to authorize native effects. Non-read-only context
is refused. No directories, lock files or journal records are created.

I3 checkpoint freshness correction: a newly observed Verified checkpoint may append
fresh timestamp evidence for the same step/idempotency/status only when resources,
mutable hashes, retained artifacts and error-free body are unchanged, timestamp is
nondecreasing/nonfuture and <=60s old. Exact duplicate is idempotent. Conflicting
same-status bodies still fail; prior evidence remains immutable and bounded by
4096 receipts. Latest per step/key determines setup completeness. General Checkpoint
uses the same owner/helper, including step ID in identity matching.

Verified error-free checkpoint retained-artifact IDs are merged into the entry's
retained_artifacts inventory by the registry owner. Failed/prepared checkpoints do
not confer artifact ownership; duplicate verified receipts are idempotent. This
keeps removal/purge inventories aligned with setup's actual owned configuration.

UninstallOwned accepts only an Owned entry's complete exact resource identity set
and a fresh error-free Verified uninstall receipt emitted after Runtime removal and
fixed systemd unlink verification. No public dispatcher accepts RegistryChange or
caller-supplied receipts. The transition retains config/artifact claims, clears
confirmed removed physical identities/native setup checkpoints/version claims and
marks RemovedRetained with incomplete setup, allowing a later setup to reacquire
resources without replacing preserved settings. Originals/receipt stay in journal.
