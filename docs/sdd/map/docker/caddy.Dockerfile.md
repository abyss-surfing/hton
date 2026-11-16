# docker/caddy.Dockerfile

Stable ID: `build.caddy` · Phase: `R1` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: Pinned Caddy + existing Naive upstream fork

Implementation wave: `I3-B` (planned ownership, not completion).

## Features and responsibility

- build

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Preserve provenance; no arbitrary plugin download at boot.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
