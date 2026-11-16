# core/minimum/managed_amnezia.py

Stable ID: `core.minimum.managed-amnezia` · Selected package: `M1-B` · Status: planned M1 contract

Managed native configuration, provisioning and lifecycle. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `plan(normalized config, pins, observations, receipts) -> ManagedAmneziaPlan`.

- `prepare(plan, journal, scoped secret handles) -> NativeReceipt`.

- `reconcile(plan, receipt, current native snapshot) -> ManagedServiceReceipt`.

- `client_inventory(accepted native client set, configured labels, name policy, link label) -> typed ClientCredentialInventory`. `link_label` is a display prefix; the fragment is `<link_label> XRay` or `XRay`; the credential display name/id stays the identity.

- `connection_fragment(label) -> str` — app connection name for the native protocol (OR-60).

## Calls

- `prepare` -> `core.minimum.docker.command`.

- `prepare` -> `core.minimum.docker.qualify`.

- `reconcile` -> `core.minimum.docker.snapshot`.

- `client_inventory` -> `core.minimum.access-links.encode`.

## Called by

- `core.minimum.install.main` -> `plan`.

- `core.minimum.install.main` -> `prepare`.

- `core.minimum.install.main` -> `reconcile`.

- `core.minimum.install.main` -> `client_inventory`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.
