# configurator/src/migration.rs

Stable ID: `configurator.migration` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Preserving sequential platform/module format migration and optional legacy import

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- migration

## Methods, arguments and result

- `migrate` — arguments: source snapshot, target format versions, transformation catalogue; result: PreparedChangeSet with originals or structured blockers.
- `importLegacy` — arguments: protected legacy source, explicit mapping policy; result: PreparedChangeSet; never a live env writer.

## Structures and injected dependencies

MigrationStep/DeprecatedRecord/PreparedChangeSet; format transformation pipeline

Dependencies: versioned transform catalogue, protected original/deprecated writer, Core validation.

## Logic

- Chain format versions, not software releases; no format change requires no migrator.
- Every rename/reset/removal preserves original values in protected source-associated deprecated records.
- Unsupported keys retain source path/version; repeated migration never overwrites earlier originals.
- Preserve editable Docker customization, comments where practical and secrets always.
- Malformed target remains diagnosable; no runtime activation before validation.

## Calls / reads

- [core/src/config.rs](../../core/src/config.rs.md) — `migrate` -> `validate`.
- [core/src/secrets.rs](../../core/src/secrets.rs.md) — `migrate` -> `generate`.
- [core/src/journal.rs](../../core/src/journal.rs.md) — `migrate` -> `prepare`.

## Called / read by

- [configurator/src/main.rs](main.rs.md) — `dispatch` -> `migrate`.
- [configurator/tests/changes.rs](../tests/changes.rs.md) — `run` -> `migrate`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
