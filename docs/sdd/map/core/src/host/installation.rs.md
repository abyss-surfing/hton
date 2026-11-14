# core/src/host/installation.rs

Stable ID core.host-installation · R1 · I3-A

Explicit protected install coordinator. Reconcile validates root/euid, immutable
plan inputs, config read sets, registry/runtime revisions, install identity, host
reservations and exact candidate before writes. It seeds typed desired input and
selected modules on first root, preserves saved values on repeat, persists the
approved allocation through `PortSettingsWriter`, completes supported owned setup,
issues/reuses the DNS-01 certificate, and uses normal ingress prepare/apply. It
binds `/var/www/index.html` absence or exact content/filesystem identity into the
plan, then ensures that exact file before setup. The renderer publishes the captured
bytes as immutable `site-index.html` in the same bundle as the Caddy route; no host
webroot is mounted. It returns `siteReady:true`
only after local TLS/SNI/HTTP 200/exact body verification with source
identity/digest/bytes unchanged around the probe; the bounded probe supports the
webroot's 1 MiB file limit. The result reports
`externalDns: unverified`. Dry-run has no storage effects.
Initial root serialization delegates to `core.config.initial_desired` and
`root_document`; this keeps the typed virtual defaults and bootstrap document aligned.

`selection_offer(paths,args,ctx)` is a strictly read-only, catalogue-backed offer
for install TTY selection. It returns fixed profile suggestions, supported R1 module
IDs, saved optional IDs, offer digest, root revision/expected absence and whether a
profile was ignored on an initialized config. It rejects corrupt/incoherent config
and orphan module/Docker settings without creating directories. `reconcile` accepts
only internal digest-bound repeated selected IDs and rechecks the current root and
catalogue offer; it returns a typed `InstallSelectionCandidate` binding catalogue
version, offer digest, selected/prior IDs, exact root revision or absence and a
canonical candidate digest. First-root menu selection is direct consent and is
revalidated before execution; initialized-install changes retain the review gate.

`InstallPlan.site_index` binds either exact `/var/www/index.html` content plus
file/parent identity or exact absent-path identity. Mutating rechecks compare this
basis before writes; `ensure_index` repeats it before creating the placeholder.
Local readiness re-reads source identity, content digest and bytes before and after
the fixed localhost HTTPS/SNI probe and compares the response body exactly to those
bytes. Public DNS or off-host reachability is outside this result.

## Current public-ingress installation binding

`reconcile(paths, args, ctx) -> Result<Value>` owns fixed install dispatch. It
parses only typed site/selection inputs, validates fixed HostPaths and handles an
exact pending port-allocation recovery before it opens or writes install state.
Read-only `--input-preflight` and mutating execution both recompute their own
`InstallPlan`; the mutating path rechecks configuration, registry, runtime,
reservation evidence, site source identity and exact candidate before effects.

The current mutating sequence journals and commits the service-owned port candidate,
installation identity and initial canonical config, creates protected config/work
inventory, persists allowed typed input/selection changes, registers Core, initializes
runtime identity/approvals/logging, and runs Core plus selected owned module setup.
Missing InternetBS secret versions return a resumable partial result. Otherwise the
coordinator issues/reuses the configured DNS-01 certificate, verifies the captured
site source has not changed, prepares/applies the normal ingress bundle and performs
an exact local TLS/SNI/HTTP 200/body probe. Site success is local evidence only;
`externalDns` remains `unverified`.

Foreign integration is intentionally fail-closed before bootstrap effects whenever
Amnezia is selected or an install plan carries foreign bindings/remap approvals.
The exact installed native listener, endpoint/SNI, restart/data ownership and
snapshot/remap/restore evidence do not yet have a qualified producer. The current
coordinator therefore cannot deliver the requested Amnezia split and must not claim
whole-system ready. Do not weaken this gate or infer compatibility from Docker
candidate discovery. See OR-12/OR-14, D101/D103/D104, the foreign-adapters SDD and
shared install contracts.

After ingress apply returns a receipt, the install operation retains its accepted
and pending bundle identities, run intent, member outcomes, survivors and issues
in its own durable partial result before checking site readiness. The child ingress
transaction remains separately identifiable; the install transaction ID is never
replaced by it. A failed site probe must not erase already-applied runtime state.

`verify_local_https_site(app, source, ctx) -> Result<bool>` checks only the local
site certificate/SNI/HTTP response and exact immutable body. It does not verify the
Amnezia SNI route or native handshake. Consequently this probe alone is not the
whole-product readiness criterion: `ready:true` is reserved for a later qualified
install path that verifies both the site and Amnezia routes. If that foreign route
verification has not run, return a partial/degraded result with `ready:false`, even
when `siteReady:true`.

`InstallPlan` remains read-only execution input, not activation authority. It binds
root/module read sets, selection and input candidate digests, complete TCP/UDP
reservation evidence, site-index basis, allocation candidate, foreign bindings,
certificate requirements, conflicts, approvals and fixed phases. The first-install
path composes defaults in memory and validates expected absence before persisting.
Saved site changes require an exact protected TTY review, verified into the opaque
`InstallInputApproval` before any install effects, and commit through the generic
approved writer. A first root uses direct typed input consent. Changed saved module
selection remains blocked pending its own review path. `PortSettingsWriter` owns
candidate persistence/recovery.

Current renderer wiring routes the configured site SNI to Caddy with TLS termination
at Caddy and emits qualified foreign SNI routes only when the graph supplies a
qualified private endpoint. HAProxy remains byte-preserving TCP passthrough. The
installer does not create Amnezia qualification evidence, edit foreign containers
or infer public DNS reachability.

## Input contract and ownership

`scripts/install` gathers typed non-secret site `InstallInput` via bounded TTY prompts. Core allocates distinct persisted 5000–6000 VPN backend ports using fresh reservations; a service sharing TCP/UDP uses one number. Allocation commits through each service's existing settings owner. The shared public HAProxy edge remains TCP/443. Foreign remaps require exact consent/snapshot gates.
