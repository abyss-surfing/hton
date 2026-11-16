# accounts/src/model.rs

Stable ID: `accounts.model` · Phase: `R2` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Login/profile/named credential ownership

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- personal accounts

## Methods, arguments and result

- `authorize` — arguments: actor, resource, operation; result: grant / denied.
- `rename` — arguments: owner, credentialId, name; result: metadata revision.

## Structures and injected dependencies

User/Profile/CredentialMetadata/Grant; ownership authorization and metadata

Dependencies: account repository, actor/ownership policy; no backend secret redisclosure.

## Logic

- Stable IDs, date-default editable label; admin metadata visibility.
- No secret reads in account/list APIs.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [admin/src/api.rs](../../admin/src/api.rs.md) — `route` -> `authorize` (R2 only).
- [accounts/src/issuance.rs](issuance.rs.md) — `create` -> `authorize` (R2 only).
- [publication/src/model.rs](../../publication/src/model.rs.md) — `planPublication` -> `authorize` (R2 only).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
