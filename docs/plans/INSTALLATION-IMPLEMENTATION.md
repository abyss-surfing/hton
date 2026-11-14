# M1 installation implementation plan

Status: prepared design, 2026-11-06. OR-49/OR-52 client-page and private-share
publication are implemented in the minimum path as of 2026-11-07 (unit evidence
only; T-14 real-client acceptance remains open). OR-53 adds one-week log
retention; OR-54 removes separate backup/restore from R1 scope. Phase: planning
plus those owner-authorized implementation slices. Authority:
[owner requirements](../OWNER-REQUIREMENTS.md) OR-15/30/32/34/36–49/52–54,
[decisions](../DECISIONS.md) D143–147/D154/D157–D159. This is the tracked canonical
replacement for local ignored task drafts. This file does not itself authorize
additional execution.

> **Target split note (OR-68, 2026-11-12).** This M1 plan describes the pre-split
> monolith and remains the contract for the current data plane. The edge, site
> and certificate packages below are extraction sources, not target ownership:
> the target Triodos project owns them. The split has its own design, contract
> and migration phases; no runtime cutover is authorized by this plan or note.

## Scope and contract precedence

Next delivery is M1: the existing minimum Python Core, fresh managed Amnezia Xray,
associated legacy preservation, the shared HAProxy/Caddy site and host policies,
MTProto/Outline/Hysteria accepted defaults, private authenticated upstream DNS,
protected exports and optional client-key page, publication of the accepted
guest inventory and bearer URL into the private `.secrets/share` repository,
one-week bounded log retention, and recovery by reinstall from that private
repository instead of a separate backup/restore feature. Full R1 Rust integration, general
configurator/API/accounts/full admin, other VPN catalogue items, client agent
implementation and server IPv6 stay deferred. Existing clients may have IPv6 and
DNS leak risks even while the server is IPv4-only; report per-client evidence.

Owner record > current area SDD > this integration plan/file graph > historical
sections/examples/indexes. Latest SDDs:

| Contract owner | Definitive responsibility |
|---|---|
| [config](../specs/sdd-config.md) | Exact new fields, name inventory, versions and migration semantics |
| [managed Amnezia](../specs/sdd-managed-amnezia.md) | Image/native/app/account/export/lifecycle contracts |
| [foreign adapters](../specs/sdd-foreign-adapters.md) | Association qualification, snapshots, explicit legacy/remap consent |
| [proxy](../specs/sdd-proxy.md) | One edge/Caddy, exact/wildcard ordering, private fallbacks, ports |
| [DNS](../specs/sdd-dns-dot.md) | Shared server DNS policy and per-client evidence boundary |
| [client page](../specs/sdd-client-access-page.md) | Client-only typed inventory, optional exact route, protected publication/refresh |
| [certificates](../specs/sdd-certs.md) | One issuance/renewal owner, wildcard DNS-01 and trusted distribution |
| [minimum installer](../specs/sdd-minimum-installation.md) | Existing minimum execution/delivery/recovery closure |
| [interfaces](../INTERFACES.md) | Shared records/errors and dependency ports |
| [file graph](../sdd/INSTALLATION-FILE-GRAPH.md) | Exact methods/paths, package ownership, reciprocal call projections |

Code-backed observations are in [security review](../SECURITY-MASKING-REVIEW.md).
They describe current deficiencies, not owner-approved features or evidence of the
previous ban's cause. New design never claims invisible/anonymous/unblockable VPN.

## Frozen behavior

- scripts/install remains the one rollout command, literal current-source delivery
  to the configured target with stable mutable config/work. No snapshots/symlinks
  for source distribution, target builds, project home dependency or unused R1 files.
- Preserve argument-free initial protocol composition (OR-32), display it and save
  the first reviewed choice; later runs respect saved selections. Invalid enabled
  module prerequisites report blockers; never silently exclude required protocols.
- Fresh missing module offers provision; old installed missing mode remains associate.
  No unmanaged adoption/account overwrite. Generated SSH identity uses configured
  user, Ed25519 key and the explicitly accepted root-equivalent NOPASSWD scope.
- TCP/443 belongs to HAProxy. Only selected QUIC uses public UDP/443. No public
  8443 in the owned composition, no temporary raw public VPN listener, no second
  UDP/443 owner. Preserve unassociated third-party resources and report conflicts.
- All names are config-owned. Exact protocol/site names precede own wildcard
  redirects. Ordinary REALITY-name HTTPS gets 403 Access denied from the existing
  Caddy; owned unassigned one-label wildcard gets 308 to apex root; other unknown
  names/no-SNI close before VPN. Foreign legacy profile removal still needs the
  exact DROP-LEGACY-SNI approval. Unsupported ECH cannot bypass classification.
