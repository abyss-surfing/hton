# bin/hton

Stable ID: `cli.entry` · Phase: `R1` · Kind: source

I3-A dispatcher/help source written; command integration remains incomplete. Purpose: Thin public command dispatcher

Implementation wave: `I3-A` (planned ownership, not completion).

## Features and responsibility

- CLI

## Methods, arguments and result

- `dispatch` — arguments: command, argv; result: command exit code.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- Resolve project root from executable location.
- Select only declared commands; pass arguments without eval.
- Do not bootstrap/render while accepted runtime starts.

## Calls / reads

- [scripts/help](../scripts/help.md) — `dispatch` -> `run`.
- [scripts/install](../scripts/install.md) — `dispatch` -> `run`.
- [scripts/deploy](../scripts/deploy.md) — `dispatch` -> `run` for explicit local release delivery.
- [scripts/install-ubuntu](../scripts/install-ubuntu.md) — `dispatch` -> `run`.
- [scripts/install-unknown](../scripts/install-unknown.md) — `dispatch` -> `run`.
- [scripts/config](../scripts/config.md) — `dispatch` -> `run`.
- [scripts/feature](../scripts/feature.md) — `dispatch` -> `run`.
- [scripts/update](../scripts/update.md) — `dispatch` -> `run`.
- [scripts/diag](../scripts/diag.md) — `dispatch` -> `run`.
- [scripts/keys](../scripts/keys.md) — `dispatch` -> `run`.
- [scripts/share](../scripts/share.md) — `dispatch` -> `run`.
- [scripts/certs](../scripts/certs.md) — `dispatch` -> `run`.
- [scripts/restart](../scripts/restart.md) — `dispatch` -> `run`.
- [scripts/service](../scripts/service.md) — `dispatch` -> `run`.
- [scripts/switch](../scripts/switch.md) — `dispatch` -> `run`.
- [scripts/remap-undo](../scripts/remap-undo.md) — `dispatch` -> `run`.
- [scripts/doctor](../scripts/doctor.md) — `dispatch` -> `run`.
- [scripts/whoami](../scripts/whoami.md) — `dispatch` -> `run`.
- [scripts/validate](../scripts/validate.md) — `dispatch` -> `run`.
- [scripts/runtime](../scripts/runtime.md) — `dispatch` -> `run`.
- [scripts/uninstall](../scripts/uninstall.md) — `dispatch` -> `run`.
- [scripts/tunnel](../scripts/tunnel.md) — `dispatch` -> `run` (far-future only; excluded from earlier execution closure).
- [scripts/modules](../scripts/modules.md) — `dispatch` -> `run`.
- [scripts/up](../scripts/up.md) — `dispatch` -> `run`.
- [scripts/down](../scripts/down.md) — `dispatch` -> `run`.
- [scripts/upgrade](../scripts/upgrade.md) — `dispatch` -> `run` (later-management only).

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I3-A dispatch binding (2026-11-01)

Dispatch uses a fixed public command allowlist; paths, internal OS installer helpers
and arbitrary executable names are rejected with usage exit 2. Global flags before
the command are retained as individual arguments; no eval or config sourcing.
-h/--help is intercepted before any command execution. Help labels incomplete host
composition. Until replacement, the legacy install/uninstall/whoami scripts are not
selected as target R1 implementations: they source .env or bypass current contracts.
Missing command bindings return dependency exit 3; deferred commands return failure
without effects. No fallback to legacy installer or dynamic build/download occurs.

Shared allowlist dependency: [lib/hton](../lib/hton.md) — `dispatch` → `hton_public_command`.

Install's publication inputs remain internal Core operation arguments; the
dispatcher exposes no new public domain/port flags. `scripts/install` owns bounded
TTY collection after Core reports missing values.


## I3 CLI consistency binding (2026-11-01)

Shared shell.project owns hton_bounded_arguments (128 argv, 4096 bytes each,
64 KiB total), hton_parse_common (canonical idempotent common flags, separate
positionals/help/yes metadata), hton_valid_id and fixed command availability.
No argument is evaluated or sourced. Known unfinished bindings return 3 even if
an unrelated executable happens to exist at scripts/<name>. Each declared command
has command-specific side-effect-free help; Core composition includes config,
modules, runtime and read-only diag/whoami/doctor observation. Other known
unfinished operations return their explicit Core incomplete result. Direct cmd
scripts share the same common parser.
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
