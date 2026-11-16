# core/minimum/host_access.py

Stable ID: `core.minimum.host-access` · Selected package: `M1-B` · Status: planned M1 contract

Dedicated account/key/sudoers effect owner. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `plan(configured access, account/sshd facts, receipt) -> HostAccessPlan`.

- `reconcile(approved plan, key handle, journal) -> HostAccessReceipt`.

- `deactivate(receipt, current exact facts) -> DeactivatedReceipt`.

- `purge(receipt, explicit purge consent) -> PurgeReceipt`.

## Calls

No callable dependency in the selected overlay; injected observations/typed data only.

## Called by

- `core.minimum.install.main` -> `plan`.

- `core.minimum.install.main` -> `reconcile`.

- `core.minimum.install.main` -> `deactivate`.

- `core.minimum.install.main` -> `purge`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.