- One Caddy serves admitted `/var/www/html` content plus denial/redirect/client
  page; its private TLS endpoint is REALITY's fallback. No nested MTProto hop or
  second Caddy, and no plaintext HTTP error interception by HAProxy.
- This wildcard profile requires wildcard+apex certificate through DNS-01. HTTP-01
  remains supported for other explicit non-wildcard profiles, never a silent
  substitute publishing individual protocol SANs when wildcard prerequisites fail.
- Standard Hysteria QUIC/HTTP3 wire mode is the browser-port profile; no automatic
  Salamander, random padding, fake-TCP kernel modules or port hopping. Native file
  masquerade uses a bounded copy of the admitted ordinary site; no public-domain
  self-proxy chain. One site/certificate artifact owner supplies the copy. Do not
  advertise HTTP3 on hosts/paths unsupported by the selected UDP owner.
- Shared private Unbound uses explicit authenticated TLS upstream policy with no
  hidden plaintext fallback. Explicit recursion remains diagnosable less-private
  operation. Server DNS policy is not a blanket device DNS leak guarantee.
- Full-access and guest artifacts remain separate; JSON/logs carry neither bytes
  nor secret paths. Optional bearer page includes named client keys only. Explicit
  key-page disclosure is the owner-approved exception to ordinary local delivery,
  not permission to publish SSH/server/ACME/API credentials.
- Quiet install asks nothing, uses saved/default values and `sudo -n`, skips empty
  optional page. Missing required input/privilege/scoped consent exits nonzero
  with redacted reason. It cannot silently approve foreign changes or drop legacy.
- Remove/uninstall deactivate owned services/access, retain data; only explicit
  purge destroys exact retained owned resources. Reinstall never rotates keys or
  resurrects revoked clients. Native app edits need current snapshot preservation;
  image/topology updates require exclusive maintenance, not just HTON flock.

## Implementation packages (future, one writer per path)

Exact production/test paths and method contracts are in the linked file graph.
Each package additionally owns its area SDD and corresponding mirrors; shared
INTERFACES/config changes are coordinator-owned after the freeze. No wildcard path
is a blanket write grant. Agent scheduling follows the orchestrator only after an
explicit phase change; this preparation launches none.

| Package | Exclusive implementation responsibility | Inputs / completion |
|---|---|---|
| M1-A | configuration.py, migrations.py, storage contract additions; root/module schemas, hton.example.yaml and affected docs/examples/config/modules files | Typed config/name/host/DNS/page records, preserved M0 migration inputs, stable schema additions and accepted sample syntax |
| M1-B | managed_amnezia.py, host_access.py, access_links.py, scoped docker.py extensions | M1-A ports/records; private managed layout, supporting key files, app edits, account receipts, exports and retained removal |
| M1-C | Onomat-owned DNS runtime (`services/onomat/runtime/dns/`), certificates.py, pinned image recipe/builder/images.lock source updates | M1-A records; native DNS and wildcard prerequisites, pinned source/build/provenance contract, no placeholder runtime digest |
| M1-D | protocols.py, ingress.py, client_page.py | M1-A records and B/C receipts; exact/wildcard/Caddy matrix, typed client inventory, DNS integration, UDP ownership |
| M1-E | install.py, updates.py, scripts/install/deploy/update/uninstall, lib/hton, client-page and update units, lib/install_config.py closure verification, CLI/operator docs/index materialization | A–D integrated; one journaled flow, no-prompt policy, preserved run intent, complete minimum delivery/recovery, seven-day unattended update |
| B1 after integration | External builder runs completed recipes, records real OCI/checksum/image metadata | Actual implementation complete; no tests yet, no target build or implicit publication |
| U1 after B1 | tests/minimum/test_minimum.py and test_security_contracts.py | All selected product implementation complete; unit results and owning-module corrections |
| F1 after U1 | tests/minimum/functional.py and AMNEZIA-ACCEPTANCE.md | Real pinned clients/app/native behavior, isolated controlled scope; corrections through owning modules |
| A1 after F1 | Explicit follow-up DNS/DPI/device/security/deployment scope | Decide based on U1/F1 results; not presumed passed or automatically live |

Writing tests/scenarios before U1 as contract documentation is allowed; executing
product tests before implementation completion is not. Native runtime validators
remain required product behavior. Build failures are real defects, not permission
to bypass checks or fabricate pins. Do not add the implementation-complete marker
until code integration is actually done. Future marker scope is M1, not full R1.

### Additional exact artifact ownership

