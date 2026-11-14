# HTON R1 Implementation Status

Current 2026-11-07 implementation authority: [M1 implementation plan](plans/INSTALLATION-IMPLEMENTATION.md).
It freezes OR-37–OR-49 and OR-52–OR-55 and supersedes conflicting older
design/default descriptions below; historical source/build/test observations
remain historical evidence. The OR-49/OR-52 client-page and private-share slice,
the OR-53 one-week log retention and the OR-55 update timer are implemented with
unit evidence; the disposable local component harness also passes. The Rust
workspace build was attempted and is blocked by pre-existing
full-R1 integration errors recorded below; no live/product acceptance is claimed.

> **Target split (OR-68, 2026-11-12).** This status describes the pre-split
> monolith. In the target architecture the public edge, site and certificates
> move to Triodos and HTON keeps only VPN engines, credentials and private
> listeners. The split is design/scaffold only; nothing below is a claim about
> the Triodos runtime or an authorization to cut over.

## Selected minimum delivery (2026-11-03)

OR-15 uses `python/minimum` through the default `scripts/install`, independently of the
broader Rust host gate below. The M0 source artifact builds, and 39 unit scenarios
pass. Native functional component acceptance passed; no production issuance/cutover
has been performed. Full R1 remains incomplete. Usage: [minimum install](MINIMUM-INSTALL.md).

## Full R1 snapshot (2026-11-03)

Latest integration adds fixed dispatch for saved-target `publication.switch`,
Owned `modules.remove`/separately approved VLESS config purge, isolated hash-bound
`bundle.validate`, and ownership-bounded `installation.uninstall`. Null R1
module-owned update capabilities return explicit unsupported errors; shared and
foreign images are not silently updated. Uninstall retains configuration,
certificates, code and recovery; unsupported full purge or foreign deletion blocks
before effects. Runtime has exact stopped-epoch Owned removal with durable native
absence proof, and registry recovery checks exact before/after uninstall states.
These are source implementations, not build or acceptance results.

Systemd delivery now has a concrete installer: five fixed compiled templates are
staged under protected work/systemd, journaled and linked into the system manager
before ingress activation. Foreign paths/template drift block. Uninstall checks
ownership before shutdown and removes only exact delivered links after native
removal. The native-log template is included. Delivery does not start/enable the
data service or renewal timer; boot/renewal lifecycle integration still needs
acceptance before release.

Foreign observation now attributes the native Xray process/socket/config and
observes its version, permitting native REALITY target/SNI equality. Protected
no-mount Docker writable-layer/image snapshot custody includes bounded streaming,
private archive hashing and retention after interrupted capture. Read-only
snapshot capability is separate from actual consented snapshot before remap.
These primitives do not complete external restart-owner/loop qualification,
association, remap/restore execution, install binding or native handshake checks.
The install foreign gate remains closed; no Amnezia cutover or trusted live
certificate is claimed.

Install partial results now retain actual ingress receipts; saved site-value
replacement requires protected review. Host listener-conflict checks and bounded
exact TTY input mechanics have shared owners. Deployment bounds source-copy growth,
checks reused snapshot modes, serializes promotion and fsyncs directory/link
publication. English scanning includes ignored task docs; Russian originals are
preserved under docs/_lang/ru/tasks and legacy .env.example text is English.

I4 is incomplete and the build marker remains absent. CI's final functional scripts
are still missing; U0/F0 have not started. Rust 2024 parsing/format checks, Bash
syntax, in-memory Python compilation, JSON/unique graph-ID checks, whitespace and
scoped English scans passed. No Cargo build/check, product tests, packaging,
deployment, Docker mutation or certificate issuance was executed. Parallel agents
stopped on a service usage-limit error; the coordinator completed supported
uninstall composition and static integration after the interruption.

`scripts/install` now has a bounded Core execution path for typed publication input,
persisted VPN allocation, supported owned setup, DNS-01 certificate issue/reuse,
normal ingress prepare/apply and local HTTPS verification. Read-only preflight
returns `ready` only with complete reservation evidence and no conflicts or
approvals. OR-9's editable `example.com` suggestion and TCP/443 input are not baked
defaults. First random ports in 5000–6000 are durably journaled and written to each
service's existing settings owner before root seeding; resumed installs reuse those
settings. OR-10 authorizes first assignment after collision checks; changing a saved
port needs exact local approval. Pending allocation recovery and rootless committed
candidate provenance cover the bootstrap crash window.

The installer binds `/var/www/index.html` absence or exact content/filesystem
identity in preflight, creates a placeholder only after recheck, and carries the
bounded exact bytes into an immutable `site-index.html` bundle artifact. Caddy routes
site requests only to that artifact and mounts no host webroot. `siteReady:true`
requires local trusted TLS/SNI, HTTP 200, byte-exact body and unchanged source
identity/digest around the probe; `externalDns` remains `unverified`. That local
website probe alone does not authorize whole-system `ready:true`; install now returns
partial with `siteReady:true` and `ready:false` until qualified Amnezia route and
native handshake verification exist. Missing InternetBS secret
versions produce the fixed resumable `requiredNext` action without exposing secrets.

The end-to-end owner goal OR-12 remains incomplete. The source implements the site
certificate/apply/local HTTPS path and HAProxy's exact-SNI router, but no built
release is present and D81 blocks packaging until I4 is complete. Amnezia remains
unqualified for complete restart/data/loop evidence and remap/restore execution,
so its requested 5000–6000 integration is blocked despite native attribution and
protected snapshot primitives. No trusted
certificate has been issued or live site verified by this source work. A trusted
DNS-01 credential import and evidence from the exact installed Amnezia service are
required for the external phases. No build or product tests were run for this slice.
The dated entries below are implementation history and must not be read as later
status where contradicted by this current summary.

2026-11-01. The owner explicitly instructed: “Proceed with implementing the plan in docs/tasks, taking docs/specs and docs/sdd into account.” This moves the selected R1 into implementation; it does not mean deployment to an external VPS, changes to third-party containers or publication on port 443. Authoritative requirements are in [OWNER-REQUIREMENTS](OWNER-REQUIREMENTS.md), sequence in the [orchestrator](IMPLEMENTATION-ORCHESTRATOR.md), and scope in [ROADMAP](ROADMAP.md).

## Actual progress

| Wave | Status | Remaining outcome |
|---|---|---|
| P0 | docs/tasks reconciled with current R1 | Reconcile deviations in new concrete bindings with the owning SDD/map before coding |
| I0 | Contract packages are written and build; initial workspace and Cargo.lock created | Core/runtime/logging manifests are added with their source implementations; a contract is not a completed operation |
| I1 | Main libraries are written and workspace builds | Concrete host bindings and user composition belong to I3; product scenarios have not yet been checked |
| I2 | Libraries and serializers are written and workspace builds; concrete ports are not yet integrated | Real host/native/crypto bindings and end-to-end I3/I4 composition; Amnezia qualification requires installed evidence |
| I3 | Incomplete | Bash CLI/bootstrap/install; Compose/systemd/build/validation artifacts; release documentation/provenance |
| I4 | Incomplete | Integration of all R1 without permanent stubs or duplicate rules |
| U0 | Not started | Unit testing of the completed product after I4 |
| F0 | Not started | Functional checks after the unit stage |
| A0 | Undefined | Further acceptance program based on actual U0/F0 results |

The existing Bash scaffold does not demonstrate that the target CLI is implemented. This record does not claim VPN operation, R1 coverage, successful tests or production readiness. The coordinator adds detailed build results and completed packages as they occur.

## Subsequent packages

2026-11-02. Implemented a separate `scripts/deploy` and its OR-11/D99–D100 contract: validation of local `BUILD-PROVENANCE.json` and SHA-256 inventory, `hton` group, root-owned release with group read/execute and atomic `/opt/hton/current`. `scripts/install` from an inactive prepared release invokes deploy and transitions once through stable `/opt/hton/bin/hton`; the active release does not trigger repeated delivery, and `--docker-only` remains a separate path. Changes are synchronized with the CLI, interfaces, installer SDD and file graph. Only static Bash/Python/JSON checks and `git diff --check` were run; no build, tests, packaging or VPS deployment. Packaging remains blocked until I4 is complete, and inventory validation is not publisher signature.

After running from a checkout, it was found that the sudo password was requested before rejection due to missing provenance. A preliminary read-only check was added: the source checkout explains the D81/I4 build gate before sudo; standalone deploy also checks the artifact before privilege escalation. The build gate is not bypassed, and no build runs on the target server.

2026-11-02. `core.config` gained pure `initial_desired`/`root_document`, and `core.host-installation` uses this canonical root projection for its bootstrap seed instead of a second defaults literal. This removes one source of divergence between initial snapshot/bootstrap, but the virtual snapshot is not yet used in read-only install preflight, domain inputs or port allocation; the end-to-end scenario remains incomplete. No build or tests were run.

A repeated install-selection audit found module selection is not closed into a ready install transaction. The shell now receives a read-only Core offer, checks its digest and returns the selected IDs; Core rechecks the catalogue and config revision. The selection remains a transient candidate because the install transaction does not yet commit it. `foreign-amnezia` also requires association/qualification. `plan_graph` and the writer still have no caller; OR-10 is not achieved through `scripts/install`.

2026-11-02. Added a separate `core.host-install-input-writer` and `Intent::InstallConfig`: it is limited to the existing `hton.yaml`, preserves unchanged config values, journals original bytes and supports exact-old/exact-new recovery. This is only a persistence primitive; it is not compiled, not called by `install.reconcile` and does not solve the first-install path without a pre-created `HostApplication`. `plan_graph` and `PortSettingsWriter` also remain outside install dispatch, so OR-10 is not yet achieved end to end.

2026-11-02. Reconciled install-selection and foreign-association contracts across
CLI, installer/lifecycle SDDs, shared interfaces and mirrored maps. Implemented a
read-only Core selection offer, shell TTY selection with bounded retries, and
digest/root/catalogue revalidation for the transient candidate. Profile suggestions
cannot overwrite saved selection; `--menu` revisits it. Exact Docker qualification,
local adoption approval and remap consent remain separate authorities. The install
transaction still does not commit selection or invoke the allocator/association/remap.
No build or product tests were run.

The trusted module index now declares fixed first-run suggestion sets for the four
CLI profiles, and `ModuleCatalogue` parses, validates and fingerprints them. The
shell/Core offer now consumes this mapping; this does not imply that Amnezia passed
qualification or that install commits the selection. Rustfmt parsed the changed
catalog source; no Cargo build or tests were run.

2026-11-02. Added read-only `host.observations::reservation_evidence_for` with explicit `RegistrySnapshot` and `OperationControl`: first install can obtain the same complete host TCP/UDP snapshot without first opening/creating `HostApplication`. Also added optional field `LifecyclePlan.portAllocationCandidate`, and the pure lifecycle planner copies the candidate from `PreparedOperation` into it. Host preparation and execution do not yet create or commit this candidate; this is an integration seam, not an end-to-end guarantee. Only rustfmt/static document checks; no build or tests.

2026-11-02. Added bounded read-only inventory of Docker candidates: two stable passes over complete immutable IDs with image digest, runtime state, published TCP/UDP bindings and structural fingerprint; Core links the result to `adapter.foreign` without adoption/mutation. Exact Amnezia qualification remains fail-closed: no concrete native protocol/listener/restart/ownership/snapshot facts are produced. No build or tests were run.

2026-11-02. Added typed `InstallSelectionCandidate`, `InstallPlan` and
`InstallResult` to contracts. The plan binds exact revisions/fingerprints, site and
selection candidates, persisted allocation candidate, foreign bindings, certificate
claims, approvals and fixed phases; the result retains partial/recovery receipts.
`install.reconcile` does not yet build or execute them, so port persistence and
HAProxy/site still do not work end to end. Only rustfmt/JSON/diff static checks;
Cargo build and product tests were not run.

Selection revalidation now materializes `InstallSelectionCandidate` with the
previous/new module sets, catalogue offer digest, exact root state and canonical
candidate digest. This strengthens the input to the next coordinator stage; the
config writer and install execution do not yet call it.

