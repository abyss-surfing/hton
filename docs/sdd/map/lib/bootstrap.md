# lib/bootstrap

Stable ID: `shell.bootstrap` · Phase: `R1` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Fixed host bootstrap/unit actions

Implementation wave: `I3-A` (planned ownership, not completion).

## Features and responsibility

- installer

## Methods, arguments and result

- `ensurePackages` — arguments: distro, dryRun; result: package evidence.
- `checkDependencies` — arguments: required tools; result: missing list.
- `serviceAction` — arguments: fixed action; result: status.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- No remote pipe-to-shell installer.
- Do not stop or reconfigure foreign services implicitly.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [scripts/install-ubuntu](../scripts/install-ubuntu.md) — `run` -> `ensurePackages`.
- [scripts/install-unknown](../scripts/install-unknown.md) — `run` -> `checkDependencies`.

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
unknown OS prints primary-source instructions. `scripts/deploy` copies current working-tree
product files using its bounded allowlist, excluding `.git`, `docs/_lang/ru/tasks/`, generated
output and mutable state; no release manifest is required. `lib/bootstrap` installs
OS dependencies only. No unverified download, VPS compilation, registry/secret writes
or implicit service startup occurs. Installer reconciliation is the fixed
install.reconcile Core operation.

The current minimum setup also invokes this bootstrap before starting its Python
Core, so a fresh server gets PyYAML, cryptography and Docker before module imports
or runtime checks. The fixed package/tool inventory includes OpenSSL for certificate
issuance and ingress validation. Ordinary package-install consent still applies;
minimum previews skip bootstrap and perform no package changes.

`ensurePython` is the shell-only prerequisite for initial install/setup dispatch.
After argument validation, missing `/usr/bin/python3` triggers root reentry and
ordinary package-install consent on Ubuntu/Debian, then installs only `python3`
through apt before JSON target resolution. Unknown systems report manual setup;
dry-run reports the missing interpreter without elevation or package changes.
