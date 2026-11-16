# core/src/config.rs

Stable ID: `core.config` · Phase: `R1` · Kind: source

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Data-only parsing/schema normalization/revisions

Implementation wave: `I1-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- config

## Methods, arguments and result

- `load` — arguments: configRoot, supportedFormatVersions; result: DesiredReadResult: coherent snapshot or document diagnostics.
- `validate` — arguments: DesiredSnapshot, ModuleCatalogue, RegistrySnapshot, approvals; result: ValidationReport with normalized candidate or blockers.
- `initial_desired` — instance ID, explicitly selected modules, coherent expected-absent `ReadSet`, trusted catalogue → virtual `DesiredReadResult`; pure, rejects present/incoherent/missing known absences.
- `root_document` — `DesiredSnapshot` → canonical root `serde_json::Value` for serialization; preserves the same defaults consumed by virtual preflight.

## Structures and injected dependencies

ConfigDocument/DesiredReadResult/NormalizedSnapshot; bounded reader, YAML decoder and deterministic normalizer

Dependencies: filesystem read port, schema catalogue, module check aggregator, secret availability evidence.

## Logic

- Load root/module/Docker inputs as one content-bound read set; detect changes during read.
- Parse bounded data-only YAML; reject duplicate/unsafe keys/tags, traversal and symlink escape.
- Unknown values are reported and preserved; R1 never deletes/migrates them automatically.
- Root feature objects contain enabled; each module settings document has its own schemaVersion and settings.
- Disabled incomplete settings remain editable; only enabled closure requires activation readiness.
- Explicit values precede versioned defaults; no nondeterministic secret generation or env/UI defaults.
- Canonical initial root defaults live here. `initial_desired` and `root_document` share one typed snapshot; host installation must not keep a duplicate root bootstrap literal. Optional features default off; explicit selected feature IDs are validated against the catalogue and root schema. Descriptor module defaults stay catalogue-owned.
- Malformed managed YAML returns per-document diagnostics instead of breaking tool startup.
- Read/validate is pure; later preservation/migration belongs to standalone configurator.
- Core config remains read/validate-only. The InstallationCoordinator owns the narrowly scoped install-input candidate/commit and uses the shared descriptor-relative storage CAS port; no general config patch writer is introduced.
- Module document moduleId must match the registered object/file identity; mismatch is not implicit rename or adoption.

## Calls / reads

- [schemas/hton-v1.schema.json](../../schemas/hton-v1.schema.json.md) — `validate` -> declarative dependency.
- [schemas/hton-v2.schema.json](../../schemas/hton-v2.schema.json.md) — `validate` -> declarative dependency (R2 only).
- SecretAvailability.version — supplied protected version inventory only; no secret resolution/material read.
- [schemas/module-v1.schema.json](../../schemas/module-v1.schema.json.md) — `validate` -> declarative dependency.
- [core/src/catalog.rs](catalog.rs.md) — `validate` -> `resolve`.
- Trusted ModuleCheck.check ports — pure module checks; Core config owns aggregate validation.

## Called / read by

- [scripts/install](../../scripts/install.md) — `run` -> `load`.
- [scripts/config](../../scripts/config.md) — `run` -> `load`.
- [core/src/main.rs](main.rs.md) — `run` -> `load`.
- [configurator/src/migration.rs](../../configurator/src/migration.rs.md) — `migrate` -> `validate`.
- [controller/src/api.rs](../../controller/src/api.rs.md) — `handle` -> `load`.
- [tests/unit/config-contracts.rs](../../tests/unit/config-contracts.rs.md) — `verify` -> `validate`.
- [core/src/lifecycle.rs](lifecycle.rs.md) — `plan` -> `load`.
- [configurator/src/main.rs](../../configurator/src/main.rs.md) — `dispatch` -> `load`.
- [configurator/src/changes.rs](../../configurator/src/changes.rs.md) — `commit` -> `load`.
- [configurator/src/changes.rs](../../configurator/src/changes.rs.md) — `commit` -> `validate`.
- [core/src/selection.rs](selection.rs.md) — `prepare` -> `load`.
- [core/src/port_allocation.rs](port_allocation.rs.md) — `plan` -> validated settings/read set; config remains read/validate-only.
- [core/src/host/installation.rs](host/installation.rs.md) — current bootstrap seed -> `initial_desired`/`root_document`; virtual first-install preflight is the next integration step.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I1 concrete implementation binding (2026-10-31)

`ConfigSource` is a bounded read/list port over a protected configuration-root
handle; no caller-supplied absolute pathname. `load(source, supportedVersions)`
reads hton.yaml, every known module settings document and editable Docker YAML
input, then repeats inventory and byte reads. Changed inputs return read-conflict,
not a partly accepted snapshot. Stable double reads detect ordinary editor races;
they do not promise atomicity against arbitrary uncoordinated/ABA writers.
`LoadLimits` caps file/total bytes, files, tokens and YAML nesting. Token preflight
rejects anchors, aliases and tags before decoding; a seeded data visitor rejects
duplicate/non-string keys and depth/node excess. Error prose omits source snippets.
Root/module JSON schemas are embedded trusted data; JSON-schema errors expose
known locations/keyword categories only. Unknown keys are preserved in source bytes
and block normalization, rather than silently removed. Module identity is checked.
`normalize` uses only versioned declared defaults, records provenance, and never
allocates secrets. `validate` adds catalogue, registry, approvals and injected pure
module/secret checks. Activation readiness is required only for enabled closure.

Version-1 normalization defaults: omitted feature selections false; installation
systemd true/pathShim false; logging info, `/var/logs/hton`, 64 MiB rotation budget,
7-day retention; onboarding gateway and no preferred target; ingress scope public.
Explicit false, zero and null are never replaced. Defaults are proposed engineering
bindings and recorded as provenance. Existing invalid explicit values stay errors.
Core certificate secrets are activation inputs only when HTTPS domains are configured;
o-domain installation can remain unpublished. Typed in-memory callers are rechecked
against the same schemas as filesystem input before normalization.

## OR-39/OR-40 planning amendment

Read canonical sdd-config, sdd-installer, sdd-foreign-adapters and INTERFACES:
all SNI are config-owned and resolved once; administrative username comes from
`foreign-amnezia.settings.admin.user`. Managed provisioning is separate from
foreign association, starts on private bindings and delegates account effects to
HostAccess. Exact future file ownership and schema changes remain implementation
prerequisites in TASK-AMNEZIA-PROVISION; this mirror does not change live parsers.
