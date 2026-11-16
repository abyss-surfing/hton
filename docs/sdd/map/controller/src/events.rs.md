# controller/src/events.rs

Stable ID: `control.events` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Bounded redacted operation event log

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- reactivity

## Methods, arguments and result

- `subscribe` — arguments: actor, cursor, budget; result: events + resync marker.

## Structures and injected dependencies

OperationEvent/EventCursor/ResyncMarker; bounded metadata stream

Dependencies: durable redacted event reader, scope filter, retention/backpressure policy.

## Logic

- Sequence IDs; bounded history; no credentials in events.
- Unavailable events do not affect data plane.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [admin/src/events.rs](../../admin/src/events.rs.md) — `stream` -> `subscribe`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
