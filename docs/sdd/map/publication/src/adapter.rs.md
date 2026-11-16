# publication/src/adapter.rs

Stable ID: `publication.adapter` · Phase: `R2` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Pinned reverse-tunnel backend integration

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- local site

## Methods, arguments and result

- `configure` — arguments: publication, scoped agentRef, backend; result: private tunnel config.

## Structures and injected dependencies

PublicationArtifacts; existing tunnel backend integration

Dependencies: qualified tunnel serializer, scoped agent grants, shared render/apply.

## Logic

- Qualify rathole first, frp/Chisel/SSH alternatives.
- No global shared token masquerading as per-user scope.
- TLS browser leg and encrypted agent leg are independent.

## Calls / reads

- [publication/src/model.rs](model.rs.md) — `configure` -> `planPublication` (R2 only).
- [accounts/src/issuance.rs](../../accounts/src/issuance.rs.md) — `configure` -> `create` (R2 only).
- [core/src/apply.rs](../../core/src/apply.rs.md) — `configure` -> `apply`.

## Called / read by

- [tests/integration/publication-isolation.sh](../../tests/integration/publication-isolation.sh.md) — `verify` -> `configure` (R2 only).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
