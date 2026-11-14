# SDD foreign adapters — Amnezia/Outline coexistence (t08)

## Purpose

Coexist with pre-existing VPN stacks (Amnezia, Outline, telemt) without
breaking their clients: transparent SNI-based passthrough + journaled remaps.

## Requirements

- ADP-0 R1 includes already-installed Amnezia and Xray on the shared TCP/443
  entry. Owner D80 accepts native Amnezia REALITY through byte-preserving TCP
  passthrough when the installed stack supports the researched workflow. Its
  temporary authentication certificate is distinct from the genuine own-domain
  HTTPS certificate required for the ordinary HTON site. Do not terminate REALITY
  at the edge or add a custom Amnezia certificate manager. Qualify exact installed
  version/resources, SNI/target (no routing loop), restart owners and clients before
  concrete integration; no assumption of compatibility for other native protocols.

- ADP-1 In associate mode Amnezia remains an independently managed external component. HTON checks
  compatibility and port conflicts, and offers a safe, consented remap when
  needed. Any remap preserves the Amnezia certificate/TLS behavior required by
  its clients; do not assume raw passthrough is compatible with every native
  domain setup.
- ADP-2 Preserve unassociated amnezia-awg* (UDP) and amnezia-dns; only explicitly registered lifecycle scope permits whole-server start/stop, without undeclared config mutation.
- ADP-3 telemt: stays until owner tests the new MTProto path; removal is a local explicit-consent operation → then the normalized features.mtproto selection in hton.yaml.
- ADP-4 Foreign Outline defaults to preservation. Only an explicitly selected
  config/modules/foreign-outline.yaml settings action and local consent may stop the detected
  container and change its restart policy. Journal actual prior state/policy;
  keep the container. Undo restores observed state, without auto-starting a
  container that was previously stopped.
- ADP-5 An Amnezia reinstall or native domain setup may claim port 443;
  `hton update` detects the conflict and offers a consented safe remap where
  technically compatible. Document the verified native setup and recovery flow.
- ADP-6 Generic foreign manager installs retain their explicit ownership workflow.
  Managed Amnezia provision is a separate native typed path owned by
  [sdd-managed-amnezia](sdd-managed-amnezia.md), not a foreign-menu shortcut.
- ADP-7 Coordination rule: any action touching co-tenant containers requires
  explicit consent (stop-point), even with `-y`.

## Design

- CLI/Bash stays thin; typed Core adapters own the shared plan/apply from sdd-control.md:
  detect -> validate compatibility -> journal -> consent -> mutate -> verify client access.
- Verification: after remap, passthrough branch handshakes against the moved
  backend directly (SNI=mimic) → edge integration test.

## Failure modes

| Case | Behavior |
|---|---|
| rebind fails mid-way | original container state already journaled → auto-restore, abort install with report |
| owner installs Amnezia with domain | verify native certificate and port behavior; report conflict and offer compatible remap |
| journal missing/corrupt | refuse mutation steps; report recovery guidance |

## Acceptance

- Amnezia clients connect pre/post remap without config change.
- On the supported Docker CLI/Engine baseline, the fixed Xray config reader accepts
  the exact regular-file archive and stable content, while rejecting link members,
  extra files, malformed/truncated TAR, mount overlap, fingerprint drift, changing
  content and over-limit data without exposing raw config or secret fields.
- `hton remap-undo` restores stable configuration and client behavior; regenerated IDs/timestamps may differ.
- Ending uninstall leaves foreign stack stable configuration and client access equivalent to the pre-hton snapshot.
- Reject unsupported recreation semantics, incompatible TLS classification and external drift; test interrupted mutation and partial rollback recovery.

## Association and whole-server lifecycle

ADP-2's preservation rule applies to unassociated resources and undeclared config
mutations. It does not exempt explicitly associated Amnezia resources from the
owner's later whole-server up/down requirement. Record exact associated resources,
restart owners and original policies; discovery alone gives no authority. Down
stops the associated composition, continues after errors and prevents boot restart.
No configuration/certificate ownership transfer is implied. See sdd-lifecycle.md.
Exact installed Amnezia identity/SNI/target/restart compatibility remains unverified
and blocks its concrete integration. The certificate-scope choice is settled by
owner D80; see [pinned research](../AMNEZIA-NATIVE-EVIDENCE.md). No additional
certificate manager is approved.

### Explicit association and qualification gate

