# tests/unit/config-contracts.rs

Stable ID: `test.config` · Phase: `R1` · Kind: test

Planned file binding; no implemented functionality is claimed. Purpose: Schema semantic, duplicate-key, secret traversal and revision conflicts

Implementation wave: `U0` (planned ownership, not completion).

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
- Cover omitted/present malformed enabled, disabled unavailable settings, deterministic defaults, upgrade default diff, dependency cycle and rejected implicit cascade.
- Verify batch dependent removal, shared-engine retention including core consumers, wildcard/dual-stack collisions and equivalent CLI/UI/YAML normalized plans.
- Verify toggle retention versus revoke/purge and stale revision rejection without changing accepted runtime.
- Development order: execute only after the selected project implementation is complete; unit tests precede functional tests, further acceptance is planned from results.
- Cover coherent tree reads, disabled/incomplete modules, unknown-key preservation and deterministic defaults; advanced merge/migration cases belong to later configurator tests.

## Calls / reads

- [core/src/config.rs](../../core/src/config.rs.md) — `verify` -> `validate`.
- [core/src/plan.rs](../../core/src/plan.rs.md) — `verify` -> `plan`.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
