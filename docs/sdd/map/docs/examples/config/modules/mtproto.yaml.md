# docs/examples/config/modules/mtproto.yaml

Stable ID: `example.module.mtproto` · Phase: `design` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: Per-module settings example mtproto

Implementation wave: `I3-C` (planned ownership, not completion).

## Features and responsibility

- example

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Versioned module settings example, separate from root enabled selection.
- No real secrets or proof of deployed availability.

## Calls / reads

- [schemas/module-v1.schema.json](../../../../schemas/module-v1.schema.json.md) — declarative dependency.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Target M1 example contract

Use docs/plans/INSTALLATION-IMPLEMENTATION.md and the exact config-owned shapes in
sdd-config. Old example bytes are baseline until M1-A updates schema/parser/example
together. No fixed personal domain/secret URL is a product default. Root example
omits core.clientAccessPage.path so a fresh install auto-generates it; an explicit empty value opts out; names empty and refreshSeconds 60;
module examples use example.com illustrative separate protocol names, own-mode
fields and selected public UDP443. Actual example.com/client-access is supplied instance
config only. No actual keys/passwords/image placeholder digests in examples.
