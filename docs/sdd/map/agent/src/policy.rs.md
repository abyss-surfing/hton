# agent/src/policy.rs

Stable ID: `agent.policy` · Phase: `R2` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Controlled candidate selection

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- failover

## Methods, arguments and result

- `select` — arguments: allowedProfiles, evidence, pin, egressPolicy; result: selected path/reason.

## Structures and injected dependencies

RankedProfile/SelectionReason; future explainable ranking

Dependencies: measured evidence, explicit constraints/manual pin; automatic routing remains separate decision.

## Logic

- Hysteresis/cooldown/backoff and new-flow switching.
- Preserve strict family/direct-fallback policy and no SSH autossh.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [agent/src/main.rs](main.rs.md) — `run` -> `select` (R2 only).
- [tests/integration/client-failover.sh](../../tests/integration/client-failover.sh.md) — `verify` -> `select` (R2 only).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
