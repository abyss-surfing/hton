# scripts/uninstall

Stable ID: `cmd.uninstall` · Selected package: `M1-E` · Status: planned M1 contract

Thin retained-removal / explicit purge entry. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `run(common flags, optional purge request) -> exit code`.

## Calls

- `run` -> `shell.project.hton_cli`.

## Called by

Entry/declarative/final-evidence artifact; producer/consumer binding is in the plan.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.

## Historical baseline contract (superseded where M1 differs)

# scripts/uninstall

Stable ID: `cmd.uninstall` · Phase: `R1` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Ownership-aware full removal

Implementation wave: `I3-A` (planned ownership, not completion).

## Features and responsibility

- CLI

## Methods, arguments and result

- `run` — arguments: --keep-certs, --keep-env, --foreign, --purge; result: exit code / redacted result.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- Local full maintenance may stop protected core.
- Restore foreign state before purge; retain incompatible/recovery-required originals.

## Calls / reads

- [core/src/plan.rs](../core/src/plan.rs.md) — `run` -> `plan`.
- [core/src/apply.rs](../core/src/apply.rs.md) — `run` -> `apply`.
- [adapters/foreign](../adapters/foreign.rs.md) — `run` -> `restore`.
- [lib/hton](../lib/hton.md) — `run` -> `invokeHelper`.

## Called / read by

- [bin/hton](../bin/hton.md) — `dispatch` -> `run`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


## I3 final shell binding (2026-11-01)

The bounded shell adapter owns syntax/ordinary prompts only. It calls fixed Core
operation IDs with --config-root/--work-root, canonical common flags, -- delimiter
and literal operation argv; no actor JSON, YAML/env sourcing, generic shell or
domain state writer. --yes is ordinary prompt metadata, never foreign/443/purge
consent. All actual module/config/cert/native effects remain under Core/
runtime owners and typed journals. Missing installed helper returns 3, usage 2,
consent 4, native failure propagates. Dry-run never bootstraps dependencies or
writes a journal/distribution/config. Runtime status directly uses independent
runtime; public start/rollback ask Core to compose an admitted typed runtime request.

Bootstrap owns only OS package/distribution mechanics. It accepts Ubuntu/Debian
with literal validated os-release fields; no sourced os-release or pipe-to-shell.
Existing supported Docker is preserved; package conflicts are not auto-removed.
Automatic Docker repo setup uses distro-correct HTTPS origin and signed apt metadata;
unknown OS prints primary-source instructions. Current-source deployment is owned by
scripts/deploy and excludes .git, docs/_lang/ru/tasks/, generated output and mutable state; it
requires no release manifest. Bootstrap does not copy application source or compile
it. No unverified download, registry/secret write or implicit service startup occurs.
Installer reconciliation is the fixed install.reconcile Core operation.
