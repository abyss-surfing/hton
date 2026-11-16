# modules/vless.yaml

Stable ID: `module.vless` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: vless declarative capability descriptor

Implementation wave: `I1-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- vless

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Phase: R1; execution location: server.
- Resolve shared services: xray, unbound.
- Own settings/dependencies/check identifiers only; no executable hooks or arbitrary image/mount strings.
- Reference the canonical feature settings schema; declare versioned defaults, required-on-enable fields, allowed listener scopes, feature dependencies and service consumers separately.
- Disabled configuration retains schema-valid settings without new runtime effects or credential issuance. Availability and phase gate activation, not retention.
- Root controls enabled only; versioned settings live in config/modules/vless.yaml.
- Declare setup checkpoints, semantic check, lifecycle/update behavior and explicit resources/ownership; add stays disabled and resumable.
- `portAllocation: null` declares that VLESS is multiplexed through the shared HAProxy/Caddy edge and has no separate externally allocated backend listener.
- Public-edge route claims remain owned by the shared proxy/catalogue path; null means no dynamic Host ListenerClaims are materialized for VLESS itself.
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

I1 binding defaults transport=ws and path.secretRef=vless/path; this is a reference,
never generated secret content. Requires Xray/private Unbound, xray.check/render/
observe/setup capability IDs. It owns no update image/artifact; update is null until
a module-specific updater and pinned owned target exist. Actual path evidence is supplied by secret
owner before route collision checks; native/client qualification remains separate.
