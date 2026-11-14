# SDD installer — declarative bootstrap and reconciliation

> Authority: [owner requirements](../OWNER-REQUIREMENTS.md). R1 is standalone CLI/Core with manual YAML validation and safe apply; API/configurator/web implementation is deferred. Native checks are required, Docker staging optional, IPv6 far future. See [final audit](../FINAL-REVIEW.md) for unresolved design details.

## Product language

Per OR-13/D102, product source, comments that are user-visible, CLI help,
prompts, errors, diagnostics and logs are English. Product localization is deferred;
do not add translated product strings or localization frameworks in this phase.
Russian prose belongs only in `docs/_lang/ru/`; canonical architecture/specification
documents are English. This rule concerns language content, not the programming
language selected by a module.

## Purpose

scripts/install self-configures mandatory core and selected VPN modules from hton.yaml.
It shares normalization, planning, apply and recovery with the web/controller.
Schema/secrets/migration are owned by sdd-config.md; flags by ../CLI.md.

## Requirements

- INS-0 `scripts/install` is the first setup command (D25): validate the current local
  source tree, deploy its bounded product files through `scripts/deploy` into
  `/opt/hton`, then install required base tools and Docker/Compose. Do not download
  a release as the deployment input or compile on the target. Build helpers must
  already exist in the source tree when required; missing helpers block install
  with a clear message. External credentials/domain ownership cannot be invented
  by autopilot.
- INS-1 Detect OS through os-release; Ubuntu/Debian use distro-correct repositories
  and codenames. Other systems receive manual dependency guidance and rechecks.
- INS-2 Repeated install is a reconciler: compare observed state with desired core
  and modules, validate supported existing components, apply only necessary changes.
  Dry-run performs no writes/downloads/packages/container operations.
- INS-3 hton.yaml is canonical. Create from schema defaults, optional guided profile
  and menu; core is mandatory, optional modules start off. Persist selections;
  --menu revisits them. Reuse saved answers/config on subsequent runs. Detect legacy .env and explain explicit manual conversion in R1; automated import belongs to the deferred configurator.
- INS-4 Fresh managed Amnezia uses the typed provision contract in
  [sdd-managed-amnezia](sdd-managed-amnezia.md). Other foreign-stack setup remains
  an explicit owner-selected manager workflow. Record created ownership; never infer installed foreign resources
  belong to hton. Their side effects require the same consent as adapter changes.
- INS-5 Resolve selected modules and dependencies; check address/protocol/port and
  SNI reservations, host capabilities, cert inventory, supported images and resource
  evidence. Record locally approved listener/privilege envelopes for web changes;
  they cannot grant arbitrary future Docker or host access.
- INS-6 Foreign port-holder conflicts require stop/repoint/abort consent with
  protected originals and typed confirmation where required. Store resolution
  in managed installation state; it is not a shell-interpolated config variable.
- INS-7 Snapshot supported launch semantics before any foreign mutation; refuse
  unsupported configurations and external drift. Apply/recovery use sdd-control.md.
- INS-8 Install data/guardian/ACME timer units; control is later optional, optional PATH
  shim by explicit selection. Units call internal helpers, not recursive wrappers.
- INS-9 Install all of hton under `/opt/hton/`; use `/run/hton` for runtime state. Avoid global system configuration changes before apt-style delivery unless required. Project persistence uses host bind mounts; no named Docker volumes.
  Generated accepted outputs are disposable only when recovery no longer needs them; editable Docker inputs are retained; templates remain versioned.
- INS-10 --yes never bypasses foreign-change, public-443 or privilege/policy consent.
  Noninteractive missing consent fails before effects, with an actionable plan.
- INS-11 Bootstrap CLI management locally/through SSH; web administration is optional and deferred beyond the first iteration. Start core even with no
  selected VPN. Until trusted HTTPS is ready, expose setup through the local path,
  not an unauthenticated public wizard. Finish with ready/degraded/unverified report.

## Uninstall

- UNI-1 Remove only recorded own resources, including module services, core units,
  shim and generated configs. Core non-disableable means no feature off toggle,
  not prevention of explicit local full-stack stop/uninstall.
- UNI-2 Restore foreign originals with drift checks before deleting recovery data.
- UNI-3 --foreign additionally removes menu-created foreign resources only.
- UNI-4 hton.yaml, secrets and retained legacy .env are not silently deleted;
  keep/archive/remove behavior and confirmations are defined in ../CLI.md.
