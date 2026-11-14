# adapters/ipv6

Stable ID: `adapter.ipv6` · Phase: `far-future` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Server native/HE provisioning and evidence

Implementation wave: `deferred-far-future` (planned ownership, not completion).

## Features and responsibility

- IPv6

## Methods, arguments and result

- `qualify` — arguments: engine, upstream; result: IPv6 evidence.
- `planUpstream` — arguments: native|he, settings; result: approved host plan.

## Structures and injected dependencies

Typed request/result functions defined by methods below; state stays in the owning domain record

Dependencies: explicit ports from INTERFACES; no construction of hidden global config/engine authority.

## Logic

- Host IPv6 alone does not prove container egress.
- No client protocol-41 requirement or invented prefix delegation.

## Calls / reads

- [core/src/checks.rs](../core/src/checks.rs.md) — `qualify` -> `observe`.
- [core/src/plan.rs](../core/src/plan.rs.md) — `qualify` -> `plan`.

## Called / read by

- [accounts/src/egress.rs](../accounts/src/egress.rs.md) — `enforce` -> `qualify` (far-future only; excluded from earlier execution closure).
- [scripts/tunnel](../scripts/tunnel.md) — `run` -> `qualify` (far-future only; excluded from earlier execution closure).
- [scripts/tunnel](../scripts/tunnel.md) — `run` -> `planUpstream` (far-future only; excluded from earlier execution closure).
- [tests/integration/client-ipv6.sh](../tests/integration/client-ipv6.sh.md) — `verify` -> `qualify` (far-future only; excluded from earlier execution closure).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
