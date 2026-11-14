# core/minimum/client_page.py

Stable ID: `core.minimum.client-page` · Selected package: `M1-D` · Status: M1 contract; OR-49 path generation and receipt redaction implemented in source (unit evidence only)

Client-only inventory snapshot and static page generation/refresh. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
Implemented methods do not imply product/live acceptance.

## Methods and typed results

- `generate_path() -> one CSPRNG URL-safe segment` (`secrets.token_urlsafe(24)`, at least 128 bits, not reserved).

- `plan(page settings, accepted typed inventory, host policy) -> ClientPagePlan`.

- `render(plan, current typed inventory) -> escaped bounded HTML + digest`.

- `refresh(accepted snapshot, native observations, journal, Store) -> ClientPageReceipt or withdrawn`; the persisted receipt omits the bearer path and the in-memory return carries it to the route writer.

## Calls

- `refresh` -> `core.minimum.client-page.plan`.

- `refresh` -> `core.minimum.client-page.render`.

## Called by

- `core.minimum.install.main` -> `plan`.

- `core.minimum.install.main` -> `render`.

- `core.minimum.install.main` -> `refresh`.

- `core.minimum.install.main` -> `generate_path`.

- `core.minimum.client-page.refresh` -> `plan`.

- `core.minimum.client-page.refresh` -> `render`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.