- UNI-5 Journal uninstall; incomplete restoration prevents purging recovery data.

## Acceptance

On a qualified host with required domain, certificate credentials and native
Amnezia evidence, `scripts/install` deploys the current working source to `/opt/hton`,
then leaves the active system there with HAProxy listening on TCP/443. Exact site
SNI serves the configured site over HTTPS with a publicly trusted certificate and
no client certificate warning; qualified Amnezia SNI reaches its private native
backend byte-for-byte. Install reports ready only after local route, certificate,
HTTP status and site-body checks plus the qualified Amnezia route/handshake check.
Unavailable prerequisites produce a resumable degraded/blocked result with the
specific missing evidence; they never produce a false ready result. Repeated
install reuses saved domain, certificate and service-port choices. Acceptance
scenarios also cover uncommitted source deployment, absence of `.git` and
`docs/_lang/ru/tasks/*` under `/opt/hton`, declined conflicts, unsupported OS/package state,
no-write dry-run, interrupted apply, ownership-aware uninstall and clean-VM restore.

Accepted reboot startup uses runtime startAccepted and cached pinned bundles/images, never install/update/compiler. See sdd-runtime-safety.md; minimal core is mandatory installation, not a data-plane dependency on management.

## Current setup/storage contract

The fixed systemd installation owner stages the five compiled platform unit
templates under protected `/opt/hton/work/systemd`, journals their exact digest
before linking them into `/etc/systemd/system`, and reloads the manager. Existing
links must point to those exact files; occupied foreign unit paths or changed
template bytes block rather than overwrite. Repeating an interrupted install
rechecks all bytes and links before completing its pending receipt. Unit delivery
does not enable/start the data service or renewal timer; independent runtime
acceptance and durable run intent remain the lifecycle authority. Guardian delivery
precedes ingress activation because runtime must start its independent supervisor.

Owner OR-12 completion is a live `example.com` site with a trusted certificate and
TCP/443 SNI separation from the existing Amnezia service. The site terminates TLS
on owned Caddy; qualified Amnezia REALITY is passed through unchanged by HAProxy.
Readiness requires D81's full implementation/build/U0/F0 sequence, DNS-01
issuance and local HTTPS/body verification, and exact native remap/handshake plus
restore evidence. No source-only or render-only result is production completion.

Install distribution under /opt/hton with protected code; editable inputs under
config/, durable registry/progress/data/bundles under Git-ignored work/, sockets
under /run/hton. Templates seed missing Docker inputs and never replace edits.
Do not install/start the optional control/API/web modules in R1. API interfaces
are specified now; full configurator/migration implementation is deferred.

### Privileged source-tree deployment and group access (owner OR-11/OR-14, D99–D100/D103)

`scripts/deploy` copies the current working source tree into `/opt/hton`; it does not
require a prepared release, provenance manifest, clean Git state, or commit. It
copies the current versions of the product files and source crates, including
uncommitted changes, from the fixed distribution allowlist. It excludes `.git`,
`docs/_lang/ru/tasks/`, generated `target/`, `var/`, credentials and mutable runtime/config/
work/log data; generated `etc/unbound/root.key` is explicitly excluded. It never
copies Git history or task planning files to the server. A bounded source inventory is hashed before staging and verified
again after copy; an existing content-addressed snapshot is reused only if its
complete inventory matches exactly, including the absence of excluded files. This
verifies copy integrity, not publisher authenticity.

Deploy installs a content-addressed source snapshot under
`/opt/hton/deployments/<tree-digest>` and atomically switches `/opt/hton/current`.
Stable code paths resolve through `current`; `/opt/hton/config`, `work`, and `logs`
remain outside the switched code tree and are never replaced or recursively
chmod/chowned. Previous source snapshots remain available for recovery. Deploy does
not fetch from the network, compile on the target, start services, or modify foreign
resources. The first deploy initializes only an absent/empty `/opt/hton`; a non-empty
unmanaged target fails closed for manual migration.

Deploy creates system group `hton` and adds `SUDO_USER` when present. Deployed source
is root:`hton`: directories 0750, ordinary files 0640, executable entry points 0750.
Group members can read and execute code but cannot write it and receive no sudo
grant. Mutable config/work and private credential/runtime material remain
root-private with their owning modes.

