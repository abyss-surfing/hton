# Minimum trusted website and Amnezia installation

Current 2026-11-06 planning authority: [M1 implementation plan](plans/INSTALLATION-IMPLEMENTATION.md).
It freezes OR-37–OR-45 and supersedes conflicting older design/default descriptions
below; historical source/build/test observations remain historical evidence. No
new M1 product implementation, build or acceptance is claimed.

Run from the current working checkout:

```sh
scripts/install
```

The script requests sudo itself, deploys the minimum runtime closure into the
configured directory (default `/opt/hton`) and continues from that protected
deployment. Full-R1 Rust sources, module descriptors, schemas, build inputs and
build-only scripts are not delivered to a minimum target. No release manifest,
clean Git tree or Rust helper build is required. `/opt/hton` excludes `.git` and
`docs/tasks`.
Run the same command again to resume or verify; the random loopback backend port
in 5000–6000 stays assigned to this installation.

Prerequisites: Linux/systemd, Python 3 with **PyYAML 6.x** (`python3-yaml`; see
`python/minimum/requirements.txt`), Docker with boot startup enabled, curl,
OpenSSL and the system CA trust bundle. The installer and CLI helpers run as
`/usr/bin/python3 -E -s -B` (ignores `PYTHONPATH`, no user site, no
bytecode), so no environment variables affect imports.
On Ubuntu/Debian, normal installation offers to install missing system packages
with root/sudo and ordinary consent (`--yes` accepts package installation).
Missing Python 3 is installed before target resolution; the remaining dependencies
are installed before minimum Core starts. Previews install nothing and require
Python 3 to be present. Docker must be enabled for startup after provisioning.
The site DNS must point to this server, with external TCP/443 allowed. This code
does not perform an automatic public IP/DNS identity lookup. A stale AAAA record
can send browsers to another server; manage DNS through your provider.

The default issuer is Let's Encrypt with HTTP-01. No DNS API account/key is
required. Keep public TCP/80 reachable for initial issue and future renewal. An existing
HTTP server can serve the challenge through a verified webroot; it is not stopped
or reconfigured. A temporary standalone listener is used only when the port is
free; TCP/443 remains HAProxy's shared entry. Before
first issue, explicit setup compares public A/AAAA with the configured direct
entry and shows generic manual DNS instructions when needed. Configure
`core.edge.publication.addresses` for NAT or a known external entry; an observed
outbound IPv4 is only the default for a direct VPS. Public DNS checks do not run
from background renewal. This minimum IPv4 profile requires no AAAA publication.

DNS-01 is an explicit alternative through `core.certificates.challenge: dns-01`.
Its currently implemented API adapter is InternetBS; other adapters are pending.
Only that selected DNS mode asks for its API key/password or accepts
`--dns-credentials /absolute/private.json` with exactly `apiKey` and `apiPassword`.
Never send secrets in chat. Switching an accepted/credentialed installation to
another mode requires explicit migration; an unissued legacy minimum attempt
without keys is automatically moved to HTTP-01. Saved native port/state is retained.

The existing default container is `amnezia-xray`; `--amnezia-container` selects an
exact name or full ID. The supported native configuration is one VLESS REALITY
TCP/443 listener in a no-mount bridge container, with native TCP/UDP 443 publication
and no Compose/Swarm controller. Existing dynamic local bridge dependencies,
including Amnezia DNS, are preserved by network ID. Fixed IPAM requests and
unsupported network drivers remain blocked. Qualification rejects unsupported layouts before
stopping the VPN. The operator sees a short explanation, container identity and target address,
then types `y` at `Move Amnezia and continue? [y/N]` to approve locally. Empty input
cancels. `--yes` does not bypass this dedicated authorization.

The installer preserves native keys/config, retains the exact original container,
and captures its writable image plus a private durable archive. HAProxy listens
on public TCP/443: the site SNI goes to Caddy HTTPS with a publicly trusted cert;
native REALITY SNI goes unchanged to `127.0.0.1:<saved-port>`. Site and native SNI
must differ. Site content is a bounded snapshot of `/var/www/index.html`, or a
built-in placeholder when absent. This minimum supports that single HTML file.
It does not publish arbitrary static assets or arbitrary native VPN protocols.

Before reporting `ready`, install verifies site certificate/system trust and body,
then performs an authenticated native VPN request through HAProxy. Renewal runs
from a systemd timer and retains the last accepted configuration on failure.
An unfinished cutover has an independent recovery timer; the original container
is retained for exact restoration. Accepted data containers restart via Docker
without a running Core daemon. Other installed VPN containers remain untouched.

Component acceptance uses disposable containers, loopback high ports and a private
CA. Passing it does not mean that production DNS, public reachability or real
certificate issuance have been verified. Those depend on the local installation.
Full R1 management commands do not control minimum-owned state yet.