I4 parallel audit 2026-11-02: HAProxy/Caddy renderers, Core Compose rendering,
ACME units and certificate coordinator already exist, but `install.reconcile` does
not compose or invoke them. No isolated I3-B artifact edit can make installation
automatic without bypassing coordinator preflight, approval and journal ordering.
Foreign-association audit confirms exact Docker inspection can supply structural
launch/restart/binding/network/mount/state facts, but not Amnezia/Xray native
version, active listener, REALITY SNI/target or non-Docker restart owners; therefore
it cannot yet emit qualification evidence or safely admit `foreign-amnezia` to the
allocator. The pinned Amnezia 5.0.3 source confirms the Xray JSON path
`/opt/amnezia/xray/server.json`. A bounded host-support read-only fixed-path reader
now brackets two Docker archive copies with three exact-ID structural inspections,
validates a single regular bounded TAR member, and returns protected bytes with a
content digest only when both reads and all fingerprints match. It is not yet consumed
by Core's native parser and does not solve version/owner/snapshot gaps or qualify
Amnezia. The SDD/interface/maps now specify that seam. Rustfmt parsed/formatted the
host-support source; no Cargo build or product tests were run. Scoped review found no
obvious reader traversal/argv race issue; F0 must confirm the strict USTAR member
shape against the supported Docker CLI/Engine, including symlink and mount cases.

Fixed the shell flow: every guided install now reads the Core selection offer; on first run, the TTY presents an explicit choice even without `--profile`/`--menu`, while a rerun preserves the selection without editing unless `--menu` is passed. This prevents losing the initial selection, but Core still lacks a complete install transaction.

1. I3-A/B/C: Bash CLI/bootstrap/install, bounded native/Docker/systemd bindings,
   independent guardian process, certificate/archive cryptography, release artifacts.
2. I4: integrate all R1 domain ports without permanent stubs or successful no-ops.
3. After I4 is complete: U0 → F0 → A0 decision. Do not run test jobs before then.

The orchestrator and map define specific file and authority assignments. Optional
protocols are not added merely to fill out the catalogue. API daemon/configurator/
web/accounts are deferred; IPv6, accounting and external client uplink are outside R1.

## Known blockers and limits of evidence

- Amnezia primary sources are researched in [AMNEZIA-NATIVE-EVIDENCE](AMNEZIA-NATIVE-EVIDENCE.md).
  Owner D80 accepted native REALITY passthrough: a genuine domain certificate is
  required for the ordinary HTTPS site, while native REALITY authentication
  semantics are preserved. Before a specific adapter, establish the exact installed
  version/container identity, loop-free SNI/target, ports, mounts and restart owners.
  This acceptance does not confirm actual compatibility or authorize a certificate
  manager. Qualification on a safe test environment remains in F0 after U0.
- Verify a free-domain example and DNS-01 compatibility against current conditions
  during installer implementation; a name alone does not prove automation.
- Before I3, define the exact install permission profile based on INTERFACES §12:
  root-owned distribution, separate config/work permissions, OS/sudo authorization
  and restricted runtime. A written description does not prove those permissions are configured.
- Strong atomicity of the future configurator with arbitrary external editors is
  not yet promised; an editor coordination contract is needed before its later
  implementation. This does not block manual YAML check/plan/apply or the narrow R1 selection writer.
- Deployment, real client/DPI/load properties and external server availability
  are not confirmed. Starting development does not remove local approval boundaries.

## Completed initial I0 package (2026-10-31)

- `contracts/src/config.rs`, `operations.rs`, `lib.rs`: validated logical IDs,
  separate desired/installed/observed DTOs, checks, lifecycle/effect/setup/update
  plans, typed partial outcomes and narrow capability ports without host authority.
- `runtime-contract/src/lib.rs`: independent bundle/admission/state/lease/action/
  probe records; bounded JSON decoder, validated IDs/digests/relative artifact
  paths and pinned image references. Deserialization does not create an AdmittedBundle.
- `Cargo.toml`, both package manifests and `Cargo.lock`: only the two authored
  contract packages; no empty Core/runtime implementations or R2/web dependencies.
- Updated mirrored maps and file-index bindings, tasks and current-phase references.

Build checks were run: `cargo check --workspace --locked` and
`cargo fmt --all --check` succeeded. `cargo tree --locked -p hton-runtime-contract`
confirmed the dependency closure contains only serde/serde_json and their transitive
dependencies; Core, management contracts and web are excluded. Rust 1.99.1 toolchain
was prepared in `/tmp/hton-cargo` and `/tmp/hton-rustup` without changing the user PATH.
The listed cargo commands from the repository root reproduce this on a machine with Rust.
A build is not U0/F0; product scenarios were not run.

Limits of this package: wire records do not check Docker/DNS availability,
semantics of specific modules, caller permissions, lease transition validity or
upper bounds of requested budgets. These are the responsibility of config/catalog/runtime
owners during I1 implementation, before effects. Arbitrary settings/original bytes
are hidden from Debug; ordinary logs must accept only SafeFields, not serialized
protected DTOs. Services, installer and safe application for target R1 are not implemented yet.

## Implemented I1 package (2026-10-31)

- `crates/core/src/config.rs`: bounded YAML scanner/decoder, schema checks without
  external schema fetch, coherent read set, pure normalize/validate through supplied
  module checks and secret-version metadata. No image pulls or credential generation.
- `crates/core/src/catalog.rs`, `plan.rs`, `platform/modules/{index,services,vless,foreign-amnezia}.yaml`:
  trusted catalogue, namespaces host/project/service, dependencies/shared consumers,
  conflicts and exact-input effect plans. Disabling a service’s last consumer includes DisableAutostart → Stop; associated config is not adopted.
- `crates/core/src/journal.rs`, `registry.rs`, `secrets.rs`, `lock.rs`: protected durable
  intent/CAS/checkpoints, recovery without blind replay, explicit adoption, scoped
  immutable versions and explicit disclosure; corruption is retained for repair.
- `fs-support/` (`hton-storage`): descriptor-anchored NOFOLLOW capabilities,
  bounded reads/listing/locks, private files and fsync/rename publication. CAS is guaranteed for cooperating
  writers, not arbitrary external editors.
- `logging/`: independent closed-field events, bounded queue/worker/rotation,
  observable drops and transient memory diagnostics without files in dry-run.
- `runtime/`: private admitted bundle, retained artifact bytes, durable intent/
  epoch/lease CAS, fixed actions, pre/post observations, partial shutdown,
  idempotent startup, reverse-topological compensation, independent-supervisor
  acknowledgement and semantic probe coordination. These algorithms are written through
  trusted ports; real host/native/probe/supervisor implementations are still needed.
- Contracts extended with `ListenerNamespace`, `Condition::Autostart`; maps/call graph
  aligned with pure planning and supplied evidence, without hidden probe/write calls.

Integration checks: `cargo check --workspace --locked` and
`cargo fmt --all --check` succeeded. `cargo tree --locked -p hton-runtime`
confirms that Core, management contracts, YAML/schema parser and web are absent
from the runtime dependency closure. Documentation graph checks found no missing
maps, unknown targets or absent reciprocal call edges. This is a static build/document
reconciliation, not U0/F0 or security/production acceptance.

I3 must provide `AdmissionPorts`, `NativeAction` (including checking actual
launch against admitted artifacts), `ShutdownAuthority`, `ProbePort` and
`GuardianSupervisor`; the current runtime binary returns unavailable/69 until this
composition exists. Connect `JournalStore.with_log_sink` in the production entry, and
lifecycle/apply must first reconcile unfinished/recovery records. SystemControl has
no cancellation registry; the entry injects the required OperationControl. Trusted
preparation/observation ports must not replace unknown evidence with success. Root/schema
logging defaults and concrete LoggingPolicy must be connected in I3 with explicit
checks of all constraints.

## Implemented I2 libraries (2026-11-01)

- Engine serializers for HAProxy/Caddy/Xray/Unbound and protected immutable bundle staging;
  native transition bindings enter the plan before approval, and validation evidence
  is a required publication condition. Preview performs no native effects.
- Resumable setup, selection, lifecycle/update, resource/adoption policy;
  apply, certificate, encrypted archive/restore, UUID identity/export and observation
  coordinators through explicit ports. These are domain-owner sources, not
  evidence that Docker, ACME, archive encryption or the installed CLI work.
- Journal stores terminal OperationResult and verified receipts; recovery does not
  replay unknown effects. Structural registry fingerprints are separate from
  mutable native state; verified recreation requires explicit RebindOwned.
- Runtime accounts for retired services during shutdown and rollback. Restore reservation
  blocks concurrent up until the operation securely completes or recovers.
- SecretStore accepts bounded opaque material and preserves current-version metadata;
  saved bundles continue using their explicit versions. The first UUID for a disabled
  profile remains Pending until a truly accepted bundle is confirmed.
- platform/images.lock.json contains verified digest-pinned registry manifests and inspected
  linux/amd64 image metadata. No pull/run or native qualification was performed.

I3/I4 obligations: real validators and semantic probes; exact Docker launch/
observation fingerprints; reconciliation of native locators after apply and image update;
update ownership for shared engine resources and feature versions; bounded
redacting stdout/stderr collector before container startup (Docker logging disabled);
Caddy reload through a private Unix socket; ACME/X509/archive crypto; setup/bootstrap
and accepted identity inventory without trusting stale metadata cache. Unknown
observations must not become success. Native runtime actions are still unavailable/69; read-only status is connected in I3.

Static build `cargo check --workspace --locked` succeeds. Product
unit/functional tests, native validators and deployment were not run; completion
of I2 libraries does not mean R1 or acceptance is complete.


## I3-A begins: public dispatch (2026-11-01)

bin/hton uses a single fixed allowlist in lib/hton rather than selecting any
executable path; an unknown command exits 2. Common flags before the command are
retained as separate argv. Help is intercepted before effects; scripts/help no longer
claims readiness for the .env-era installer, IPv6 or unavailable operations.
Legacy install/uninstall/whoami through the new dispatcher are temporarily unavailable
(exit 3) until replaced by the target R1 owners. Other missing bindings also exit 3;
deferred upgrade/tunnel exit 1. The presence of old files is not considered R1 host
composition. Direct legacy entry scripts have not yet been migrated.

Static check `bash -n bin/hton scripts/help lib/hton` passes. Commands were not
executed for functional checks: U0/F0 remain after I4. The CLI parser and all
command bindings are not yet complete; this package completes dispatch/help.


## I3-A: config check → Core (2026-11-01)

Implemented scripts/config, fixed invokeHelper and crates/core/src/main.rs. The helper reads config
through FsConfigSource, bounded load and built-in catalogue/normalize; .env is not
loaded. JSON/text contain only diagnostics/scope, without normalized settings.
Registry/module-semantics/secret/native checks remain unverified; a successful exit
does not prove readiness to apply. plan/stage/apply remain unavailable.

The release must install a prebuilt hton-core in libexec; the CLI does not run
cargo or download code. Static workspace build and Bash syntax were checked; commands
and product tests were not run.


## I3-A: modules list → inventory owners (2026-11-01)

Added scripts/modules list, fixed modules.list helper operation and pure lifecycle
inventory projection. RegistryStore.inspect reuses the bounded registry decoder
and invariants without creating JournalStore. Known absence differs from unavailable;
corrupt/unreadable inputs are not masked as an empty installation. The list retains
registry-only entries when desired config has errors and separates membership/setup/
ownership, desired enabled and native-state=unverified. Stdout contains no resource
locators, checkpoint payloads or raw registry diagnostics. The local read-only actor
is euid, without mutation scopes or consent refs. No native actions/observations.

Mutating modules operations accept IDs/--save-only syntax, but remain unavailable
until host capabilities are connected; none is replaced by a hidden legacy path.
Cargo manifest/lock were updated to use the already-present rustix for euid. Unit/functional
tests were not run; current evidence is static build/syntax/graph checks.


## I3: independent runtime status (2026-11-01)

Implemented scripts/runtime status and libexec/hton-runtime status transport; the shell
calls runtime directly, without Core/compiler/desired YAML. RuntimeStateStore.inspect
reads existing state.json and reuses state invariants; it does not create initial
state, directories, locks or journals. Absent/failed differ from stopped intent.

