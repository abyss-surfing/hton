# agent/src/main.rs

Stable ID: `agent.entry` · Phase: `R2` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Optional local connectivity agent

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- smart-connect

## Methods, arguments and result

- `run` — arguments: signed profile, scoped binding; result: bounded agent.

## Structures and injected dependencies

Typed request/result functions defined by methods below; state stays in the owning domain record

Dependencies: explicit ports from INTERFACES; no construction of hidden global config/engine authority.

## Logic

- No VPS-only inference about home connectivity.
- No secret redisclosure, remote shell or arbitrary scanner.

## Calls / reads

- [agent/src/probes.rs](probes.rs.md) — `run` -> `check` (R2 only).
- [agent/src/policy.rs](policy.rs.md) — `run` -> `select` (R2 only).

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
