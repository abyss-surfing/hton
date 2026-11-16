# docs/examples/config/modules/foreign-amnezia.yaml

Stable ID: `example.module.foreign-amnezia` · Phase: `design` · Kind: configuration

Illustrative stored allocation only; `scripts/install` chooses a free random port and reuses it. Purpose: Per-module settings example foreign-amnezia

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
- Example illustrates one persisted allocation in 5000–6000; real install generates a free value and reuses it. Native listener remains independently observed. Outline 8443 is preserved unless explicitly registered and included in a separately reviewed remap.

## Calls / reads

- [schemas/module-v1.schema.json](../../../../schemas/module-v1.schema.json.md) — declarative dependency.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

The current instance values may be collected during initial `scripts/install`; this example does not create product defaults. Missing non-secret input is guided; credentials remain SecretStore refs.

## Target M1 example contract

Use docs/plans/INSTALLATION-IMPLEMENTATION.md and the exact config-owned shapes in
sdd-config. Old example bytes are baseline until M1-A updates schema/parser/example
together. No fixed personal domain/secret URL is a product default. Root example
omits core.clientAccessPage.path so a fresh install auto-generates it; an explicit empty value opts out; names empty and refreshSeconds 60;
module examples use example.com illustrative separate protocol names, own-mode
fields and selected public UDP443. Actual example.com/client-access is supplied instance
config only. No actual keys/passwords/image placeholder digests in examples.
