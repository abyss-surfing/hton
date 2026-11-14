# core/src/main.rs

Stable ID: `core.entry` · Phase: `R1` · Kind: source

Fixed helper CLI binding; remaining operation coverage is tracked in implementation status. Purpose: Structured helper CLI binding, replaceable language

Implementation wave: `I3-A` (planned ownership, not completion).

## Features and responsibility

- control

## Methods, arguments and result

- `run` — arguments: fixed operation, bounded payload; result: structured result and public exit status.

## Structures and injected dependencies

Typed request/result functions defined by methods below; state stays in the owning domain record

Dependencies: explicit ports from INTERFACES; no construction of hidden global config/engine authority.

## Logic

- Routes fixed helper operations into the shared application library.
- No data-plane request-time role.
- Maps `approval.required` to CLI exit 4 while preserving the structured error response; ordinary failures remain exit 1.
- Maps completed `diag.observe`, `doctor.observe` and `identity.whoami` metadata to redacted JSON or
  small human output. Unverified identity evidence is informational (exit 0);
  blocking diagnostic findings return exit 1. `--quiet` removes the routine status
  header but preserves requested report data; JSON is unaffected.
- Fixed `install.selection-offer` is read-only and forwards only optional profile/menu
  hints to the installation offer owner. It may be emitted as JSON for bounded shell
  parsing; no modules/config are written. Its operation ID is not a mutating helper.

## Calls / reads

- [core/src/config.rs](config.rs.md) — `run` -> `load`.
- [core/src/plan.rs](plan.rs.md) — `run` -> `plan` (future operation composition).
- [core/src/apply.rs](apply.rs.md) — `run` -> `apply` (future operation composition).
- [core/src/lib.rs](lib.rs.md) — `run` -> declarative dependency.
- [core/src/host/observations.rs](host/observations.rs.md) — `run` -> `diagnose` / `whoami` through fixed host dispatch.
- [logging/src/lib.rs](../../logging/src/lib.rs.md) — `run` -> `emit` (future installed logging composition).

## Called / read by

- [lib/hton](../../lib/hton.md) — `invokeHelper` -> `run`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I3-A read-only helper binding (2026-11-01)

The installed binary is libexec/hton-core; shell invokeHelper selects the fixed
config.check operation and supplies distribution-relative config root. Bounded
argv accepts only --config-root PATH, --json, --quiet and --dry-run. It opens
FsConfigSource without directory creation, calls load with format version 1 and
normalize with embedded ModuleCatalogue. Output contains redacted issues and
explicit unverified scope; normalized settings/material are never serialized.
Registry/module semantic/secret-version/native qualification remains unverified
until the corresponding production ports are composed; this is not apply evidence.
Exit 0 means the available syntax/normalization checks passed, not installation or
publication readiness. Errors return 1, usage 2. No target-dependent log files,
state initialization, external observation or implicit compiler download occurs.

Additional read-only calls: config.normalize and catalogue.builtin.

Core host-operation transport maps `approval.required` to public CLI exit 4 as
specified in [CLI.md](../../../../CLI.md); usage remains 2 and other operation
failures remain 1. The structured JSON error is still written before process exit.

I3-A modules.list takes explicit --config-root and --work-root plus output flags.
Read-only local context derives actor ID from effective OS UID, without mutation
scopes or consent refs. RegistryStore.inspect reads existing state; absent work/file
is reported absent and permission/corruption errors unavailable. Desired load errors
do not hide registry entries; enabled remains unknown. Lifecycle.list owns the
metadata projection. stdout excludes raw registry diagnostics/resources/checkpoints;
registry error messages are static. Native health remains unverified; read failure
returns 1 with available inventory retained. No journal/store creation or network.

Inventory calls: RegistryStore.inspect and lifecycle.list; no execute/transition.
