# SDD config — editable tree and R1 validation

Authority: [owner requirements](../OWNER-REQUIREMENTS.md), especially Q21–Q25,
Q30–Q34; [architecture](../ARCHITECTURE.md). All behavior is planned.

## Storage, formats and authority

`/opt/hton/config/hton.yaml` stores schemaVersion, instance, installation, logging,
core ingress/certificates, feature participation and onboarding. Each installed
module has `config/modules/<id>.yaml` with its own schemaVersion, moduleId and settings.
Feature objects in the root have only `enabled`; settings live in the module file.
The compiled in-memory snapshot combines those documents without another persisted
dialect. Omitted features are disabled; enabling an uninstalled/incomplete module fails.
The registry in `work/` records installation facts, not another enablement source.

`config/docker/` contains editable deployed Docker inputs. Templates in Git seed
missing inputs and never overwrite customization. Effective accepted Compose/native
files are immutable outputs in `work/bundles/`; services mount accepted copies,
not live editable files. Native third-party formats remain explicit adapter inputs;
HTON settings use YAML, with no live `.env` alternative.

Secret references resolve under `config/secrets/` with protected immutable versions.
Deprecated originals stay beside their source as `*.deprecated.yml`, protected as
potential secrets. Configuration archives include this entire tree, excluding neither
secrets nor preserved originals; distribution and service data are separate.

## R1 interface and normalization

Core `load(tree, supportedVersions)` returns documents, content revisions and
structured diagnostics even if a target is malformed. Its coherent `ReadSet` binds
the complete bounded inventory as well as bytes: callers may add a descriptor-known
missing module document to `absentObjects` only from that coherent inventory, never
by treating an omitted/unreadable listed file as absent. `validate(snapshot, catalogue,
registry, approvals)` returns normalized candidate plus schema/module/global checks.
No invalid target is applied. Diagnostic/bootstrap settings of the tool are independent
of the managed target. Read-only checks do not repair or rewrite files.

### Canonical virtual initial state

`core.config` owns `initialDesired(instanceId, selectedModules, absentReadSet,
catalogue)` and `rootDocument(snapshot)`. The initial composer is pure and accepts
only a coherent read set proving `root` and each selected module settings object
absent; it returns the typed initial `DesiredSnapshot` with the same root defaults
that `rootDocument` serializes for bootstrap. Root defaults (including the isolated
loopback TCP/4443 prepublication listener, empty domains, installation/logging/
onboarding values and disabled optional features) have one owner and cannot be
duplicated in `host.installation`. Catalogue module defaults remain catalogue-owned
and are not materialized in the virtual snapshot. Explicit initial module selection
is validated against known root feature identities and represented in that same
snapshot. This lets first-install planning bind expected absence and later write the
exact reviewed state only after preflight/consent.

Reject duplicate keys, unsafe YAML tags/alias expansion, traversal, symlink escape,
oversize/depth and malformed secret references. Unsupported keys are diagnostics,
not silent deletion. R1 leaves original bytes intact and blocks affected activation;
the later configurator preserves/moves such values through explicit transformations.
Do not make invalid YAML usable by discarding its unknown fields.

Normalize explicit values before versioned defaults; false/zero/explicit null retain
their meaning and may still fail schema validation. Schema default annotations do
not mutate source. Named objects are stable-key mappings; ordered lists retain
module-defined semantics. No environment/UI/native hidden defaults. Root and module
versions are independent format versions, distinct from software/image versions.

Disabled installed modules may retain incomplete settings without activation checks;
validate syntax/known value types and report diagnostics. Required secrets, readiness,
approvals and availability are enforced for the enabled service closure. Dependencies
compute shared engines without silently enabling optional features. Reject cycles,
listener wildcard/transport overlaps, SNI/path collisions and disabled required modules.

Every descriptor binds a settings schema, versioned defaults, pure semantic checker,
bounded optional probes, resources and lifecycle/update adapter. Core aggregates
`code, moduleId, configPath, severity, blocking, repairHint, evidence` with values
redacted. Unknown/unavailable evidence never becomes success. Validation neither
pulls images nor allocates credentials; explicit apply performs declared mutations.

## Persisted ingress and foreign listener targets

The site publication inputs already belong to `core` in `config/hton.yaml`:
`core.edge.domains` supplies site names, `core.edge.listen` supplies the public
listener transport/port, and `core.certificates` supplies the DNS-01 InternetBS
CA/challenge/provider and protected API-key/password `secretRef`s. Install reads
these values from one coherent snapshot; it does not keep a parallel domain,
listener, or ACME configuration, nor hardcode `example.com`.

