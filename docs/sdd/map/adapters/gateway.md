# adapters/gateway

Stable ID: `adapter.gateway` · Phase: `optional` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Existing router/Linux client configuration adapter

Implementation wave: `unselected-optional` (planned ownership, not completion).

## Features and responsibility

- TV
- Quest

## Methods, arguments and result

- `configure` — arguments: qualified deviceProfile, routePolicy; result: gateway plan.

## Structures and injected dependencies

Typed request/result functions defined by methods below; state stays in the owning domain record

Dependencies: explicit ports from INTERFACES; no construction of hidden global config/engine authority.

## Logic

- Use qualified Passwall2/sing-box/Mihomo packages; do not rewrite engines.
- Preserve LAN, DNS/IPv6/endpoint routes and session-only SSH.

## Calls / reads

- [core/src/exports.rs](../core/src/exports.rs.md) — `configure` -> `exportProfile`.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
