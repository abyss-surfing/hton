# scripts/diag

Stable ID: `cmd.diag` · Phase: `R1` · Kind: source

Bounded read-only local evidence binding; unsupported sections remain explicit.
Purpose: Read-only normalized evidence

Implementation wave: `I3-A` (planned ownership, not completion).

## Features and responsibility

- CLI
- safety

## Methods, arguments and result

- `run` — arguments: --section, --verbose, --json; result: exit code / redacted result.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- Distinguish local observation, external probe and unknown.
- Report desired/pending/accepted and policy readiness separately.
- `diag.observe` never performs public network queries; only registered Docker/systemd
  resources and durable runtime metadata are observed (at most 128 resources).
- Unsupported sections are listed as unverified; no native locators, fingerprints,
  secrets or backend output are returned.

## Calls / reads

- [core/src/checks.rs](../core/src/checks.rs.md) — `run` -> `observe`.
- [core/src/host/observations.rs](../core/src/host/observations.rs.md) — `run` -> `diagnose`.
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