Output contains durable intent/epoch/bundle/lease/maintenance metadata and historical
resource statuses with timestamps; native observations and bundle admissibility remain
unverified. Raw state/resource fingerprints are not output. State error exits 1,
usage 2, missing installed helper 3. Native start/rollback are not yet connected;
the internal binary retains unavailable/69 for effects.

Workspace cargo check --locked, formatting and Bash syntax pass. Runtime dependency
closure still excludes Core/management/YAML. Product tests and native operations
were not run.

## I3: integrated host/build/CLI package and D81 (2026-11-01)

The owner requested larger iterations, parallel multi-agent development and builds
only after all plans are fully ready (D81). Two implementers prepared part of the
CLI/build changes and stopped at the service limit; the coordinator continued and
integrated their saved source changes. No build was run after the clarification.
Earlier compile results above are historical, not evidence for new changes.

Implemented:

- `host-support/` / `hton-host`: independent typed Docker/systemd read-only drivers;
  private bounded process runner, absolute tools/cleared env/local socket, process
  group, cancellation/deadline/output cap, bounded slots/reaping. Docker facts
  verify immutable IDs/repository digests; protected launch/env/labels/mounts are
  not serialized and have redacted Debug. systemd accepts literal service/timer,
  fixed show properties and verifies the exact returned identity.
- CLI: shared bounded argv/common flag parser and unified availability policy;
  repeated boolean flags are normalized. Documented config option syntax, module IDs
  and runtime cardinality are checked before reporting unavailable. Command-specific
  help distinguishes implemented bindings from missing host operations. An executable
  file with an arbitrary name is no longer considered a ready command.
- `tooling/package-release`, workflows, scratch artifact-export Dockerfile and
  BUILD-RELEASE.md: external locked Rust/musl build of pre-prepared helpers,
  clean committed source, provenance/per-file/archive checksums, deterministic
  archive metadata. This is an unsigned candidate, not signature/acceptance evidence.
  Until the exact I4 completion marker, CI runs only source/doc checks; builds are
  blocked. After implementation: build → explicit U0 → F0; no publish/deploy.

Limits: hton-host is not yet connected to the complete NativeAction/AdmissionPorts; observation
facts do not prove ownership, exact launch equivalence or semantic health. Native
mutation, validators, collector/guardian, installer, certificate/archive crypto and
the remaining end-to-end R1 composition are still I3/I4 work. Amnezia installed evidence
has still not been obtained; neither foreign resources nor public port 443 were changed.

Only source/document checks were performed: rustfmt parse/format, bash -n, TOML/YAML/JSON
syntax and reciprocal file graph (237 units, 0 errors), git diff --check. Cargo
check/build, release recipe, Docker/systemd commands and unit/functional tests were
not run. No completion marker was added: selected plans are not fully implemented.


## Build and first U0 unit layer (2026-11-01, continued)

The workspace had not been built after integrating saved source changes. Nine
compilation errors were fixed and the code was brought to a successful full build:

- `crates/core/src/host_crypto.rs` removed — it duplicated module `host_crypto/mod.rs` (E0761);
  the directory is active (application/archive/certificates/identities/inventory/restore).
- `crates/core/src/host/effects.rs`: `plan.candidate_hash.as_str()` for formatting (E0277).
- `crates/core/src/render.rs`: `hashes` receives `contracts::Sha256Digest` with explicit handling of
  `InvalidIdentifier` instead of `convert(..)` with `wire::InvalidValue` (E0308).
- `crates/core/src/host_crypto/inventory.rs`: three unqualified `digest(..)` → `config::digest(..)`.
- `runtime/src/guardian.rs`: `Lease` was extended `started_from_stopped: false` (trial armed from running).
- `runtime/src/host.rs`: implemented `start_candidate` — delegates the flow
  admit→arm→apply→verify the same executor as `Activate`, without duplicating
  business logic; `HostEffectGuard` marked as an RAII lock.

Checks actually performed:

- `cargo check --workspace --offline` — succeeded (main warnings were fixed `cargo fix`).
- `cargo fmt --all` — applied; `cargo fmt --all --check` is clean.
- `cargo build --workspace --offline` — succeeded (compilation and linking of all binaries).
- `cargo test --workspace --offline` — 24 unit tests pass: validation of logical
  identifiers, `Sha256Digest`/`ImageDigest`, known-answer SHA-256 `digest`,
  `content_revision` (object identity); `catalog`: `valid_hostname`/`listener_overlap`
  (IPv4/IPv6 wildcard, v4-mapped); `runtime-contract`: `ArtifactPath` (path traversal),
  `HostName`, digest and canonical `hash`.

This is the start of U0 (unit), not completion R1 or acceptance. Unit coverage still needs expansion
(plan, registry/journal/secrets, runtime/adapters/logging/backup), F0 functional
scenarios and A0. Completion marker not added.

## U0: plan and manifest (admit) unit layer (2026-11-02)

Unit tests were added for the planned U0 step:

- `crates/core/src/plan.rs`: `runtime_binding_revision` is deterministic and distinguishes
  revision/run-intent/run-epoch/accepted-bundle; `step` produces stable, bounded
  action IDs (prefix + 32 hex) even with maximally long service/resource.
- `runtime/src/manifest.rs`: independent fail-closed admission through a mock
  `AdmissionPorts` — stale caller, stopped intent, maintenance, active lease,
  mismatch manifest digest, unparseable manifest, empty service list,
  duplicates service_id, mismatch artifact/service, oversized artifact, regression
  credential fence, absence native evidence, identical before/after, resource drift,
  corrupt artifact bytes; `admit_accepted` rejects stopped and foreign accepted
  bundle; happy-path `admit` returns `AdmittedBundle` with activated status,
  resources and artifact bytes.

Checks: `cargo test --workspace --offline` — 46 unit tests (contracts 6, core 16,
runtime 19, runtime-contract 5), 0 failures; `cargo fmt --all --check` clean.

Limits: the full `plan()`/`prepare_actions()` (exact-input effect planning) directly
is not covered — fixtures are required `NormalizedSnapshot`/`DependencyGraph`/registry/
observations; only pure helper functions are covered. Dependency-cycle and separate
compensation invariants `validate_material` were not isolated in separate tests.
Remaining: registry/journal/secrets, runtime/adapters/logging/backup, F0 and A0.

## Historical trial prototype: HAProxy SNI passthrough (2026-11-02)

The owner requested a trial of an HTTPS daemon with a placeholder from `/var/www`, HAProxy and
Amnezia integration (D82). The test environment is local; the port is restricted to loopback `127.0.0.1:4443`.
Live port 443 and container `amnezia-xray` were not switched or restarted.
This is local prototype evidence for D84, not a production `scripts/install` configuration;
the installation target now specifies `example.com:443` and separate persistent VPN
ports. Project Caddy does not mount `/var/www`: it publishes exact bytes of the
verified `index.html` as an immutable `site-index.html` bundle artifact and routes
the site only to it (D98), preventing access through symlinks.

Environment: Amnezia is already deployed (Docker, running `amnezia-xray` container with
Docker `restart=always` policy). The sandbox manually routes SNI
`www.googletagmanager.com` to host port 443. Amnezia publishes TCP and UDP
`443` over IPv4/IPv6; its version, other restart owners, native SNI/target and safety
of remapping remain unqualified. HAProxy inside the sandbox uses image `haproxy:lts-alpine`.
Only current local facts are confirmed, not the Amnezia version or production VPS.

Test environment `/home/user/hton-sandbox` (Docker Compose): `nginx:alpine` (TLS 8445,
self-signed `hton.local`) + `haproxy:lts-alpine` (TCP/SNI, loopback 4443 only).
`/var/www/hton-demo/index.html` contains a static placeholder from `platform/web/mask`;
nginx mounts `/var/www` read-only and serves `hton-demo`. Verified after connecting `/var/www`:
- SNI `hton.local` → HAProxy → nginx → placeholder (HTTP 200; certificate is local and self-signed, TLS 1.3 handshake succeeds).
- SNI `www.googletagmanager.com` → HAProxy → TCP passthrough → `amnezia-xray:443`;
  upstream responds with TLS 1.3 and certificate `*.google-analytics.com`. This does not
  confirm native transport or a successful Amnezia client connection. The path was
  checked without modifying live Amnezia. `docker compose config -q` succeeded.

Conclusion: SNI routing on a shared port (VPN passthrough + HTTPS website) works.
This does not confirm a public HTTPS certificate, valid DNS publication, native
transport type, full client VPN handshake or downtime-free cutover. Moving to live
443 requires installation-time Amnezia qualification and a specific port ownership
change plan with consent for VPN downtime. Core dispatch remains incomplete; this local
test environment is not deployment or R1/F0 acceptance. No build or product tests
were run in this iteration.

## CLI consent exit mapping (2026-11-02)

`crates/core/src/main.rs` now returns public CLI exit 4 for `approval.required`, preserving
the structured error payload; other Core operation failures return 1 and usage stays
2. Updated both `docs/sdd/file-index.json` and the mirrored Core entry contract.
Rustfmt parsing, `bash -n` for fixed dispatchers and JSON parsing of the file index
passed. No Cargo build or tests were run.

## Core lifecycle dispatch: up and rollback (2026-11-02)

`lifecycle.up` now starts the saved accepted composition through Core lifecycle
journaling and fixed runtime IPC; it does not compile pending YAML or pull images.
`runtime.rollback` accepts only the exact previous-compatible bundle bound to the
current committed lease. Runtime rechecks epoch, resource receipts, fallback
authority/digest, credentials and certificate validity before and after inverse
actions. A concurrent `down` wins by epoch; a successful one-step rollback consumes
the previous-compatible pointer. Interfaces, decision D83, CLI and SDD maps were
updated. Rust syntax/format checks and whitespace checks passed; no build or tests.

Dispatch remains incomplete: `publication.switch`,
`foreign.restore`, `doctor.fix/watch`, `installation.uninstall`, `bundle.validate` and
`modules.remove/purge/update` still fall through the generic incomplete-operation
error. CLI names/allowlists alone do not count as bindings.

`doctor.observe` is wired read-only through `host.observations::diagnose` and returns
the same bounded services/runtime/Docker counts, issues and unverified sections as
`diag.observe`, under operation ID `doctor.observe`. It accepts no operation argv,
does not probe public networks and performs no repair. `doctor.fix` and `doctor.watch`
remain unbound: there is no exact owned-resource repair planner/action path or
implemented single-owner scheduler. Other listed operations remain unwired because
the required owner boundaries are absent: listener-holder/cutover authority for
`publication.switch`; drift-aware ForeignLifecycle transition and durable restore for
`foreign.restore`; ownership-scoped host removal plus foreign restore before purge
for `installation.uninstall`; protected BundleId lookup and isolated hash-bound test
runner for `bundle.validate`. The R2 web publication feature is not the public-port
switch operation. No Cargo build or product tests were run for this addition.

## Core diagnostics and identity dispatch (2026-11-02)

`diag.observe` now uses only registered local Docker/systemd/runtime facts with a
128-resource observation cap; unsupported requested sections remain unverified
and it issues no network requests. `identity.whoami` is an explicit read-only
operation that may make bounded public IPv4 and configured-domain DNS-A probes
(maximum 8 domains); other identity fields stay `unprobed`. Default output is a
small human report; `--json` returns redacted metadata and `--quiet` does not hide
requested report data. Maps and call graph were updated. Rust syntax/format and
document graph checks were performed; neither command nor its external probes were
run, and there was no build or product test.

## Core services.restart dispatch (2026-11-02)

`services.restart` enters the Core lifecycle journal and sends a fixed typed Runtime
action carrying the exact accepted bundle ID, planned run epoch and a nonempty
service selection. Core resolves omitted selection from the saved enabled optional/
data service closure, intersects it with installed Owned registry resources and
current accepted receipts, and excludes core ingress; shared services such as Xray
follow their enabled consumers. Core rechecks desired/config, registry and runtime
read sets before IPC. Runtime independently admits the bundle, rejects stale epoch,
empty/Associated/missing resources and active trial/maintenance state, and restarts
only exact Owned identities; launch drift cannot become Recreate or artifact rewrite.
Post-restart probes are scoped to selected services. Unsupported probe kinds produce
journaled Partial with successful Runtime receipts and survivor IDs preserved.

