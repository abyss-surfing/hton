# admin/src/auth.rs

Stable ID: `admin.auth` · Phase: `later-management` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Vetted owner login/session/CSRF integration

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- auth

## Methods, arguments and result

- `authenticate` — arguments: request, session context; result: actor / denied.
- `bootstrap` — arguments: local oneTimeProof, password; result: owner session.

## Structures and injected dependencies

Session/OwnerBootstrap/AuthenticatedActor; web authentication adapter

Dependencies: vetted password/session primitives, CSRF policy, rate limiter, clock and protected credential metadata.

## Logic

- Argon2/session libraries; rate limits and secure cookies.
- No public unauthenticated setup or handmade crypto.
- Deferred optional management iteration; web always calls management API, parameter edits use configurator and Core semantics.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [admin/src/main.rs](main.rs.md) — `serve` -> `authenticate`.
- [admin/src/api.rs](api.rs.md) — `route` -> `authenticate`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
