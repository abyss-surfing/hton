# tests/minimum/test_minimum.py

Stable ID: `tests.minimum.unit` · Selected package: `U1` · Status: implemented unit contract (evidence only)

Final existing and new unit contracts. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
Passing unit tests do not imply product/live acceptance.

## Methods and typed results

- `run(finished implementation, isolated fixtures) -> unit results`.

- Covers deployment closure, `--rotate-client-page`/`HTON_INSTALL_SOURCE` forwarding,
  one-week log rotation/pruning and symlink-safe pruning, the seven-day update
  policy/stamp, the daily update unit text and a real temporary-repository
  `fast_forward`.

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

## Historical baseline contract (superseded where M1 differs)

# tests/minimum/test_minimum.py

Stable ID: `tests.minimum.unit` · M0

Unit acceptance of protected settings, ownership, remap and recovery.
Contract: docs/specs/sdd-minimum-installation.md and docs/BUILD-RELEASE.md.
Build has no deploy/service effects. Unit checks use private fixtures. Functional
checks create only labelled exact-ID temporary containers on loopback high ports;
cleanup verifies their per-run label and never changes production resources.