`bundle.validate` contract gap is now reconciled in `INTERFACES.md`, `sdd-control.md`,
and the `validation.bundle` / `cmd.validate` maps. The command contract resolves only
protected BundleIds, binds results to candidate/artifact/image hashes, requires an
isolated private test project with exact cleanup/recovery, and forbids public binds
or foreign mounts. No `validation.bundle` implementation, Core dispatch, semantic
runner or attestation storage is wired yet. This contract pass does not complete the
operation; build and tests remain deferred.
`--wait` defaults to 10 seconds, accepts 1..300 and narrows the Runtime deadline.
No build, tests, external services or trial containers were run.

The provided `/tmp/hton-rustup` rustfmt parsed and formatted the touched Rust files;
the subsequent `rustfmt --check` passed. JSON and Markdown whitespace checks also
passed. No Cargo command, build or test was run.

## example.com / HAProxy / Amnezia live preflight (2026-11-02)

The owner updated the authoritative A records for `example.com` and `www.example.com`
to this host (`203.0.113.10`). Direct DNS checks through authoritative servers,
Cloudflare and Google resolvers returned the new address; the host's configured
resolver still cached the previous value. Before switching traffic, direct IP
observations showed TCP/443 still belongs to `amnezia-xray` and returns its
REALITY fallback certificate; TCP/80 belongs to Apache. There is no trusted
`example.com` certificate installed yet.

Read-only, secret-filtered Xray evidence: local image `amnezia-xray:latest`
(`Xray 26.7.28`, no upstream version label), container restart policy `always`,
no declared mounts, one VLESS/TCP/443 REALITY inbound, exact server name and
target `www.googletagmanager.com` / `www.googletagmanager.com:443`. Config remains
inside the container's writable layer; a recreation without a qualified snapshot/
restore path could lose Amnezia state. No Amnezia client handshake was run.

At the time of the earlier D89 observation, Amnezia 8443 conflicted with
`outline-operator-two` on TCP and UDP; that separate container was not touched. D91
supersedes 8443 as the target with persistent unique 5000–6000 allocations. The
Outline mapping remains preserved unless explicitly integrated and separately
authorized for compatible remap. The installed Core's `install.reconcile` currently reports
`publication: not-changed`; `/opt/hton` and its protected ACME secret store are
absent. Existing certificate SDD requires DNS-01 through the supported InternetBS
ACME flow; credentials have not been configured. No Docker mapping, firewall,
Apache, Amnezia, Outline, certificate or public listener was changed in this step.

## Persistent VPN port allocation implementation (2026-11-02)

Implemented a descriptor-owned allocation binding for Amnezia, dynamic host
ListenerClaim materialization from the same mapping, a pure planner for unique
random 5000–6000 ports with saved-value reuse, complete bounded Linux/Docker
reservation evidence, exact registry ID+fingerprint matching, digest-bound local
TTY approval, and a journaled narrow module-settings CAS writer with exact recovery.
First-install missing module settings are now modeled explicitly in `ReadSet` and
can be created atomically from trusted descriptor defaults with the first port; the
journal records whether the original document existed so recovery distinguishes
absence from an existing empty file. Descriptor settings object IDs now match the
config reader's canonical `module.<id>` keys.
The shell/Core input path no longer asks users to type Amnezia TCP/UDP ports.
Repeated allocation commit is a no-op when saved values already match.

The allocation approval owner now also exposes a first-install path that performs
the same exact local TTY challenge against a pre-generated instance ID and returns
an operation-scoped in-memory capability without creating config/work/journal state.
It can be consumed by the existing allocation writer after bootstrap, but the
installation coordinator does not call it yet. No first-install port has therefore
been saved end-to-end by `scripts/install`.

`LocalForeignObserver::inspect_candidate` now rechecks the selected full Docker ID
against a fresh complete inventory, reads the fixed fingerprint-bracketed Xray
server config, and parses a bounded allowlist for one VLESS/VMess TCP REALITY
inbound. Secret-bearing JSON stays local to the parser and output is redacted. This
provides only native-config evidence; inspection still returns `moduleUnsupported`
because installed version, live listener, external restart/data owners and protected
remap snapshot are not established. It does not qualify or associate Amnezia.

For this implementation slice, rustfmt `--check`, JSON parsing of the file index
and `git diff --check` passed. No Cargo build or product tests were run; the install
and qualification paths remain unfinished.

## Install preflight and allocation persistence (2026-11-02)

The read-only `install.reconcile --input-preflight` now composes the catalogue's
selected and retained allocation bindings, OS entropy, fresh TCP/UDP reservation
evidence and immutable `PortAllocationCandidate`; the candidate is bound to the
same reservation basis in `InstallPlan`. A structural catalogue resolver supports
preparing that candidate before certificate issuance; certificate claims remain
phase requirements and ordinary resolution must pass before ApplyIngress. Root
revisions and virtual first-install absence are rechecked for stale snapshots. The
mutating install path now commits that exact candidate through `PortSettingsWriter`
before root configuration seeding, and repeated/resumed installs read its persisted
service settings. Port journal recovery can run before `installation.json` exists.
Saved-port changes retain their separate approval; OR-10 authorizes a first
assignment after fresh collision checks. Safe Amnezia snapshot/remap/restore remains
unwired and its installed resource is not qualified. If `ss` cannot prove a Docker
proxy socket belongs to one exact Docker binding, that socket remains an unknown
reservation and may block reuse. No live Amnezia remap, build or product tests are
claimed. Static formatting, JSON parsing and `git diff --check` passed.

## Saved allocation on module enable (2026-11-02)

`modules.enable` now plans the prospective selected graph before writing selection,
using fresh complete host TCP/UDP reservation evidence and OS entropy only when a
managed service setting has no saved port. OR-10 authorizes first assignment after
fresh collision checks without a second TTY prompt; only changing an already saved
value requires exact local approval. Core rechecks the config read set, registry
revision and exact fresh reservation digest after any required TTY review, then
commits through `PortSettingsWriter` before selection/apply. Repeating enable reads the saved service settings, produces a
no-op allocation and does not reroll the port. An interrupted exact allocation is
recovered and the caller retries. This direct host-dispatch flow is not yet composed
through a parent `Intent::Lifecycle`; if the candidate is lifecycle-owned, this
dispatcher refuses to take recovery ownership. The same writer/reuse guarantee now
applies to `scripts/install` through its immutable `InstallPlan.port_allocation_candidate`.
The unqualified Amnezia resource remains blocked until exact native listener,
version, restart-owner and snapshot evidence are available. No build or product
tests were run.

2026-11-02. Recovery of the sole unfinished `Intent::PortAllocation` was connected
at the start of ordinary mutating `install.reconcile`. Read-only preflight scans the
bounded journal inventory and returns `recoveryPending`; the shell calls ordinary Core
reconcile without repeating prompts/bootstrap. Core checks the journal again, does not
take over an allocation owned by an unfinished lifecycle, and restores only exact
old/new bytes through the narrow writer. Recovery requires euid 0 and local
`os.uid.0`, preserves the original operation ID only for terminal publication and
does not change the candidate. This closes the crash window after durable allocation
intent, but initial install approval/commit and HAProxy/site execution are not connected.
Rustfmt, JSON, Bash syntax and diff checks passed; Cargo build/tests are deferred until implementation is complete.

2026-11-02. Added a policy execution seam to `LifecycleService`: add/enable commit
the exact `LifecyclePlan.port_allocation_candidate` through a separate
`PortAllocationCommitPort` after durable parent intent and before selection/apply; enable
is blocked if the trusted prospective graph requires allocation and the candidate
is absent. Production Core dispatch/host composition does not yet provide this port
or prepare the prospective selection graph with complete host reservations, so this
is not an end-to-end CLI guarantee. Changes and lifecycle map passed rustfmt/diff static
checks; build/tests are deferred.

## Current install execution slice (2026-11-02)

`install.reconcile --input-preflight` now reports `ready` only when its immutable
plan has no conflicts or approvals and complete fresh host reservation evidence.
Typed first-publication inputs and the menu candidate are digest-bound and
revalidated by the mutating Core call; their explicit CLI/TTY collection is not
duplicated by another approval prompt. OR-10 authorizes a first random port
assignment only after collision checks; only changing a previously saved port
requires the exact local allocation approval. The allocation writer enforces this
distinction and stores the candidate through its own durable intent/CAS before
installation identity or root config publication. Port intent recovery no longer
requires `installation.json` to exist.

The install coordinator now saves the site domain, public TCP/443 and non-secret
certificate account email, seeds fixed InternetBS DNS-01 SecretRefs, runs supported
owned setup, imports no credential bytes through install argv, requests cert issue/
reuse, and runs normal effects prepare/apply. Missing DNS credential versions
return `partial` with the fixed `requiredNext:["importInternetbsDnsCredentials"]`
marker. Caddy receives the exact bounded UTF-8 `/var/www/index.html` bytes as an
immutable `site-index.html` bundle artifact and routes site requests only to that
artifact; it mounts no host webroot. Success is returned only after a fixed local
TLS/SNI/HTTP 200 probe serves that exact body and source identity/digest/bytes remain
unchanged across the probe. Any unsuccessful phase returns `partial`, not
`ready:true`. Public DNS remains explicitly `unverified` and no automatic identity
probe occurs. The separately qualified Amnezia remap remains blocked.

Amnezia remains fail-closed as foreign/unqualified: native version/listener,
restart/data ownership and exact snapshot/remap/restore evidence are still absent.
Accordingly the requested Amnezia 5000–6000 remap is not claimed as executable.
No build or product tests were run for this slice.

## Current source deployment and readiness guard (2026-11-02)

`scripts/deploy` now inventories the current working tree directly, including
uncommitted changes and the Rust source/build inputs needed to represent the same
working version under `/opt/hton`. It excludes `.git`, `docs/_lang/ru/tasks/`, `target/`,
`var/`, generated `etc/unbound/root.key`, and mutable/credential state. It does not
require `BUILD-PROVENANCE.json`, a release archive or a clean commit, and does not
build on the target. A content-addressed source snapshot is copied and verified,
then `/opt/hton/current` switches atomically. Existing snapshots are reused only
when their complete file inventory matches. Source files are opened without
following symlinks, copy hashes are streamed, concurrent deploys are serialized,
and the source inventory is rechecked before switch. These hashes verify copy
integrity, not source authenticity. The stable CLI runs the active snapshot with
config/work state at the stable `/opt/hton` root.

Installation previously could report `ready:true` after proving only local site
HTTPS while the Amnezia route remained unqualified. That result is now fenced:
site verification is reported as `siteReady:true`, but install returns partial and
`ready:false` until qualified Amnezia route and native-handshake verification are
implemented. The foreign remap gate remains fail-closed. Bash/Python/Rust syntax,
file-index JSON and whitespace checks passed; no product tests, Cargo build,
release package or VPS deployment were run. I4 remains incomplete, so D81 still
blocks the release build and U0/F0 remain pending.

## Minimum OR-15 source closure, 2026-11-03

The selected five-module Python Core, default CLI deployment handoff and deployment
inventory are implemented independently of incomplete full R1. Minimum verification
is now authorized in build, unit, functional order. This records source completion
of the selected closure, not production or full-R1 readiness. Functional fixtures
never authorize touching existing production containers or issuing certificates.

<!-- hton-implementation-complete: M0 -->

### M0 acceptance evidence (2026-11-03)

- Build: tooling/build-minimum succeeded; deploy inventory has 153 product files.
- Source tree: `984e69e4ab401e04d11516b8d6bc735d850052447ea40d30914c497c1a7a1d4c`.
- Unit: 14 scenarios passed, including automatic cutover failure restoration.
- Functional: pinned issuer/provider available offline; real HAProxy/Caddy HTTPS
  and trust rejection; authenticated REALITY routing and invalid-user rejection;
  real Docker writable-layer snapshot/recreate, complete Config/HostConfig
  preservation and retained original verification. All passed using fixtures.
- Production not installed by the agent: local sudo requires interactive owner
  authentication; real DNS provider credentials were not supplied. No real ACME
  issuance or production remap is claimed. Run scripts/install as documented.