For production public site ingress, the desired edge is public TCP/443. The existing
`hton.example.yaml` TCP/4443 remains a development/prepublication example; loopback
4443 is only for an isolated development sandbox. Initial publication setup belongs
to `scripts/install`: when no site domain is configured, its guided input defaults the
public listener choice to TCP/443. The user reviews exact proposed changes, including
any change from a seeded 4443 value, before commit. `example.com` is a current instance
value, never a product default. Existing non-empty edited
values are reused; changing them requires explicit guided choice and exact revision
recheck, never silent overwrite.

`core.edge.testTunnel` is an optional, non-production test route for the
Echo development reverse proxy (OR-67). It contains a canonical `sni` and a
loopback `port` in 1024–65535; `enabled: false` leaves the block preserved but
inactive. The name must differ from the site apex, `www`, every REALITY name,
the MTProto camouflage and the chain SNI. The route adds no certificate name and
no public listener: HAProxy forwards that SNI to `127.0.0.1:<port>`, where an
operator-controlled SSH reverse forward terminates. Normal installs omit the
block; only `--test-sni` plus `--test-tunnel-port` enable it, and
`--no-test-tunnel` removes it.

The foreign Amnezia module document persists `settings.hostPorts.tcp` and
`settings.hostPorts.udp` as the desired external host-published ports by transport.
These must share one Core-allocated 5000–6000 value when both transports bind; the
allocator persists and reuses that value. Disabled legacy values remain diagnosable.
Native in-container ports are
read from fresh exact registered-container evidence and preserved unchanged; they
are never guessed from the host mapping or rewritten by this setting. Legacy
out-of-range values remain syntactically loadable for exact review/migration; schema
syntax is not allocation policy. Example numbers are illustrative only.
Allocation, service-to-settings mapping and narrow persistence are owned by
[sdd-port-allocation](sdd-port-allocation.md); config validation consumes its
candidate and does not generate random values.

### Bounded install-input publication

`InstallInput` is typed, non-secret first-publication data: validated site domain,
public edge endpoint and input provenance. VPN ports are Core-generated allocation
candidates, not TTY input.
Missing values are collected by the bounded TTY prompt; on a configured publication whose port differs from the required public TCP/443 target, install proposes the change for explicit review but does not apply a default. No generic `example.com` or
8443 defaults exist. Existing values are shown and reused. Changes require a
revision-bound, digest-specific review that `--yes` cannot supply. DNS API material
is not part of `InstallInput`; protected secret import/resolution and saved
`core.certificates.*.secretRef` remain the only credential path.

