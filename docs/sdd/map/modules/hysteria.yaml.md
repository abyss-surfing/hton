# modules/hysteria.yaml

Stable ID: `module.hysteria` · Phase: `optional` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: hysteria declarative capability descriptor

Implementation wave: `unselected-optional` (planned ownership, not completion).

## Features and responsibility

- hysteria

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Phase: optional; execution location: server.
- Resolve shared services: hysteria.
- Own settings/dependencies/check identifiers only; no executable hooks or arbitrary image/mount strings.
- Reference the canonical feature settings schema; declare versioned defaults, required-on-enable fields, allowed listener scopes, feature dependencies and service consumers separately.
- Disabled configuration retains schema-valid settings without new runtime effects or credential issuance. Availability and phase gate activation, not retention.
- Root controls enabled only; versioned settings live in config/modules/hysteria.yaml.
- Declare setup checkpoints, semantic check, lifecycle/update behavior and explicit resources/ownership; add stays disabled and resumable.
- Execution evidence and product tests are final waves after implementation; native runtime validation remains product behavior.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/catalog.rs](../core/src/catalog.rs.md) — `describe` -> declarative dependency.
- [modules/index.yaml](index.yaml.md) — declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