When `scripts/install` runs from a source tree outside the active `/opt/hton/current`,
it invokes that tree's `scripts/deploy`, then re-enters `/opt/hton/bin/hton install`
with the original bounded arguments. The active tree is detected to avoid a loop.
`--docker-only` remains the independent dependency-bootstrap mode. Deploy copies
prebuilt `libexec` helpers if they are present; it never builds them. Missing Core
or runtime helpers remain an explicit install blocker. When started without root,
guided install invokes `sudo` for the complete operation; standalone deploy invokes
`sudo` only for copy/promotion. Both validate the source tree before requesting
sudo, and dry-run remains unprivileged.

The product-file allowlist and exclusions are owned by `scripts/deploy`; do not use a
Git archive, release package or provenance manifest as the installer input. Build
artifacts already present in the working tree can be copied as product helpers,
but deploy never creates them. D81 still controls when the project build and final
unit/functional acceptance sequence may run.

Check domain/DNS or offer text instructions for obtaining a free domain, including
a suitable example chosen at implementation time; third/fourth-level domains are
allowed. Missing domain/certificate can finish unpublished/degraded installation
with the applicable blocking-risk warning. Report installed, configured, running
and externally verified separately. Retain completed steps and resume partial
installation. modules add leaves new modules disabled/incomplete until enable.

## Public ingress install reconciliation (owner request D88)

### Persisted module selection and profile suggestions

`--profile` is a catalogue-owned first-install suggestion, not an alternate
configuration source and not authorization to install or mutate modules. On an
uninitialized installation, Core resolves the suggestion against the trusted
catalogue and presents the suggested module IDs for explicit selection; without a
selected profile, optional modules remain off. Core persists the confirmed
selection through the canonical root configuration owner. On an initialized
installation, saved selection is authoritative: a profile argument cannot rewrite
it. `--menu` asks Core to present the saved selection for an explicit edit and
builds a revision-bound transient candidate. First-install selection is composed
with trusted defaults and expected root absence. The shell does not enumerate module
IDs, duplicate profile maps, or edit YAML.

The install coordinator builds its selected graph from that persisted desired
selection and the same trusted catalogue used by `modules.list`, validation and
lifecycle. Only graph services with trusted allocation bindings can enter
`plan_graph`; discovery or a profile suggestion alone never associates a foreign
resource. A repeated install loads the persisted selection and service settings,
then reuses their durably saved allocation values, including after a process or
host restart and during recovery. The service settings document is the source of
truth; in-memory choices or a journal record alone do not satisfy persistence. It
must not draw fresh entropy for an already allocated service. A new value requires
first assignment or a separately approved change. Profile/menu selection changes
are resolved and revalidated as exact digest-bound input before bootstrap or other
host effects. Explicit typed inputs and exact menu selection are direct consent;
Core commits changed values with narrow CAS only after blockers clear. Unresolved
selection, dependency, association or port-conflict blockers stop reconciliation
without partially applying a new selection.

Initial catalogue engineering choice: `minimal` suggests no optional module,
`home` suggests VLESS, `compat` suggests foreign Amnezia, and `lab` suggests both.
These are suggestions only and require explicit selection. Catalogue validation
rejects suggestions for unknown, unavailable or non-R1 modules; adding another
suggestion changes the trusted catalogue revision.

The installer consumes publication and foreign-listener values from the saved,
validated configuration; it never bakes `example.com` into root config. For the
owner's current instance Core suggests `example.com` during first publication when
no domain is saved, and the user may edit it. Public ingress is HAProxy TCP/443.
Amnezia and other selected VPN services use distinct random
backend/host-listener ports from 5000–6000 inclusive. Their assigned values are
stored by each service's existing configuration owner, not duplicated in a global
installer setting. Initial domain input is gathered inside `scripts/install`; port
values are allocated by Core, so manual YAML editing and port entry are not needed.
If no site domain is configured, Core suggests `example.com` and public TCP/443. The
existing TCP/4443 sample is development/prepublication; a loopback 4443 endpoint is
only for an isolated sandbox. `example.com`, the 5000–6000 policy and HAProxy TCP/443
are requirements; a generated port is a persisted instance value, not a fixed
product default. The site's ACME API keys remain protected
`core.certificates.credentials.*.secretRef` values and are never part of `InstallInput`.
Subsequent `scripts/install` calls reuse saved ports unless an operator explicitly
requests a change. Amnezia's native in-container ports are observed from its exact
registered container and preserved; `hostPorts` only requests external host
bindings. A repeated install reuses the qualified certificate, accepted artifacts
and exact foreign mapping; unchanged state is a no-op.