`InstallationCoordinator.prepareInputCandidate(InstallInput, DesiredReadResult)`
creates a pure candidate restricted to `core.edge.domains` and `core.edge.listen`.
`core.port-allocation` separately plans candidates; InstallationCoordinator commits
them into each service's existing settings owner. `InstallationCoordinator.commitInputs(candidate,
expectedReadSet, changeReview, OperationContext)` is the sole install-specific writer.
Core config remains read/validate-only; the coordinator uses its validated snapshot
and the descriptor-relative storage CAS primitive. The candidate preserves unrelated
bytes/unknown fields and returns `readConflict`, `invalidCandidate`, `notAuthorized`,
`storageFailure`, or `recoveryRequired` as typed outcomes.

The installer first preflights the transient candidate against all host/foreign port
evidence. Any conflict returns without config write, journal, package, secret,
certificate, container or listener effect. After conflict-free preflight and separate
public/foreign consent, it journals intent, commits the narrow patch by CAS, and
continues using the new read set. Missing non-TTY input returns `inputRequired` before
effects. A discovered but unregistered foreign service is never silently changed to
clear an allocation conflict.

## Desired, pending and accepted state

Plans bind a coherent read set of root/module/Docker bytes, secret versions,
catalogue/image versions, registry revision, approvals and runtime preconditions.
Detect files changing during the read and retry or return a precise conflict.
A plan is invalidated by any change to its actual inputs. Unrelated later API edits
can be rebased before producing a new plan; old plans are never applied unchanged.

Manual invalid desired bytes remain available for repair while accepted services
continue. Save-only creates desired/pending drift and no runtime promotion.
Accepted snapshots and share output never use an unapplied draft. Runtime acceptance
does not silently rewrite a failed manual edit or start a stopped server.

Automatic editing, merging, migration and maintenance belong to the deferred
[standalone configurator](sdd-configurator.md). R1 manual editing plus Core safe
apply does not require that module or a running API. Future UI/API use parameter
batches through it, not whole-file replacement.

## Acceptance

Validate root and per-module examples; omitted/malformed enablement; incomplete add;
unsupported/duplicate keys preserved; independent format versions; secret protection;
invalid target diagnostics; deterministic defaults; shared engines and cycles;
Docker customization accepted/rejected without loss; coherent-read race detection;
zero-write plan; failed apply preserving manual desired bytes and accepted runtime.
Future merge/migration tests belong to sdd-configurator.md rather than R1 gates.

## R1 bounded composition writes versus deferred editor

Manual editing is the general R1 settings path. Required lifecycle commands may
seed a missing module/Docker file and atomically persist the bounded
features.<id>.enabled selection. This narrow installer/lifecycle responsibility
is not a general parameter editor, merge engine or migrator. `core.selection`
owns that small writer; Core config remains read/validate-only. It requires a
valid root, revision recheck and preserved original, and refuses races rather
than overwriting changes. Stop does not use it. The later configurator reuses the
same snapshot/commit rules while adding parameter-batch semantic merge.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

The module document moduleId must match its registered object/file identity; a mismatch is a semantic error, never an implicit rename/adoption.

## M1 configuration freeze — OR-37–OR-44, D143–D145

All fields below are the next implementation contract, not current parser support.
This Markdown freeze is authoritative for future schema/DTO/example changes; keep
live JSON/schema/module descriptor edits in the same implementation package as their
consumers. Root/module schemaVersion stays 1 for backward-compatible additions;
future schemaVersion 2 shares these core shapes without enabling deferred features.

| Configuration owner | Field / type | Meaning / activation condition |
|---|---|---|
| root instance.label | Optional printable display label, 1–64 chars | App connection name prefix; native fragment is `<label> XRay` or `XRay`, overridden by `clientAccessPage.names` |
| root core.edge | domains: ordered exact DNS names | First is canonical website apex; no product-specific defaults |
| root core.edge | hostPolicy.wildcardZone: DNS base | Explicitly owned one-label wildcard zone; instance example.com |
| root core.edge | hostPolicy.redirectTarget: canonical HTTPS root URL | Must equal the configured apex root; fixed target, no request reflection |
| root core.edge | hostPolicy.unknown: const reject | Names outside exact assignments/covered wildcard close at edge |
| root core.certificates | wildcardDomain: same zone, challenge: dns-01 | One wildcard plus apex; no protocol names as individual SANs in this profile |
| root core.dns | upstreamMode: tls-forward or recursive | New privacy setups use authenticated TLS forwarding; recursion is explicit less-private mode |
| root core.dns | upstreams: mapping stable ID -> {addresses, port, tlsName} | Literal IPv4 bootstrap addresses, port 1–65535, validated DNS auth name; system CA trust; selected upstreams are ordered by IDs in upstreamOrder |
| root core.dns | upstreamOrder: ordered upstream IDs; fallback: fail-closed | Nonempty for tls-forward; no hidden OS/plaintext fallback |
| foreign-amnezia.settings | mode: associate or provision | Existing missing mode = associate; fresh absent document offers/persists provision |
| foreign-amnezia.settings | container: bounded name; serverNames: ordered exact names | Provisioned own REALITY names; first is exported primary. Associated legacy state is classified through migration below |
| foreign-amnezia.settings | compatibility.legacyServerNames: ordered exact names | Retained qualified old native names, distinct from desired own-name routes; no fresh third-party additions |
| foreign-amnezia.settings | hostPorts.tcp/udp: ports | Same saved private 5000–6000 backend allocation; observed native ports are separate |
| foreign-amnezia.settings | provision.clientName: OR-56 native client name | Initial guest; not a replacement for the live native client list |
| foreign-amnezia.settings | admin.access: generate or none | Provision-only; generation requires configured username and sshPort |
| foreign-amnezia.settings | admin.user: portable username; sshPort: port | No independent hardcoded exporter username/SSH port; see managed SDD |
| mtproto.settings | backend: mtg; camouflageName: exact DNS name | Separate own configured fakeTLS name, private frontend and same-Caddy site fronting |
| outline-wss.settings | serverName: exact DNS name; streamPath/packetPath: absolute HTTP paths | Own configured WSS name and ordinary exact paths; auth is cryptographic key, never path secrecy |
| hysteria.settings | hostname: exact name; listen: {transport: udp, port: 443, scope: public, addressFamilies: [ipv4]} | Explicit UDP/443 owner with standard native HTTP3 masquerade; no arbitrary public-port override |
| amneziawg.settings | backend: amneziawg-go; hostname: exact name; subnet: IPv4 /30 or larger; listen: {transport: udp, port: 5000–6000, scope: public, addressFamilies: [ipv4]} | Dedicated public AmneziaWG2 endpoint; persisted port and per-peer keys; no HTTPS/SNI claim |

For Outline, preserve existing secretRef path input as a legacy variant in parser
until an explicitly confirmed conversion to configured ordinary paths; report the
conversion plan and preserve originals. Do not accept both variants for one path.
Generated cryptographic secrets remain in protected storage. Do not rotate keys
or break already-issued WSS links merely to change a path's appearance.

Unknown keys are preserved/diagnosed; additionalProperties false for each new
closed object. Retain incomplete disabled module settings; require completeness
on activation. Unsupported/invalid username, TLS name, URL, bootstrap address,
duplicate names and resource conflicts have structured errors without rewriting.
Root invariants and types have one owner in config; adapters validate native support.

The initial composition remains OR-32's MTProto/Hysteria/Outline suggestion without
flags plus selected Amnezia mode; display/persist it once, retain saved explicit
choices, and block an unsupported activation with its missing prerequisites.
Do not silently disable modules to avoid final evidence or missing ports.

### Name inventory, host responses and certificate coverage

Derive ResolvedNameInventory once from the coherent root/module ReadSet; no second
root protocol-name table. All renderers, client exports and probes consume it.
Exact assignments precede wildcard redirects; canonicalize DNS case/IDNA through
one bounded parser, reject trailing-ambiguous/URL/port/embedded-NUL input, wildcard
REALITY names, collisions and native fallback loops. Wildcard SAN *.zone covers
one label only. Every admitted host policy has matching trusted certificate scope.

The primary own REALITY name maps to HTTP 403 Access denied in the existing Caddy;
other configured protocol names retain accepted site policies. Unassigned covered
one-label hosts redirect 308 to the apex root. Uncovered/foreign unknown names and
no-SNI close before any backend. Generic HTTP 421 handles Host/SNI mismatch after
TLS. No arbitrary received SNI triggers certificate issuance. Optional/deferred TLS
modules retain module-owned names; current release never claims them qualified.

### Compatibility and migrations

M1 uses bounded registered minimum migrations, not the deferred general configurator.
A read-only migration plan binds original bytes/read-set and lists each source ->
target, preserved archive and approval. Existing mode absence means associate;
classify saved native serverNames by exact observed legacy identity and desired own
config, preserving originals. Never manufacture ownership or change legacy native
dest during classification. Missing own names require configured input; OR-38 values
are instance examples, not global built-ins. Legacy Google/native compatibility stays
on its distinct accepted listener until OR-36's DROP-LEGACY-SNI consent.

Saved UDP/8443 or single-SNI chains produce a reviewed migration, not silent reroll.
Old standalone HTTPS 4443 is development/private only; no new public non-443 profile.
DNS policy changes from recursive/OS to TLS forwarding are explicit and can break
resolution; preserve previous config and accepted runtime on failure. Archive secret-
bearing originals under protected versioned storage. A schema-compatible additive
field still requires changing typed runtime consumers together with the schema.

Renames of accepted accounts require an explicit HostAccess migration and receipts.
Reinstall/renew never choose a new SNI, client key or name inventory. Publish new
names only after valid certificate coverage and reviewed client overlap/cutover.

### Optional client page — OR-45/OR-49

`core.clientAccessPage` is a closed root object: `path: string` absent by default,
`names: mapping CredentialId -> string` default empty, `refreshSeconds: integer`
default 60 (10–3600). Path is a single case-sensitive segment validated by
sdd-client-access-page; the secret URL is never derived from domain/host/client
names and is never logged. An absent `path` means publication is enabled and the
installer generates a CSPRNG segment on first install; an explicit `path: ""` is a
deliberate opt-out. The main example omits `path`; the explicitly supplied current
instance uses `client-access`. Protected native client inventory, exact
route/collision rules, refresh and disclosure are owned by
[sdd-client-access-page](sdd-client-access-page.md). Configuration write still uses
coherent CAS; `-q` does not synthesize optional input or grant missing consent.

### Initial seeding and example ownership

The [target M1 example](../examples/M1-CONFIGURATION.md) provides reviewable root
and per-module values during Markdown-only planning; M1-A updates the main YAML
example and typed schemas together later. New trusted initial module settings use
admin.access generate, configured example user hton-amnezia, SSH port 22 and guest
label; these are config-owner seed values, never independent exporter fallbacks.
The initial seed may derive/persist static/assets/media names from the explicitly
configured owned wildcard base. Once saved, each name is independently editable;
changing base/site domain does not silently rename established clients. Existing
settings always win and legacy missing values follow the migration plan. No personal
domain or bearer page URL is seeded from Git. The initial DNS policy seed uses the
explicit example's editable authenticated primary upstream, not OS resolver fallback.