Foreign discovery is a read-only candidate inventory. It may return immutable
container IDs and bounded, redacted metadata for a local operator to choose from;
the inventory is neither an association nor evidence that a candidate is Amnezia.
No port allocator, lifecycle operation or config writer may consume a discovered
candidate directly.

After the operator selects one exact immutable container ID, a qualification owner
must inspect that exact ID and return typed evidence bound to its current structural
fingerprint. The evidence must cover image identity/version, effective launch and
restart policy, TCP/UDP host bindings and native listeners, network attachments,
mounts and their data owners, and the Amnezia/Xray protocol facts required by ADP-0:
REALITY SNI/serverNames, target/dest, private endpoint and absence of a routing loop.
Sensitive launch/config material stays protected; reports contain only bounded
redacted facts and digests. A mismatch, inaccessible config, incomplete inventory,
unsupported launch/mount semantics or stale fingerprint is a qualification failure,
not a reason to infer support from the image name or open ports.

The host-support namespace listener observation reads TCP LISTEN/UDP bind rows
from the exact container namespace and brackets them with immutable Docker identity,
structural fingerprint, PID start time and namespace inode. That method alone does
not attribute the socket. The separate fixed Xray process observer establishes
one matching LISTEN inode owned by a supported native process/config argv and
executable digest in the exact bounded Docker process tree, with namespace and
process identity rechecks. Only the latter can establish process attribution.

The exact Docker inspect already provides a local image ID, effective launch
metadata, Docker restart policy, published port mappings, attached networks and
mount source/destination/read-write metadata, all bound by the structural
fingerprint. These are the only currently established ownership-related facts.
The image ID is not an Amnezia/Xray software version or a verified upstream image
manifest. The Docker restart policy does not rule out Compose, systemd or another
external controller. Mount metadata does not establish host data ownership or
provide durable copies of mounted data. No current read-only owner enumerates and
correlates all such controllers/data owners with the exact container, so those
coverage claims remain false rather than inferred.

Qualification is read-only and does not register or mutate the resource. A distinct
local confirmation binds `RegistryChange::Adopt` to the exact candidate ID,
fingerprint, minimal lifecycle scope, observed restart owner/policy and qualification
evidence digest. The journalled adoption records `Ownership::Associated` and grants
only the approved lifecycle rights. It does not approve changing host bindings.
Only after that durable association and a fresh registry/config read may
`core.port-allocation` allocate or reuse the descriptor-owned service setting.

If an allocated target conflicts with the current binding, remap is a second
operation and requires its own exact approval, snapshot and durable intent. An
association approval, allocation approval, `--yes`, or discovery result cannot
substitute for remap approval. If the current binding already matches the saved
allocation, install reuses the saved number and does not recreate the foreign
container merely to prove the no-op mapping.

### Fixed native Xray config observation

Engineering binding for the pinned Amnezia 5.0.3 source: the only native config
read allowed by this R1 observer is the enum-selected Xray server JSON at the
confirmed path `/opt/amnezia/xray/server.json`. The selected full immutable
container ID and caller-observed structural fingerprint are mandatory inputs;
there is no caller path, command, shell, container name or fallback path. The host
observer obtains the file through Docker's read-only copy/archive path (no process
execution inside the foreign container), validates one bounded regular archive
member for that exact file, and rejects links, path escape, duplicate members,
unknown archive extensions, malformed headers, truncation and oversized data. It
performs exact-ID inspect → fixed-file read → exact-ID inspect → fixed-file read →
exact-ID inspect; all structural fingerprints and both content digests must match.
This is a point-in-time observation, not a lock or ownership grant.

Raw bytes travel only as a protected, non-serializable host-support value with
redacted `Debug`; the Core parser borrows values from those bytes and returns only
bounded native protocol/listener/security/SNI/serverNames/target facts and secret
presence markers. It never copies private keys, UUID/client IDs, or arbitrary JSON
into evidence, logs, Debug, or wire output. The `serverConfigPath` constant is
confirmed in the pinned upstream source, but an actual installed version, mount or
symlink layout is not inferred from that source. Missing, linked, mounted,
malformed, unstable or unsupported config blocks qualification.