### Persistent random VPN port allocation (owner OR-10 / D91)

`core.port-allocation` is the single policy owner for distinct externally reachable
VPN listener/backend services. It allocates one distinct host port per service ID in
the inclusive 5000–6000 range using OS cryptographic randomness. When a service
binds TCP and UDP, the same allocated number is used for both transports. Public
HAProxy TCP/443 is a separate shared edge and is never allocated from this range;
logical protocols multiplexed over that same edge do not receive extra public
listeners. A standalone VPN service/backend that owns its own listener receives its
own distinct allocation.

The allocator consumes one coherent desired read set and fresh TCP/UDP reservation
evidence for all host listeners and exact registered foreign resources. It considers
saved allocations of installed services, including disabled services, when enforcing
uniqueness. For each service, a valid saved allocation is reused if it remains in
range and does not conflict. Missing allocations are selected from currently free
numbers without replacement within the candidate set. Occupied candidates are
discarded; if no free value remains, preflight returns a blocker and makes no
configuration, package, certificate, container or listener change.

The allocation candidate is part of the immutable `InstallPlan`, bound to its
read-set, reservation evidence, selected service set and expiry. First assignments
are shown in preflight and follow OR-10 collision checks without a second approval;
changing a saved allocation requires exact review, separate from foreign-remap
consent. Core journals the candidate before writing, then
`PortAllocationCoordinator.commit` stores each value in the service's
existing module settings (`foreign-amnezia.settings.hostPorts` or that module's
declared listener setting) by per-document CAS. Recovery reuses the journaled
candidate; subsequent install runs read and reuse the committed config value rather
than generating another one. A conflict, invalid legacy value or requested change
returns the exact current/proposed mapping and requires the applicable explicit
review. The allocator is also used by module add/enable so those paths cannot invent
a second port-selection rule. It does not gain authority over a discovered foreign
container; association, native qualification, writable-layer snapshot and consent
remain separate requirements.

The allocation module contract is owned by [sdd-port-allocation](sdd-port-allocation.md)
and bound to [core/src/port_allocation.rs](../sdd/map/core/src/port_allocation.rs.md).
InstallationCoordinator supplies the selected service set, coherent settings and
registry snapshot, fresh reservation evidence and explicit CSPRNG entropy; it binds
the returned candidate into `InstallPlan` and delegates journaled settings CAS back
to that owner. `core.plan` consumes the immutable candidate and never allocates.
`InstallPlan` carries the complete fresh host `ReservationEvidence` independently
of the optional allocation candidate, so the public TCP/443 edge is covered even
when no managed VPN backend is selected. Any allocation candidate must bind the
same reservation digest and observation validity interval. Before its first effect,
`execute` reobserves the full TCP/UDP host reservation set and requires the exact
digest still to match and remain fresh; stale or changed evidence makes the plan
`planStale` and does not draw replacement entropy.

`scripts/install` remains a bounded shell adapter. Its single Core operation
`install.reconcile` is owned by `InstallationCoordinator.preflight/execute`; that
coordinator composes the existing config/catalogue/plan/certificate/apply owners
and the new exact `ForeignRemap` port. It does not run Docker/shell commands or edit
foreign configuration itself. The coordinator first performs a read-only preflight
of the complete requested graph, host listener and UDP claims, exact registered
foreign identities, DNS-01 credential availability, native qualification and
approvals. Any unresolved conflict blocks the operation before package, path,
journal, certificate, container, or listener effects. The current `outline-operator-two`
TCP/UDP 8443 bindings are outside the new range and remain untouched unless Outline
is explicitly registered as an HTON VPN service and included in a separately reviewed,
compatible remap. Discovery alone never adds it to the managed allocation set. Any
foreign remap still requires exact identity/fingerprint, durable snapshot and consent.

### Initial guided inputs and safe config commit

