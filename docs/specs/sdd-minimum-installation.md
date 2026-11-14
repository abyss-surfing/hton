# Minimum installation execution binding

Authority: OR-15 selects this bounded delivery independently of unfinished R1.
Existing OR-3/8/10/11/12/13/14 protections remain applicable. This is an engineering
binding, not a full R1 completion claim.

For this profile, core/minimum is the standalone Core implementation invoked by
scripts/install after protected source deploy. Python 3 standard library and fixed
Docker interfaces need no missing Rust release or compilation on the VPS. Broader
Rust R1 stays outside this executable closure and must refuse mutations while a
protected minimum-mode marker exists. There are never two active controllers.

Modules: storage owns protected settings/state/journal/locks; docker owns bounded
exact-ID transport, full launch preservation and snapshot/reversible remap;
certificates owns pinned DNS-01 issue and trust/key/expiry validation; ingress owns
immutable HAProxy/Caddy artifacts and local probes; install owns resumable sequencing.
Errors never expose raw native output or secrets. No web/API daemon is required.

Canonical settings remain config/hton.yaml (JSON is valid YAML) and
config/modules/foreign-amnezia.yaml. hostPorts persists one random free 5000–6000
TCP/UDP number; retries never reroll. Protected originals/journal are under
work/minimum. Existing full-R1 runtime state conflicts rather than being overwritten.

Pinned owned HAProxy/Caddy use host network. HAProxy binds IPv4 TCP/443 and routes
only exact site/native REALITY SNI. Caddy terminates site TLS on a saved loopback
port and serves an immutable bounded /var/www/index.html snapshot.
OR-62/D168: when a real site root exists, the same Caddy also listens on
`0.0.0.0:80` with automatic HTTPS disabled; it serves only
`/.well-known/acme-challenge/*` from the site webroot and answers every other
request with `308` to `https://<request-host><request-uri>`. The listener is
omitted for a placeholder install so TCP/80 stays free for standalone HTTP-01;
once it is permanently owned, renewal must use the proved webroot instead.

Amnezia native TCP/UDP 443 is published at 127.0.0.1:saved-port. Remap leaves native config/keys
unchanged; OR-35/D133 adds a separate, explicitly consented own-SNI/dest
migration that preserves keys, short IDs, clients and the listener.
The issuer remains pinned acme.sh. D115 adds Let's Encrypt HTTP-01 without DNS
credentials as the default; DNS-01/InternetBS is an explicit adapter. Protected
credential import never exposes material through chat/argv/logs. Direct HAProxy
TLS remains an unaccepted alternative.

Read-only qualification/host reservations precede effects. Reject mounts, unsupported
networks/native config, external controller metadata and occupied ports. Local TTY
consent binds exact Docker ID/config/launch, SNI and loopback port; --yes cannot
replace it. Preserve private inspect, paused commit and durable image archive.
Compare native config before/after capture and before stop. Journal every intended
effect before disabling original restart, stop/rename and replacement activation.
Preserve captured Config/HostConfig except declared image/name/bindings/restart.
Retain the original container; never delete it. Retry checks exact known identity
and phase, refusing drift. Prepare cert/native-validated ingress before cutover.

An independent bounded recovery timer restores the original after an expired
unfinished cutover, stopping only recorded owned ingress/replacement resources
before recovering original bindings/name/restart. Committed data-plane containers
restart independently of Core. Renewal reuses issuer state and retains valid certs
on failure. Report site TLS, route and native authentication separately; open ports
or source rendering alone do not mean ready.

Complete the selected implementation, then source-helper build/package, unit, then
functional checks. Full R1 marker stays absent. Actual issuance/cutover needs local
sudo and real domain/provider credentials; unavailable prerequisites are resumable
blockers, never false success.

Execution details: minimum files are mandatory deployment inputs. Docker must
already be enabled for boot; fail before cutover with an actionable instruction
otherwise. Do not silently change global daemon configuration. Accepted owned
containers use Docker restart policies. Protected JSON rejects duplicate keys and
non-finite numbers. Atomic writes reject unsafe existing targets. A same-generation
retry verifies admitted running ingress before reporting ready. Renewal reports
retained native proof separately from an explicit fresh authentication probe.

