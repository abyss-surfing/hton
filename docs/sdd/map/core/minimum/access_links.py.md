# core/minimum/access_links.py

Stable ID: `core.minimum.access-links` · Selected package: `M1-B` · Status: planned M1 contract

Pure typed guest/full-access Amnezia export encoding. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `encode(accepted profile, host receipt, scoped key handle, DNS evidence) -> typed client/admin protected artifacts`.

## Calls

No callable dependency in the selected overlay; injected observations/typed data only.

## Called by

- `core.minimum.install.main` -> `encode`.

- `core.minimum.managed-amnezia.client_inventory` -> `encode`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.
