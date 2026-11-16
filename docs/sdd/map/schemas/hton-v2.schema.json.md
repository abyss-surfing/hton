# schemas/hton-v2.schema.json

Stable ID: `schema.v2` · Selected package: `M1-A` · Status: planned M1 contract

Same shared core shape in future root schema. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
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

# schemas/hton-v2.schema.json

Stable ID: `schema.v2` · Phase: `R2` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: Future field shape

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Adds account defaults and future modules without making R1 accept them.
- Extend v1 explicitly with R2 fields; common feature schemas and defaults retain the same semantics.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/config.rs](../core/src/config.rs.md) — `validate` -> declarative dependency (R2 only).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## OR-39/OR-40 planning amendment

Read canonical sdd-config, sdd-installer, sdd-foreign-adapters and INTERFACES:
all SNI are config-owned and resolved once; administrative username comes from
`foreign-amnezia.settings.admin.user`. Managed provisioning is separate from
foreign association, starts on private bindings and delegates account effects to
HostAccess. Exact future file ownership and schema changes remain implementation
prerequisites in TASK-AMNEZIA-PROVISION; this mirror does not change live parsers.

## Frozen M1 schema delta

Canonical field table: docs/specs/sdd-config.md, M1 configuration freeze. Root
schemas add closed hostPolicy/core.dns/core.clientAccessPage objects; module schema
adds managed/associate/admin/provision/legacy-name fields, separate Outline host,
ordinary path variant and UDP443 constraints. Keep disabled incomplete settings
editable; semantic completeness belongs to activation. Existing schemaVersion and
legacy keys are retained through the bounded migration contract, not silently dropped.
Implement schemas with their typed consumers in M1-A; no live JSON support is
claimed by this Markdown contract. The main root example omits page path and relies on first-install generation; an explicit empty path is the opt-out.