Minimum acceptance: build validates the deployable Python/shell closure and emits
its content inventory. Unit scenarios cover protected files, duplicate settings,
exact launch preservation, persisted-port constraints, ownership drift and
idempotent restoration. Functional scenarios use disposable exact-ID containers,
loopback high ports and a private test CA to exercise real HAProxy/Caddy routing;
the production default trust store must reject that CA. They never mutate the
installed Amnezia, public listeners, DNS or production ACME account. A successful
fixture run is component evidence; live readiness still requires install probes.

Stopped abandoned candidate ingress may be removed only after its exact saved
launch/artifact receipt verifies; running uncommitted candidates require recovery.
The original foreign container is never removed by this cleanup.

Native fixture note: modern Xray freedom blocks private destinations by default.
Only the disposable test server explicitly allows its loopback fixture target;
production native configuration is never changed for this test. Reference:
[Xray freedom finalRules](https://xtls.github.io/en/config/outbounds/freedom.html).

Recovery and renewal units bind an immutable protected deployment snapshot. A
code-only scripts/deploy promotion does not disable these already-installed units:
only that exact saved snapshot may execute recover/renew outside current. A new
explicit install updates the binding before cutover; arbitrary old sources cannot
mutate state. Accepted Docker routing remains independent of that source binding.

Docker Engine 29 canonicalizes the nullable HostConfig.OomKillDisable default to
false at create. docker.create_payload owns this explicit semantic normalization;
the saved original remains byte-for-byte unchanged. Other captured launch fields
are compared exactly. Fixture acceptance checks the entire recreated Config and
HostConfig, not only port bindings.

## Preserved native DNS network attachments

The installed native Amnezia also uses `amnezia-dns-net`. The minimum no-mount
bridge profile supports default `bridge` plus up to seven existing local bridge
networks with dynamic addresses. Reject fixed IPAM requests, endpoint links/driver
options, external Compose/Swarm ownership and non-local/non-bridge drivers. Read
network definitions by immutable ID; never create, remove or reconfigure native
networks or other endpoints. Preserve network IDs, explicit aliases and gateway
priorities via Docker create NetworkingConfig. Dynamic runtime IP/MAC and generated
DNS names are observations, not launch requests. Verify attachments before/after
cutover and on retry; native launch fingerprints bind declared topology.

This is preservation of OR-15's native DNS dependency, not adoption of its DNS
service or network. The previous exactly-one-bridge restriction was an incomplete
engineering binding. Foreign topology changes invalidate saved custody. Test the
real default-bridge plus native-DNS pattern with a disposable labelled network,
including its destruction only after exact fixture ownership verification.

Source-to-deployment handoff must tolerate a pre-sudo session without refreshed
hton membership. A failed unprivileged active-link resolution falls through to
current-source preflight and self-sudo, without relaxing protected permissions.

## Interactive terminal and redacted failure diagnostics

Normal bounded prompts open /dev/tty as separate read/write streams; text r+
requires seeking and is invalid on this Linux terminal. Secret prompts delegate
to stdlib getpass's controlling-TTY handling, treating fallback warnings as errors.
Never read provider credentials visibly or request them in chat. Acceptance must
exercise actual prompts, defaults, remap consent and secret echo suppression under
a controlling pseudoterminal, not only mocked input streams.

Unexpected failures report a safe exception kind and the last owned source
module/function/line (and numeric errno when present). Do not include exception
messages, locals, native output, full paths or configuration. Store that same
redacted location in the private minimum log. Expected InstallError messages stay
human-readable. This narrows the previous unhelpful generic error without exposing
secret data and does not weaken qualification or change authorization.

## Configured installation inputs and root

OR-16: etc/install.json contains non-secret bootstrap configuration in canonical
root-config shape: schemaVersion, installation.directory and optional core edge
/domains and certificates/email. Saved config/hton.yaml is authoritative; CLI
assertions must agree with saved values. Source values fill absent installed
values; if still missing, prompt locally. Persist validated domain/email/directory
before native remap approval, preserving unrelated settings. Never source config
as executable code or store DNS credentials in this bootstrap document.

lib/install_config.py owns finite duplicate-rejecting JSON parsing and root
normalization. scripts/deploy, CLI and minimum Core use it. Initially directory must
be an absolute canonical direct child of /opt using safe ASCII path characters.
Default is /opt/hton if unspecified. The deployed source's protected
<root>/deployments/<digest> ancestry determines its root independently of later
bootstrap edits. Installed directory disagreement blocks rather than moving state.
All minimum data/client/recovery paths and unit WorkingDirectory follow that root.
Unit-name and public-listener conflicts continue to block another installation.
No native/accepted installation is automatically moved when source directory changes.

## Install orchestration and target-only setup

OR-17: scripts/install compares physical CWD with configured root. Outside root it
validates source delivery, obtains sudo, calls scripts/deploy, then cd root and invokes
that root's scripts/setup. Inside root it skips delivery and invokes the same setup.
scripts/setup checks CWD first and refuses any mismatch before sudo, prompts, state,
packages or containers. It must run the active protected source at that root;
site/secret collection and Core execution live only on the setup path. Core's
independent recover/renew actions remain valid from systemd and do not use this
interactive wrapper. Install --dry-run from source previews delivery without
changing directories into an absent target or touching sudo/state.

Before invoking minimum Core, non-preview setup runs the shared OS dependency
bootstrap. This installs/checks PyYAML, cryptography, Docker/Compose and required
tools before Python imports or configuration effects. Failure stops setup; reruns
reuse installed dependencies. Dry-run performs no package installation.

Remap prompt usability (OR-18/D112): show a short multi-line explanation and the
container identity, preserved settings/original, new loopback endpoint and brief
VPN interruption. Ask Continue [y/N], accepting only explicit local y/yes. Empty,
n/no and other input decline. Scope digest remains internal and binds the exact
observed resource/config/port as before; the global --yes flag cannot supply it.
Do not require the operator to retype a hash. The old manual REMAP syntax in
historical evidence describes older installers, not the current UI.

## HTTP-01 delivery and migration (OR-19/20/21)

Current implementation work selects Let's Encrypt HTTP-01 by default, with pinned
acme.sh as the single issuer and existing guarded certificate delivery/renewal.
The short-lived issuer uses host networking and standalone IPv4 TCP/80; it never
binds or changes TCP/443. Require a free local 80 before issuance and retain the
same check on due renewal. Valid material is reused before allocating a listener.
Only explicit dns-01 selects the existing InternetBS adapter and private API input.
Other DNS providers remain explicit unsupported adapters, not required accounts.

An uncommitted legacy minimum attempt with no certificate, issuer job, saved DNS
credentials or explicit DNS credential import may migrate its generated DNS-01
choice to HTTP-01. Persist the chosen challenge in protected state so subsequent
retries/renewal keep it. Committed or credentialed existing DNS deployments retain
their selected mode. Never alter the original container or saved backend port for
this migration. Native issuer failures identify HTTP prerequisites without leaking
provider output. Native cache reuse and renewal-due requests must not force issue
just because delivery failed.

publication.py owns explicit-setup public DNS/IPv4 observation. It compares public
A records against configured entry IPv4 addresses; for direct single-entry setup
only, it may observe outbound IPv4 as a labelled default. Non-direct publication
requires explicit entry addresses and challenge routing; never infer all fleet
nodes must equal that egress address. Query chosen public resolver over trusted,
bounded HTTPS; also check AAAA, which this IPv4-only minimum cannot publish.
DNS mismatch shows generic A/AAAA instructions and bounded manual retry, or an
explicit error for noninteractive use. DNS APIs remain an explicit configured
certificate alternative; DNS-01 does not by itself correct wrong site routing.
No identity discovery runs in background renewal, recovery or ordinary diagnostics.
Publication observation is not a guarantee of incoming port reachability; actual
CA validation and trusted served-certificate probes remain the readiness proof.

Public bootstrap configuration ships no personal domain/email. Existing protected
config retains instance values. Fresh installations prompt/persist missing values.
The minimum profile must reject extra site names explicitly until multi-domain
issuance/routing is implemented; never silently discard configured names.

Renewal window is derived from the certificate validity interval (one third of
lifetime, bounded to 1–30 days), rather than assuming every certificate lasts
90 days. Native --days 1 eligibility is paired with Core's trusted-material/cache
reuse; it does not schedule daily issuance and no --force is used. On issuance
failure, retain the accepted certificate and report HTTP/DNS prerequisites.

### Existing HTTP listener

When TCP/80 is occupied, do not stop or reconfigure its owner. HTTP-01 may use a
configured existing webroot, or bounded candidates /var/www and /var/www/html.
Create only the .well-known/acme-challenge path when absent, with public-read and
owner-write permissions. Traverse paths descriptor-relatively with no symlinks;
parents must not be group/other writable and must belong to root or invoking
administrator. Existing challenge directory must belong to the setup actor and
be publicly readable without group/other write. Never chown/chmod an existing
external web directory. Prove the selected domain route using an exclusive random
file and exact local HTTP response before persisting the webroot. Remove only that
probe and exact newly created empty directories. Issuer receives only the public
challenge directory as writable mount; account/private key mounts remain separate.

Webroot mode does not bind TCP/80. Standalone mode retains bounded free-port
checks. Revalidate the public HTTP route on due renewals using only localhost;
this is not public identity discovery. Native acme.sh creates/removes only its
own challenge token; concurrent unrelated files must survive. Preserve the
selected method/webroot in protected state for unattended renewal. Unknown or
unwritable external HTTP layouts block with an actionable error, not a forced
service stop or an unrelated directory takeover.

Record the approved public webroot owner UID for background checks without SUDO_UID.
New public directories and exclusive proof files get explicit 0755/0644 through
their verified descriptors, including under systemd UMask=0077. Existing directory
permissions/owners and unrelated challenge files are preserved. Public proof files
contain only random challenge data; private accounts/keys stay root-private.

Explicit setup refreshes its owned renewal/recovery unit source bindings even if
site/certificate generation is unchanged, after actual route/authentication checks.
This delivers management fixes without restarting the accepted data plane.
Routine renewal does not rewrite those unit bindings. Bootstrap HTTP transport
settings fill only absent installed settings, preserving canonical configuration.

### Owned launch digest across Docker lifecycle

Docker Engine 29 changes HostConfig.OomKillDisable from null at CLI create to
false at first start. Canonical owned launch digests normalize that documented
nullable default to false, using the same Docker normalization as replacement
payloads. New receipts carry launchVersion=2. Version-1 receipts accept only their
exact current digest or the exact nullable-default representation of that same
launch; all other fields/mounts/config/IDs and artifact hashes remain bound. True
is a real policy change and must never be normalized. Unknown receipt versions
block. Do not delete/reissue or re-adopt an issuer merely because its null default
was resolved by Docker. Resume the exact saved issuer/output and validate/reuse
its completed certificate before cleanup. Native functional acceptance must call
actual record_owned/owned around container create/start/exit, not only validators
and protocols without the production custody checks.

A daemon restart (host reboot) also re-encodes a nil `HostConfig.Dns` recorded at
create as `[]`. Canonical launch digests and the foreign equality check normalize
that restored empty form back to null, matching the create-time receipt; an actual
DNS list change is still drift. This keeps D117's scope: only Docker's zero-value
representation changes.

### Readable status and bounded command failures

OR-23: default human output distinguishes STEP, OK and ERROR. Final success names
the HTTPS URL, trust and automatic renewal; setup failure explicitly says setup
stopped. Code delivery reports only copied code; full-R1 helper availability and
snapshot IDs remain structured metadata. Secret-safe command failures include a
fixed operation label, exit code and a reason from a closed known-error mapping.
Drain bounded stderr without printing or persisting raw text, argv or stdin. Unknown
errors still name the failed operation and exit code. Private diagnostic records
may include those same safe fields and owned caller location, never raw streams.

Native snapshot validation must preserve config file permissions. The observed
Amnezia config is root-owned mode 0001; its running native container can read it.
Its isolated validator adds only DAC_READ_SEARCH to cap-drop ALL, with no network,
read-only image, no host mounts and fixed executable/config path. This permits
reading captured files without changing originals or permitting writes. No such
capability is added to unrelated service roles.

## Native REALITY own-SNI migration (OR-35/D133)

The minimum installer may migrate the installed native REALITY listener to the
deployment's own SNI. The mode is derived from the current native configuration,
not from a flag:

- the own name is `--chain-sni` when set, otherwise the site domain; it is added
  in front of the existing `serverNames`, and the legacy name is retained until
  clients are reissued;
- native `dest` becomes `<docker-bridge-gateway>:<mtg-port>` when the neutral
  MTProto chain is enabled, otherwise `<docker-bridge-gateway>:<site-port>`;
  it never points at public TCP/443, so unauthenticated bytes reach the local
  site through `mtg`/Caddy without a routing loop;
- `privateKey`, `shortIds`, `flow`, clients and the listener port are preserved;
- HAProxy routes the own name (and the retained legacy name) to the native
  backend, with the site backend as a health-checked fallback;
- the separate self-steal container is not created in migrated mode; a retired
  container is stopped by the normal ownership transition.

The ACME certificate covers the site domain plus every own-domain REALITY name
(the site domain or its subdomains); a name that is a third-party legacy SNI is
routed but cannot be issued. Caddy loads only that trusted certificate and
answers the REALITY browser host with the OR-44 fixed `Access denied` 403 body, never the
site HTML. A self-signed REALITY certificate is never generated or served
(OR-38/OR-44/D169).

OR-63/D170 accepts the apex SNI direction: when a REALITY name equals the site
domain, HAProxy routes the apex to the native backend with the site as a checked
backup and the browser reaches the ordinary site through the native fallback;
only remaining REALITY subdomains return the denial body. `www.<domain>` is a
308 redirect to the apex. Certificates always include `www.<domain>`; a
`dns-01` profile whose `certificates.wildcardDomain` equals the site domain also
requests `*.<domain>` and verifies the wildcard SAN. New deployments do not add
protocol-specific third-level names.

The migration is guarded by an exact `DROP-LEGACY-SNI` phrase, separate from the
remap confirmation; a plain `y` or `--yes` cannot supply it. Backward-compatible
service is the default: the native listener keeps the legacy name and `dest`,
and the own SNI is served by the copied-clients listener. REALITY binds the
fallback to a single `dest`, so keeping the legacy name in `serverNames` after
moving `dest` does not preserve legacy clients (verified empirically). An install
migrated by the earlier draft offers to restore the saved previous native bytes;
unattended renewal never migrates or restores and keeps the accepted mode. The migrated bytes are rendered, validated with
the pinned `xray -test` through a read-only bind mount, copied into the
HTON-owned replacement, re-read for digest equality, and the previous bytes are
kept under `work/minimum/native/`. Original and replacement native digests are
tracked separately so the parked original remains verifiable and rollback
restores the prior route.

## Optional MTProto, Hysteria, own Outline-WSS and AmneziaWG2 (OR-29/D127, OR-66/D174)

`core/minimum/protocols.py` owns declarative settings, artifact rendering,
credential generation, container creation/validation and probes for four opt-in
protocols. Activation comes from root `features.<id>.enabled`; `--with`/`--without`
edit that selection. Settings live in `config/modules/mtproto.yaml`,
`hysteria.yaml`, `outline-wss.yaml` and `amneziawg.yaml` and are retained when
disabled.

- MTProto: pinned `mtg`, loopback-only listener, generated fakeTLS secret, exact
  camouflage SNI routed before the own-domain default and before native Amnezia.
  The camouflage name must be a valid DNS name distinct from site and REALITY SNI.
  The secret is generated once and reused; no silent reroll. Success requires the
  container to stay running and accept a loopback TCP connection. Telegram client
  interoperability remains a real-client acceptance step, not inferred from a port.
- Hysteria2: pinned server, explicit public UDP port (default 443), password auth,
  the existing site certificate mounted read-only and a site masquerade proxy.
  Validation starts the real server in an isolated network namespace. Functional
  acceptance uses a real client SOCKS listener and relays a request through QUIC.
- Own Outline: the pinned official `outline-ss-server` v1.9.2 binary is verified by
  sha256 and runs on a loopback web listener with `websocket-stream` and
  `websocket-packet` paths. Caddy reverse-proxies those secret paths under the site
  TLS certificate; HAProxy still owns TCP/443. Keys and paths are generated once
  and reused; the dynamic access key is written 0640 `root:hton` like the other
  share material. Disabling removes the listener without deleting the saved
  material.
- AmneziaWG2: pinned `amneziawg-go:3.1.20260828` with `awg`/`awg-quick`, a
  persistent public UDP port and bridge-network `hton0`; `/dev/net/tun`,
  `NET_ADMIN`, contained forwarding/NAT and `/32` peer routes. Server and peer
  X25519/preshared keys plus `Jc/Jmin/Jmax`, `S1-S4` and non-overlapping
  `H1-H4` are generated once and persisted. Client `.conf` export is structurally
  limited to guest fields; server `ListenPort`, hooks and `SaveConfig` are
  rejected. Validation starts the real config in an isolated TUN namespace;
  functional acceptance is the D6ton `backend/amneziawg` path in
  `scripts/awg-verify`.
  The host publishes the selected UDP port to container UDP/51820. Native
  validation, runtime launch and probes use the interface from `hton0.conf`.

Protocol containers use the shared ownership receipt and launch-drift functions in
`ingress.py`. Candidate containers are created before cutover, stopped on rollback
and promoted to `restart=always` only after both ingress and protocol probes pass.
Promotion is separated from the pure state transition so a native failure still
restores the prior active set. Retired containers are removed only after exact
receipt verification on a later prepare. Runtime stores and share files are
`root:hton` 0750/0640 (OR-46); JSON results and logs never include link material.

The full-R1 module catalogue and native protocol certification remain outside
this minimum binding and unclaimed. The minimum-profile Outline WSS is implemented
by this binding, but real Outline-client interoperability still needs acceptance.

## OR-41–OR-44 consolidated planning correction

The [source review](../SECURITY-MASKING-REVIEW.md) owns current-code findings.
Latest sdd-proxy and sdd-config contracts supersede old public UDP/8443, shared-SNI
chain and universal site-200 wording: TCP/443 and optional UDP/443; separate exact
protocol names; unknown covered own wildcard names redirect to the apex; other
unknown names are rejected. Ordinary TLS fallback for the configured REALITY host
is a simple 403 from the existing website Caddy. No second Caddy or MTProto hop.
Reuse the current admitted content mechanism for `/var/www/html`; preserve saved
alternative webroots until reviewed, never overwrite site files with a denial page.
Client/server DNS privacy remains unverified until the per-mode contracts and final
evidence in sdd-dns-dot are implemented and qualified. Do not advertise site HTTP3
without compatible public UDP ownership/behavior. No product implementation occurs
in this planning pass.

## Current M1 successor and bounded scope

M0 behavior/source history above is retained for migration evidence. The next
[frozen M1 plan](../plans/INSTALLATION-IMPLEMENTATION.md) selects exact successor
files and graph; it does not activate full R1 Rust. Configuration load/check is
separated from install sequencing; managed Amnezia/account/encoding and private DNS
have dedicated owners; optional protocols stay in protocols; static client-page
inventory/render/refresh has its own owner; ingress owns shared Caddy/HAProxy only.
The shell remains a fixed argument/elevation adapter and deploy delivers these
used Python helpers/refresh units literally, without build artifacts or unused R1.

Run intent scopes DNS, selected protocol and page-refresh lifecycle; disable/down
must not leave a public key page or revive it from a timer. Stop/recover can use
accepted receipts when desired YAML is broken. New image/topology updates obey
exclusive native-app maintenance; ordinary renewal is not a second config writer.

### Native connection label

The native `vless://` fragment is the app-facing connection name, not the
credential identity. It resolves from `core.clientAccessPage.names[credential]`,
else `<instance.label> XRay` (`instance.label` is normally the country, 1–64
printable characters including spaces and emoji), else `XRay`. The xray `email`, share filename,
secret inventory and client-page row keep the credential id. The ready summary,
`native_share_links`, `.secrets/share` publication, client-page refresh and
`scripts/clients add` use the same precedence; changing the label rewrites
generated links, never the running native client list or keys.
