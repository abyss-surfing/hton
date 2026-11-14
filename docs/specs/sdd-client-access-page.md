# SDD client access page — passwordless client-key delivery

Authority: OR-45/OR-49, D146–D147, D154, D157. M1 read-only addition, not
admin/accounts/API work. Existing Caddy serves an immutable client-only page.
Contract/file ownership: [implementation plan](../plans/INSTALLATION-IMPLEMENTATION.md),
[file graph](../sdd/INSTALLATION-FILE-GRAPH.md), config and INTERFACES.

## Configuration and interaction

Root `core.clientAccessPage` has `path: string` (absent by default) and `names: mapping
CredentialId -> bounded display string` (default `{}`). Nonempty path enables the
page; no second enabled flag. Store only the relative path, e.g. `client-access`, not
a full URL/hostname or leading slash. Names override inventory labels without
changing credential identity. Future credentials inherit their native/export names;
missing native display names use a stable explicit identifier, not a guessed person.

Install distinguishes absent from empty:

- **absent `path`** is the default and means “publish automatically”. On the first
  install the installer generates one CSPRNG segment (`secrets.token_urlsafe(24)`,
  at least 128 bits) and persists it as `core.clientAccessPage.path` in the runtime
  config and then in `.secrets/profile.yml`. Reinstall, renew and `-q` preserve the
  saved value and never generate a second one.
- **explicit empty `path: ""`** is a deliberate opt-out. It disables the route and
  removes the generated artifact; no automatic generation may override it.
- **`--rotate-client-page`** replaces an existing path with a fresh CSPRNG segment
  and updates the served artifact, so the old URL stops resolving immediately.
  Rotation never runs implicitly on renew or reinstall.

The supplied current instance resolves to `https://example.com/client-access`; examples
shipped as product defaults omit `path` so a fresh install publishes automatically.
An interactive install may ask once before replacing an absent path with the
generated one; declining stores explicit `path: ""`. `-q` never asks and applies the
same absent/empty rule. The bearer URL is written to the private
`.secrets/share/client-page.md` and printed to the local operator console by a
successful install (OR-64); JSON output, receipts, plans and logs never contain
it.

Validate 1–64 ASCII letters/digits/dot/underscore/tilde/hyphen in a nonempty single
segment starting with a letter or digit; reject slash, query, fragment, escapes,
dot segments, controls and collisions with site/admin/API/ACME/WSS/published route
ownership. This includes case-sensitive exact matching; do not casefold the secret
path. Record an internal opaque route ID, not the URL in ordinary status/log/error
output. Caddy route inventory protects this path as secret-bearing config despite
it being outside config/secrets.

## Typed inventory and publication

`ClientCredentialInventory` is produced from accepted active profile state, never
unapplied drafts or arbitrary files in share/. Each entry has stable credentialId,
profileId/protocol, displayName, status, typed client payload and source revision.
Only active exportable client entries are shown. Inventory covers all managed
Amnezia clients, each Outline user, configured MTProto and Hysteria credentials,
and other selected supported exports. Shared protocol credentials are labeled as
shared rather than falsely attributed to individual people. For associated/native
profiles without exportable current material, mark coverage unavailable; do not
fabricate/recover admin secrets or claim page completeness. If an enabled profile's
promised client list cannot be established, page generation blocks/degrades with
explicit coverage metadata while retaining unrelated accepted VPN/site services.

Allow only typed client credentials: guest VLESS/native client profiles, ss/Outline
client YAML, Telegram client links and Hysteria client URLs. A vpn:// scheme is not
sufficient proof of guest status: full-access envelopes containing SSH user/private
key/password/server administration fields are forbidden. Exclude admin.vpn, host
SSH key, REALITY server private key, root passwords, API/DNS/ACME credentials and
management URLs categorically. Server codecs own guest encoding; page code cannot
parse arbitrary exports into trusted client entries or guess role by filename.

`client_page.plan(config, accepted ClientCredentialInventory, HostResponsePolicy)`
is pure -> ClientPagePlan(coverage, routeId, source revisions) / pathConflict,
invalidPath, clientCoverageIncomplete, forbiddenCredential. `render(plan, inventory)`
returns protected UTF-8 HTML plus SHA256, escaping labels and all attribute/text
values, with bounded rows/bytes (256 entries, 1 MiB). Coordinator writes it via
Store and supplies a dedicated Caddy-readable immutable artifact, not the entire
secret store. Private keys remain in owner scopes; intended client payloads become
public only on this explicitly enabled bearer route. The full URL is written to the
private `.secrets/share/client-page.md` and is printed by a successful install
in the local human summary (OR-64); JSON output and logs never contain it.

## Same Caddy, exact route and privacy headers

