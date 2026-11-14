# core/minimum/docker.py

Stable ID: `core.minimum.docker` · Selected package: `M1-B` · Status: planned M1 contract

Exact native observations and fixed Docker mechanics. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `qualify(exact ID, typed associated/managed layout, budget) -> NativeObservation or unsupported`.

- `command(fixed bounded argv, budget, protected I/O) -> bounded result or InstallError`.

- `snapshot(exact ID/fingerprint, snapshot intent) -> ProtectedSnapshotReceipt`.

## Calls

No callable dependency in the selected overlay; injected observations/typed data only.

## Called by

- `core.minimum.managed-amnezia.prepare` -> `command`.

- `core.minimum.managed-amnezia.prepare` -> `qualify`.

- `core.minimum.managed-amnezia.reconcile` -> `snapshot`.

- `core.minimum.dns.prepare` -> `command`.

- `core.minimum.protocols.prepare` -> `command`.

- `core.minimum.ingress.prepare` -> `command`.

## Implementation amendment (client config copy)

`native_config(container, *, timeout=30)` reads the native Xray config with a
bounded copy budget and an explicit timeout; the client script raises it for
interactive add/remove on a slow daemon. No other authority changes.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.

## Historical baseline contract (superseded where M1 differs)

# core/minimum/docker.py

Stable ID: `core.minimum.docker` · M0 · source

Bounded exact-ID Docker transport, native qualification and launch-preserving remap.

Contract: [minimum installation](../../../../specs/sdd-minimum-installation.md).
Shared interfaces: docs/INTERFACES.md. Inputs are bounded local values or protected
Store records; results are verified records or InstallError without raw native
output. Dependencies follow the family contract. State/effects are confined to
/opt/hton and exact admitted Docker IDs; install alone coordinates phases.

`network_attachments(value, required=False)` normalizes declared attachment
requests, accepting the empty IPAM fields emitted for dynamic addresses by
Docker and rejecting static addresses, link-local IPs and unsupported endpoint
options. `verify_networks(value)`
reads existing exact-ID definitions and verifies local bridge dependencies; the
daemon-owned default `bridge` is exempt because Docker recreates it with a new ID
after an unclean shutdown. `create_payload` includes NetworkingConfig. Fingerprint
v2 hashes declared network names, aliases and gateway priority while omitting the
volatile default-bridge ID; custom dependency IDs remain strict. HTON-owned job
fingerprints retain their existing launch scope. No native network mutation is
authorized. Errors distinguish network/controller blockers from external identity
files.

`native_bridge_gateway(value)` returns the observed default-bridge IPv4 gateway
used as the OR-35 native self-steal destination. `replace_native_config(container,
source)` copies one protected 0600, single-link, caller-owned config file into the
fixed native path through Docker's copy transport; callers must re-read and compare
the digest before activating the container.

NativeError carries fixed operation label, exit code and a closed safe reason.
run drains bounded stdout/stderr; it never formats raw argv, output or input into
errors/logs. Known tool errors become plain text (for example access denied).
