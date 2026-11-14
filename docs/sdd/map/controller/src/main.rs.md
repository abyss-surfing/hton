# controller/src/main.rs

Stable ID: `control.entry` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Private bounded Unix API service

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- control

## Methods, arguments and result

- `serve` — arguments: socket policy; result: request loop.

## Structures and injected dependencies

ManagementServer; bounded optional transport loop

Dependencies: token/peer auth, request decoder/limits, dispatcher, durable worker/task supervision.

## Logic

- No public listener; own resource limits and stable socket directory.
- Controller failure cannot stop accepted VPN.

## Calls / reads

- [controller/src/api.rs](api.rs.md) — `serve` -> `handle`.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
