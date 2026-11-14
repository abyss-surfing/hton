# schemas/hton-v1.schema.json

Stable ID: `schema.v1` · Selected package: `M1-A` · Status: planned M1 contract

Root host/DNS/page fields, closed typed schema. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

Declarative artifact; its producer/consumer is specified in the M1 plan.

## Calls

No callable dependency in the selected overlay; injected observations/typed data only.

## Called by

Entry/declarative/final-evidence artifact; producer/consumer binding is in the plan.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.

## Historical baseline contract (superseded where M1 differs)

# schemas/hton-v1.schema.json

Stable ID: `schema.v1` · Phase: `R1` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: Release field shape

Implementation wave: `I1-A` (planned ownership, not completion).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Reject unknown roots, feature shape drift and core.enabled.
- Present feature objects require enabled; omitted features are disabled. Disabled settings still satisfy their field schema.
- Feature schemas are the canonical shape referenced by descriptors; shared v1/v2 fields must remain equivalent. JSON Schema default annotations do not perform normalization.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/config.rs](../core/src/config.rs.md) — `validate` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Frozen M1 schema delta

Canonical field table: docs/specs/sdd-config.md, M1 configuration freeze. Root
schemas add closed hostPolicy/core.dns/core.clientAccessPage objects; module schema
adds managed/associate/admin/provision/legacy-name fields, separate Outline host,
ordinary path variant and UDP443 constraints. Keep disabled incomplete settings
editable; semantic completeness belongs to activation. Existing schemaVersion and
legacy keys are retained through the bounded migration contract, not silently dropped.
Implement schemas with their typed consumers in M1-A; no live JSON support is
claimed by this Markdown contract. The main root example omits page path and relies on first-install generation; an explicit empty path is the opt-out.
