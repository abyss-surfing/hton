# configurator/src/main.rs

Stable ID: `configurator.entry` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Standalone configuration module entry independent of managed target validity

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- configurator

## Methods, arguments and result

- `dispatch` — arguments: typed ConfigRequest, ToolContext, ActorContext; result: ConfigResponse or durable operation reference.

## Structures and injected dependencies

ConfiguratorRequest/ToolContext; standalone entry with independent diagnostics/auth

Dependencies: Core read/validate, parameter change service, migration service; tool bootstrap independent of target.

## Logic

- Bootstrap diagnostics/auth independently of editable target YAML.
- CLI may call directly; management API delegates token-scoped requests.
- Own parameter editing/merging/migration only; reuse Core validation and safe apply.

## Calls / reads

- [configurator/src/changes.rs](changes.rs.md) — `dispatch` -> `prepare`.
- [configurator/src/changes.rs](changes.rs.md) — `dispatch` -> `commit`.
- [configurator/src/migration.rs](migration.rs.md) — `dispatch` -> `migrate`.
- [core/src/config.rs](../../core/src/config.rs.md) — `dispatch` -> `load`.

## Called / read by

- [controller/src/api.rs](../../controller/src/api.rs.md) — `handle` -> `dispatch`.
- [scripts/upgrade](../../scripts/upgrade.md) — `run` -> `dispatch`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
