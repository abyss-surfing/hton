# accounts/src/issuance.rs

Stable ID: `accounts.issuance` · Phase: `R2` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Multi-profile single-disclosure transaction

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- one-time tokens

## Methods, arguments and result

- `create` — arguments: ownerId, profiles, name, requestId; optional family only with far-future capability grant; result: one-time creation response.
- `revoke` — arguments: actor, credentialId; result: verified revoke.

## Structures and injected dependencies

IssuanceBatch/DisclosureState/OneTimeResponse; scoped credential activation

Dependencies: backend capability grants, secret generator, shared safe apply, durable consumed marker.

## Logic

- Resolve granted Amnezia default profile.
- Persist consumed delivery before sending; retry returns metadata only.
- Policy-clean active/fallback bundles before confirming revoke.
- Initial accounts support IPv4; strict family selector and its implementation are far future.

## Calls / reads

- [accounts/src/model.rs](model.rs.md) — `create` -> `authorize` (R2 only).
- [core/src/secrets.rs](../../core/src/secrets.rs.md) — `create` -> `generate`.
- [accounts/src/egress.rs](egress.rs.md) — `create` -> `enforce` (far-future only; excluded from earlier execution closure).
- [core/src/apply.rs](../../core/src/apply.rs.md) — `create` -> `apply`.

## Called / read by

- [admin/src/api.rs](../../admin/src/api.rs.md) — `route` -> `create` (R2 only).
- [publication/src/adapter.rs](../../publication/src/adapter.rs.md) — `configure` -> `create` (R2 only).
- [tests/integration/identity-policy.rs](../../tests/integration/identity-policy.rs.md) — `verify` -> `create` (R2 only).
- [tests/integration/identity-policy.rs](../../tests/integration/identity-policy.rs.md) — `verify` -> `revoke` (R2 only).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