`InstallInput` contains `siteDomain`, `publicEdge: HostEndpoint`, and an optional
non-secret Let's Encrypt account contact email; it contains no port selection,
DNS credential bytes, container locator, raw config patch or shell command. A new
site publication requires the email if no saved value exists. When the certificate
configuration is absent, the narrow writer adds fixed supported InternetBS DNS-01
metadata and SecretStore references `dns/internetbs/api-key` and
`dns/internetbs/api-password`. Existing issuer/provider/reference settings are
preserved on repeat installs; credential values are imported separately from
protected files through SecretStore.
`PortAllocationCandidate` is a separate Core-owned typed plan, not TTY input. Before bootstrap, Core's pure
`install.reconcile --input-preflight` checks saved values; only a structured
`inputRequired` result with exit 0 and an exact `requiredInputs` list may trigger
bounded TTY prompts. The shell retries the pure preflight once with internal operation options
`--site-domain` and `--edge-port`; the shell never asks for random VPN ports. No
public install port flags are added. The public edge suggests TCP/443 only for first
unconfigured publication. Each absent backend allocation is generated by Core from
fresh evidence and displayed as part of the exact plan.
Dry-run, `--json`, and non-TTY mode never prompt. A blocked/unavailable result stops
before bootstrap or package effects. Existing public edge values are reused unless
the operator supplies an explicit typed replacement; typed input is direct consent
to that exact value and is revision-bound. Existing saved VPN allocations are
reused; changing one requires exact local allocation review that `--yes` cannot
supply. No TTY plus missing values returns `inputRequired` without effects.

The coordinator forms transient site and port-allocation candidates, runs read-only
exact host/foreign conflict preflight before any write, and returns on any occupied
candidate. Outline's existing TCP/UDP 8443 mapping remains untouched; it is outside
the new allocation range and is no longer the requested Amnezia target. Only after
conflict-free plan and separate exact public/foreign consent does it journal intent
and call the bounded owner-specific CAS writers against the captured coherent read
set. CAS preserves
unrelated customized fields and returns conflict if any root/module revision changed.
`core.config` remains read/validate-only; the coordinator uses its validated snapshot
and the shared descriptor-relative storage CAS primitive. Certificate issuance,
container remap and edge apply follow the successful commit. This avoids saving an
input set that cannot proceed because its requested host ports are occupied.
When the selected foreign module has no settings file yet, preflight uses its
trusted descriptor defaults without creating a file and binds that exact absence
in the coherent read set. After the full host/foreign conflict checks and consent,
the allocation writer atomically creates that module document with defaults and
the chosen persistent port in one expected-absence CAS. A standalone seed before
preflight is forbidden; recovery distinguishes absent from an existing empty file.
This path is only for a service not yet registered in the instance; a registered
service whose settings document disappeared blocks for repair and is never reseeded.

After a conflict-free preflight, local TTY consent is bound separately to the
exact foreign resource/fingerprint/old and new protocol-port bindings and to the
public TCP/443 listener plan. Missing consent fails before effects, including in
non-interactive mode. The coordinator journals intent and a protected exact launch
snapshot before remap. It obtains or reuses a trusted own-site certificate through
the existing DNS-01/Let's Encrypt coordinator, stages the exact HAProxy SNI route
and Caddy site certificate, remaps only the registered Amnezia port bindings,
then applies and verifies the public ingress candidate. HAProxy preserves the
REALITY bytes for the exact qualified native SNI; HTTPS for the configured site
terminates in the owned Caddy service. There is no Amnezia certificate manager.
The HAProxy backend for Amnezia is a qualified private endpoint, never the public
edge or the REALITY `dest`, so the route cannot loop.

Every phase has typed before/after receipts. If remap, apply or verification fails,
compensation restores the exact original foreign listener/container state and the
previous accepted edge; failure to prove restore is durable `recoveryRequired`,
not success. Unsupported Docker launch semantics, writable-layer snapshot failure,
external drift, unknown restart owners, missing DNS secretRef, unavailable trusted
certificate, or failed native checks stop with a precise blocker. No HTTP-01,
untracked certbot, host firewall redirect, or ad-hoc Docker recreation fallback is
implicit. The existing DNS-01 InternetBS workflow and persistent ACME timer own
issuance and renewal. Public A/IPv4 identity probes remain exclusive to explicit
`identity.whoami` (OR-8); install can report them unverified and must not invoke
that probe internally.

### Typed install plan and result contract

