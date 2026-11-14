# scripts/help

Stable ID: `cmd.help` · Phase: `R1` · Kind: source

I3-A dispatcher/help source written; command integration remains incomplete. Purpose: CLI usage

Implementation wave: `I3-A` (planned ownership, not completion).

## Features and responsibility

- CLI
- safety

## Methods, arguments and result

- `run` — arguments: --help; result: exit code / redacted result.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- Print declared commands and implemented/unavailable status.
- `doctor` is described as local read-only observation only; fix/watch are listed as unavailable until their owners are implemented.
- Install public argv remains unchanged; missing non-secret inputs are prompted only after Core's no-effects `inputRequired` response, while DNS secretRef stays in saved configuration and exact foreign/443 approvals are not supplied by `--yes`.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [bin/hton](../bin/hton.md) — `dispatch` -> `run`.

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

Shared allowlist dependency: [lib/hton](../lib/hton.md) — `run` → `hton_public_command`.


## I3 CLI consistency binding (2026-11-01)

Shared shell.project owns hton_bounded_arguments (128 argv, 4096 bytes each,
64 KiB total), hton_parse_common (canonical idempotent common flags, separate
positionals/help/yes metadata), hton_valid_id and fixed command availability.
No argument is evaluated or sourced. Known unfinished bindings return 3 even if
an unrelated executable happens to exist at scripts/<name>. Each declared command
has command-specific side-effect-free help; fixed Core composition currently includes
config/modules/runtime plus diag/whoami/doctor observation. Other operations report
their explicit unavailable result rather than claim completion. Direct cmd scripts share the same common parser.
Config accepts documented --file/--plan syntax and enforces operation applicability
before unavailable outcomes; modules/runtime enforce ID/cardinality constraints.
Repeated boolean common flags are normalized, not forwarded repeatedly to strict
helpers. --yes remains ordinary-prompt metadata and grants no consent. Legacy
functions remain defined for unmigrated direct scripts but target commands do not
call them. Static Bash syntax checks are not product U0/F0 acceptance.

`install` retains its current argv shape; initial missing site/foreign host-port inputs are gathered by a bounded prompt. `--yes` does not replace install review or public/foreign consent.
