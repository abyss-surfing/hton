# tests/integration/client-ipv6.sh

Stable ID: `test.client-ipv6` · Phase: `far-future` · Kind: test

Planned file binding; no implemented functionality is claimed. Purpose: IPv4/CGNAT to controlled IPv6-only TCP/UDP, TUN versus L3 claims

Implementation wave: `deferred-far-future` (planned ownership, not completion).

## Features and responsibility

- acceptance

## Methods, arguments and result

- `verify` — arguments: isolated fixture + pinned versions; result: test evidence.

## Structures and injected dependencies

Final-phase scenario functions and isolated fixtures

Dependencies: completed release interfaces and controlled fixture ports; run only in U0 or F0 as assigned.

## Logic

- Inject the named failure and verify observable behavior, not source-text similarity.
- No destructive production target or implicit third-party probe.
- Development order: execute only after the selected project implementation is complete; unit tests precede functional tests, further acceptance is planned from results.

## Calls / reads

- [adapters/ipv6](../../adapters/ipv6.md) — `verify` -> `qualify` (far-future only; excluded from earlier execution closure).
- [agent/src/probes.rs](../../agent/src/probes.rs.md) — `verify` -> `check` (R2 only).

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
