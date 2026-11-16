# etc/hysteria/config.yaml.template

Stable ID: `template.hysteria` · Phase: `R1` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: QUIC endpoint native config

Implementation wave: `I2-A` (planned ownership, not completion).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Unique UDP binding and supplied certs; no Caddy UDP collision.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/render.rs](../../core/src/render.rs.md) — `compile` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
