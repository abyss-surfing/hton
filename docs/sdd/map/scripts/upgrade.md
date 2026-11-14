# scripts/upgrade

Stable ID: `cmd.upgrade` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Deferred protected code/config/module update coordinator

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- cmd

## Methods, arguments and result

- `run` — arguments: upgrade selection, approved source revision, --json; result: UpgradeOperationResult.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- Future command: protect originals/code revision, controlled git pull, preserve config/secrets, invoke module-owned updates.
- No dirty-source overwrite, implicit image update on up or unsupported data rollback promise.
- Full implementation deferred; R1 help reports unavailable.

## Calls / reads

- [configurator/src/main.rs](../configurator/src/main.rs.md) — `run` -> `dispatch`.
- [core/src/lifecycle.rs](../core/src/lifecycle.rs.md) — `run` -> `execute`.

## Called / read by

- [bin/hton](../bin/hton.md) — `dispatch` -> `run` (later-management only).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
