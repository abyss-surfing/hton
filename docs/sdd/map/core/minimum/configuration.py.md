# core/minimum/configuration.py

Stable ID: `core.minimum.configuration` · Selected package: `M1-A` · Status: planned M1 contract

Typed coherent configuration and resolved name/host-policy inventory. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `load(protected config Store, saved state) -> ConfigSnapshot + ReadSet`.

- `validate(snapshot, reservations, native evidence) -> NormalizedInstallConfig or CheckReport`.

- `connection_label(instance mapping) -> bounded printable label or None` — the app
  connection name (normally the country), distinct from the native credential id.

## Calls

- `load` -> `core.minimum.storage.Store` and `parse_document`. The parsed value
  and digest come from the same byte read.

- `provision.clientName` validation -> `core.minimum.clients.NAME_PATTERN` (single
  OR-56 client-name authority).

## Called by

- `core.minimum.install.main` -> `load`.

- `core.minimum.install.main` -> `validate`.

## OR-61 implementation amendment

`client_page(root) -> ClientPage` parses only `core.clientAccessPage` and its
`path`/`refreshSeconds`, so page availability does not depend on unrelated
blocking diagnostics in `validate`. `refresh_client_page` in the install owner
uses this parser directly and uses the accepted site domain when it replaces
the protocol inventory.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.
