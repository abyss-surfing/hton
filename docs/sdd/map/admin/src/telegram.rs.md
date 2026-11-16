# admin/src/telegram.rs

Stable ID: `admin.telegram` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Optional allowlisted Telegram interface

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- Telegram

## Methods, arguments and result

- `poll` — arguments: tokenRef, allowedIds, interval; result: bounded task.

## Structures and injected dependencies

Typed request/result functions defined by methods below; state stays in the owning domain record

Dependencies: explicit ports from INTERFACES; no construction of hidden global config/engine authority.

## Logic

- Ignore unauthorized senders before action/disclosure.
- Reuse controller scope; no stop-point bypass or R2 secret replay.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- [controller/src/api.rs](../../controller/src/api.rs.md) — `poll` -> `handle`.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
