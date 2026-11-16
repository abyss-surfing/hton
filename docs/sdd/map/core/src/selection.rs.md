# core/src/selection.rs

Stable ID: `core.selection` · Phase: `R1` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Narrow lifecycle-only enabled-selection persistence

Implementation wave: `I2-B` (planned ownership, not completion).

## Features and responsibility

- core

## Methods, arguments and result

- `prepare` — arguments: DesiredSnapshot, moduleIds, enabled, expectedRootRevision; result: SelectionCandidate or conflict.
- `commit` — arguments: SelectionCandidate, expectedRootRevision, OperationContext; result: ConfigCommitResult.

## Structures and injected dependencies

SelectionCandidate; narrow root enabled-selection writer

Dependencies: coherent root reader, protected original journal, bounded publication/coordination port.

## Logic

- Only seed/persist bounded participation transitions needed by R1 lifecycle; not general config editor.
- Preserve originals, reject changed inputs and diagnose invalid root; no automatic semantic merges.
- General parameter editing/merge/migration belongs to later configurator.
- Use explicit injected storage/clock/adapter ports and typed results in INTERFACES; do not duplicate another owner's business rule.

## Calls / reads

- [core/src/journal.rs](journal.rs.md) — `commit` -> `prepare_intent`.
- [core/src/config.rs](config.rs.md) — `prepare` -> `load`.

## Called / read by

- [core/src/lifecycle.rs](lifecycle.rs.md) — `plan` -> `prepare`.
- [core/src/lifecycle.rs](lifecycle.rs.md) — `execute` -> `commit`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I2 concrete binding

`SelectionWriter` owns protected root config capability and JournalStore.
`prepare(source,moduleIds,enabled,expectedRootRevision,ctx) -> SelectionCandidate`
is read-only: bounded config.load, exact baseline bytes, schema-valid known IDs,
only features.<id>.enabled changed, re-decode/reload prospective bytes. Candidate
bytes are private and Debug-redacted; callers see change/revision/read metadata.
`commit(candidate,expectedRootRevision,ctx) -> SelectionCommit` rechecks exact bytes
and root revision, journals a typed Selection intent and protected original before
cooperating-writer CAS/fsync. Read-only mode cannot commit; invalid root is refused.
No merge/migration/editor safety against arbitrary uncoordinated writers is claimed.

Selection commit never activates services. It returns exact resulting revision for
apply owner; prospective effect planning must use those exact bytes. A no-change
candidate is recognized without creating files or a journal.