## Configuration before the first installation

Edit the non-secret source input `platform/etc/install.json`: `installation.directory`
(default `/opt/hton`), `core.edge.domains` and `core.certificates.email`. Public source defaults contain no personal site/email; each installation keeps
its own settings. Run `scripts/install` without repeating those flags.
If either site value is absent from both installed and source configuration, the
installer prompts locally and saves it in `<directory>/config/hton.yaml` before
asking for native remap approval. Existing installed values take precedence over
source inputs and remain intact on later deploys. DNS API keys belong only in the
private secret store, never this source configuration.

The minimum target can be a canonical direct child of `/opt`. Choose it before
the first installation; changing the source target does not migrate active data
or reconfigure an existing accepted installation. Deployed recovery/renewal keeps
using its exact protected root. Full R1 custom-directory support is not promised.

## Optional MTProto, Hysteria, own Outline WSS and AmneziaWG2

Four protocols are opt-in. A first selection with no optional-protocol keys
enables all of them; `--with` / `--without` on `scripts/install` or
`scripts/setup` override that choice:

```sh
scripts/install
```

On the first selection this enables MTProto, Hysteria2, own Outline WSS and
AmneziaWG2 without feature flags. The shared REALITY/MTProto SNI defaults to the configured
site mask domain itself; `--chain-sni` is only an optional subdomain override.
MTProto camouflage defaults to the same SNI, Hysteria defaults to UDP/8443 and
the pinned Outline binary path is built in. `--with`/`--without` remain optional
overrides; a saved selection is authoritative. Generated secrets and paths stay under the protected
minimum work tree. If the SNI is a subdomain, the installer requests a
certificate SAN for it and the name must already resolve to the server.

A test-only reverse-tunnel route is available for the Tartarus VPN-blocking
test: `--test-sni DOMAIN --test-tunnel-port PORT` adds one dedicated SNI and
routes it through HAProxy to `127.0.0.1:PORT`, where a taenarum reverse forward must
already listen. It is off by default and is removed with `--no-test-tunnel`.
The port is never published and the SNI must not collide with a production name.

By default the native REALITY listener keeps its legacy name and `dest`, and the
own SNI is served by the separate copied-clients listener, so existing Amnezia
profiles keep working. Switching the native listener to the own SNI is an
explicit opt-in: the operator must type `DROP-LEGACY-SNI`; a plain `y` or `--yes`
does not authorize it. After such a migration the installer offers to restore the
saved previous native configuration on the next interactive install. Unattended
renewal never migrates or restores.

- MTProto uses the pinned `mtg` image. Its secret is generated once and kept
  under the protected work tree. With the neutral chain, `mtg` listens on the
  Docker bridge gateway and sits behind the native REALITY fallback, so the same
  own SNI carries REALITY and MTProto; without it, HAProxy routes only the exact
  configured camouflage SNI to loopback `mtg`. A separate camouflage name must
  differ from the site and Amnezia REALITY names. The share link is printed on
  completion and written to `work/minimum/share/mtproto.txt` (0640
  `root:hton`, readable by the `hton` group).
- Hysteria2 uses the pinned Hysteria server image on UDP/8443 (override with
  `--hysteria-port`), password authentication and the existing trusted site
  certificate. It masquerades unauthenticated QUIC as the site. The authenticated
  client probe runs during cutover. The link is written to
  `work/minimum/share/hysteria.txt` (0640 `root:hton`).
- Own Outline runs the pinned official `outline-ss-server` v1.9.2 binary on a
  loopback web listener with `websocket-stream` and `websocket-packet` paths.
  Caddy reverse-proxies those secret paths under the site TLS certificate, so
  Outline clients v1.15+ use `wss://<site-domain>/<path>` through HAProxy TCP/443.
  `--outline-binary` supplies the pinned binary; its sha256 is verified.
  Each user gets a unique key and a separate
  `work/minimum/share/outline-<user>.yaml` (0640 `root:hton`); the user list is
  `work/minimum/secrets/outline-users.json`, generated from Amnezia client
  suffixes if absent and editable before installation. The server exposes
  loopback Prometheus metrics with an `access_key` label; `hton usage` (or
  `scripts/usage`) reports per-user totals, and a five-minute timer persists
  history in `work/minimum/usage.json`. Use `--without outline` to remove the
  listener.
- AmneziaWG2 runs the pinned AmneziaWG 3.1 userspace image on a dedicated public
  UDP port from 5000-6000. It creates `hton0` inside its own bridge container,
  uses per-peer X25519 and preshared keys, and exports one complete `.conf` per
  native Amnezia user. `Jc/Jmin/Jmax`, `S1-S4` and non-overlapping `H1-H4`
  values are generated once and retained; the client `.conf` contains no server
  `ListenPort`, hooks or `SaveConfig`. Server peers are `/32` only, so the
  endpoint never installs a default route on the host. Per-peer files are
  `work/minimum/share/amneziawg-<user>.conf` (0640 `root:hton`). Use
  `--without amneziawg` to remove the listener while retaining keys.

