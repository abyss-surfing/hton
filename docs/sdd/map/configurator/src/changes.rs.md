# configurator/src/changes.rs

Stable ID: `configurator.changes` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Atomic parameter batches with semantic conflict detection

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- configurator

## Methods, arguments and result

- `prepare` — arguments: objectIds, ParameterChanges, touchedValueBaselines, operationPreconditions, actor; result: ProtectedDraftId and redacted preview.
- `commit` — arguments: draftId, requestId, editorCoordinationProof; result: ConfigCommitResult or retained ConflictDraft.

## Structures and injected dependencies

ParameterChange/ProtectedDraft/ConflictDraft/ConfigCommitResult; semantic batch coordinator

Dependencies: draft store, coherent reader, schema/module checks, editor lease/conditional publication port, journal.

## Logic

- Re-read and preserve independent edits; missing differs from null; lists follow module strategy.
- Reject semantic touched-value/dependency conflicts, not every file revision change.
- Validate combined candidate; commit all files recoverably or none.
- Retain conflicting prepared drafts and all originals; redact secret values.
- Publication race with uncooperative direct editors is unresolved until a conditional/editor-lease protocol is proven.

## Calls / reads

- [core/src/config.rs](../../core/src/config.rs.md) — `commit` -> `load`.
- [core/src/config.rs](../../core/src/config.rs.md) — `commit` -> `validate`.
- [core/src/journal.rs](../../core/src/journal.rs.md) — `commit` -> `prepare`.
- [core/src/plan.rs](../../core/src/plan.rs.md) — `commit` -> `plan`.

## Called / read by

- [configurator/src/main.rs](main.rs.md) — `dispatch` -> `prepare`.
- [configurator/src/main.rs](main.rs.md) — `dispatch` -> `commit`.
- [configurator/tests/changes.rs](../tests/changes.rs.md) — `run` -> `prepare`.
- [configurator/tests/changes.rs](../tests/changes.rs.md) — `run` -> `commit`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