The config reader supplies only a subset of `protocol_facts`. Exact Xray
process/socket attribution and the narrow binary version observer now provide
current executable/native build evidence. Docker inspect provides Docker policy and
mount metadata; it does not establish external restart controllers. The protected
snapshot owner implements writable-layer/image custody for the strict no-mount
baseline; mounted data and native recreate/restore remain unsupported. Read-only
qualification proves applicable snapshot capability rather than capturing bytes.
Missing external restart owners, resolved edge-loop exclusion or exact recreation
semantics still block integration; historical creator-client provenance alone does
not block an otherwise proven current native layout.

### Recovery snapshot coverage

A remap snapshot is not complete merely because `docker inspect` output was saved.
Before stopping/recreating the container, the foreign owner must durably protect the
exact recreation inputs and writable-layer state that would otherwise be lost,
plus the pre-change runtime state. Every mount is classified: container-managed
writable data must be captured; bind/volume data must either be included through an
explicitly owned bounded snapshot mechanism or proven not to be modified by the
operation. If that proof or capture is unavailable, remap is refused. The snapshot
manifest binds source container ID/fingerprint, image digest, create configuration,
host bindings, networks, mounts, restart policy, running/stopped state and captured
data digests; secret bytes remain protected and are never exposed in receipts.

Recovery may restore only from that exact snapshot and only when the currently
observed resource matches the journaled post-remap fingerprint or an explicitly
recorded interrupted recreate state. Any unrelated drift, missing snapshot bytes,
changed mount source or uncertain create outcome remains `recoveryRequired`; it
must not fall back to a fresh image or default Amnezia config. Recreated container
IDs are expected to differ and must be rebound only after exact receipt plus fresh
inspection. Protected writable-layer/image snapshot custody is implemented for the strict
no-mount baseline. Native recreation/restore, mounted-data custody and end-to-end
recovery remain incomplete; a stored image archive alone does not authorize them.

## Exact listener remap for `scripts/install`

The requested instance flow is Amnezia REALITY moved from host TCP/UDP 443 to its
Core-allocated port in `settings.hostPorts.tcp` and `.udp`, with HAProxy taking
public TCP/443. Both transport values are the same 5000–6000 allocation. These
settings change only host publication; native in-container listening ports are
freshly observed and preserved as-is. The value is persisted and reused, never
rerolled on reinstall. Remap itself never
changes Amnezia's native in-container listener, REALITY keys, `serverNames`,
`dest`, client profiles, or certificate owner. OR-35/D133 adds a separate,
explicitly consented own-SNI requalification: it preserves keys/short IDs/clients
and the listener port, adds the own name to `serverNames`, and points `dest` at an
internal bridged target (`mtg` or the site TLS backend); it is never an implicit
remap side effect. HAProxy routes the exact qualified
REALITY SNI byte-for-byte to Amnezia's private container endpoint; it routes the
configured own-site SNI to Caddy with the separately issued trusted certificate.
The route target is checked not to resolve to the edge or create a host-port loop.
OR-31 originally selected a separate HTON-owned REALITY container with the native
keys and clients on the neutral site mask SNI (default `<site>`, optional
subdomain). OR-35/D133 supersedes that container for the native listener: after
the consented migration the native listener itself carries the own SNI and
forwards unauthenticated bytes to `mtg`/Caddy, and the separate container is no
longer created. Remap still never changes native `serverNames`/`dest` implicitly.

The new `ForeignRemap` capability is separate from `ForeignLifecycle.transition`
(which only starts/stops and changes autostart). It is a fixed typed owner with
these methods:

- `preflight(exact ResourceIdentity[], expectedFingerprint, target HostEndpoint,
  TransportPortSet, Budget)` returns fresh `ForeignEvidence`, destination
  reservation/`PortConflict[]`, supported-recreation evidence and a bounded plan
  fingerprint; it is read-only and discovers no authority.
- `snapshot(exact ResourceIdentity[], beforeFingerprint, OperationContext)` returns
  a protected `ForeignSnapshotHandle` plus digest/manifest of exact image/config
  writable layer, create config, host bindings, network attachments, mounts,
  restart policies and runtime state. Secret-bearing data is opaque and redacted.
- `execute(RemapPlan, ForeignSnapshotHandle, exact ApprovalEnvelope,
  OperationContext)` rechecks the complete source fingerprint immediately before
  effects, changes only the requested host bindings for the same qualified
  resources, and returns `ForeignRemapReceipt` with prior/new exact identities,
  mappings, fingerprints and timestamps.
