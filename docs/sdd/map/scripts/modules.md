# scripts/modules

Stable ID: `cmd.modules` · Phase: `R1` · Kind: source

Read-only modules list binding written; mutating command composition remains planned. Purpose: Installed module lifecycle and resumable setup

Implementation wave: `I3-A` (planned ownership, not completion).

## Features and responsibility

- cmd

## Methods, arguments and result

- `run` — arguments: list|add|enable|disable|remove|purge|update, IDs, --save-only, --json; result: exit code and OperationResult.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- Use Core lifecycle directly without API daemon.
- Public syntax and exit codes are owned by docs/CLI.md.
- Report partial states/errors explicitly; preserve typed request/idempotency and consent scope.

## Calls / reads

- [core/src/lifecycle.rs](../core/src/lifecycle.rs.md) — `run` -> `plan`.
- [core/src/lifecycle.rs](../core/src/lifecycle.rs.md) — `run` -> `execute`.

## Called / read by

- [bin/hton](../bin/hton.md) — `dispatch` -> `run`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

I3-A: modules list invokes fixed modules.list through shell.project.invokeHelper.
It accepts common JSON/quiet/dry-run flags; no IDs on list. Mutating subcommands
remain unavailable until production lifecycle ports are composed; no legacy fallback.

Read-only call: [lib/hton](../lib/hton.md) — run → invokeHelper(modules.list).


## I3 CLI consistency binding (2026-11-01)

Shared shell.project owns hton_bounded_arguments (128 argv, 4096 bytes each,
64 KiB total), hton_parse_common (canonical idempotent common flags, separate
positionals/help/yes metadata), hton_valid_id and fixed command availability.
No argument is evaluated or sourced. Known unfinished bindings return 3 even if
an unrelated executable happens to exist at scripts/<name>. Each declared command
has command-specific side-effect-free help; only config/modules/runtime invoke
their currently composed helpers. Direct cmd scripts share the same common parser.
Config accepts documented --file/--plan syntax and enforces operation applicability
before unavailable outcomes; modules/runtime enforce ID/cardinality constraints.
Repeated boolean common flags are normalized, not forwarded repeatedly to strict
helpers. --yes remains ordinary-prompt metadata and grants no consent. Legacy
functions remain defined for unmigrated direct scripts but target commands do not
call them. Static Bash syntax checks are not product U0/F0 acceptance.


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
