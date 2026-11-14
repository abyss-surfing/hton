# scripts/update

Stable ID: `cmd.update` · Selected package: `M1-E` · Status: OR-55 implemented in the minimum path (unit evidence only)

Fast-forward `origin/main`, then run the non-interactive minimum update of the
HTON-owned composition. Implementation wave: `I3-A`.

## Features and responsibility

- CLI
- safety

## Methods, arguments and result

- `run` — arguments: common flags, `--dry-run`, internal `--auto`; result: exit code / redacted plan or update result.

- `updates.fast_forward(root)` — required fast-forward of `main`; returns bounded status.

- `updates.due(work)` — seven-day policy against the protected stamp.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: `lib/hton` dispatch, `python/minimum/updates.py`, `scripts/deploy` and the minimum Core.

## Logic

- Fast-forward `main` to `origin/main` before any delivery or container effect.
- Deploy when the source and configured target differ; otherwise update in place.
- Run the minimum `update` action (pinned images re-pulled; changed owned containers recreated); foreign containers are untouched.
- `--auto` skips work until seven days have passed since the last successful install/update.
- Unattended runs never prompt or perform public DNS/identity observation; native client-list drift blocks and requires an explicit install.

## Calls / reads

- [core/minimum/updates.py](../core/minimum/updates.py.md) — `run` -> `fast_forward`, `due`.
- [core/minimum/install.py](../core/minimum/install.py.md) — `run` -> `main(update)`.
- [scripts/deploy](deploy.md) — `run` -> `run` when source and target differ.

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

## OR-55 implementation binding (2026-11-07)

`scripts/update` now implements the fixed OR-55 operation for the minimum
profile. `run` fast-forwards `main` to `origin/main` through
`python/minimum/updates.py` (`fetch origin main` + `merge --ff-only`), then
deploys from the checkout to the configured target when they differ and invokes
the minimum `install.py update` action. The action re-pulls pinned images and
recreates changed HTON-owned containers only; independently installed VPNs and
unrelated host containers are outside scope. A failed fast-forward stops before
deploy/container effects. `--auto` adds the seven-day schedule check against
`work/minimum/update.json`; the generated `hton-minimum-update.timer` invokes
`scripts/update --auto --quiet` daily and the action is due only after a week
without a successful install/update. `--dry-run` reports the plan without Git,
network or Docker effects. Unattended update never prompts, performs no public
DNS/identity observation and requires committed state. The elevated minimum
`update` runs with the invoking checkout as `HTON_INSTALL_SOURCE`, so an update
started from a checkout also refreshes its private `.secrets/share` artifacts
when that directory exists. See OR-55/D160.
