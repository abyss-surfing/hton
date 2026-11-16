# logging/src/lib.rs

Stable ID: `infra.logging` · Phase: `R1` · Kind: source

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Shared secret-safe structured events and bounded rotated log sink

Implementation wave: `I1-B` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- core

## Methods, arguments and result

- `emit` — arguments: EventContext, severity, code, SafeFields; result: LogWriteStatus.
- `configure` — arguments: LoggingPolicy, installerSelectedDirectory; result: ConfiguredSink or bounded fallback diagnostics.

## Structures and injected dependencies

LogSink/SafeFields/EventContext/LoggingPolicy; bounded writer/rotation

Dependencies: installer-selected protected directory, clock, bounded queue and disk/rotation limits; no Core imports.

## Logic

- Automatic configurable size/age rotation; log pressure never blocks working VPN.
- Timestamp/module/operation/transaction IDs; redact secrets and full credential URLs at all levels.
- Bound queues, permissions and incident exports; logging failure is observable but cannot undo runtime acceptance.
- Independently buildable infrastructure package; no Core/configurator/API imports. Runtime and Core inject this same sink contract.
- Read-only/dry-run operations use transient stderr/in-memory sink; no log directory/file/rotation writes during zero-write planning.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/main.rs](../../core/src/main.rs.md) — `run` -> `emit`.
- [core/src/apply.rs](../../core/src/apply.rs.md) — `apply` -> `emit`.
- [core/src/journal.rs](../../core/src/journal.rs.md) — `prepare` -> `emit`.
- [core/src/certificates.rs](../../core/src/certificates.rs.md) — `issue` -> `emit`.
- [core/src/backup.rs](../../core/src/backup.rs.md) — `backup` -> `emit`.
- [runtime/src/main.rs](../../runtime/src/main.rs.md) — `execute` -> `emit`.
- [runtime/src/guardian.rs](../../runtime/src/guardian.rs.md) — `monitor` -> `emit`.
- [core/src/registry.rs](../../core/src/registry.rs.md) — `transition` -> `emit`.
- [core/src/lifecycle.rs](../../core/src/lifecycle.rs.md) — `execute` -> `emit`.
- [core/src/setup.rs](../../core/src/setup.rs.md) — `prepare` -> `emit`.
- [adapters/resources.rs](../../adapters/resources.rs.md) — `executeStep` -> `emit`.
- [tests/unit/logging-contracts.rs](../../tests/unit/logging-contracts.rs.md) — `verify` -> `emit`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Concrete implementation binding (2026-10-31)

Concrete R1 binding: an independent `LogSink` accepts only closed EventCode,
Severity, Component, validated correlation IDs and closed SafeFields primitives;
there is no arbitrary message/URL/backend-output field at any verbosity. Disk mode
receives protected hton-storage Directory; transient mode does not open/create any
file. Each event is bounded JSON; try_send to a bounded sync_channel worker avoids
unbounded queues and keeps disk I/O off emit callers. Size/age rotation uses a fixed bounded ring, serialized with a
short bounded shared-storage lock. Queue/serialization pressure returns typed Dropped status; disk/lock failures
increment observable dropped counters, never panics or stops VPN. Configurable byte/count/age limits have
hard trusted maxima. No Core/management/runtime contract dependency.

## Implemented ownership details

Implemented LogSink::configure starts a bounded-channel background worker;
LogSink::transient creates no directory/thread/file. emit never does filesystem I/O
or waits on queue space. Statistics exposes written/dropped counters including worker
I/O failures. Rotation has a bounded 16-slot maximum and age/size limits; reducing
policy limits removes stale known ring slots by exact bytes. The fixed EventCode/
Component/Outcome vocabularies have no arbitrary prose or backend stderr slot.

Read-only `transient(policy)` retains a bounded in-memory metadata queue;
`drain_transient() -> Vec<Vec<u8>>` retrieves closed-format events for CLI diagnostics.
No files/worker/rotation exist in this mode. Both enqueue and drain use try-lock
and report/drop on contention rather than blocking operation callers.

`EventContext` carries optional validated operation/transaction/module/service/
resource/object correlation IDs. `SafeFields.reason` is a closed `FailureReason`
enum, preserving diagnostic reason without free-form backend output. Journal
prose and protected records are not serialized through the ordinary log sink.

I3 NativeOutput is a closed event for independent collector metadata. Raw native
payloads remain excluded; collector reports bounded byte counts/severity classes,
not arbitrary native diagnostic text. This is deliberately metadata retention,
not a promise that full native messages are preserved after redaction.

LoggingPolicy.bounded(level,totalCeiling,retentionDays) is the single pure policy
constructor for Core/runtime projection consumers. It accepts 1..=7 retention
days (OR-53), validates ranges and maps to <=16 ring files of <=1MiB, always within
the requested byte ceiling. No duplicate ring-sizing rules in host compositions;
no file operations in policy construction.
