# core/src/backup.rs

Stable ID: `core.backup` · Phase: `deferred` · Kind: source

I2 coordinator implementation binding; native host drivers and final acceptance remain pending. Purpose: Encrypted coherent backup/restore

Implementation wave: `I2-C` (coordinator source present; final acceptance pending).

## Features and responsibility

- backup

## Methods, arguments and result

- `backup` — arguments: archive kind, logical name, retention policy, OperationContext; result: EncryptedArchiveMetadata.
- `restore` — arguments: archive handle, approval, current inventory, OperationContext; result: RestoreResult with stopped intent or structured blockers.

## Structures and injected dependencies

ArchiveInventory/RestorePlan; config-only/full durable backup coordinator

Dependencies: protected config/work readers, encryption/archive port, restore journal, version/module/secret checks.

## Logic

- Snapshot compatible config/secrets/fences/journal and runtime bundles.
- Keep identity outside backup; do not execute archive paths.
- Config-only archive and full durable-state backup have different inventories/permissions.
- Restore checks format versions, secrets and module availability without starting services; explicit up follows.
- Keep needed journal/originals/accepted material; exclude reconstructible caches and runtime sockets.
- Restore keeps stopped intent; down suspends periodic jobs, and only explicit up/accepted start restores autostart.

## Calls / reads

- [core/src/lock.rs](lock.rs.md) — `backup` -> `acquire`.
- [logging/src/lib.rs](../../logging/src/lib.rs.md) — `backup` -> `emit`.

## Called / read by

- No product command exposes this module after D159/OR-54; the Rust archive code is dormant pending full-R1 reconciliation.
- [tests/integration/legacy-restore.sh](../../tests/integration/legacy-restore.sh.md) — `verify` -> `restore`.
- [tests/unit/backup-contracts.rs](../../tests/unit/backup-contracts.rs.md) — `verify` -> `restore`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


## I2 concrete binding

Typed coordinators own sequencing and validate invariants; trusted injected ports
own bounded native processes/transport and protected logical inventories. Ports
must fail unavailable/unsupported explicitly and never return synthetic success.
Mutation requires OperationContext Mutating, live deadline/cancellation and durable
journal intent before any secret, certificate, archive or runtime effect. Secret
payload wrappers redact Debug and are never cached with ordinary operation status.
Product tests remain deferred until I4; compilation is integration, not acceptance.

BackupCoordinator.backup(BackupRequest,ctx), plan_restore(archiveId,approvalId,ctx)
and restore(RestorePlan,ctx) use logical object inventory exclusively. ArchiveKind
Configuration includes config/referenced secrets/policy fences; FullDurable additionally
includes registry/journal/accepted bundles/certs/ACME state/required pinned image
artifacts. Caches, sockets, host decryption identity and distribution code are not
archive categories. Entries carry version/hash/size/protection, never executable
paths. Crypto streams bounded chunks; no whole archive is allocated in memory.
The qualified age/archive host port must reject raw links, traversal, duplicates,
unknown entries, output-size/hash violations and unauthenticated data before writes.

Plan restoration validates exact logical inventory, formats, referenced secrets,
module availability and credential-fence compatibility. Inventory compatibility
uses existing config/module owners; it does not install or assume foreign ownership.
Protected originals and Restore intent precede all restore effects. RestoreRuntime
first verifies full shutdown, then reserves durable runtime maintenance by CAS.
A racing up causes reservation refusal before restoration writes. Its RestoreFence
contains exact maintenanceId/stoppedEpoch/fence floor; every write/publish rechecks
that reservation. Archived running intent/epoch/lease never overrides current state.
Runtime accepted boot/promotions respect the reservation independently of Core locks.

Restore publication preserves the CURRENT stop fence and active recovery journal,
merging retained archived journals instead of replacing the running restore intent.
Only after verified restored objects and a durable completed journal terminal does
release_completed release the same operation's reservation; an uncertain write or
release retains recoverable originals and the startup block. An identical completed
retry returns original historical result metadata and may reconcile that exact
reservation release; it does not re-run restoration. Explicit up is always separate.

I3 concrete archive locations: ArchiveEntry.location is a typed Config/Work inventory
scope plus bounded logical components, validated by category-owned allowlists.
It never permits absolute/traversal paths, arbitrary roots or executable archive
entries. IDs bind the canonical location; missing current files can be restored.
Decryption identity is explicitly excluded from inventory and must have an external
copy. Restore raw runtime/fence state is staged, never written over current stop/
maintenance/epoch; accepted refs/fences publish only through runtime owner.
Retention stays under configuration lock and protects every archive while an
unfinished Restore journal intent exists, including crash-recovery records.