Protocol containers are owned, labelled, drift-checked and committed with the same
candidate/active receipts as the site and edge. A failed cutover stops candidates
and restores the previously accepted protocol containers. Disabling a protocol
removes its listener on the next successful install but retains its settings and
credentials. `--yes` never replaces the local confirmation for changing a saved
camouflage name, UDP port or Outline binary.

These rules describe the minimum profile only. Full-R1 catalogue entries remain
unimplemented and are not claimed by this section; the minimum-profile Outline WSS
is implemented here but real-client interoperability still needs acceptance.

## Command responsibilities

`scripts/install` must run from the cloned project root. It compares its source
root with the configured target and, when they differ, runs deploy (group,
modes, protected replacing delivery, stale managed-path pruning, migrations),
changes into the configured directory, and invokes that target's
`scripts/setup` as root with `SUDO_USER` preserved. From an installed copy whose
source equals the target it skips delivery and invokes setup directly.
`scripts/setup` refuses a different working directory, then collects missing
parameters/secrets and configures the platform. Each run also applies registered
migrations once: obsolete runtime files are removed, permissions are normalized
and stale managed paths are pruned. Systemd recovery/renewal and the seven-day
unattended update timer remain independent of this interactive wrapper.

Every explicit `scripts/install` also updates the installed release: when the
source root is a Git checkout on `main`, it fast-forwards to `origin/main` before
delivery, then re-pulls the pinned images and recreates owned containers whose
image changed. Local commits, a dirty conflict, a non-main branch, a detached HEAD
or an unavailable remote are reported and the source already present is installed;
install must still work offline. `scripts/update` performs the same fast-forward
first and requires it to succeed, then runs the minimum `update` action for the
HTON-owned composition. Independently installed VPNs and unrelated host containers
are not updated. `update --dry-run` reports the plan without Git, network or
container effects.

The installer enables `hton-minimum-update.timer`, which checks daily at a
randomized quiet hour but acts only when no successful install/update was recorded
for seven days (`work/minimum/update.json`). A fresh explicit install therefore
suppresses the unattended run for that week. The unattended action uses saved
values, never prompts, performs no public DNS/identity observation, requires a
committed installation and cannot perform the first install or a native
migration/remap. A failure retains the last working composition and is retried on
a later daily check. If the installed target is not a Git checkout (deployed from a
separate source directory), the unattended run refreshes pinned images only; run
`scripts/install` from the source checkout to deliver new source.

For an existing HTTP server, setup tries verified public webroots /var/www and
/var/www/html, or the explicitly configured core.certificates.http.webroot. It
creates only a public .well-known/acme-challenge directory when absent and writes
short-lived proof files there. Only that public subdirectory is mounted in the
issuer; account/private-key directories are separate. Unknown/unsafe webroots
require an explicit route/configuration choice, never a service takeover.

## Target M1 differences (planned)

The existing commands described above are M0. M1 adds fresh managed Amnezia,
separate configured protocol names, authenticated upstream DNS and the current
host-response policy. All use the [frozen plan](plans/INSTALLATION-IMPLEMENTATION.md).
No public8443 in the owned composition; selected Hysteria uses UDP443.
The example omits `core.clientAccessPage.path`, so a fresh install generates a
CSPRNG secret URL (at least 128 bits, one safe segment) and enables the page.
An explicit `path: ""` is the operative opt-out; `--rotate-client-page` replaces
the URL immediately. Quiet install never prompts and applies the same absent/empty
rule with noninteractive elevation. Only named client credentials appear on the
bearer page; no admin SSH key/full-access export. The current instance URL is an
operator config value, not a distributed default. Full administration remains TODO.

## First managed install (M1)

1. Fill `.secrets/profile.yml`: site domain/SNI, certificate email and challenge.
   `site.root` points at the website to publish (both flows serve it read-only).
   Leave it empty: install looks for `index.html`/`index.htm` in `/var/www/html`
   then `/var/www` and asks once whether to use that site as the domain default
   (default yes; the answer is saved as `core.site.root` or
   `core.site.useExisting: false`). Quiet, JSON and renewal runs adopt a detected
   site without asking. A configured directory without an index stops the install
   instead of replacing the site; the placeholder is used only when no document
   root exists.
   DNS-01 needs the InternetBS key/password; HTTP-01 needs public TCP/80. `outline-wss`
   is disabled until its pinned `outline-ss-server` binary exists.
2. `scripts/secrets init` (if the profile is absent), edit, then `scripts/secrets import`.
3. Run `sudo scripts/install` from the source checkout.

