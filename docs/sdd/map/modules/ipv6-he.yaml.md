# modules/ipv6-he.yaml

Stable ID: `module.ipv6-he` · Phase: `far-future` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: ipv6-he declarative capability descriptor

Implementation wave: `deferred-far-future` (planned ownership, not completion).

## Features and responsibility

- ipv6-he

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Phase: far-future; execution location: host.
- Resolve shared services: he-adapter.
- Own settings/dependencies/check identifiers only; no executable hooks or arbitrary image/mount strings.
- Reference the canonical feature settings schema; declare versioned defaults, required-on-enable fields, allowed listener scopes, feature dependencies and service consumers separately.
- Disabled configuration retains schema-valid settings without new runtime effects or credential issuance. Availability and phase gate activation, not retention.
- Root controls enabled only; versioned settings live in config/modules/ipv6-he.yaml.
- Declare setup checkpoints, semantic check, lifecycle/update behavior and explicit resources/ownership; add stays disabled and resumable.
- Execution evidence and product tests are final waves after implementation; native runtime validation remains product behavior.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/catalog.rs](../core/src/catalog.rs.md) — `describe` -> declarative dependency (far-future only; excluded from earlier execution closure).
- [modules/index.yaml](index.yaml.md) — declarative dependency (far-future only; excluded from earlier execution closure).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