`contracts::operations::InstallPlan` is the immutable preflight result. It binds
`PlanId`, stable `instanceId` and `ExpectedInputs` (coherent config,
registry/runtime revisions, run epoch and exact resource fingerprints), mandatory
`host_reservation_evidence`
(`ReservationEvidence`) for the complete fresh host TCP/UDP snapshot, optional exact
selection and site-input candidates, the prospective catalogue `DependencyGraph`, configured public edge,
optional `PortAllocationCandidate` bound to that same reservation digest and
validity interval, exact foreign resource identities and current /
proposed host bindings, preserved native endpoints, certificate claims, blockers,
typed approval requirements, a closed effect-phase sequence and expiry. It carries
no approval capability, credentials, Docker argv or shell strings. Each foreign
`ResourceIdentity` keeps its structural fingerprint; native binding values are
observations to preserve, never allocator targets.

Approval requirements distinguish input changes, selection changes, port
allocation changes, public listener changes and exact foreign remaps. The plan only
describes these reviews; separate host owners mint the corresponding opaque
capabilities. The coordinator phase vocabulary is fixed: persist inputs, persist
selection, persist allocations, issue/reuse certificate, snapshot foreign resource,
remap foreign listener, apply ingress, verify ingress, compensate. Preflight is
pure/read-only and cannot report a conflict-free plan when any pre-effect conflict
evidence is missing. A certificate claim is a later phase requirement: failure to
issue/reuse material leaves the install degraded and blocks ordinary catalogue
resolution immediately before ApplyIngress, without invalidating earlier receipts.

`InstallResult` wraps the common `OperationResult` with typed phase receipts,
certificate state, foreign-remap transaction references and an explicit marker for
unverified external DNS. Partial or compensated work remains visible through its
phase/member outcomes; a missing external DNS probe never becomes a success claim.

The root-input persistence primitive is bound as
`core/src/host/install_input_writer.rs`: the generic writer validates its opaque
review capability, journals `Intent::InstallConfig` with protected original bytes,
and performs exact root-document CAS/recovery. A narrow install-only method accepts
the exact typed site input as direct user consent after full read-set recheck; it
does not grant that authority to other writers.

### Zero-effect first-install composition

The first-install path must not seed `hton.yaml`, `installation.json`, journal
directories, or module settings merely to obtain a `HostApplication` before
preflight. The InstallationCoordinator constructs an in-memory initial desired
snapshot from trusted schema/catalogue defaults and an empty registry, with `root`
and each selected descriptor settings object marked absent in one coherent read
set. It then builds the selected service graph, observes complete host TCP/UDP
reservations, plans persistent port assignments, and completes host/foreign
conflict checks before asking for exact consent. Immediately before the first
effect it rechecks that every expected-absent object is still absent. A collision
or concurrent file creation exits without bootstrap writes.

Typed CLI site values and the exact `scripts/install --menu` selection are direct
consent to those exact candidate values; they are revalidated against the read set
and written with narrow CAS. OR-10 directly authorizes a first random port
assignment after fresh collision checks; an additional local approval is required
only to change an already saved port. Before bootstrap, Core rechecks the plan and
host reservation evidence. Bootstrap creates protected metadata and writes the
journaled candidates; interruption after the allocation intent reconciles that
exact candidate and never draws a replacement port. The allocation writer journals `Intent::PortAllocation` before
the service-settings CAS; normal mutating `scripts/install` checks for one unfinished
allocation intent before starting new work, recovers it by exact before/after
bytes, then returns partial so the next invocation reloads the now-persisted
service setting. If the exact candidate is attached to an unfinished lifecycle
parent, install hands recovery back to that lifecycle owner and reports its
transaction ID without touching the allocation. Multiple unfinished allocations
block for explicit reconciliation. Read-only input preflight checks bounded
journal inventory only to return `recoveryPending`; it performs no recovery.
After the owning settings CAS is durably complete, service settings become the
durable source of the same port for future installs. Repeated `scripts/install` loads saved service settings, so
`plan_graph` retains each value and the allocation writer is a no-op. New random
ports are assigned only when a selected managed service has no saved allocation.

Fresh profiles keep optional modules disabled. A VPN allocation is made only for
a service selected for managed integration and represented by its trusted
descriptor/registry binding. Docker discovery alone never selects or registers a
foreign service and never authorizes its mutation.