- Full R1 remains incomplete; its build/acceptance marker is still absent.

### M0 native DNS attachment fix (2026-11-03)

The first actual install correctly retained the live VPN but exposed an incomplete
engineering assumption: native Amnezia has both bridge and amnezia-dns-net. The
old single-network check rejected it before any remap. D108 adds bounded dynamic
local bridge preservation by immutable ID, explicit aliases and gateway priority,
with exact topology drift checks and read-only dependency verification. Native
networks, DNS and peers are never modified.

A second reproduced defect caused set -e to exit before sudo if the caller's old
session could not traverse the protected deployed snapshot. Active-link resolution
is now best-effort before sudo; protected modes stay unchanged. The actual
scripts/install --dry-run reproducer now exits 0 in that session.

Build e9471b411c32a5d2c9b4210de8b2bdeebfa9ea858341854daf3cf06a2ab52f27
passed, followed by 17 unit checks and all native functional checks. The Docker
fixture now creates its own labelled second bridge, starts the replacement,
checks full launch and both network identities, then removes only those fixtures.
Read-only actual Amnezia dependency preflight accepts both existing local bridge
networks. Production original remains running on its original TCP/UDP 443. No
real certificate issuance or production cutover is claimed by this fix.

### M0 interactive terminal defect and diagnostics (2026-11-03)

The next live attempt reported an opaque unexpected exception before input. The
agent cannot read the private root journal without owner sudo; its exact recorded
exception kind was requested, not assumed. Independently, a real controlling TTY
reproducer confirmed that open(/dev/tty, r+) raises UnsupportedOperation on this
Python/Linux environment. Both normal and secret input used that invalid mode.
D109 replaces ordinary input with separate non-seeking streams and delegates
secret input to getpass. Unexpected errors now report owned module/function/line
and exception kind; private logs include the same redacted metadata, never raw
exception messages, locals or native output.

Build ab11b6bfa23a989fdbfc52147561cdbcc8c3f6be76f989b26b133f2a4cff6ba3
passed, then 19 unit scenarios and all native functional component checks passed.
New PTY acceptance exercises real ordinary/default input, exact remap consent,
private credential persistence and secret echo suppression. Diagnostic acceptance
checks that unexpected exception text is withheld while the owned failure location
is retained. The existing native camouflage TLS route also passed read-only.
No production certificate issuance or remap was executed; retrying the protected
installer is still required to establish live readiness.

### Configured inputs and install/setup split (2026-11-03)

OR-16/D110 adds platform/etc/install.json with the owner domain/email and configured
installation.directory. Shared lib/install_config.py owns finite parsing and
protected-root resolution. Default target remains /opt/hton; canonical installed
settings win, absent inputs are prompted and saved in config/hton.yaml before
native consent. Docker client custody, native state, units, CLI and source delivery
use the configured root. Existing state is never implicitly relocated.

OR-17/D111 adds target-only scripts/setup and a physical-CWD install orchestrator:
outside target, deploy then cd then setup; inside target, setup without deploy.
Wrong-directory setup refuses before sudo/effects. Build
92ed087e380a103183ee6831a8544f7a1d081ff3e1b3fb1ccd9ab584729daddf
passed, followed by 27 unit checks and all native functional component checks.
The new checks cover configured deploy previews, settings persistence/precedence,
missing prompts, root-dependent units, install delivery/CD/setup order and
deployment-failure propagation. No production setup or issuance was run.

Outstanding OR-17 work: Git-first Docker update is not implemented pending the
required owned-versus-all-host scope clarification. Its old config-apply alias is
disabled; dry-run reports scopeRequired. No Git pull, image upgrade or unrelated
container change was performed. Full R1 custom-root/schema migration and broader
management remain outside minimum acceptance and are not claimed ready.

### Readable remap consent (2026-11-03)

OR-18/D112 replaces operator hash retyping with a readable multi-line summary and
explicit y/yes approval, declining by default. Exact native identity/config/port
is still bound by an internal scope digest and --yes cannot supply approval.
Real PTY consent and hidden-secret acceptance passed along with all 28 unit
scenarios. A running old installer still requires the old REMAP string shown in
its terminal; source edits do not change that already-loaded process.

Native functional component acceptance also passed after the consent change:
issuer/provider availability, trusted-site routing checks, genuine REALITY success
and invalid-user rejection, and lossless multi-network Docker snapshot/recreate.
No production issuance or cutover was performed by the agent for this UI change.

### Public-product scope correction (2026-11-03)

OR-21/D114 establishes fleets of VPS/domains as product scope. The minimum
profile's one-site selector and InternetBS binding are implementation limitations,
not architecture requirements. Owner domain/email in platform/etc/install.json are still
instance values in the public bootstrap and must move to deployment-specific input
without overwriting protected settings. HTTP-01, provider-neutral/topology-aware
DNS preflight, multi-domain selection and fleet challenge/renewal coordination
remain unimplemented. Research/specs/interfaces now distinguish those concerns.
This document does not claim fleet readiness or completed issuer migration.

### HTTP-01 without DNS credentials and existing HTTP service (2026-11-04)

D115/D116 implement the previously missing runtime change: default HTTP-01 in
the pinned acme.sh lifecycle, explicit DNS-01 adapter selection, narrow migration
of unissued/uncredentialed legacy attempts, and provider-neutral setup DNS help.
Public source defaults contain no personal domain/email; protected instance config
is preserved. Public identity checks are explicit setup only; background renewal
reuses its saved challenge transport and actual-certificate renewal window.

An existing HTTP listener is not stopped, restarted or reconfigured. The issuer
can use a verified webroot, mounting only its public challenge subdirectory. Saved
owner identity supports unattended renewal without SUDO_UID; new public proof
data is readable under the timer's private umask. Existing files/permissions and
other challenge tokens are preserved. Explicit same-generation setup refreshes
owned timer source bindings without restarting accepted site/VPN containers.

Build 421c1c69e1a6a868a8569e315d3fdab2ae5c492a14d12e81f0ce7c71a0ab5adc
passed, then all 38 unit/component scenarios and native functional acceptance
passed. The local ACME authority exercised real pinned acme.sh issuance in both
standalone and existing-webroot modes, checking exact token/thumbprint, public
file readability, key/SAN match, unmodified foreign tokens and private-CA rejection
by production trust. HAProxy/Caddy, REALITY positive/negative authentication and
lossless multi-network Docker recreate also passed.

Actual setup-assistance observation matched example.com A to observed outbound IPv4
203.0.113.10 through Cloudflare. The existing Apache challenge route was proven
via /var/www/html with a temporary token, then cleaned. No real ACME issuance or
production native remap was performed by the agent. Operator scripts/install remains
required for privileged live acceptance. Fleet/multi-domain coordination, additional
DNS adapters/CA failover and the pending Git-first update scope remain incomplete.

### Docker owned-launch false drift after real issuance (2026-11-04)

The real ACME container exited 0 but cleanup/recovery ownership validation failed.
A disposable version-only job using the production issuer argv reproduced it:
Docker 29 resolves OomKillDisable null to false on first start, changing the raw
launch hash. D117 centralizes that default in docker.canonical_host_config, versions
new owned receipts and accepts legacy receipts only under that exact null/false
representation. True, other launch fields, unknown receipt versions and artifact
drift still refuse mutation. The exact saved job/output can therefore be resumed
and validated without discarding the completed certificate.

Build 8faa79eaa46645f67a08d7a8ad5eb752415644f38a40424a219b750bf5747086
passed, then 39 unit/component scenarios and all native functional acceptance
passed. HTTP fixtures now call actual record_owned/owned before and after native
client execution for both standalone and existing-webroot modes. Additional
offline Xray probe passed with production client capabilities. No production
container deletion, certificate reissue or VPN cutover was performed by the agent.

### Foreign replacement false drift at cutover (2026-11-04)

A live `scripts/install` reached route-verification and stopped with "Foreign
launch/config drift; refusing overwrite". The replacement passed the create-time
`check_foreign` but failed the same check after `docker start`, because Docker 29
reports an unset OomKillDisable as `false` on create and `null` after start. The
same false positive then hit `rollback`, leaving the replacement active as
`/amnezia-xray` and the original parked/exited. D119 canonicalizes both the
expected and observed HostConfig through `docker.canonical_host_config` before
the equality check, matching the D117 normalization already applied to owned
receipts. Verified against the live Docker 29.9.2 socket (create=false,
start=null) and with a new unit regression that accepts null/false and still
rejects a genuine OomKillDisable=true.

### REALITY auth false failure on cold cutover (2026-11-04)

After D119 and D120, `scripts/install` still stopped at "Checking HTTPS and the VPN
connection" with `Network request failed (exit 28)`. Reproduction against the
live socket showed the individual pieces all work: the auth probe succeeds
against the original (443), the replacement (5617), and a reproduced haproxy
route; the snapshot REALITY config is byte-identical to the original. The failure
is a cold-start race: a freshly started replacement's REALITY endpoint needs
~7-9 s before it can complete the authenticated tunnel, but `authentication` ran
~1-2 s after `replacement-start` and the first curl attempt timed out. D121
retries the authenticated probe against a bounded 30 s readiness budget. Cold
start now succeeds (~8.7 s) where the old code aborted immediately.

### Minimum MTProto/Hysteria/Outline support and `cmd`→`scripts` rename (2026-11-04)

OR-25/D123 adds opt-in support in the minimum profile through a separate
`python/minimum/protocols.py` owner. MTProto uses the pinned `mtg` image behind an
exact camouflage-SNI HAProxy route; Hysteria2 uses the pinned server image on an
explicit UDP port with password auth and the existing site certificate; existing
Outline is associated by exact container name/full ID and preserved without any
mutation or remap. Settings are stored in `config/modules/{mtproto,hysteria,
foreign-outline}.yaml`; generated secrets and share links stay under the protected minimum work tree
(`root:hton` 0750/0640). Candidate/active protocol containers use the existing
ownership receipts, drift checks, rollback and restart policy. `--yes` cannot
replace the local confirmation for changing a saved camouflage name, UDP port or
Outline association.

OR-26/D124 renames the command directory `cmd/` to `scripts/` and merges it with
the existing build/release scripts. Command file names, behavior and stable map
IDs (`cmd.install`, etc.) are unchanged; paths, CI scans, links, error text, the
mirrored `docs/sdd/map/scripts/` directory and `docs/sdd/file-index.json` are
updated.

Verification actually run: 50 minimum unit scenarios pass; Python compilation,
Bash syntax, `platform/images.lock.json` and `docs/sdd/file-index.json` JSON parsing pass.
Disposable native acceptance passed with the pinned images: mtg generated a secret,
started and accepted a loopback TCP connection; Hysteria2 authenticated a real
client over QUIC and relayed a TCP request through its SOCKS5 listener; the exact
live Outline container was observed read-only and accepted by the association
validator. No production install, port change, container mutation, certificate
issuance or public cutover was performed; local sudo needs interactive owner
authentication. Full R1 and `outline-wss` remain unimplemented and unclaimed.

## REALITY self-steal sandbox proof (2026-11-04)

Live read-only observation: HAProxy on public TCP/443 allows `example.com` and
`www.googletagmanager.com`; `example.com` goes to Caddy with a trusted
`CN=example.com` Let's Encrypt certificate and the `/var/www/html` body, while
`www.googletagmanager.com` goes to `amnezia-xray` and returns the Google fallback
certificate. The running native listener has one REALITY inbound with
`serverNames`/`dest` `www.googletagmanager.com` and 12 clients. The native
container is on Docker bridge `172.17.0.6`; Caddy is host-networked on
`127.0.0.1:9443`.

Disposable sandbox, no live configuration changed: Xray 26.7.28 on
`127.0.0.1:15617` with `serverNames: ["example.com"]` and
`dest: 127.0.0.1:9443` returned the real `CN=example.com` certificate and site body
for an unauthenticated TLS handshake. A second sandbox bound Caddy to
`172.17.0.1:19443`; the result was reachable from inside `amnezia-xray` and served
the same certificate/site. An HAProxy configuration with the REALITY backend as
primary and the site backend as a health-checked `backup` passed `haproxy -c`.