- `restore(ForeignRemapReceipt, ForeignSnapshotHandle, expectedCurrentFingerprint,
  OperationContext)` is the exact inverse and refuses drift; it returns a verified
  restore receipt or durable `recoveryRequired`.

The policy owner is `adapters/foreign.rs`; bounded Docker-engine mechanics belong
to `host-support/src/docker.rs` behind fixed methods taking immutable container
IDs and exact expected state. No free-form Docker argv, image pull, raw shell,
client-supplied locator or generic Compose recreation is allowed. Snapshot must be
durable before stopping/recreating a container; a Docker writable layer (including
Amnezia's generated config/keys) is not discarded or silently substituted with the
image default. Unsupported launch or restart semantics refuse remap. A registered
resource and both its original restart policy and host protocol bindings are
required; raw discovery alone does not associate it.

Consent is resource-scoped and names exact source/destination TCP and UDP bindings,
container identities, restart impact, snapshot digest and public TCP/443 plan.
`--yes` never authorizes it. All requested target transports are checked before
any certificate issuance, journaling, package installation or container mutation.
The old requested 8443 target is superseded. Outline's observed TCP/UDP 8443 is
outside the allocation range and remains untouched. If the owner explicitly associates
Outline as an HTON service, it can receive its own allocation only through the same
compatibility, fingerprint, snapshot and consent gates. Discovery never authorizes
that association or remap.
For allocation policy and the exact boundary between managed settings and foreign
container mutation, see [sdd-port-allocation](sdd-port-allocation.md).

Full foreign install integration remains incomplete. `adapter.foreign` has
qualification and exact host-publication remap/restore planning; host-support has
native config/process/version observation plus protected no-mount image snapshot
custody. The normal `publication.switch` dispatch now checks exact saved targets
and complete listener collision evidence before public consent and shared ingress
apply/rollback; it does not mutate foreign containers. The native foreign remap /
restore executor is not yet bound. Previously observed Xray REALITY uses SNI and
target `www.googletagmanager.com`; target/serverName equality is normal and does
not itself prove an edge loop. Current native version, external restart owners,
resolved route-loop exclusion and authenticated client handshake still require
fresh exact evidence. Unknown historical installer/client provenance is diagnostic;
it is not a substitute for verifying the currently running native stack.

## Development sequence

### Minimum loopback remap binding (OR-15)

The selected minimum uses `127.0.0.1:<saved-port>` as HAProxy's Amnezia backend.
The native in-container listener, REALITY configuration and keys remain unchanged.
The supported baseline is one exact no-mount Docker bridge container whose TCP/UDP
443 bindings can be remapped together. Preserve a protected full inspect and
writable-layer/image snapshot before effects. Prepare a stopped replacement from
that snapshot through the local Engine create API, preserving captured Config and
HostConfig except Image, exact PortBindings and initial disabled restart policy.
Network attachments with mounts, external controllers or nondefault IPAM require
their own qualified owner and remain unsupported. Created IDs are journaled before
start. Retain the original container with restart disabled for exact restoration;
never use `docker rm` on the foreign original or replace it with the image default.
Only the typed foreign coordinator may request this scope after exact review.

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

### Fixed native Xray version observation

The read-only native observation boundary permits one version-only process after
exact process/config/socket attribution: Docker `container exec`, exact immutable
ID, unprivileged UID/GID 65534, fixed working directory `/`, and fixed
`/usr/bin/xray version` argv. No shell, caller arguments, paths, environment
credentials or arbitrary exec operation are exposed. The observer rechecks the
same native process attribution and executable SHA-256 immediately before and
after execution, exact Docker structural fingerprint and running PID/start state.
Output has a 4096-byte ceiling and must match the known two-line native Xray
version statement; stdout/stderr never enter reports or logs. This narrow
engineering exception supersedes the earlier blanket no-execution wording only
for this version command. It observes the installed Xray binary's reported build;
it does not establish the Amnezia app/installer version or infer a verified
upstream binary signature. Unsupported output or drift fails qualification.
The command is product runtime observation, not a development-time live invocation.

### Protected Docker writable-layer snapshot binding

`ForeignSnapshotStore.capture` is a privileged effect owner used only after the
coordinator has durably recorded exact snapshot intent and obtained resource-scoped
consent. It accepts exact immutable ID, structural fingerprint, a unique opaque
64-hex snapshot ID and bounded deadline. Its fixed store is
`/opt/hton/work/foreign-snapshots/<snapshotId>` (0700/0600). It preserves the complete
secret-bearing Docker inspect document privately, and captures the writable layer
with Docker's fixed pause-and-commit operation. The immutable resulting image ID
is saved to a private bounded archive through a streaming sink (<=2 GiB), hashed
and fsynced before the completion manifest is published. No original container,
image or data is removed, renamed or reconfigured. Docker may briefly pause the
source for the coherent commit; this is a declared resource effect, not discovery.

The initial supported snapshot baseline has no mounts, declared volumes, bind
sources or tmpfs. Any such data ownership rejects capture before Docker mutation;
no mounted data is silently excluded. This snapshot supplies durable rootfs/image
bytes plus exact creation/runtime metadata, but does not establish that every
metadata field has a supported recreation algorithm. Remap still needs independent
launch-schema compatibility and external restart ownership qualification.

A prepared marker and exact inspect bytes are fsynced before commit. A crash or
failed command retains them and the deterministic local snapshot-image tag; an
incomplete snapshot refuses automatic recapture or remap because a cancelled
Docker CLI does not prove the daemon-side commit was cancelled. Completed handles
are reusable only after exact manifest and streamed archive digest validation.
No recovery operation falls back to image defaults. Snapshot pruning is a separate
future owner; capture performs no destructive cleanup.

### Qualification, provenance and snapshot sequencing

Qualification proves the exact installed native Xray version, executable identity,
effective runtime layout and protocol semantics. The historical Amnezia GUI/client
that originally created the container may be unknown; creator provenance is
diagnostic metadata, not a substitute for current native compatibility evidence
or an additional impossible qualification requirement. Native client authentication
is still verified at final acceptance and after remap.

The read-only qualification result carries `SnapshotCapabilityEvidence`, bound to
the observed structural fingerprint, for an implemented protected snapshot baseline.
It never creates or requires a stored snapshot. Registry adoption records the
qualified lifecycle scope/original policies without a made-up snapshot artifact.
Actual `ForeignSnapshotHandle` custody is obtained only through a separately
consented, journaled snapshot stage before any remap/recreation. Remap plans bind
that exact handle's artifact ID, source fingerprint, manifest and image/archive
digests. This supersedes earlier evidence DTO wording that required a preexisting
`preserved_snapshot` in read-only discovery/qualification. No original is stopped,
renamed, recreated or removed before the actual snapshot is durably complete.

## Managed versus associated Amnezia — OR-37–OR-40

ADP-1's independent-ownership rule applies to `mode: associate`. `provision` is a
separate managed lifecycle; it creates only the pinned supported XRay service and
records managed ownership. It never adopts an existing same-name container.
ADP-6 foreign-menu restrictions do not prohibit this explicit managed capability.
Absent mode on an existing installation means associate; fresh setup persists its
reviewed mode. See [provisioning task](../plans/INSTALLATION-IMPLEMENTATION.md).

The managed adapter owns native config/container reconciliation; a separate typed
host-access owner creates the configured `admin.user`, key and validated sudoers
entry. Pure export encoding consumes that same identity. Installer coordinates
these owners; association grants no host-account creation privilege. NOPASSWD: ALL
and full-access vpn credentials remain root-equivalent; ordinary users receive the
guest link. Reject unmanaged-account collisions; reuse only recorded identities.

Create managed backends on private bindings immediately; qualification accepts
that shape rather than opening public ports temporarily. Separate exact routes
use the module-configured REALITY and MTProto names (OR-38/OR-39), superseding the
OR-31 minimum single-SNI chain described above. REALITY fallback normally targets
the private real-site TLS backend directly. Nesting through MTProto is not the
new default and requires independent compatibility proof. Keep existing accepted
chains until a reviewed transition; no automatic client breakage.

Wrong authentication is protocol-specific: an ordinary TLS handshake reaching a
fakeTLS listener does not prove every malformed protocol packet receives HTTPS.
Final evidence must distinguish ordinary HTTPS, a valid client, a wrong key/UUID,
a replay and malformed input for each pinned backend. Do not claim universal
fallback before that evidence. See the hardening task's matrix and limitations.

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

Current M1 managed provisioning, supported app edits and host access are owned
only by [sdd-managed-amnezia](sdd-managed-amnezia.md). This association SDD must
not acquire duplicate account/image/export business logic. Existing native client
recovery must respect current revocations and exact native shape; a managed before-
image is not a registered foreign original. See D145 and the M1 migration matrix.