Until the project-specific managed image is published, provisioning uses the
pinned upstream `ghcr.io/xtls/xray-core` image staged into the Amnezia-compatible
`/opt/amnezia/xray` layout. To keep an existing Amnezia container instead, set
`modules.foreign-amnezia.settings.mode: associate` and select its container.

Everything the installation keeps under `/opt/hton` is readable through the
`hton` group (OR-46): directories are `0750`, files `0640`, owned `root:hton`;
`scripts/install` normalizes `config/`, `work/` and a copied `.secrets/` on every
run. Group members can therefore read share links and stored secrets without
sudo, so membership is credential-equivalent for VPN material. Transient lock
files may stay root-only.

A successful managed installation reports the native Amnezia service in its
summary (`Amnezia: REALITY on TCP 443`), prints the client-access page URL
(`Client access page: https://<domain>/<secret-path>`) and the directory with the
generated files (`Client files: /opt/hton/.secrets/share`), but no individual
credential links. Each guest `vless://` URI is written to
`work/minimum/share/amnezia-<client>.txt` (0640 `root:hton`). An enabled client-access page
also lists the current native clients. After a successful install, the same
accepted guest inventory is published to `/opt/hton/.secrets/share` and to the
source checkout `.secrets/share` when present: one `<user>.md` per user with the
Amnezia link, matching Outline key, AmneziaWG `.conf` and the shared MTProto/Hysteria links, plus
`client-page.md` with the generated `https://<domain>/<secret-path>` URL. These files are `0640`, stay outside the
site and page roots, and are removed or replaced only after another successful
publication; a failed install writes nothing. Browser access to the page is
`GET/HEAD https://<domain>/<secret-path>`; the path is printed only in the local
human install summary (never in JSON, quiet runs or logs) and is not stored in
`work/minimum/client-page/receipt.json`. The root-equivalent
full-access `vpn://` export and the dedicated `hton-amnezia` SSH account required
by OR-37 are not emitted yet: `admin.access` is read but not wired into the
managed transaction.

Re-running the installation does not rotate accepted credentials. The managed
container is recreated only for a changed image, launch shape or REALITY `dest`;
recreation reuses the live native REALITY key pair and the complete current client
array from the container (falling back to the protected `amnezia/server.json`
snapshot), so already imported Amnezia profiles keep working. MTProto, Hysteria and
Outline secrets come from the protected work store and are reused, and an unchanged
profile is not re-imported over the runtime config. Unreadable saved native material
blocks with `recoveryRequired` instead of rerolling keys, and an ambiguous Docker
inspect/copy failure aborts the run rather than removing the container.

On a reinstall the previous site/edge containers (including retained older
generations) and the previous protocol containers are stopped before the new
candidates are started, so the port probes see only the new generation; the old
site, edge, protocols and a previously running native container are restarted if
the transaction fails, and removed only after a committed cutover. Without that
ordering, HAProxy/Caddy `SO_REUSEPORT` could answer a probe from an older
generation and report a body mismatch.


## Native Amnezia clients

The native Xray container stores REALITY clients in
`inbounds[0].settings.clients` in `/opt/amnezia/xray/server.json`.
Use [scripts/clients](scripts/clients.md) to list, add, remove or refresh them.

    ./scripts/clients list
    sudo ./scripts/clients add alice bob carol
    sudo ./scripts/clients rm alice
    sudo ./scripts/clients refresh

Listing requires Docker access. Mutating commands and refresh require root.
An add validates every name before writing, restarts Xray once and creates
one private bundle per user. The protected guest cannot be removed. Refresh
regenerates existing links and bundles from the live configuration while
preserving UUIDs, keys, short IDs, SNI and the running process. Pass names to
refresh specific clients, or omit them to refresh all clients.

Names are case-sensitive, up to 32 characters, and start with a Latin or
Russian Cyrillic letter or a digit. Subsequent characters can also be
underscores, dots or hyphens. The same rule applies to
`foreign-amnezia.settings.provision.clientName`. Bundles live under
`.secrets/share/` with mode 0640. If filename normalization would change
the identity, or the name is reserved, the filename includes a stable suffix.
A successful install replaces previously recorded bundle filenames.

A per-client entry in `core.clientAccessPage.names` overrides the connection
label. Otherwise the display name is `instance.label` followed by `XRay`,
or just `XRay` when no label is configured. The client identity still names
the bundle and the native client record.

An interactive install can migrate saved REALITY subdomains to the site
apex. Regenerate and redistribute affected links with `scripts/clients refresh`.
The `www` alias redirects to the apex. Renewal and unattended update preserve
accepted SNI names. VLESS uses `encryption=none` in its URI and REALITY for
transport security.

If more than one container matches the native Xray command, pass the full ID
through `--container`.
