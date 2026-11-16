# tests/integration/legacy-restore.sh

Stable ID: `test.migration` · Phase: `R1` · Kind: test

Planned file binding; no implemented functionality is claimed. Purpose: Native configuration restore without implicit startup; later migration coverage

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

## Calls / reads


## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
