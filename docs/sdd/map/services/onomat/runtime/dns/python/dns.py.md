# services/onomat/runtime/dns/python/dns.py

Stable ID: `core.minimum.dns` · Selected package: `M1-C` · Status: planned M1 contract

Owned by Onomat; HTON consumes the renderer and applies the private Unbound
service. Private Unbound DNS policy, artifacts and bounded lifecycle. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `plan(typed DNS policy, consumer capabilities, upstream trust) -> DnsPlan or unsupported`.

- `render(DnsPlan, private endpoint, pinned trust artifact) -> UnboundArtifacts`.

- `prepare(admitted plan, artifacts, Docker/store ports) -> DnsReceipt`.

## Calls

- `prepare` -> `core.minimum.docker.command`.

## Called by

- `core.minimum.install.main` -> `plan`.

- `core.minimum.install.main` -> `render`.

- `core.minimum.install.main` -> `prepare`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.
