# accounts/src/egress.rs

Stable ID: `accounts.egress` · Phase: `far-future` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Strict per-credential IPv4/IPv6 policy

Implementation wave: `deferred-far-future` (planned ownership, not completion).

## Features and responsibility

- IPv6 egress

## Methods, arguments and result

- `enforce` — arguments: credentialId, family, adapterEvidence; result: outbound policy / blocked.

## Structures and injected dependencies

Typed request/result functions defined by methods below; state stays in the owning domain record

Dependencies: explicit ports from INTERFACES; no construction of hidden global config/engine authority.

## Logic

- Not tunnel transport family or public address allocation.
- No wrong-family literal/AAAA-A or outage downgrade.
- Unsupported foreign backend blocks selection; no global outbound flip.

## Calls / reads

- [core/src/catalog.rs](../../core/src/catalog.rs.md) — `enforce` -> `describe`.
- [adapters/ipv6](../../adapters/ipv6.md) — `enforce` -> `qualify` (far-future only; excluded from earlier execution closure).

## Called / read by

- [accounts/src/issuance.rs](issuance.rs.md) — `create` -> `enforce` (far-future only; excluded from earlier execution closure).
- [tests/integration/identity-policy.rs](../../tests/integration/identity-policy.rs.md) — `verify` -> `enforce` (far-future only; excluded from earlier execution closure).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
