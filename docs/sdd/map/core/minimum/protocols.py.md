# core/minimum/protocols.py

Stable ID: `core.minimum.protocols` · Selected package: `M1-D` · Status: planned M1 contract

Optional protocol-native settings/rendering/lifecycle. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `plan(normalized per-module settings, pins, saved receipts) -> ProtocolPlan`.

- `prepare(admitted plan, certificates, private DNS endpoint) -> ProtocolReceipts`.

- `probe(accepted generation, explicit bounded probe context) -> per-protocol VerificationResult`.

- `client_inventory(accepted protocol receipts, scoped client keys, labels) -> typed ClientCredentialInventory`.

AmneziaWG2 uses `hton0.conf` and listens on container UDP/51820. The stable
public UDP port is used for the Docker publication and guest endpoint. The
native receipt binds the immutable server artifact; the peer key store remains
separate so peer changes can be prepared alongside an accepted generation.

## Calls

- `prepare` -> `core.minimum.docker.command`.

## Called by

- `core.minimum.install.main` -> `plan`.

- `core.minimum.install.main` -> `prepare`.

- `core.minimum.install.cutover` -> `probe`.

- `core.minimum.install.main` -> `client_inventory`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.

## Historical baseline contract (superseded where M1 differs)

# core/minimum/protocols.py

Stable ID: `core.minimum.protocols` · M0 · source

Optional MTProto, Hysteria2 and own Outline-WSS support for the minimum profile.

Contract: [minimum installation](../../../../specs/sdd-minimum-installation.md).
Shared interfaces: docs/INTERFACES.md. Inputs are bounded local values, protected
Store records and exact Docker inspection results; results are verified records or
InstallError without raw native output. State/effects are confined to /opt/hton,
protected minimum work state and exact admitted Docker IDs.

`selected` validates the saved feature map and merges `--with`/`--without`.
`plan` validates module settings, chooses the stable MTProto loopback port,
generates or reuses protected MTProto/Hysteria/Outline credentials, renders
immutable artifacts and verifies the pinned Outline binary digest. Settings
changes require an explicit local confirmation and never reroll silently.

`validate` starts the real pinned mtg, Hysteria and Outline binaries in
read-only isolation; their configuration must load and remain running. `prepare` creates
labelled candidate containers using the shared ingress ownership receipts.
`stop_active`/`start_candidates`/`probe` participate in the cutover transaction.
`promote` changes restart policy and may fail; `accept` is the pure subsequent
state transition, so rollback still has the prior active set. `rollback_candidates`
stops uncommitted containers; `cleanup` removes only receipt-verified abandoned or
retired containers. Share links are written 0600 and never returned in JSON/logs.

`native_selfsteal_config` rewrites only the native REALITY `serverNames`/`dest`
for the OR-35/D133 migration while preserving keys, short IDs, clients and the
listener. `mtproto_port` exposes the persisted port without creating secrets.
With `migrate_native`, the plan omits the separate self-steal container, targets
native `dest` at the Docker bridge gateway (mtg for the neutral chain, otherwise
the site backend) and binds mtg to that gateway.

MTProto routing is exact camouflage SNI to a loopback mtg listener before the
own-domain default; in the migrated neutral chain mtg is reached only through the
native REALITY fallback. Hysteria uses the existing site certificate and explicit UDP
port with password auth and site masquerade. Own Outline runs Shadowsocks over
WebSockets on a loopback listener; Caddy reverse-proxies the secret paths under
the site certificate. Full-R1 catalogue activation and real client
certification remain outside this binding.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
