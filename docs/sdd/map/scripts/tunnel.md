# scripts/tunnel

Stable ID: `cmd.tunnel` · Phase: `far-future` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Server HE/native status and optional host tunnel

Implementation wave: `deferred-far-future` (planned ownership, not completion).

## Features and responsibility

- CLI

## Methods, arguments and result

- `run` — arguments: up|update|status, --force, --json; result: exit code / redacted result.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- Server provisioning differs from client IPv6-egress feature.
- No unapproved host route replacement.

## Calls / reads

- [adapters/ipv6](../adapters/ipv6.md) — `run` -> `qualify` (far-future only; excluded from earlier execution closure).
- [adapters/ipv6](../adapters/ipv6.md) — `run` -> `planUpstream` (far-future only; excluded from earlier execution closure).
- [lib/hton](../lib/hton.md) — `run` -> `invokeHelper`.

## Called / read by

- [bin/hton](../bin/hton.md) — `dispatch` -> `run` (far-future only; excluded from earlier execution closure).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
