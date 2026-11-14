# core/minimum/migrations.py

Stable ID: `core.minimum.migrations` · Selected package: `M1-A` · Status: planned M1 contract

Bounded registered M0-to-M1 transformations. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `plan(coherent existing config/state, target contracts) -> MigrationPlan + preserved originals`.

- `apply(reviewed plan, read-set, journal, Store) -> MigrationReceipt or conflict`.

## Calls

- `apply` -> `core.minimum.storage.commit`.

## Called by

- `core.minimum.install.main` -> `plan`.

- `core.minimum.install.main` -> `apply`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.
