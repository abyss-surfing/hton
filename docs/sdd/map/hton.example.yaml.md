# hton.example.yaml

Stable ID: `config.example` · Phase: `R1` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: Release editable config example

Implementation wave: `I3-C` (planned ownership, not completion).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- All optional features off; raw credentials absent.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

TCP/4443 in this generic example is a development/prepublication value. Initial public setup is guided by `scripts/install`, which suggests TCP/443 only when no site domain/publication is configured; it does not require manual editing.

## Target M1 example contract

Use docs/plans/INSTALLATION-IMPLEMENTATION.md and the exact config-owned shapes in
sdd-config. Old example bytes are baseline until M1-A updates schema/parser/example
together. No fixed personal domain/secret URL is a product default. Root example
omits core.clientAccessPage.path so a fresh install auto-generates it; an explicit empty value opts out; names empty and refreshSeconds 60;
module examples use example.com illustrative separate protocol names, own-mode
fields and selected public UDP443. Actual example.com/client-access is supplied instance
config only. No actual keys/passwords/image placeholder digests in examples.
