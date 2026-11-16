# core/src/host/mod.rs

Stable ID: core.host · R1 · I3-A

Installed application composition: fixed paths, bounded CLI operation dispatch,
local OS actor/context, protected stores and dependency injection. Domain algorithms
remain in their existing owners. open(readOnly) never initializes files/locks/logs;
bootstrap/seed is an explicit journalled mutation. Installation identity is stable
random metadata, never a display-name guess or a newly generated ID on retries.
Methods: context(paths,operation,mode) -> OperationContext; open(paths,ctx) ->
HostApplication; execute(operation,args,ctx) -> redacted metadata or explicitly
scoped disclosure. Only trusted local root can mutate fixed /opt/hton paths.
No generic shell, user actor assertion, raw YAML sourcing or success on unknown facts.

Fixed `install.reconcile` dispatch delegates to `core.host-installation`, which
recomputes typed plan evidence, persists allowed inputs and VPN allocations,
completes supported owned setup, issues/reuses certificates and uses the normal
ingress prepare/apply path. It must not return whole-system ready based only on
local TLS/SNI/HTTP 200 and exact site-artifact body verification; when qualified
Amnezia route verification has not run it returns partial with `siteReady:true`
and `ready:false`. Foreign Amnezia remap remains qualification-blocked.
The fixed read-only `install.selection-offer` dispatch also delegates to the
installation owner before `HostApplication::open`, so first-install offers do not
create config/work/journal state. It reports `recoveryPending` for unfinished
first-allocation transactions and only trusts rootless settings with exact committed
journal provenance. `install.reconcile` rechecks the offered digest/selection and
executes the supported install phases; Amnezia remap remains fail-closed until its
foreign qualification/snapshot contract is implemented.
The crate contains and calls the pure `core.host-install-input` candidate composer;
the install-only writer uses a narrow direct-consent path for exact typed input and
does not change authority of generic writers.
The shell `scripts/install` does not directly invoke cert, Docker, HAProxy or firewall APIs.
The crate privately reexports `host.approvals.authorize_allocation` and
`authorize_first_install_allocation` with their opaque `AllocationApproval`
capability for allocation owners; no CLI/API payload can construct or deserialize
that capability.
Generic install-input writers retain the separate protected `InstallInputApproval`
capability; it grants no allocation, public-listener or foreign-resource authority.
The allocation approval path is used only when changing a saved port. OR-10
authorizes first random assignment after fresh reservation checks without an
additional TTY approval; `--yes` cannot authorize a saved-port change.
The crate contains `core.host-install-input-writer`, a root-only journal/CAS
primitive with exact recovery. Installation uses a narrow direct-consent commit for
exact typed domain/edge/email input after read-set recheck; the generic approval
path retains its existing authority boundary.

`lifecycle.up` dispatches to the Core lifecycle bridge's accepted-start request,
the same request used by `runtime.start`; it starts the saved composition and
does not compile pending YAML or pull images. `config.apply` owns candidate
preparation and activation. `runtime.rollback` dispatches to the same lifecycle
journal with an exact `previousCompatible` BundleId; runtime revalidates its protected
fallback proof, exact resource bindings and policy/certificate fences before and
after the restore.

`diag.observe` and `identity.whoami` are read-only host compositions. Diagnostics
use registered local resources only; public IPv4/DNS-A observations are exclusive
to an explicit `identity.whoami` call and all unavailable evidence remains visible.
`doctor.observe` is a no-argument read-only adapter to the same `diagnose` owner,
returning the same local service/runtime/Docker report under its own operation ID.
It does not invoke probes, repairs or scheduling. `doctor.fix` and `doctor.watch`
remain on the generic `module.incomplete` path until their own-resource repair
planner and optional single-owner scheduler are implemented.

`publication.switch` delegates to the saved-target publication owner, which checks
complete fresh host listener evidence and uses shared local public consent and
admitted reversible apply. The optional port asserts saved YAML; validation-only
remains read-only. It never remaps a foreign holder or claims native handshake readiness.
`bundle.validate` delegates to protected exact candidate qualification, with native
checks, isolated client/server scenarios, exact scoped cleanup and hash-bound
attestation; unsupported foreign semantic scenarios remain explicit blockers.

Core dispatch operations still unwired: `foreign.restore` needs its qualified ForeignLifecycle observer/transition port,
drift check, local consent and durable restoration.
`installation.uninstall` delegates to the ownership-bounded uninstall coordinator:
it binds exact Owned registry/native IDs, persists whole-server stopped intent,
removes native resources through independent Runtime and removes only exact delivered
systemd links. Config/certificates/code/recovery are retained. Associated resources,
full purge and foreign deletion remain preflight blockers without qualified owners.
The R2 publication feature is distinct from public listener cutover.

`services.restart` dispatches to the same typed lifecycle journal bridge as up/down
and accepted rollback. Omitted selectors are resolved from exact installed Owned
registry resources for enabled optional/data services; core ingress is excluded.
Runtime independently narrows them to current admitted accepted services. Unsupported semantic health probes
produce a journaled Partial result with Runtime receipts/survivors retained.

`modules.remove`, `modules.purge` and `modules.update` delegate to `host/modules.rs`.
The supported resource-free Owned VLESS feature uses prospective selection/effects
and journaled membership removal, preserving shared engines and data. Separate
purge requires exact protected local consent and deletes only retained VLESS settings
with drift/re-add recovery checks. Native/Associated removal remains unavailable
without a qualified exact stop port. Null R1 descriptor update capabilities return
precise unsupported errors; they never update shared/foreign engines or use StopAll.

`modules.enable` now plans managed VPN allocation from the exact prospective
`SelectionCandidate.read_result()` before any selection write: normalize, resolve
against registered runtime observations, bind the trusted catalogue mapping and
`plan_graph` with the same exact RegistrySnapshot plus complete fresh host TCP/UDP
evidence. Missing entropy, graph conflicts, incomplete/stale evidence, changed
read sets or registry revision fail closed. First assignment is authorized by
OR-10 after collision checks; changed saved fields require the separate exact local
`AllocationApproval`. After TTY review, configuration,
registry and reservation digest/freshness are rechecked before the protected
`PortSettingsWriter` journals and commits the exact candidate. Allocation settings
commit precedes SelectionWriter and effects prepare/apply. An unfinished exact
allocation intent is recovered by its writer and the invocation returns partial
for retry; it never creates replacement entropy in that recovery pass.

This is currently a direct-dispatch transaction sequence, not a bound
`LifecycleService` execution: the CLI path does not persist a parent
`Intent::Lifecycle` containing the allocation candidate, and cannot resume a
candidate-bearing lifecycle plan. If an unfinished allocation is linked to such a
lifecycle intent, this dispatcher refuses to take recovery ownership. The generic
lifecycle policy commit port and its production host composition remain separate
work; do not claim full lifecycle candidate correlation from this dispatch.

`foreign_observations::LocalForeignObserver` binds the adapter's read-only port to
`HostObserver::docker().foreign_candidate_facts`. Candidate inventory is complete
only after stable repeated full-ID inspections. Exact-ID native Amnezia qualification
and registered foreign observation remain typed unsupported until native protocol,
listener, ownership and snapshot facts have a trusted producer. No candidate is
associated or mutated by this binding.
