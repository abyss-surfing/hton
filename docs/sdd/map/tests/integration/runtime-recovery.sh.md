# tests/integration/runtime-recovery.sh

Stable ID: `test.recovery` · Phase: `R1` · Kind: test

Planned file binding; no implemented functionality is claimed. Purpose: Kill control plane, invalid candidate, lease expiry and cold accepted boot

Implementation wave: `F0` (planned ownership, not completion).

## Features and responsibility

- acceptance

## Methods, arguments and result

- `verify` — arguments: isolated fixture + pinned versions; result: test evidence.

## Structures and injected dependencies

Final-phase scenario functions and isolated fixtures

Dependencies: completed release interfaces and controlled fixture ports; run only in U0 or F0 as assigned.

## Logic

- Inject the named failure and verify observable behavior, not source-text similarity.
- No destructive production target or implicit third-party probe.
- Development order: execute only after the selected project implementation is complete; unit tests precede functional tests, further acceptance is planned from results.
- Cover partial up/down, reboot after down, broken desired config, explicit accepted start, and guardian trial versus newer stopped epoch.

## Calls / reads

- [runtime/src/main.rs](../../runtime/src/main.rs.md) — `verify` -> `execute`.
- [runtime/src/guardian.rs](../../runtime/src/guardian.rs.md) — `verify` -> `monitor`.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
