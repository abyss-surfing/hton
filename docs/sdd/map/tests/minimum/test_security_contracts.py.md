# tests/minimum/test_security_contracts.py

Stable ID: `test.minimum-security` · Selected package: `U1` · Status: implemented unit contract (evidence only)

Final host/DNS/page/migration pure and failure scenarios. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
Passing unit tests do not imply product/live acceptance.

## Methods and typed results

- `run(finished implementation, bounded fixtures) -> unit results`.

- Covers generated-path entropy/uniqueness, explicit opt-out, rotation, receipt redaction and private `.secrets/share` publication/stale cleanup (OR-49/OR-52).

## Calls

No callable dependency in the selected overlay; injected observations/typed data only.

## Called by

Entry/declarative/final-evidence artifact; producer/consumer binding is in the plan.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.