M1-A root example is `hton.example.yaml`; future root example is
`docs/examples/hton.future.yaml`. Module examples are exactly foreign-amnezia,
mtproto, outline-wss and hysteria YAML under docs/examples/config/modules. Include
these root/module branches in schemas/hton-v1.schema.json, hton-v2.schema.json and
module-v1.schema.json. Registry ownership/receipts in M1 use protected minimum
state rather than pretending the full R1 registry is active.

M1-E updates docs/MINIMUM-INSTALL.md, CLI.md, README.md, docs/README.md,
IMPLEMENTATION-STATUS.md, BUILD-RELEASE.md and docs/sdd/file-index.json/module-index.json.
Keep the minimum delivery closure explicit; include newly added minimum Python
helpers and page refresh units, exclude builds/tests/future Rust and planning docs.
No production schema support is claimed merely by shipping a reference example.
The later full-R1 descriptor/CapabilityBindings provision field must be updated
atomically with its typed consumers in that run; do not insert it early in M1.

## Migration/cutover matrix

| Starting state | Required transition |
|---|---|
| Fresh host, no Amnezia/config | Review saved/default composition, provision missing managed service, never adopt a name collision |
| Existing module missing mode | Associate; preserve original/resources/credentials, collect missing own-name settings |
| Existing legacy REALITY Google SNI | Keep old native path and distinct accepted own listener until explicit OR-36 migration; unknown names never reach it |
| Existing shared REALITY/MTProto chain | Plan separate names/direct fallback, overlapping reissued clients where possible; do not change native dest unconsented |
| Existing owned public UDP/8443 | Show UDP/443 binding/client reissue and downtime plan; preserve previous accepted state until consent/validation |
| Existing unrelated public 8443 | Conflict report, no touch without separate owner approval; no claim whole host has no 8443 |
| Existing HTTP-01 single-host cert | Reuse if it already covers required scope; otherwise DNS-01 wildcard prerequisite, preserve working site on failure |
| Existing native app client edits | Snapshot current clients, preserve keys/labels through update; block topology drift and uncoordinated edits |
| Empty optional page path | Absent path auto-generates on first install; explicit empty opts out; quiet applies the same rule |
| Page path/key/name changes | New snapshot/generation, remove old route; native key revocation is separate from page-path rotation |
| Existing invalid desired config | Retain accepted VPN/site; report exact blockers; recovery/down cannot depend on reparsing broken desired bytes |

## Completion and acceptance obligations

U1 covers typed fields/unknown keys/coherent snapshots, name/cert/route collisions,
private port reservations, associated-vs-managed state, keys/key files/exports,
account collision/privilege/PAM failures, preserved native clients, migration/races,
remove-vs-purge, secret-free errors/quiet behavior and page client/admin separation.

F1 covers clean provision and associated legacy regression; actual Amnezia import,
client add/revoke/restart/export; guest VLESS/Outline/Telegram/Hysteria clients;
apex 200/body, REALITY 403, wildcard 308, wrong Host 421, unknown/no-SNI rejection;
private-first lifecycle, no public 8443, UDP/443 ownership/native HTTP3 behavior;
certificate renewal failure; tunnel drop/reboot/run-intent/rollback; DNS upstream
certificate failure and no plaintext fallback; page headers/path isolation/refresh.

Client DNS leak evidence is explicitly per app/OS/mode and physical-interface capture
with synthetic queries, including tunnel loss, IPv6, browser DoH and endpoint bootstrap.
Unsupported devices/modes are unverified, not universal coverage. Use bounded local
fixtures for F1; public A/IP observations remain explicit under OR-8. No uncontrolled
scans or real browsing histories in evidence.

## Readiness and external facts

No unresolved architectural owner choice is required to start the selected M1
implementation after phase authorization. Exact OCI digests, per-architecture
archive authentication, native package availability, actual app/OS DNS behavior,
non-root/tool compatibility, effective host SSH/PAM admission and real-network
reachability are build/acceptance facts to establish in B1/U1/F1/A1. Never invent
them or declare them passed. A failed proof blocks that feature/release claim and
must be fixed in its owner, not silently dropped from the accepted composition.

Existing non-Markdown runtime artifacts still represent M0/R1. Their exact delta
is frozen here/config/mirrors, and belongs to the authorized future implementation
packages. This planning pass changes Markdown only; current code is not advertised
as supporting any newly planned field or behavior.

M1-B/E shared deployment contract: new mutable `<installation>/host-access/`
holds dedicated SSH homes outside 0700 config/work stores. Deploy's exact retained
top-level allowlist gains host-access; it is not shipped from source or pruned.
HostAccess owns per-receipt identity/modes; protected private keys stay in config/
secrets. M1-E alone changes deploy, after consuming M1-B's declared home receipt.