Serve only GET/HEAD at exact `/<path>` on the configured base site host, with
HTTPS and Host/SNI match. Other methods return generic 405; descendants/trailing
slash/encoded lookalikes do not expose the artifact and follow ordinary site policy.
The route precedes the website file server but never applies to REALITY-denial or
wildcard redirect hosts. Do not create a matching file in /var/www/html, a public
symlink, directory listing or a generic /share endpoint. Existing site data is not
modified. Enable/disable and artifact switch are admitted with the same generation.

Set `Cache-Control: no-store`, `Referrer-Policy: no-referrer`, `X-Robots-Tag: noindex,
nofollow, noarchive`, `X-Content-Type-Options: nosniff`, and CSP allowing no scripts,
frames or external resources (`default-src 'none'; frame-ancestors 'none'; base-uri
'none'; form-action 'none'`). Page is simple HTML; no JS/CDN/analytics/third-party QR
service. No robots.txt/sitemap link that would advertise the secret URL. No route
access log, key-bearing error/log/body snippet, or automatic URL print by install.
These reduce accidental exposure but do not erase browser history or revoke copies.
The URL is a shared bearer secret and the owner-selected short path can be guessed;
no claim of password-equivalent protection. Changing URL revokes page access only,
not credentials already copied; actual keys need native revocation.

## Private share publication (`.secrets/share`)

After a successful `install`, the same accepted guest inventory is written as
private Markdown/Outline artifacts in the runtime secrets repository
`<installation>/.secrets/share` (`0640`, group-readable, outside every site or
page root) and copied into the source checkout `.secrets/share` when that private
repository exists. Share output is one `<user>.md` bundle per user containing
the Amnezia link, a matching Outline key and the shared MTProto/Hysteria links,
plus `client-page.md` with the bearer URL and an explicit warning that it is a
secret. Protocol-per-file names are superseded (OR-58/D164). Publication is
transactionally after success: a failed install writes nothing, and stale files
recorded from the previous published set are removed. The artifact set never
contains admin keys, host SSH material or private server keys. The URL path is
not written into `work/minimum/client-page/receipt.json` or `secret-share.json`;
those receipts list only safe artifact names and coverage metadata.

## Refresh, revocation and lifecycle

Install writes page only after accepted credential/topology state. Reinstall
rebuilds from current native state and same names/path; renewal does not generate
new credentials or silently publish drafts. HTON-controlled client revocation
withdraws the page before changing native keys, then publishes the accepted new
inventory; do not restore a stale page containing a revoked credential on rollback.

External Amnezia edits do not use HTON's lock. Page is explicitly a generated
snapshot, not a live API. Include generation time/coverage on the page. A bounded
local client-page refresh timer reconciles exact accepted native credential state,
withdraws the route/artifact on unknown/conflicting state, and never changes VPN
configuration. Interval is `core.clientAccessPage.refreshSeconds` (default 60,
range 10–3600); no public network requests. A key revoked in the native backend
already stops working; snapshot omission/label updates may lag until refresh.
OR-61/D167: page availability is owned by `core.clientAccessPage` path/refresh
validation alone. An unrelated blocking diagnostic elsewhere in the configuration
(for example a wildcard domain paired with `http-01`) must not withdraw an already
accepted page; the refresh timer parses the page settings directly when full
validation has no `desired` root and keeps publishing. The page remains one shared
bearer admin URL for every user; per-user URLs are not introduced.
A newly added client appears on next accepted refresh. State cannot truthfully be
called instantaneous; copied valid keys remain reusable until native revocation.
Disable/down stops page refresh and serving under the saved server run intent;
remove retains protected history, purge removes explicitly owned retained artifacts.

## Final scenarios

U0 after full implementation: absent/explicit-empty/valid/invalid/colliding path,
generated-path entropy and uniqueness, quiet input policy, rotation, client/admin
payload separation, escaping, deterministic inventory, bounded output, coverage
gaps, revocation fences and generation races, redacted receipts and private share
files. F0 after U0: actual named client exports work; no admin payload is
retrievable; exact apex URL/headers/methods; foreign/wildcard/denial hosts and
encoded paths cannot retrieve it; disable/rotate/reinstall/down/reboot/refresh and
app add/revoke work with documented freshness; `.secrets/share` contains only the
active per-user guest files and `client-page.md`.
No full admin, account sessions or one-time secret-delivery semantics are added.

### Snapshot availability limit

If the local refresh timer/service itself fails, a static last-generated page may
remain until the coordinator withdraws its route; there is no request-time API or
JavaScript freshness enforcement. Report generation time and refresh health rather
than promise instantaneous withdrawal. Native revocation remains authoritative
regardless of stale HTML. Future live authenticated administration is separate TODO.
