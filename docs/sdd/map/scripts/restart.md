# scripts/restart

Stable ID: `cmd.restart` · Phase: `R1` · Kind: source

Fixed shell binding. Purpose: request restart of selected accepted owned services

Implementation wave: `I3-A` (CLI binding present; final acceptance pending).

## Features and responsibility

- CLI
- safety

## Methods, arguments and result

- `run` — arguments: services, --wait; result: exit code / redacted result.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- Catalogue service IDs only; Core resolves exact installed Owned registrations and the accepted bundle. Omitted IDs select enabled optional/data services and exclude core ingress.
- Runtime re-admits the current accepted bundle and restarts exact Owned service resources under the current epoch; no arbitrary native locator or StopAll path.
- Require post-restart Running status and admitted launch match, then run declared semantic probes. Unknown/unsupported evidence remains Partial with receipts/survivors retained.
- `--wait` is range-checked (1..300); Runtime action timeouts remain bounded by the operation budget.

## Calls / reads

- [core/src/host/mod.rs](../core/src/host/mod.rs.md) — `run` -> fixed operation dispatch.
- [core/src/host/lifecycle.rs](../core/src/host/lifecycle.rs.md) — `run` -> journaled restart plan.
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