Not applied: native `serverNames`/`dest`, HAProxy/Caddy artifacts, minimum state
requalification, client reissue, or any public cutover. The required migration
also needs the site/REALITY overlap guard changed for self-steal and a consented
way to update the saved native digest; hand-editing state is not supported.

## Single-443 composition and Outline replacement (2026-11-04)

OR-29/D127 requires MTProto, Hysteria2 and an HTON-owned Outline on public 443.
The foreign `outline-operator-two` container was removed; its official
`outline-ss-server` v1.9.2 binary is staged at
`/opt/hton/bin/outline-ss-server` with the recorded sha256. The
`/home/operator-two/outline` data directory remains under the other user's home and was
not deleted. Ports 8443 are free.

The own Outline is designed as Shadowsocks-over-WebSockets behind the site TLS
terminator: `websocket-stream` and `websocket-packet` paths on a loopback
listener, reverse-proxied by Caddy, reached through HAProxy TCP/443. The minimum
profile now implements an own `outline-wss` plan with Caddy WS routes, container
lifecycle, access-key output and unit tests; the live install still has not been
applied. MTProto/hysteria are implemented but not enabled in the live minimum
install. Hysteria2 binds UDP/8443 directly; HAProxy is TCP-only and cannot proxy
it, and the owner restricted TCP/443 to VLESS/HTTPS traffic.
No HAProxy/Caddy/state or live install mutation was performed in this step;
interactive sudo is required.

## Minimum delivery-closure audit (2026-11-04)

OR-30/D128 makes `/opt/hton` delivery always minimal. Full source inventory:
159 files / 2.54 MB. Minimum runtime closure: 44 files / 304 KB. The minimum
closure contains `bin/hton`, the three `lib/` helpers, `python/minimum/*.py`,
`platform/etc/install.json`, `platform/images.lock.json`, README/LICENSE, `docs/CLI.md` and
`docs/MINIMUM-INSTALL.md`, plus the shell entrypoints and CLI wrappers excluding
the four build/bootstrap-only scripts. Full-R1 Rust sources, adapters, contracts,
runtime crates, schemas, module descriptors, systemd templates, compose/docker
inputs, validation/web assets, Cargo files, `platform/hton.example.yaml` and unused docs
are not delivered.

`scripts/deploy` has no profile argument and always delivers the minimum
inventory. It prunes files inside selected top-level paths and removes managed
top-level paths left by an older full delivery, while preserving `config/`,
`work/`, `libexec/`, `deployments/`, locks and markers. `lib/hton` no longer
passes a deploy profile;
`tooling/build-minimum` packages the minimum closure. Verification: 52 minimum
unit tests pass, the deploy dry-run reports the expected count, and a
temporary-tree prune test preserved config/work/libexec while removing `cmd/`,
`platform/systemd/`, `crates/core/src/` and `Cargo.toml`. The live `/opt/hton` has not yet been
redeployed; the next privileged minimum install or deploy applies the prune.

## Install rollout and incremental migrations (2026-11-05)

OR-34/D132 makes `scripts/install` the rollout command. It must run from the
cloned project root; when the source root differs from the configured target it
deploys the current minimum closure, then runs the installed setup as root with
`SUDO_USER` preserved. Deploy creates the `hton` group, adds the invoking user
and applies `root:hton` modes. `python/minimum/migrations.py` keeps a protected
ledger at `work/minimum/migrations.json` and applies each migration once, in
order. Initial migrations remove the obsolete single-key Outline material and
the superseded foreign-outline document, then normalize runtime/config
directory and file permissions. The source-equal installed copy skips delivery
and applies migrations/setup only. 53 unit tests pass; no live install was run
in this step.

## Single neutral SNI chain (2026-11-04)

OR-31/D129 implements one neutral 443 SNI for REALITY and MTProto. It defaults
to the configured site mask domain; `--chain-sni` is an optional subdomain
override. Sandbox proof: REALITY with `serverNames=example.com` and `dest=mtg`,
followed by mtg domain-fronting to Caddy, returned the real `CN=example.com`
certificate and `/var/www/html` body; mtg logged parsing the ClientHello. The
minimum profile now accepts `--chain-sni`, creates an HTON-owned `selfsteal` Xray
container from the native private key/short IDs/clients, routes it in HAProxy
with Caddy as backup, enables Caddy TLS/host coverage for the chain SNI, requests
a certificate SAN, and advertises the chain SNI in the MTProto link. The old
native `www.googletagmanager.com` route remains during client migration; native
`serverNames`/`dest` are not changed.

Live state: the default mask SNI is `example.com`, already covered by the installed
certificate. `www.example.com` also resolves to `203.0.113.10`; selecting it with
`--chain-sni` requires a SAN certificate, which the privileged install would
issue. 52 minimum unit tests pass, including self-steal
config, chain HAProxy/Caddy routing and mtg fronting. The first minimum selection
now enables MTProto/Hysteria2/Outline WSS without feature flags; the shared SNI
defaults to the site mask domain, Hysteria2 to UDP/8443, and the Outline binary
path is built in. No live deploy/install or Telegram-client acceptance was
performed.

## Native REALITY own-SNI migration (2026-11-05)

OR-35/D133 makes the own SNI (`example.com` by default, or `--chain-sni`) part of
the installed native REALITY listener instead of a separate container.
`core.minimum.protocols.native_selfsteal_config` rewrites only `serverNames` and
`dest` while preserving the private key, short IDs, flow, clients and listener
port. `scripts/install` derives the own name from the current native config,
requires a separate local `y/yes` consent, and applies the change to the
HTON-owned replacement during cutover. With the neutral chain, `mtg` binds the
Docker bridge gateway and is reached only through the native REALITY fallback
(`dest=<gateway>:<mtg>`); without the chain, native `dest` targets the site TLS
backend on the gateway and Caddy additionally listens there. HAProxy routes the
own SNI (and the retained legacy name) to native with the site backend as a
health-checked fallback; the separate self-steal container is retired after
migration. Original and replacement native digests are tracked separately, and
unattended renewal never migrates. The Amnezia app's exported full-access
`vpn://` key therefore carries the own SNI after the consented migration.

Verification run: 61 minimum unit scenarios pass, including the native config
rewrite, native self-steal HAProxy/Caddy rendering and the migrated plan; the
rendered migrated HAProxy config passes pinned `haproxy -c`; pinned Xray accepts
the rewritten REALITY JSON via `run -test` through a read-only bind mount. A
disposable probe showed Docker refuses `cp` into a read-only container, so the
validator mounts the protected directory read-only instead. No live install,
native config mutation or public cutover was performed; the live native listener
still has `serverNames=www.googletagmanager.com` until the owner runs the
consented install.

The first live consented attempt reached route verification but failed there: the
migrated native `dest` pointed at the candidate mtg, which is started only later
in the old ordering, so HAProxy selected the healthy native backend and the site
probe could not complete. `protocols.start_chain` now starts mtg (and a legacy
self-steal container) before edge/site start and route verification; candidates
are still stopped by rollback on any later failure. The rollback restored the
previous edge/self-steal route; the native replacement remains migrated and the
next install resumes without repeating the prompt.
The next resume exposed an approval-scope drift: the saved remap approval bound
the original container's native digest and names, while the check recomputed the
scope from the now-migrated replacement and failed. State now records
`originalNativeNames` (lazily read from the parked original when absent) and
recomputes the approval from `originalNativeDigest`/`originalNativeNames`, so a
consented migration does not invalidate the original remap consent. Committed
rollback also removes stopped ingress/protocol candidates instead of leaving
them orphaned.

OR-36/D134 makes backward-compatible SNI the default after a disposable
pinned-Xray test showed that legacy-SNI clients are reset once native `dest`
points at the internal site chain, even when the legacy name remains in
`serverNames`. The installer now keeps the native listener unchanged and the
separate copied-clients listener for the own SNI unless the operator types
`DROP-LEGACY-SNI`; an existing migrated installation is offered a restore from
`work/minimum/native/previous.json` on the next interactive install, and renewal
keeps the accepted mode. The restore path clears migration markers, records
`legacySniRestored`, preserves the reverted bytes in `native/migrated.json`, and
the legacy-SNI admin key works again against the untouched native listener.

## M1 current status — implementation in progress

The [tracked M1 plan](plans/INSTALLATION-IMPLEMENTATION.md) freezes current
installation/security/page requirements and exact file owners. New managed native,
host-access, encoding, DNS, client-page and config helpers are implemented or
partially wired; see the dated progress sections below. No M1
implementation-complete marker is present; the 2026-11-07 build, 155-test U1 run
and disposable F1 component harness are recorded below, while B1 digests, the
Amnezia acceptance matrix and A1 remain open. Current code still has the
limitations recorded in SECURITY-MASKING-REVIEW.
Full-R1 historical completion/check entries above do not qualify this new scope.
The schema/example/descriptor changes and new image digest are assigned to future
packages; examples/M1-CONFIGURATION.md is the reviewable target during planning.

### Client-page and private guest-share publication (2026-11-07)

OR-49/OR-52 are implemented in the minimum Python path and covered by unit
contracts. A fresh profile with no `core.clientAccessPage.path` receives a
CSPRNG path (`secrets.token_urlsafe(24)`, one safe segment); explicit `path: ""`
is the opt-out and `--rotate-client-page` replaces the URL. The page continues to
be served from `work/client-page` before the site file server and lists active
native/protocol guest credentials with the privacy headers. A successful install
writes `amnezia-<name>.md`, `mtproto.md`, `hysteria.md`, `outline-<name>.yaml`
and `client-page.md` to runtime `.secrets/share` and to the source checkout
`.secrets/share`, removes stale files from the previous publication, and updates
the profile path. `client-page/receipt.json` and `secret-share.json` omit the
bearer path. This is unit evidence only; T-14 real-client acceptance remains
open and no build/live-publication acceptance is claimed.

### One-week log retention and no separate backup (2026-11-07)

OR-53 is implemented in the minimum path. `minimum.log` rotates on the existing
1 MiB ceiling or when its first entry is seven days old; `_prune_logs` then
deletes archived `minimum.log.*` files whose first entry (or mtime when
unparseable) is seven days old, skipping symlinks. Log pressure still cannot
block recovery or VPN traffic. The Core/runtime projection now defaults
`logging.retentionDays` to 7, accepts only `1..=7`, and both schemas cap the
value at 7; the example/profile templates use 7. Two unit contracts cover
old-entry rotation/pruning, recent-material retention and symlink refusal.

OR-54 removes separate backup/restore from R1 scope. `scripts/backup`, its
`lib/hton` dispatch, help/CLI rows, SDD interface rows and the `cmd.backup`
file-index unit are gone. The private `.secrets` repository (`profile.yml`,
`images.lock.json`, `share/`) is the durable copy; recovery is reinstall from it
followed by `scripts/up`. The Rust archive modules remain unexposed dormant code
and are scheduled for removal with the deferred full-R1 reconciliation, not a
compatibility shim.

Build/check evidence, 2026-11-07: `tooling/build-minimum --output /tmp/hton-build`
produced `hton-minimum-08baf647c6c89931.tar.gz` (58 files); the minimum suite ran
155 tests successfully; anonymity tests (3), tracked-shell `bash -n`, in-memory
Python compilation, JSON parsing and `git diff --check` passed. `cargo fmt --all
--check` is clean with Rust 1.99.1.

The Rust workspace build is **blocked**: `cargo build --workspace --locked` and
`cargo check --workspace --locked` on 1.99.1 fail in the deferred full-R1
`hton-core` with 42 errors. Two `hton-host` hard errors were fixed during this
build (`u64` size comparison in `docker.rs`, closure lifetime in `ports.rs`), and
a borrowed reservation fingerprint in `host/observations.rs` was cloned; the
remaining errors are pre-existing integration gaps, not consequences of OR-53/54:
missing `ForeignCandidate*`/`ForeignEvidence` imports in
`host/foreign_observations.rs`, `hton_contracts`/`hton_runtime_contract`
`ServiceId`/`BundleId` duplication, `CatalogueEntry` API drift (`location`,
`ModuleId` vs `ObjectId`), `ContentRevision`/`ReadDependency` mismatches,
`Sha256Digest` private-path import, and `u32`/`u64`/`usize` or `i64`/`u64`
boundary casts. Fixing these is full-R1 I4 work and remains outside the authorized
M1 slice; the M1 minimum artifact and its unit suite are unaffected.

