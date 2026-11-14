# agent/src/probes.rs

Stable ID: `agent.probes` · Phase: `R2` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Home-vantage bounded diagnostic checks

Implementation wave: `deferred-selected-release` (planned ownership, not completion).

## Features and responsibility

- diagnostics

## Methods, arguments and result

- `check` — arguments: approved endpoints, paths, budget; result: timestamped evidence.

## Structures and injected dependencies

ProfileProbeRequest/MeasuredProfileEvidence; bounded local observations

Dependencies: approved endpoints/profiles, clock, bounded native probe runner; no route ownership.

## Logic

- Real UDP checks where claimed; DNS/TLS/HTTP independent.
- Unknown cause stays unknown; no browsing payload capture.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [agent/src/main.rs](main.rs.md) — `run` -> `check` (R2 only).
- [tests/integration/client-ipv6.sh](../../tests/integration/client-ipv6.sh.md) — `verify` -> `check` (R2 only).

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