`core/src/host/installation.rs::reconcile` implements the bounded owned path through
typed input persistence, exact VPN allocation persistence, safe `/var/www` index
capture, Caddy's exact captured response body, owned setup, DNS-01 certificate issue/reuse,
normal ingress prepare/apply, and a local served-site probe.
Foreign Amnezia remap remains gated on qualification and is never bypassed. A
successful install response means the local HTTPS listener served the exact
protected `/var/www/index.html` body with trusted TLS and matching SNI,
with source identity, digest and bytes unchanged across the probe. It reports
`externalDns: unverified` and does not assert public DNS or off-host reachability.

### Current Core install selection and input path (partial)

`InstallInput` contains site domain, public edge and optional non-secret
certificate account email; `inputRequired` names missing values from
`siteDomain`, `edgePort` and `certificateEmail` together. VPN ports are never TTY input.
`scripts/install` obtains a
read-only Core selection offer, validates the catalogue/root digest, and sends
selected module IDs back for Core revalidation. Selection is still a transient
candidate, revalidated against the current catalogue/root before execution.

The strict read-only `--input-preflight` now composes an `InstallPlan` from a
coherent config/registry/runtime basis and fresh host reservations. It uses a
structural catalogue resolver to prepare VPN port candidates before certificate
issuance, while carrying certificate claims as later-phase requirements. A stale
root read, changed virtual-absence inventory, host listener collision, allocation
issue, missing required credential reference, or unqualified native/foreign
resource remains a preflight blocker; missing certificate material is handled by
the issue/reuse phase and ordinary catalogue readiness is rechecked immediately
before ApplyIngress. Public DNS-A stays unverified.
Preflight itself writes nothing. Mutating reconciliation recomputes the plan and
rechecks config read set, registry/runtime revisions, install identity, reservation
digest and candidate before writes. The exact candidate is journaled and committed
through `PortSettingsWriter` before installation identity/root config publication.
OR-10 directly authorizes a first random assignment after collision checks; a
change to an existing saved port still requires separate exact allocation consent.
Explicit domain/edge/email input and the exact menu selection are direct consent;
Core applies changed existing-root values through typed CAS writers without a
second TTY prompt. Generic input writers retain their separate opaque approval.
Interrupted allocation is recovered before rerolling; future invocations load
persisted ports. The first-root path allows only distribution-owned
`distribution.lock` in the work directory before Core effects.

Before package/bootstrap effects, read-only preflight validates `/var/www/index.html`
through bounded no-follow reads and binds either its absence/parent identity or its
content digest/filesystem identity into the immutable install plan. Mutating Core
rechecks that basis; only then may it create the default index if absent. The
renderer publishes the validated index bytes as the immutable `site-index.html`
artifact in the same candidate bundle as the fixed Caddy route. Caddy rewrites
non-VLESS site requests to that one artifact; it does not mount the host webroot or
another mutable source directory, so unrelated webroot files and symlinks are
excluded. This keeps site content and route in one rollback unit and avoids
Caddyfile parser transformations of arbitrary page bytes.

The execution sequence saves typed desired input (including account email and
fixed InternetBS SecretRefs), persists approved allocation, completes Core and
selected owned-module setup, calls `certs.issue`, then normal effects prepare/apply.
Missing InternetBS secret versions return the fixed partial marker
`requiredNext:["importInternetbsDnsCredentials"]`; the shell imports files through
`keys.manage dns` and retries with identical install inputs. No secret bytes appear
in install arguments or responses. Incomplete phases return `partial`, never
`ready`. Readiness requires local `127.0.0.1:443` TLS+SNI GET with HTTP 200 and the
exact `/var/www/index.html` body, with unchanged source identity, digest and bytes
before and after the probe. This proves local serving only: `externalDns` is
`unverified`, and installation does not automatically query public DNS or identity.

The shell's separate `install.reconcile --input-preflight` mode is read-only even
under a mutating CLI context and never falls into seed/setup. It accepts no typed
values or a partial/full set of the fixed input pairs, returning `inputRequired`
for all missing fields. With sufficient values it returns `status:ready` only if
the full immutable plan has no conflicts or approvals; otherwise it returns
`preflight` and the shell stops before bootstrap. Normal no-argument reconciliation
reuses saved inputs and allocations. Unsupported selected modules and unqualified
foreign Amnezia remain blockers. Allocation consent currently occurs in mutating
Core reconciliation only when replacing an existing saved value; OR-10 directly
authorizes first assignment after the exact collision-free candidate is checked.