### Install-time and seven-day unattended update (2026-11-07)

OR-55 is implemented in the minimum path. `lib/hton` fast-forwards `main` to
`origin/main` (`fetch` + `merge --ff-only`) before an explicit install; this is
best effort, so an offline, dirty/diverged, detached or non-main checkout is
reported and the existing source is installed still. `scripts/update` requires the
fast-forward to succeed, then deploys (when source and target differ) and runs the
non-interactive minimum `update` action, which re-pulls pinned images and
recreates changed owned containers. Foreign/independent containers are excluded.
`python/minimum/updates.py` owns the redacted status, the seven-day policy and
`work/minimum/update.json`; a successful install/update records the stamp.
`install.py` generates and enables `hton-minimum-update.service`/`.timer`; the
timer checks daily at 04:30 with a one-hour randomized delay, and the action runs
only when the stamp is at least seven days old. Unattended update never prompts,
performs no public DNS/identity lookup, refuses native client-list drift and
requires committed state. `update --dry-run` is network/effect-free. Unit tests
cover the policy, timer text, stamp and a real temporary-repository fast-forward;
no live update or product acceptance is claimed.

## M1 code implementation progress (2026-11-06)

M1-A–M1-D modules are implemented and covered by the unit layer; M1-E is
partially wired. This is implementation progress, not product acceptance and not
the full-R1 completion marker.

- M1-A: `python/minimum/configuration.py` (`load`/`validate`/typed records/resolved
  names), `migrations.plan`, schema additions in `hton-v1/v2` and `module-v1`, and
  synchronized root/module examples.
- M1-B: `managed_amnezia.py` (X25519 key material, native `server.json`, prepare/
  reconcile, native `client_inventory`), `host_access.py` (plan/reconcile/
  deactivate/purge with an injected fixed command boundary), `access_links.py`
  (guest-only encoding; admin/secret payloads rejected).
- M1-C: `services/onomat/runtime/dns/python/dns.py` (owned by Onomat; authenticated
  TLS-forward or explicit recursive render, pinned Unbound validation,
  fail-closed), `docker/amnezia-xray.Dockerfile`,
  `docker/amnezia-xray-entrypoint.sh`, `tooling/build-amnezia-image` (refuses to
  invent digests; missing pins are a precise blocker).
- M1-D: `client_page.py` (typed plan/render/refresh, escaping, bounds, privacy
  headers, withdrawal), `core.minimum.storage.Store.remove`, and an exact
  `/client-page` Caddy route before the site file server in `ingress.configs`.
- M1-E: `scripts/deploy` delivers the new modules, preserves `host-access/`, and
  excludes the build-only script; `install.py` adds the `client-page-refresh`
  action, client-page systemd units/timer, generation binding for the page path,
  and deploy/refresh wiring. Repo unit files are added under `platform/systemd/`. A
  dedicated journaled managed transaction (`managed_state_record`,
  `rollback_managed`, `run_managed`) is selected for `mode: provision` before the
  foreign association flow; it prepares the managed container, certifies, prepares
  protocols/ingress/page, starts, verifies and commits, with rollback-on-failure and
  idempotent rerun. The association flow is unchanged. A standalone private
  `.secrets/` repository (`lib/secrets_store.py`, `scripts/secrets`) holds the full
  profile (root config, modules, SNI/domains, enablement, optional Docker/site
  inputs and secrets) and is imported into the protected `config/` store on first
  install; missing profiles seed defaults and wait for operator confirmation,
  while `-q`/`--json` never prompt. The ready summary now names the native
  Amnezia listener in both flows, prints each exportable guest `vless://` REALITY
  link and writes it to `share/amnezia-<client>.txt` (`root:hton` 0640); the client page
  includes the managed container's native clients. Recreation after an image,
  launch or `dest` change now reuses the live REALITY key pair and complete client
  array (protected snapshot as fallback), protocol secrets are reused from the work
  store, and ambiguous Docker errors abort instead of removing the container.
  Full-access `vpn://` and `host_access.reconcile` wiring remain pending OR-37 work.
  The website document root is now explicit (`site.root` → `core.site.root`, empty
  auto-detects `/var/www/html` then `/var/www`); a configured root is validated and
  mounted read-only for the existing `file_server` route, so install never
  substitutes the placeholder over a real site. A managed reinstall now stops
  previous site/edge and protocol containers before probing shared ports, restores
  them on failure and removes them after commit; this fixes old generations
  answering the 443/9443 probe via `SO_REUSEPORT` and the accumulating ingress
  containers.
- Image policy (hybrid): `install`/`renew` resolve the private
  `.secrets/images.lock.json` (fallback `.secrets.example` then the
  repository baseline), re-pull each image by its mutable `sourceTag` on every
  run, and recreate containers when the resolved image changes. REALITY/client
  keys, SNI, module settings and configuration stay fixed.
- U1: `python3 -B -m unittest discover -s tests/minimum -p 'test_*.py' -v`
  ran 155 tests successfully (2026-11-07), including managed happy-path,
  rollback, idempotency, generated/opt-out/rotated page paths, private-share
  publication and the new one-week log rotation/pruning contracts.
- F1: `tests/minimum/AMNEZIA-ACCEPTANCE.md` records the matrix; it is **not
  executed** (needs B1 image and a live isolated target). The separate disposable
  component harness `python3 -B tests/minimum/functional.py` passed on 2026-11-07
  against the pinned fixture images: native acme.sh HTTP-01 standalone and
  existing-webroot issuance with token proof and default-trust rejection,
  HAProxy/Caddy exact-SNI site routing, authenticated VLESS REALITY and invalid
  credential rejection, read-only native validator custody, and Docker
  snapshot/recreate with data, launch metadata, DNS network attachments and
  original preservation. It uses local loopback fixtures and does not cover the
  Amnezia app/import, client DNS, live certificate issuance or any public host.
- B1: blocked on an actual multi-arch build; `platform/images.lock.json` intentionally has
  no fabricated managed digest.

Remaining for full M1 closure: obtain real B1 digests through the pinned builder
and run F1/A1 on a live isolated target. `dns.prepare` and `host_access.reconcile`
remain module-level effect owners pending their live consumers in the managed
transaction's B1/F1 evidence. The SDD `docs/sdd/file-index.json` carries the twelve
new M1 units; `module-index.json` is unchanged because no new catalogue module
document was added. No acceptance claim is made for any remaining item.

### Owned-launch false drift after host reboot (2026-11-07)

A live `scripts/install` on example.com stopped at the cutover suspension check with
`Owned native launch drifted; refusing mutation` (ingress.owned). The host had
rebooted between container creation and the install, and Docker 29.9.3's daemon
restore rewrote each owned container's `HostConfig.Dns` from `null` (create-time)
to `[]`. `launch_digest` hashed the raw HostConfig, so all four active ingress/
protocol containers and both prepared candidates appeared drifted even though
their labels, mounts, config and artifacts were unchanged. Comparing create-time
and restored `hostconfig.json` files reproduced the single field difference.

D161 normalizes the restored `Dns` empty-list encoding back to the create-time
null form through `docker.canonical_host_config` (opt-in in launch digests and
the foreign equality check; replacement payloads are unchanged). Verification
against the live socket: the
new digest matches the stored receipt for all eight owned containers, `owned()`
accepts the four running actives including their artifact claims, and a changed
`Dns` list still raises drift. Unit coverage adds null/empty lifecycle cases for
both owned receipts and the foreign equality check; the full minimum suite passes
158 tests. No production container was stopped, removed or re-created by this
diagnosis.

### Native client names accept Cyrillic and uppercase (2026-11-07)

The interactive client workflow rejected `Alice`, `ГОСТЬ` and `Иван` because
`clients.NAME_PATTERN` allowed only lowercase ASCII. Per OR-56/D162 the pattern
now accepts a leading letter or digit followed by Latin or Russian Cyrillic
letters in any case, digits, `_`, `.` and `-`, up to 32 characters; the exact
case is kept in the xray `email`, share title and link fragment, while `slug()`
lowercases for the artifact file name. `scripts/clients add`/`rm` prompt for a
missing name on a TTY and confirm `rm`; `list` needs the Docker socket group,
while `add`/`rm` keep root because the share file is `root:hton 0640` per OR-46.
Unit coverage exercises uppercase Latin, Cyrillic, `ё`, leading
digits, the 32-character bound, and Cyrillic slugs; `docs/MINIMUM-INSTALL.md`
matches the new syntax and the `list`/`add`/`rm` root split.

The same form now governs the configured initial guest. `configuration` imports
`clients.NAME_PATTERN` for `foreign-amnezia.settings.provision.clientName`
(schema updated to match), and share-artifact names accept Unicode word
characters while still refusing separators, leading dots and traversal. A
Cyrillic guest therefore installs as `#Гость`, `amnezia-гость.md` / `.txt` and a
client-page row named `Гость`. Unit coverage adds a Cyrillic/uppercase
`clientName` validation case and a `secret_share_artifacts` Cyrillic artifact.

### App connection name shows the country label (2026-11-07)

OR-57/D163 separates the app-facing connection name from the credential. The
native VLESS fragment now prefers `core.clientAccessPage.names[credential]`, then
`instance.label`, then the credential id; `displayName`, the xray `email`, the
share filename and the client-page identity still use the username. `instance.label`
accepts a bounded printable label including spaces and flag emoji (1–64 chars) and
is present in both root schemas; `emit_ready`, `native_share_links`,
`publish_secret_share_artifacts`, `refresh_client_page` and `scripts/clients add`
apply the same precedence. Unit coverage validates the label, keeps
`displayName`/credential separate from the fragment, and checks the ready/share
link fragment. Changing the label affects newly generated links; the running
container's client list and keys are untouched.

### Per-user share bundles and group-readable artifacts (2026-11-07)

The live report (`cat /opt/hton/.secrets/share/amnezia-d6.md: Permission
denied`) showed `scripts/clients` wrote `root:root 0640`, so group `hton` could
not read the file, and the install published protocol-per-file artifacts rather
than per-user bundles. OR-58/D164 now produces one `<slug>.md` per user with the
Amnezia link, matching Outline key, shared MTProto/Hysteria links and the
client-page URL; `client-page.md` remains the operator copy. `scripts/clients`
writes/removes the same bundle in both `.secrets/share` destinations, derives
ownership from the private root (installed root is `root:hton 0640`), preserves
sections it does not own and deletes the legacy `amnezia-<slug>.md`. Unit tests
cover grouping, shared sections, Cyrillic names and stale cleanup; the script's
bundle composition was smoke-tested in a temporary repository.

### Protocol prompt default blocked on AmneziaWG2 (2026-11-07)

OR-59 asks for a protocol-selection prompt defaulting to MTProto and AmneziaWG2.
MTProto is implemented and selectable; AmneziaWG2 is not present in the codebase
or host (`modinfo amneziawg` absent, no AWG image/container), and the SDD treats
AWG as a separate UDP/kernel endpoint rather than a REALITY toggle. No selectable
AWG2 prompt option was added because it would be a dead option; the requirement
and its prerequisites (pinned engine/image, UDP allocation, per-peer keys/export,
deployment authorization) are recorded in OR-59/D165. Existing foreign
`amnezia-awg*` resources remain preserved, not adopted.

### Client-facing names use protocols, not the HTON brand (2026-11-07)

OR-60/D166: the native VLESS fragment is `XRay`, optionally prefixed by
`instance.label` (live profile: `🇱🇻 Латвия XRay`); Hysteria's direct link uses
`Hysteria2`; the Outline fallback identity is `guest`; `hton` remains only in
internal `instance.name`, units, labels and paths. `clientAccessPage.names` still
overrides a single credential's app name. Tests assert the XRay/Hysteria2 labels
and the absence of the old `#hton`/username defaults.

### Native client config mode crash-loop and refresh (2026-11-07)

