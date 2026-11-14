# schemas/module-v1.schema.json

Stable ID: `schema.module` · Selected package: `M1-A` · Status: planned M1 contract

Managed/associated module, names and UDP/443 syntax. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
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

# schemas/module-v1.schema.json

Stable ID: `schema.module` · Phase: `R1` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: Versioned per-module settings syntax contract

Implementation wave: `I1-A` (planned ownership, not completion).

## Features and responsibility

- schema

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Validate schemaVersion/moduleId/settings separately from root composition.
- Unknown keys are diagnostics, preserved until deferred explicit migration; schema failure never deletes values.
- Settings completeness and implementation availability are semantic activation checks.
- Amnezia `settings.hostPorts.tcp` and `.udp` remain syntactically broad for legacy review; semantic allocation requires one shared 5000–6000 value when enabled. The allocator, not schema range tightening, owns selection/reuse. Values are external host bindings only; native in-container ports are observed and preserved.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/config.rs](../core/src/config.rs.md) — `validate` -> declarative dependency.
- [docs/examples/config/modules/dot.yaml](../docs/examples/config/modules/dot.yaml.md) — declarative dependency.
- [docs/examples/config/modules/foreign-amnezia.yaml](../docs/examples/config/modules/foreign-amnezia.yaml.md) — declarative dependency.
- [docs/examples/config/modules/hysteria.yaml](../docs/examples/config/modules/hysteria.yaml.md) — declarative dependency.
- [docs/examples/config/modules/mtproto.yaml](../docs/examples/config/modules/mtproto.yaml.md) — declarative dependency.
- [docs/examples/config/modules/trojan-tls.yaml](../docs/examples/config/modules/trojan-tls.yaml.md) — declarative dependency.
- [docs/examples/config/modules/tuic.yaml](../docs/examples/config/modules/tuic.yaml.md) — declarative dependency.
- [docs/examples/config/modules/vless.yaml](../docs/examples/config/modules/vless.yaml.md) — declarative dependency.

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