Initialize configurable redacted logs with automatic bounded rotation and deletion
of material older than one week (default `/var/logs/hton`). Check code/config/data
rights separately. Recovery is reinstall from the private `.secrets` profile and a
separate explicit `up`; R1 has no backup/restore mode. R1 uses IPv4.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).


## I3 shell/bootstrap binding

scripts/install and other commands are bounded argv adapters to fixed Core operations;
installation shell owns current-source deployment handoff and OS package mechanics.
`scripts/deploy` copies the bounded allowlist directly from the current working tree;
`lib/bootstrap` only manages OS dependencies and delegates registry/config/secret/
setup writes to install.reconcile. No release manifest is required. No remote
artifact discovery/download or on-VPS compile is introduced.

Ubuntu/Debian bootstrap follows the [Ubuntu official apt recipe](https://docs.docker.com/engine/install/ubuntu/)
and [Debian official apt recipe](https://docs.docker.com/engine/install/debian/) with
distro-specific signed origins, HTTPS-only key transfer and fixed package names.
Existing Docker works unchanged; conflicting packages/orphan data are refused,
never auto-removed. Own interrupted repo files resume only if exact current expected
bytes match; drift retains originals. --docker-only does dependencies only;
--dry-run does no writes/packages/downloads. Unsupported OS prints manual guidance.

Units use independent runtime boot/guardian and bounded Core automatic ACME renewal.
No ExecStop on the boot-only data unit changes persistent intent during host reboot.
Public down uses lifecycle.down; accepted boot never loads YAML/compiler. Full
unit installation/restart-owner registration remains Core install.reconcile's
journaled responsibility. The shell never enables units behind that owner.

## INS-12 Managed provisioning and masking review — OR-37–OR-40

INS-4's preinstalled-container restriction applies only to associate mode. Provision
mode creates the pinned managed service and the configured dedicated SSH account
through separate effect owners; no root password is read or stored. Account name,
SNI and output links come from the same validated config snapshot. See
[provisioning task](../plans/INSTALLATION-IMPLEMENTATION.md) for mode compatibility,
privileges, protected state, recovery and per-resource cleanup.

Order: validate config/ownership/ports; record intent; prepare private backend and
real-site certificate/fallback; qualify the managed service on private bindings;
prepare host access if requested; admit the reviewed edge generation; verify final
routes and issue protected access artifacts. Do not temporarily publish a raw VPN
backend for qualification. Do not open Docker API or alter global sshd policy.
`admin.access: none` reports explicitly disabled access; generated access failure
reports degraded rather than ready. Renewal cannot provision accounts or migrate SNI.

All unit then functional acceptance follows completed implementation. Acceptance
uses the [hardening matrix](../plans/INSTALLATION-IMPLEMENTATION.md), including externally
reachable ports, protocol authentication and fallback; local rendering alone cannot
establish resistance to probing or explain the previous ban. External observations
are explicit, bounded acceptance actions, never background diagnosis (OR-8).

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

## INS-13 Client-page input and no-prompt installation — OR-45/OR-49

After reading saved config, an absent core.clientAccessPage.path publishes a
generated secret path on first install; an explicit empty path is the operator's
opt-out and is never changed into a generated one. `--rotate-client-page` is the
only explicit replacement action. Use config's single writer/CAS, never a second
shell editor. Provisioning defaults remain OR-32/D143. Nonempty page is rendered
only from accepted named client inventory, served by the existing Caddy from
`work/client-page` and refreshed locally according to its own SDD. After a
successful install, per-user guest artifacts and `client-page.md` are published to
`.secrets/share` (runtime and source checkout) with `0640`. Full admin stays TODO;
no authentication service/API is pulled into minimum.

`-q` is input policy as well as output policy: propagate NoPrompt through site/email,
page, protocol change, DNS/manual retry, legacy/remap consent and elevation; sudo
uses -n. Reuse saved inputs and defaults, skip optional empty values. Required
missing inputs/privilege/grants return typed nonzero inputRequired/policyBlocked;
--yes and quiet cannot approve DROP-LEGACY-SNI or unrelated foreign changes.
Dry-run takes precedence over prompts/effects; it never persists a page answer or
publishes artifacts. --json also cannot silently enter interactive secret prompts.