A live `sudo ./scripts/clients rm d6` returned `ERROR: Docker command failed.
The command took too long.` The native Amnezia container was in
`Restarting (23)`, and its log repeated `open /opt/amnezia/xray/server.json:
permission denied`. The image runs xray as uid 65532, while `write_config`
created the temp file `0600` and `docker container cp` preserves that mode, so
every `add`/`rm` wrote a root-only config the process could not read. The script
now copies `0644`, uses 120 s for `container cp` and `container ls`, 180 s for
`container restart`, and accepts a restart only after two consecutive
`Running && !Restarting` inspections. Live repair copied the existing client
document back with `0644` and restarted; the log shows `core: Xray 26.9.9 started`.

`scripts/clients refresh [names...]` now rewrites existing per-user bundles and
link fragments from the live config without changing UUIDs or restarting xray;
it covers clients already present in the native array but missing a share file
(for example after an interrupted `add`) and applies a changed `instance.label`
to already-issued links. `add` accepts one or more names separated by spaces,
validates the whole list before writing, and restarts xray once for the batch;
`add --json` reports a `clients` array. The installed `config/hton.yaml` on this
host predates the profile's `instance.label`, so operational links remain `#XRay`
until the label is applied to that config and `refresh` runs. Unit tests cover
the copied mode and timeout, the identifier lookup, refresh without restart, and
multi-name add with duplicate rejection before any write.

### Shared client-access URL outage and page decoupling (2026-11-07)

The live `https://example.com/4cK_GISb2L94IyZRqF4mC66xx1WJ-kIx` returned 404. The
Caddy container had no `/4cK…` route and no `/client-page` mount, and
`work/minimum/client-page/receipt.json` said `{"enabled": false, "withdrawn":
true}`. The minute timer `hton-minimum-client-page.service` withdrew the page
because full `configuration.validate` was blocking on `config.wildcard-challenge`
(`core.certificates.wildcardDomain` with `http-01`, while OR-38/wildcard requires
`dns-01`); `refresh_client_page` treated any blocking diagnostic as "page
disabled". The install at 15:46 therefore passed `client_page=None` and never
created the route or the bind mount.

D167/OR-61 decouples page availability from unrelated diagnostics:
`configuration.client_page(root)` parses only `core.clientAccessPage`, and
`refresh_client_page` uses it when `result.desired` is absent, deriving the
domain, native inventory and `instance.label` from the root document directly.
The page remains one bearer admin URL shared by every user; per-user URLs are not
introduced. Publishing still requires an install to render the Caddy route and
mount; the timer only refreshes the artifact. Unit coverage adds a blocking
`wildcardDomain`+`http-01` config with a valid page path and asserts the refresh
keeps `enabled: true` and writes `client-page/index.html`; the U1 security suite
passes 75 tests and the clients suite 20. Live application requires
`sudo ./scripts/install` after this change. The wildcard certificate itself
remains unissued until `dns-01` InternetBS credentials replace the `CHANGE-ME`
placeholders; the runtime config is still formally invalid for OR-38.

The first live install after the page change stopped in `ingress.prepare` with
`Run container task failed (exit 1)`. `caddy validate` on the generated artifact
reported `http: json: unknown field "headers"`: `ingress.configs` placed the
response-header map on the route object, which Caddy's JSON schema rejects.
The page route now carries a `headers` handler (`{"handler": "headers",
"response": {"set": …}}`) as its first handle, followed by `rewrite` and
`file_server`; the route test asserts the handler shape, and a manual
`caddy validate` of the regenerated live configuration returns
`Valid configuration`. HAProxy validation was already clean. Re-running the
install deploys this fix and recreates Caddy with the `/client-page` mount.

### Plain HTTP redirect and ACME webroot (2026-11-08)

OR-62/D168: with a real site root, `ingress.configs` now emits a second Caddy
server on `0.0.0.0:80`. It serves only `/.well-known/acme-challenge/*` from
`/site` and answers every other request with `308` to
`https://<host><uri>`; the server is absent for a placeholder install so TCP/80
stays free for standalone HTTP-01. The Caddy container already uses host
networking and `NET_BIND_SERVICE`. `caddy validate` on the regenerated live
configuration (site root `/var/www/html`, client page and REALITY routes) returns
`Valid configuration`; route tests cover the challenge path, the 308 target and
the placeholder case. Once port 80 is permanently owned,
`http_challenge.select` falls through to the proved webroot on renewal, which the
same site root serves.

The install deployed at 05:47 UTC (source deployed at 05:47, `ingress.py` written
at 05:40) now carries both fixes live. The accepted Caddy artifact has servers
`site` and `http_redirect`; `http://example.com/` answers `308` to
`https://example.com/`, and the client page returns `200` with `no-store`,
`noindex`, `nosniff`, the CSP and `no-referrer` headers while an unknown path
returns `404`. The page receipt stays `enabled: true` after repeated minute-timer
refreshes, and Caddy mounts `/client-page` and `/site` (`/var/www/html`) read-only.
The full minimum unit suite passes 175 tests, and the disposable component harness
passes end to end (316 s): pinned issuer/InternetBS provider, native HTTP-01
standalone and existing-webroot issuance, HAProxy/Caddy site routing, authenticated
VLESS REALITY, native credential rejection, mode-0001 config read and
snapshot/recreate. Production deployment remains separate. The wildcard
certificate remains unissued until InternetBS `dns-01` credentials replace the
placeholders, so the runtime config is still formally invalid for OR-38 even
though the page is published.

### Trusted certificate for REALITY names (2026-11-08)

A live `https://harbor.example.com/` failed in the browser with
`Невозможно установить безопасное соединение`: the served leaf was
`CN=harbor.example.com`, self-signed, valid 30 days. `harbor.example.com` is the
native REALITY own-SNI (`foreign-amnezia.serverNames`), routed by HAProxy to the
native backend and answered by Caddy with the OR-44 403 body.

Two defects in `run_managed` produced the untrusted certificate.
`certificates.ensure` received `plan.server_names[1:]`; that slice assumes the
first saved name is the site domain, so a configuration with
`serverNames: [harbor.example.com]` requested no additional name and the trusted
certificate covered only `example.com`. Independently, the installer minted
`reality-fullchain.pem`/`reality-key.pem` with `openssl req -x509` and Caddy
loaded them for REALITY SNI, which violates OR-38 and OR-44's retained
trusted-certificate requirement even when the main certificate would have
covered the name.

`install.reality_certificate_names(domain, names)` now returns every REALITY
name that is a subdomain of the site domain and excludes the apex (already the
primary name); third-party legacy names cannot be issued and are omitted.
`certificates.ensure` receives exactly that list, and `ingress.configs` loads
only `/certs/fullchain.pem`, so Caddy presents the ACME certificate for the
REALITY SNI and keeps the 403 route. The self-signed generation is removed.
Unit coverage asserts the certificate request includes `static.example.com`,
that no `reality-*` files are written, that Caddy has one loaded certificate,
and that third-party names are filtered.

Verified live without root: the Caddy `:80` webroot returns an exact token for
`Host: harbor.example.com`, public `http://harbor.example.com/` answers `308`, and a
regenerated configuration with the trusted-certificate-only load list passes
`caddy validate`. The installed host still serves the old self-signed leaf
until `sudo ./scripts/install` runs; that install must reissue
`example.com + harbor.example.com` over HTTP-01 through the Caddy webroot.

### Browser denial, www alias, wildcard and apex SNI (2026-11-08)

`https://harbor.example.com/` returned the site HTML with a 403 because the
REALITY route reused the site body. It now returns a fixed
`Access denied` 403 (`ingress.DENIAL_BODY`); the site body is only used by the
ordinary site route.

OR-63/D170 adds the accepted single-SNI direction. `ingress.configs` allows and
TLS-terminates `www.<domain>` and answers it with a `308` to
`https://<domain>{http.request.uri}`. The certificate request always includes
`www.<domain>` plus own-domain REALITY names; with `wildcardDomain` equal to the
site domain and `dns-01`, `certificates.ensure` also requests `*.<domain>` and
rejects a response without the wildcard SAN. `run_managed` treats a REALITY name
equal to the site domain as native self-steal: HAProxy sends the apex to the
native backend with the site as backup and no 403 route is emitted for the apex.

Verification: 3 new/updated ingress tests (fixed denial, www 308 and TLS name,
single trusted certificate) and an apex managed-install test pass; the full
minimum suite is re-run below. A regenerated configuration passes `caddy
validate`. Live application again needs `sudo ./scripts/install`; enabling apex
REALITY additionally requires `serverNames: [example.com]` in
`config/modules/foreign-amnezia.yaml` (old profiles with `harbor.example.com`
stop authenticating unless that name is retained). Wildcard issuance still needs
real InternetBS `dns-01` credentials; while they remain placeholders the
challenge stays `http-01` and the certificate covers `example.com` and
`www.example.com` only.

### Install console: page URL and share directory (2026-11-08)

The ready summary printed every MTProto/Hysteria/Amnezia link and every runtime
share file, but not the client-access page URL. OR-64/D171 changes the human
summary to two actionable lines: `Client access page: https://<domain>/<path>`
and `Client files: <installation>/.secrets/share (0640; readable by the hton
group)`. Credential links are no longer printed; they stay in the per-user
bundles under `.secrets/share` and in `work/minimum/share`. JSON output and
quiet/dry runs are unchanged and still exclude the URL; receipts, plans and logs
do not contain it. Unit coverage asserts the URL and directory lines and the
absence of a printed link.

### Apex REALITY migration on install (2026-11-08)

`harbor.example.com` remained the native REALITY name, so links still used the
subdomain. OR-63/D172 adds `apex_server_names`: an interactive `install` whose
saved names are all subdomains of the site domain replaces them with the apex,
prints a reissue notice, and recreates the native container when its stored
`serverNames` differ from the plan (the previous recreation check ignored names,
so the native config would otherwise keep the subdomain). Renewal and unattended
update never rewrite the accepted names. After the next `sudo ./scripts/install`
the guest link becomes `vless://…@example.com:443?…&sni=example.com`, MTProto/Hysteria
links are unchanged in shape, and `harbor.example.com` is no longer routed.

The VLESS `encryption=none` parameter is unchanged: VLESS has no built-in
encryption value other than `none`, and transport security is the REALITY TLS 1.3
handshake (`security=reality`). The URI is read by the client application, not by
DPI; removing the parameter would break standard imports without changing the
observable TLS profile.

The page refresh timer ran with the legacy `state.json` only, so on a managed
install it passed an empty state and refreshed the page from protocol entries
without any native Amnezia clients; the bearer page showed only MTProto and
Hysteria. `refresh_client_page` now falls back to `managed.json` for the live
container id, so the timer includes every native client. The page is refreshed
in place and lists all users again after the next install/deploy.

### AmneziaWG2 endpoint and D6ton compatibility (2026-11-09)

OR-66/D174 adds the `amneziawg` minimum-protocol module. It pins
`docker.io/amneziavpn/amneziawg-go:3.1.20260828` (AmneziaWG 3.1), allocates a
persistent public UDP port from 5000-6000, generates a server key and one
X25519/preshared pair per native Amnezia user, and renders a bridge-network
container with `hton0`, `NET_ADMIN`, `/dev/net/tun`, contained forwarding/NAT
and only `/32` peer routes. `Jc/Jmin/Jmax`, `S1-S4` and non-overlapping
`H1-H4` are generated once and persisted in
`work/minimum/secrets/amneziawg-peers.json`. Each user receives
`share/amneziawg-<user>.conf` and a matching `AmneziaWG` section in the private
per-user Markdown bundle and bearer client page. Server configuration and
`ListenPort` are rejected by the dedicated guest-export parser.

Unit coverage (`tests/minimum/test_amneziawg.py`) covers aliases, stable peer
material, config rendering, server-config export rejection, hooks/foreign-field
rejection, per-user planning, share files, state persistence and malformed
saved material. The generated `.conf` satisfies D6ton's native `vpn-root`
allow-list (`PrivateKey/Address/DNS/MTU/Jc..H4`; `PublicKey/PresharedKey/
AllowedIPs/Endpoint/PersistentKeepalive`) and its endpoint/address validation.
Local D6ton verification is provided by `scripts/awg-verify`; it was not executed
in this environment because the Docker daemon was unavailable, so runtime
interoperability remains unverified here.
