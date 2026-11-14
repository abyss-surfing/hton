# core/minimum/ingress.py

Stable ID: `core.minimum.ingress` · Selected package: `M1-D` · Status: planned M1 contract

One HAProxy/Caddy generation and route/auth checks. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `configs(resolved name/host policies, website and page artifacts, private endpoints) -> HAProxy/Caddy artifacts`.

- `prepare(admitted artifact manifest, exact resource ownership) -> IngressReceipt`.

- `probe(accepted route matrix, explicit probe context) -> RouteVerification`.

## Calls

- `prepare` -> `core.minimum.ingress.configs`.

- `prepare` -> `core.minimum.docker.command`.

## Called by

- `core.minimum.install.cutover` -> `prepare`.

- `core.minimum.install.cutover` -> `probe`.

- `core.minimum.ingress.prepare` -> `configs`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.

## OR-61/OR-62 implementation amendment

`configs` accepts an optional exact site root. With a root it emits a second Caddy
server `http_redirect` on `0.0.0.0:80`: only `/.well-known/acme-challenge/*` is
served from `/site`, every other request gets `308` to
`https://{http.request.host}{http.request.uri}`. Without a root the server is
absent so standalone HTTP-01 keeps TCP/80. The client-page route carries the
privacy headers as a `headers` handler (first `handle`), not as a route-level
`headers` object that Caddy rejects. Caddy loads only `/certs/fullchain.pem`,
which must cover every own-domain REALITY name; no self-signed REALITY
certificate is rendered or loaded (D169). REALITY browser probes receive the
fixed `DENIAL_BODY` `Access denied` 403, never the site HTML. `www.<domain>` is
an allowed TLS name and returns `308` to `https://<domain>{http.request.uri}`
(D170). A REALITY name equal to the site domain uses `native_selfsteal`: HAProxy
routes the apex to the native backend with the site as backup, and no 403 route
is emitted for the apex.

`webroot` and `prepare` consume the installer-selected document root. An
absent selection serves the placeholder and leaves TCP/80 free. The client
page compares the original request target path byte for byte. HTTPS rejects
Host/SNI mismatches with 421.

## Historical baseline contract (superseded where M1 differs)

# core/minimum/ingress.py

Stable ID: `core.minimum.ingress` · M0 · source

Immutable exact-SNI routing, native validation and authenticated probes.

Contract: [minimum installation](../../../../specs/sdd-minimum-installation.md).
Shared interfaces: docs/INTERFACES.md. Inputs are bounded local values or protected
Store records; results are verified records or InstallError without raw native
output. Dependencies follow the family contract. State/effects are confined to
/opt/hton and exact admitted Docker IDs; install alone coordinates phases.

launch_digest uses Docker-owned nullable OOM-default normalization and normalizes
Docker's restored ``Dns`` empty-list encoding back to its create-time null form.
record_owned writes launchVersion=2. owned accepts exact version-2 digests;
version-1 migration is limited to null/false OomKillDisable and does not change
resource authority. All true/other launch, identity, labels and artifact drift
remains a hard refusal.

configs accepts an optional exact MTProto camouflage/loopback-port binding. The
camouflage route is emitted before the own-domain default and native REALITY route;
the name must differ from site and native SNI. In native self-steal mode
(OR-35/D133) the own SNI is part of the native REALITY names, HAProxy routes it to
the native backend with the site backend as a health-checked backup, no separate
self-steal backend is rendered, and an optional `site_bind` adds the Docker
bridge listener needed by native `dest`. Protocol container lifecycle and
credentials are owned by core.minimum.protocols, not this file.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
