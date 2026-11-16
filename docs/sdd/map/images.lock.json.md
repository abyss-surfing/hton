# images.lock.json

Stable ID: `image.lock` · Selected package: `M1-C` · Status: planned M1 contract

Authenticated image/architecture/checksum inventory. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

Declarative artifact; its producer/consumer is specified in the M1 plan.

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

# images.lock.json

Stable ID: `image.lock` · Phase: `R1` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: Exact upstream image/source pin inventory

Implementation wave: `I2-A` (planned ownership, not completion).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- No latest or placeholder digest admitted to runtime.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/render.rs](core/src/render.rs.md) — `compile` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I2 populated inventory

Real registry manifests were fetched read-only on 2026-10-31. Manifest bodies and
inspected linux/amd64 config blobs matched SHA-256 digests; no image was pulled,
started or treated as qualified. Every runtime reference is repository@sha256,
not a mutable tag. sourceTag is provenance of resolution, never execution input.
Platforms list explicit pinned child manifests; an unlisted architecture cannot
be assumed supported. Native validators/clients and release supply-chain checks
remain outstanding in the final sequence.

Sources: [HAProxy official image source](https://github.com/docker-library/haproxy),
[Caddy image source](https://github.com/caddyserver/caddy-docker),
[Xray official registry](https://github.com/XTLS/Xray-core/pkgs/container/xray-core/),
[Unbound pinned source](https://github.com/madnuttah/unbound-docker/tree/f9f1fa3498516b18b05c1e35e5d2fc7f10ef499e),
[acme.sh source](https://github.com/acmesh-official/acme.sh).
Unbound uses the Alpine-derived distroless 1.26.1-1 image; its configured PATH and
pinned Dockerfile identify /usr/local/unbound/unbound.d/sbin/unbound. The serializer
sets that absolute native entry point; validators use sibling unbound-checkconf.
Metadata/source labels are provenance observations, not a reproducible-build proof.
