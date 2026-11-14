# modules/services.yaml

Stable ID: `service.index` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Shared service/template/change-action catalogue

Implementation wave: `I1-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Protocol IDs differ from service IDs; resolve last-consumer shutdown.
- Declare mandatory core consumers and optional consumers separately. Last optional consumer removal cannot stop a mandatory service.
- Declare fixed template/adapter IDs, pinned image references, health checks and reload/recreate policy; no user executable hooks.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/catalog.rs](../core/src/catalog.rs.md) — `describe` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

I1 binding: runtime/HAProxy/Caddy are mandatory core consumers; Xray depends on
private Unbound; foreign-amnezia is externally owned. Service records separate
fixed adapter IDs/change actions from pinned image inventory. No image digest is
invented in this file; planner requires explicit pinned installed inventory before
producing an activation plan. HAProxy depends on Caddy, whose valid mask site is
independent of optional management/API DNS and process state.
