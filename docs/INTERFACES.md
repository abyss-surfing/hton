# Detailed structures and module interfaces

Architecture contract, 2026-10-31; R1 implementation authorized by D79. Completion
and compilation evidence are tracked in [implementation status](IMPLEMENTATION-STATUS.md). Authority:
[owner requirements](OWNER-REQUIREMENTS.md), [architecture](ARCHITECTURE.md),
[area SDD](specs/README.md). Per-file method ownership and both call directions:
[file map](sdd/map/README.md). [Orchestrator](IMPLEMENTATION-ORCHESTRATOR.md)
controls execution order. Structures below are language-neutral; Rust/TypeScript
bindings in the map are proposed implementation bindings, not independent rules.

## 1. Responsibility and dependency rules

OR-21 public-product boundary: separate NodeConfig (node identity/root),
PublicationPolicy (names, entry addresses and topology), CertificatePolicy (SANs,
challenge/issuer selection), named DnsProviderAccount secret references and
CertificateStore state. Domain/email/vendor are deployment inputs. Observe a
fleet's declared public entry points; do not equate every domain with one node's
egress IP. Per-node operation does not require an always-running fleet controller.
The current minimum binding below is incomplete relative to this product scope.


The OR-15 minimum installation has the execution binding
[sdd-minimum-installation](specs/sdd-minimum-installation.md): default `scripts/install`
compares physical CWD, deploys when outside the configured target, changes into
that target and invokes scripts/setup. Setup rejects wrong CWD and invokes the
protected Python Core. Root configuration inputs are owned by lib/install_config.py. It accepts exact
site/email/native-selector/protected-DNS-input arguments; no Rust build runs on
the target. Other full-R1 mutations refuse a minimum-owned installation. Saved
module port state and source/data permissions remain canonical and persistent.

Use modules with narrow interfaces and substantial owned behavior. Prefer immutable
records and functions for transformations; use stateful objects only for owned
resources (locks, journals, runtime lease, bounded log sink). Do not invent a class
per enum or command. A process exists for authority/lifecycle isolation, not for
SOLID compliance. Dependencies are passed explicitly at construction/operation time.

| Principle | Concrete planning obligation |
|---|---|
| SRP | Config parser reads/normalizes; configurator edits; planner decides effects; renderer creates artifacts; runtime performs admitted effects |
| OCP | Trusted engine adapters implement declared check/render/setup/update interfaces; adding one changes its registration/descriptor, not every caller |
| LSP | Every adapter returns the same explicit supported/blocked/unknown results; unsupported does not mean silent success or fallback |
| ISP | Split pure module checks, bounded observations, rendering, setup and update capabilities; no giant adapter requiring every optional method |
| DIP | Core depends on typed ports; concrete host/Docker/filesystem adapters are composed at entries; runtime imports only its independent wire contract |
| GRASP Expert | Registry owns adoption facts; runtime state owns run intent/lease; secret store owns versions; descriptor owns module-specific rules |
| GRASP Controller | Core lifecycle coordinates user operations, Core apply coordinates artifact promotion; neither owns every engine rule |
| GRASP Low Coupling / High Cohesion | CLI/API/UI translate requests; business decisions remain in owners; runtime cannot call back into compiler |
| DRY | One normalizer, dependency resolver, authorization envelope check, operation journal and secret-redaction policy; no UI/CLI copies |

Wire records contain data, never function pointers, shell commands or arbitrary paths.
Trusted adapter implementations are shipped code; user YAML cannot select new code.
A shared contract package contains types/versioning only. The independently buildable
runtime wire package contains only runtime DTOs; management-only types/packages are
outside the runtime dependency closure.

## 2. Common records, errors and context

| Record | Fields / invariant |
|---|---|
| `InstanceId`, `ModuleId`, `ServiceId`, `ObjectId` | Validated logical identifiers; no path separators or implicit Docker IDs |
| `OperationId`, `RequestId`, `PlanId`, `BundleId`, `DraftId` | Distinct opaque identifiers; not interchangeable strings in internal bindings |
| `FormatVersion`, `ApiVersion`, `CatalogueVersion`, `ImageDigest` | Separate meanings; software release is not a config format version |
| `ContentRevision` | Digest of exact bytes plus object identity; semantic normalization does not erase concurrency evidence |
| `ReadSet` | Present object IDs -> content revision/format version plus explicitly absent known descriptor document IDs; root/module/Docker files and relevant secret versions; coherent inventory/read status |
| `ActorContext` | actorId, source(local/api/timer), granted scopes, local approval references, requestId; provided by trusted entry, never asserted by client payload |
| `OperationContext` | operationId, actor, executionMode(readOnly/mutating), deadline, cancellation, redacted event sink; services cannot outlive the deadline unnoticed |
| `Error` | code, safe message, moduleId?, configPath?, resourceId?, operationId, retryability, repairHint, retainedState; no raw secret/backend stderr |
| `Result<T>` | success(T) or structured error; partial operations return typed aggregate plus failed status, never success with hidden failures |
| `Evidence` | checkId, moduleId, outcome(pass/fail/unknown/skipped), timestamp, vantage, artifact/input fingerprint, bounded details |
| `CheckReport` | issues[], evidence[], blocking flag, normalized snapshot?; warnings and unavailable evidence stay visible |
| `Budget` | deadline, max concurrency, bytes, observations; missing values get trusted bounded defaults, never unbounded work |
| `ApprovalEnvelope` | exact listener/privilege/resource scope, issuer, evidence, version; UI/token cannot broaden a local grant |
| `SafeFields` | whitelisted structured metadata only; secret material and full credential URLs cannot be formatted through ordinary logger |

Error codes are stable names such as `config.syntax`, `config.unsupportedVersion`,
`config.semanticConflict`, `plan.stale`, `module.incomplete`, `resource.drift`,
`approval.required`, `runtime.stopped`, `runtime.partial`, `recovery.required`.
Transport maps them to CLI exits/HTTP status without changing domain semantics.
Native process output is bounded and redacted before entering Error/Evidence.

## 3. Configuration and composition records

| Record | Fields and owner |
|---|---|
| `ConfigDocument` | objectId, relative known location, source bytes/revision, schemaVersion, parse diagnostics; Core config |
| `DesiredReadResult` | readable document metadata plus snapshot or parse errors; invalid target does not crash tool bootstrap |
| `DesiredSnapshot` | platform settings, feature enabled selection, moduleSettings by stable ID, editableDocker inputs, readSet; no observed health |
| `NormalizedSnapshot` | desired snapshot + explicit/default provenance, supported versions, resolved references; no generated secrets/random endpoints |
| `ModuleDescriptor` | moduleId, phase, location, config schema/version, defaults, dependencies, services, static listener/route claims, optional validated `portAllocation` binding (allocationId, serviceId, settingsDocument, bindingGroup, exact path/transport fields, Host listener namespace/exposure/address template), privileges, state, check/setup/update capability IDs |
| `RegistryEntry` | moduleId, ownership(owned/associated), exact resources, setupState, checkpoints, installed artifact versions, original policies, adoption scope; no desired enabled duplicate |
| `RegistrySnapshot` | instanceId, revision, entries; malformed record diagnostics and protected recovery originals |
| `SetupState` | incomplete/complete plus blocked step and required operator action; never inferred from enabled |
| `DependencyGraph` | enabled module closure, shared service consumers, topological order, listener/route/cert/state claims and conflict report |
| `RuntimeObservation` | per-resource observed running/stopped/failed/unknown, identity/fingerprint, freshness; not written into desired YAML |
| `InstallInput` | validated siteDomain, public TCP/443 `HostEndpoint`, optional non-secret certificate account email and field provenance; VPN port candidates are Core-generated; DNS credential bytes are never input |
| `InstallSelectionOffer` | trusted catalogue version, offer digest, exact root revision or expected absence, currently saved optional module IDs, supported selectable module descriptors and profile suggestions; read-only, no config or resource authority |
| `InstallSelectionCandidate` | exact offered catalogue version/digest, selected and prior module IDs, coherent root revision or expected root absence, candidate digest; transient only, never foreign association or remap consent; Core derives the prospective snapshot from these values |
| `InstallSelectionReceipt` | installation transaction, resulting root revision, saved selection and write state; no config bytes |
| `ForeignCandidateSummary` | immutable Docker container ID, bounded redacted image/runtime/port metadata and observation time; discovery only, no module identity or authority |
| `ForeignQualificationEvidence` | exact resource ID, structural fingerprint, typed Amnezia/Xray protocol and launch facts, coverage flags, evidence digest and expiry; stale or incomplete qualification cannot authorize adoption or allocation |
| `AssociationApproval` | opaque host-issued capability bound to exact resource ID/fingerprint, qualification digest, minimal lifecycle scope, original restart owner/policy, instance, actor, operation and expiry; grants no allocation or remap authority |
| `AssociationReceipt` | journal transaction, exact associated registry entry/revision, qualification digest and lifecycle scope; no raw launch/config bytes |
| `InstallConfigCandidate` | exact permitted siteDomain, publicEdge and certificateEmail changes, prior values, new values and coherent source revisions; no arbitrary patch/path. For a new certificate configuration only, Core seeds fixed Let's Encrypt DNS-01/InternetBS metadata and fixed SecretStore references `dns/internetbs/api-key` and `dns/internetbs/api-password`; existing provider/reference values are preserved |
| `InstallInputReview` | candidate digest and trusted local review of exact config changes; `--yes` cannot synthesize review of existing values |
| `InstallConfigReceipt` | journal transaction, per-document before/after revisions and write states; no raw config bytes |
| `PortAllocationCandidate` | immutable serviceId→port map, prior values, trusted descriptor revisions, reservation evidence digest, selected service set, coherent config ReadSet including exact expected absence for descriptor-known module settings, exact separate RegistrySnapshot revision and expiry; no entropy or host probing details beyond bounded evidence |
| `PortAllocationReceipt` | optional transaction ID (absent only for exact no-op), service→port assignments, per-document resulting revisions and write/recovery state; no raw config bytes |
| `AllocationApproval` | opaque crate-private capability issued by `host.approvals`; binds candidate digest, complete candidate serialization, instance, local actor, operation ID and expiry; cannot be deserialized or constructed by callers |
| `LifecyclePlan` | exact request and `ExpectedInputs`, typed actions/changes, optional selection/setup/effect records and optional exact `PortAllocationCandidate`, issues and disruption; the candidate is immutable plan data, never fetched by a hidden ID during execute. The additive optional field defaults absent for legacy serialized records; execution of a legacy add/enable that now requires an allocation candidate fails stale/recovery review without silently allocating. |
| `PreparedOperation` | owner-produced selection/setup/effect/update/association evidence and optional `PortAllocationCandidate`; host preparation supplies it, pure lifecycle planning copies the exact value into `LifecyclePlan` without entropy or settings traversal |
| `InstallPlan` | planId, stable `instanceId` (including virtual first-install identity), and `ExpectedInputs`; mandatory `host_reservation_evidence: ReservationEvidence` for the complete fresh host TCP/UDP snapshot (including public TCP/443 even without a VPN allocation); exact selection/input candidates; exact `siteIndex` absence identity or present content/filesystem identity; prospective `DependencyGraph` and public edge; exact foreign resource identities with current/proposed host bindings and preserved native endpoints; optional `PortAllocationCandidate` whose reservation digest/time basis matches that evidence; certificate claims, conflicts, typed approval requirements, fixed effect phases and expiry; pure preflight output with no capability or secret bytes |
| `ForeignSnapshotHandle` | opaque protected logical handle, exact resource IDs, content digest, captured launch/network/mount/port/restart/runtime manifest; bytes and secret fields never serialize |
| `ForeignRemapReceipt` | transaction/resource IDs, exact prior/new container identities and host transport-port bindings, before/after fingerprints, snapshot digest, time and state; no native config or key material |
| `InstallResult` | common `OperationResult`, typed per-phase outcomes, certificate status, foreign remap transaction references and explicit unverified external DNS state |

Concrete R1 listener binding: each `ListenerClaim` carries a mandatory typed
`namespace`: Host, Project(networkId), or Service(serviceId). `public` describes
exposure and never infers a network namespace. Address/port/transport overlaps
are compared in the same explicit namespace; a host loopback endpoint therefore
conflicts with a host wildcard independently of its public flag. Runtime wire
host-published claims and private native artifacts must preserve that distinction.

Descriptors reference canonical module settings schemas. Root feature objects have
only enabled; module settings have their own document/version. Settings with no
implementation can be retained but cannot activate. Schema acceptance does not
establish installed resources, DNS/certificate readiness or granted privileges.

## 4. Core interfaces and side-effect ownership

Port allocation is owned by [`core.port-allocation`](specs/sdd-port-allocation.md)
and implemented behind the planned [`core/src/port_allocation.rs` map](sdd/map/core/src/port_allocation.rs.md).
Its explicit entropy/evidence inputs keep candidate planning deterministic; its
commit is the only writer for descriptor-allowlisted VPN port settings.

| Owner / method | Arguments | Result / effects |
|---|---|---|
| config.load | configRoot handle, supportedVersions | DesiredReadResult; bounded read-only, coherent read set or conflict |
| config.validate | DesiredSnapshot, catalogue, RegistrySnapshot, approvals | CheckReport/NormalizedSnapshot; pure checks plus explicitly supplied evidence |
| catalog.describe | ModuleId | ModuleDescriptor or unknownModule |
| catalog.profileSuggestions | fixed `ProfileId` | Catalogue-ordered optional module IDs for first-run suggestions only; validates that every suggestion is a known implemented R1 module and grants no install/association authority |
| catalog.resolve | normalized selection, catalogueVersion, releasePhase, platformEvidence | DependencyGraph; no installs/enables/effects |
| catalog.bindAllocation | DependencyGraph, PortAllocationCandidate, validated desired snapshot | DependencyGraph with exact host ListenerClaims materialized from descriptor `portAllocation` mappings; pure, rejects missing/extra/mismatched candidate bindings |
| catalog.allocationBindingsFor | selected DependencyGraph, validated NormalizedSnapshot | trusted descriptor bindings enriched with exact saved values or descriptor defaults for coherently absent first-install documents; no caller-owned path traversal |
| catalog.retainedAllocationsFor | selected service IDs, installed/retained registry service IDs, validated NormalizedSnapshot | saved allocations for nonselected installed VPN services; incomplete/mismatched transport pairs block planning |
| registry.read | InstanceId | RegistrySnapshot; registry files under work only |
| registry.transition | expected revision, RegistryChange, OperationContext | durable compare-and-set result; journal before mutating facts |
| host.observations.reservationEvidenceFor | exact `RegistrySnapshot`, injected `OperationControl`, `OperationContext` | complete point-in-time TCP/UDP reservations; read-only, maps Docker holders only by exact registered ID and fingerprint; usable before bootstrap without opening or creating `HostApplication` |
| host.docker.observeContainerNetworkListeners | exact immutable Docker `ContainerId`, expected structural fingerprint, bounded `Budget` and `Cancellation` | fingerprint- and process-instance-bound TCP LISTEN/UDP-bind rows from that container's Linux network namespace; re-inspects the exact container before/after and checks init PID start time/namespace identity; read-only and does not prove which process owns a socket or grant qualification/remap authority |
| foreign.discoverCandidates | bounded local Docker inventory, Budget | redacted `ForeignCandidateSummary[]`; read-only and never registers candidates |
| foreign.qualifyCandidate | one operator-selected immutable container ID, fresh exact inspect and bounded protocol facts, Budget | `ForeignQualificationEvidence` or typed blocker; no adoption, config write or remap |
| host.approvals.authorizeAdoption | exact qualification evidence, minimal lifecycle scope, local OperationContext | distinct local confirmation and opaque `AssociationApproval`; does not grant port allocation or resource recreation |
| registry.adopt | `AssociationApproval`, exact inspected identity/fingerprint, expected registry revision, journal, OperationContext | `AssociationReceipt`; typed `RegistryChange::Adopt` only after rechecking qualification freshness and exact identity |
| lifecycle.plan | LifecycleRequest, optional DesiredReadResult, RegistrySnapshot, RuntimeState, ActorContext | LifecyclePlan; down can plan from registry with invalid desired config |
| lifecycle.execute | exact LifecyclePlan plus fresh protected config/catalogue/registry/runtime/resource/host evidence, OperationContext | OperationResult with member outcomes; validates the exact candidate and never generates replacement entropy |
| plan.plan | NormalizedSnapshot, DependencyGraph, RuntimeObservation, RegistrySnapshot, ReadSet, approvals | immutable EffectPlan; pure calculation, no pulls/secret generation |
| `port-allocation.plan` | selected VPN service IDs, saved service settings, coherent config ReadSet, exact RegistrySnapshot revision, fresh host TCP/UDP reservation evidence | PortAllocationCandidate; OS-cryptographic random selection in inclusive 5000–6000 range for missing values, one unique port per managed VPN service, shared across TCP/UDP of one service; pure except explicit entropy input |
| `port-allocation.planGraph` | trusted ModuleCatalogue, selected DependencyGraph, validated NormalizedSnapshot, exact diagnostic-free RegistrySnapshot, fresh ReservationEvidence, explicit entropy/time/budget | PortAllocationCandidate plus graph with dynamic host ListenerClaims; composes selected and retained saved bindings without I/O/effects; rejects registry diagnostics |
| `host.approvals.authorizeAllocation` | exact PortAllocationCandidate, local OperationContext | Protected exact-scope TTY approval and opaque AllocationApproval capability; candidate service/port/path/transport mapping plus read-set revision digest is displayed and bound to instance, actor, operation and expiry; `--yes` cannot mint it |
| `host.approvals.authorizeFirstInstallAllocation` | pre-generated InstanceId, exact PortAllocationCandidate, local OperationContext, injected OperationControl | Exact local TTY review is reserved for changing an already saved port; OR-10 authorizes first assignment after fresh collision checks. A saved-port change returns the opaque crate-private `AllocationApproval`, held only in memory and bound to candidate, instance, actor, operation and deadline; no config/work/journal writes or persisted approval record; `--yes` cannot mint it |
| `port-allocation.commit` | PortAllocationCandidate, expected read set, optional opaque AllocationApproval, typed module settings store, journal, OperationContext | exact no-op without approval/journal when saved values already match; first assignment needs no separate approval; a change to a saved value requires exact capability. Same-operation/same-candidate commit retry reconciles its exact journal intent, while a different unfinished allocation digest conflicts; otherwise journals `Intent::PortAllocation { candidate, instance_id }` plus protected originals/presence before per-document CAS of descriptor-allowlisted fields; creates an absent settings document from trusted descriptor defaults in the same CAS as the allocation; local-root recovery from a later CLI operation may reconcile only that exact protected transaction; no duplicate global registry |
| lifecycle allocation preparation | add/enable request, prospective exact selection graph, coherent config read set, exact registry snapshot, fresh TCP/UDP reservation evidence, OS CSPRNG entropy and budget | exact candidate in immutable `LifecyclePlan`; `catalog.bindAllocation` materializes the corresponding claims used for blocker/effect planning. Disable/remove/down do not create allocations. Discovery alone never selects or associates a foreign service. |
| lifecycle allocation execution | plan candidate, fresh exact config/catalogue/registry/resource/reservation evidence, local `OperationContext` | revalidates the same candidate and expiry; any changed dependency returns `planStale`, never a new random value. After blocker review, a changed saved port mapping requires a distinct `host.approvals.authorizeAllocation` capability (`--yes` and generic effect approval do not satisfy it). The lifecycle intent records the exact candidate before `core.port-allocation.commit` journals protected originals and performs descriptor-allowlisted CAS. An exact saved-value no-op needs no allocation approval. |
| render.compile | normalized candidate, graph, resolved secret versions, validated Docker inputs | CandidateBundle in isolated protected staging area; deterministic artifacts |
| apply.apply | PlanId, expectedInputs, OperationContext | OperationResult; recheck, journal, render, admission/trial/accept or compensation |
| `validation.bundle.testBundle` | protected `BundleId` lookup, exact candidate/artifact/image digests, bounded fixed test topology, Budget and OperationContext | `BundleValidationAttestation`; explicit mutating qualification in an isolated private Docker project, native validators then pinned server/client scenarios, no public binds/foreign mounts; journaled cleanup only for exact created IDs/project; attestation binds all inputs, per-scenario evidence and cleanup receipt; external/device behavior remains unverified |
| journal.prepare | plan, protected originals, OperationContext | TransactionId; durable intent before effects |
| journal.finish | transactionId, typed status/effect receipts | durable terminal/partial record; no executable journal contents |
| journal.recover | transactions, observed effects, run intent, approval scope | reconciled results or recovery-required; no blind replay |
| `install.reconcile --input-preflight` | optional subset of `--site-domain D --edge-port 443 --certificate-email E`; no VPN port options or secret bytes | Strictly read-only even under mutating context; returns one of the wire results below; never seeds/setup or claims ready without complete host/foreign conflict evidence |
| `install.selection-offer` | optional `--profile minimal\|home\|compat\|lab`, optional `--menu`, coherent desired read and trusted catalogue; no selection IDs | `InstallSelectionOffer`; strictly read-only catalogue offer for TTY display, no files or foreign discovery/adoption |
| `install.reconcile` selection input | `--selection-offer-digest DIGEST`, repeated internal `--selected-module ID`, and the original optional `--profile` | reloads and validates the exact offer/root/catalogue and selected IDs; builds a transient revision-bound `InstallSelectionCandidate`; mutating install rechecks it before persistence; input-preflight remains zero-write |
| `installation.prepareSelection` | exact offer, chosen module IDs, coherent desired read or trusted first-install defaults | `InstallSelectionCandidate` and prospective desired snapshot; pure, binds exact current/absent root state |
| `installation.associateForeign` | exact operator-selected immutable resource ID, fresh `ForeignQualificationEvidence`, `AssociationApproval`, expected `RegistrySnapshot.revision` | journalled `RegistryChange::Adopt`; lifecycle scope only, no remap or allocation approval |
| cmd.install.collectInput | pure preflight result, trusted TTY source | Retry preflight with only Core-named missing non-secret values, including certificate account email when required; preserve blockers/review requirements; no effect; first-publication `siteDomain: example.com` and `edgePort: 443` suggestions only when no saved domain exists; user may edit domain; no baked config default or secret-material prompts in shell |
| cmd.install.collectSelection | Core `InstallSelectionOffer`, trusted TTY source | Show Core-provided saved choices/suggestions/options, collect exact IDs, return repeated internal `--selected-module ID` argv; profile is first-run hint, `--menu` revisits saved choices; never edits YAML or infers association |
| installation.prepareInputCandidate | `InstallInput`, coherent `DesiredReadResult` | bounded `InstallConfigCandidate` for core edge/domain only; pure, reports exact before/after values |
| installation.preflight | trusted config/work handles, canonical config read result plus optional transient candidate, registry/runtime observations, complete fresh host TCP/UDP reservation evidence, actor/context | `InstallPlan`; read-only and bounded, binds instance identity and the full reservation digest/observedAt/validUntil even when no VPN allocation exists; conflicts surface before config write, journal, package, certificate, container or listener effect |
| installation.execute | exact `InstallPlan`, transient selection/input/allocation candidates, separate local approvals, OperationContext | `InstallResult`; re-observes host reservations and rechecks the exact digest/freshness plus config/registry/runtime/resource inputs before effects; selected unqualified Amnezia/foreign bindings block before writes; supported site path persists typed inputs/allocations, sets up owned services, issues/reuses certificate, applies ingress and checks exact local site response |
| installation.commitInputs | `InstallConfigCandidate`, expected root/module read set, `InstallInputReview`, journal transaction, OperationContext | InstallationCoordinator-owned per-document CAS of allowlisted fields preserving unrelated content; `InstallConfigReceipt` or readConflict/invalidCandidate/notAuthorized/storageFailure/recoveryRequired |
| installation.reconcile (normal mode) | fixed bootstrap argv, trusted input source, OperationContext | recovers one unfinished exact allocation before new work; recomputes and rechecks the plan; blocks unqualified Amnezia before effects; otherwise commits allowed inputs/allocations, owned setup, certificate and site ingress; local site verification alone returns `siteReady:true`, `ready:false`, `partial` until qualified Amnezia route/native-handshake verification is implemented; public DNS remains unverified |

`validation.bundle.testBundle` first resolves an exact BundleId in the protected
`/opt/hton/work/bundles` store, reloads `candidate.json`, verifies its canonical
digest, every staged artifact hash and every immutable image digest before effects.
It derives a private Compose project/network and private port overrides from the
candidate's declared listener claims; no public bind, foreign mount, arbitrary image,
command or caller Compose is accepted. Native checks precede pinned server/client
scenarios. The operation journals each exact created resource before start and
cleans only those container IDs and its own network/project. Uncertain cleanup is
`recoveryRequired`, never a passing attestation. `BundleValidationAttestation`
binds BundleId/candidate/artifact/image digests, runner version, per-scenario
evidence, operation/time and cleanup receipt. External DNS/reachability, real
devices, censorship and broad client coverage remain unverified. Normal apply does
not invoke semantic tests; it may consume only a fresh successful attestation for
the same candidate.

Concrete binding: `core.host-bundle-validation.execute(app, [BundleId], ctx)`
loads protected `candidate.json` and every declared artifact, checks trusted cached
server/client pins, then runs the fixed `infra.validation-docker` sandbox. Supported
scenarios cover HAProxy/Caddy trusted HTTPS and unknown-SNI rejection, optional
shared Unbound UDP/TCP and VLESS WS/XHTTP with the exact native credential/path and
pinned Xray client. A pinned ACME image supplies fixed curl/dig clients, with its
image digest included in the attestation. Test-only endpoint substitutions, DNS
local zone, HTTP fixture, generated client and the exact fixture-only Xray egress
exception are separately hash-bound; production artifacts are unchanged. Unsupported
selected protocols/foreign routes return an explicit blocker, never native-only
success. Resource creation has deterministic operation labels/names bound by
`Intent::BundleValidation`; immutable IDs are journaled before start, and a later
explicit validation reconciles unfinished scoped cleanup. Cleanup removes only
verified sandbox containers/network and their protected copied artifacts. The
protected attestation has five-minute validity. The public shell command remains
`scripts/validate BUNDLE_ID [--json]`; its read-only Core preview is an internal interface,
not an extension of the public CLI's no-dry-run contract.

For the first install, the future coordinator builds a transient snapshot from
trusted defaults, an empty registry, and expected-absent root/module objects before
opening `HostApplication` or writing bootstrap state. It completes graph,
reservation, conflict and operation-scoped TTY review first, then rechecks expected
absence before the first effect. Fresh profiles leave optional VPN modules off;
ports are allocated only for explicitly selected managed services. An interrupted
allocation resumes its journaled candidate; subsequent installs load saved service
ports and do not draw new entropy. Since protected approval storage does not exist
yet, first-install TTY review uses an opaque in-memory session bound to the exact
candidate and generated instance; durable records begin only after accepted review.

Install selection is a distinct desired-state step. The catalogue supplies selectable
module IDs and profile suggestions; the shell renders only this trusted offer and
returns the operator's exact choices. A profile may seed suggestions only when no
selection has been saved. A repeated install retains the saved selection unless
`--menu` explicitly revisits it. Core builds a transient exact-revision selection
candidate and includes it in install preflight; no selection is persisted if a
dependency, association, reservation or other blocker prevents the plan. After full
preflight and review, installation execution journals and commits the selection with
the other allowlisted install settings. Selecting a foreign module is intent only: it does not
adopt a discovered resource, qualify it, or authorize remapping. Port allocation
consumes only the resulting selected graph after required foreign qualification and
durable association; the settings owner persists the port so later install runs
reuse the same value.
| secrets.generate | scope, kind, explicit replacement policy, OperationContext | protected version reference and explicit disclosure handle; apply only |
| secrets.resolve | validated secretRef, consumerGrant | scoped protected material; no arbitrary path/store export |
| checks.observe | known sections, probe Budget, ObservationPorts | Evidence; no mutation disguised as check |
| checks.identityEvidence | configured domains, descriptor capabilities, bounded observations | local/external/unknown report, never censorship inference from timeout |
| certificates.issue | configured names, CA mode, explicit force, OperationContext | candidate inventory; no service start or direct promotion |
| certificates.status | inventory, served evidence, clock | expiry/coverage/chain/key evidence |
| certificates.issue (install caller) | saved site CertificateNames, protected DNS-01 InternetBS `secretRef`, explicit CA mode, force=false, OperationContext | verified own-site candidate only; no listener activation; reuses valid material |
| identities.issue | identityId, qualified kind/profile, ActorContext | scoped credential result; accounts migration narrows disclosure later |
| exports.exportProfile | accepted binding, kind, target, client, output format, ActorContext | protected explicit export; never from pending desired settings |
| logging.emit | EventContext, severity, code, SafeFields | bounded LogWriteStatus; no VPN stop on log pressure |

### `install.selection-offer` wire result

The operation is strictly read-only and returns `operation:"install.selection-offer"`,
`status:"offer"`, `writes:false`, `catalogueVersion`, `offerDigest`, optional
`profile`, `suggestedModuleIds[]`, `savedModuleIds[]`, `selectableModules[]`, and
the exact root revision or an explicit expected-absent marker. The normal human
response is a bounded rendering of these same trusted IDs and states; it does not
include raw descriptor settings. TTY selection is returned as repeated internal
`--selected-module ID` values plus `--selection-offer-digest DIGEST`; the same
`--profile` accompanies them when a profile was supplied. `--menu` affects only
whether the shell requests edits and is not passed as authority. Core reloads and
validates the offer, source revision and module IDs before using that transient
candidate in install preflight. A changed catalogue/root, unknown or unavailable ID,
non-TTY missing choice, or malformed digest blocks without writes. Dry-run and
`--json` never prompt. The offer is a desired-selection input only; foreign adoption
and remap remain separate reviewed operations.

If a first-install allocation intent is unfinished, the offer returns
`status:"recoveryPending"`, `writes:false`, and bounded detail before inspecting
rootless module files. The shell invokes normal `install.reconcile` recovery before
showing the menu. A rootless module settings document is accepted only when exactly
one committed `PortAllocation` history entry binds the same instance and module
document set, and the writer confirms the current settings match that candidate.

### `install.reconcile --input-preflight` wire results

Every result below includes `operation:"install.reconcile"` and `writes:false`.
This operation reads saved config, transient values and bounded protected journal
inventory to detect exact pending allocation recovery; it never changes installation
files, journal, certificates, containers, or listeners.

| `status` / `reason` | Required fields | Meaning |
|---|---|---|
| `inputRequired` | `requiredInputs[]`, enum: `siteDomain`, `edgePort`, `certificateEmail`; optional first-publication `suggestedValues.siteDomain:"example.com"` and `suggestedValues.edgePort:443` when no domain is saved | TTY may collect or edit only the named non-secret values and retry preflight. DNS API credentials are imported from protected input files through `SecretStore`, never entered into install argv or ordinary prompts. The domain suggestion is not a baked config default. |
| `recoveryPending` | `writes:false`, `operation:install.reconcile` | Shell invokes normal mutating `install.reconcile` once. Core recovers one exact unowned pending allocation, or returns partial with the lifecycle transaction ID when its exact candidate belongs to an unfinished lifecycle parent. |
| `preflight` | immutable `installPlan`, `writes:false` | Read-only plan with exact host reservations and any port candidate; blockers and approvals are inside the plan, and this status never authorizes bootstrap or activation. |
| `ready` | immutable `installPlan`, `writes:false`; empty conflicts/approvals and complete host reservation evidence | Safe to continue the exact typed-input/menu candidate. Core recomputes and rechecks before writes. OR-10 authorizes a first random port assignment after collision checks; changing a saved port still requires exact allocation approval. |

The normal no-argument `install.reconcile` bootstrap remains a distinct operation.
It must not interpret `inputRequired`, `recoveryPending`, `preflight`, or failed results as permission
to continue seed/setup. `ready` requires fresh TCP/UDP host evidence, no plan
conflicts, no saved-value approval requirements and an executable `InstallPlan`;
it is not public-serving readiness. Mutating execution rechecks the plan and
reservation basis before writes.

`doctor.observe` is a Core read-only operation adapter over the existing local
diagnostics implementation. It calls `checks.observe` through the exact registered
resource observation owner, defaults to services/runtime/Docker, accepts no operation
arguments and returns the same status/counts/issues/unverified fields under its own
operation ID. It does not invoke `checks.identityEvidence`, network ports, planner,
apply or a repair/scheduler path. Failure to compose returns the existing structured
error; `failed` diagnostic status maps to exit 1 and `unverified` remains visible.

`LifecycleRequest` is a tagged union: Add/Enable/Disable/Remove/Purge/Update/Up/Down/
StartAccepted/RollbackAccepted/RestartServices. IDs, optional saveOnly and approval references are variant-specific;
Down cannot accept arbitrary target subsets, Up cannot accept image-update flags.
RollbackAccepted names only the exact `previousCompatible` bundle in durable runtime
state and the committed lease's protected fallback record; it cannot select any
retained arbitrary bundle.
`RestartServices` journals a nonempty explicit service set, accepted BundleId,
planned run epoch and bounded waitSeconds. Core resolves omitted CLI IDs from the
saved enabled optional/data service closure and intersects exact Installed-Owned
registry resources with current accepted receipts. Runtime verifies the same bundle
and epoch, rejects empty/Associated/stale targets, and probes only selected services.
`RegistryChange` contains typed adopt/checkpoint/install/remove/update transitions,
not client-supplied replacement JSON. Side effects are receipted by concrete adapters;
uncertain completion is reconciled before retry.

`OperationResult` contains operationId, status(succeeded/partial/failed/recoveryRequired),
transactionId?, desiredSaved, pendingBundle?, acceptedBundle?, runIntent,
memberResults[], survivors[], issues[]. A partial up/down returns failure to CLI/API.
There is no boolean `success` masking member errors.

The installation coordinator preserves the ingress apply receipt's bundle identities,
run intent, member outcomes, survivors and issues in its durable `OperationResult`,
including on partial apply or failed served-site verification. It retains its own
parent transaction identity and reports the child ingress receipt separately.

An existing-root input candidate replacing a saved value adds an `InputChange`
approval requirement to install preflight. Normal mutating reconciliation requests
exact local TTY review and verifies its protected record before bootstrap effects;
the writer consumes the opaque approval. Ordinary `--yes` cannot approve this
change. Read-only preflight remains blocked from automatically continuing it.

`EffectPlan` binds candidate hash/readSet, descriptor/image/secret versions, exact
resource before fingerprints, approvals, actions[], expectedDisruption, expiry and
rollback target. `Action` has fixed kind/resourceId/dependency IDs/preconditions/
postconditions/timeout/compensationRef. An action cannot contain a shell expression.
The render-time plan may allocate declared missing secrets only within journaled apply;
final artifact admission binds the concrete generated versions before activation.

## 5. Trusted adapter ports

These are internal capability-specific interfaces, not one mandatory mega-interface.
Each descriptor binds only the capabilities it implements.

| Port | Input | Output / rule |
|---|---|---|
| `ModuleCheck.check` | typed settings, graph context, supplied observations | structured issues; pure, no implicit Docker operations |
| `ModuleObserve.observe` | exact declared resources, Budget | evidence; explicit bounded read/probe operations |
| `ModuleRender.render` | normalized settings, service consumers, secret grants | NativeArtifacts + manifest claims; no active writes |
| `ModuleSetup.prepare` | moduleId, existing checkpoint, missing inputs, artifact inventory | SetupPlan; steps have idempotency key and user-action alternative |
| `ModuleSetup.executeStep` | approved fixed step, context | StepReceipt; checkpoint survives interruption; no arbitrary instruction eval |
| `ModuleUpdate.plan` | installed/target digest, config/data format evidence | UpdatePlan with compatibility and irreversible effects |
| `ModuleUpdate.execute` | approved UpdatePlan, context | receipts and retained prior versions, or explicit recovery-required |
| `LifecycleSnapshot.read` | moduleId, OperationContext | protected registry, runtime and ownership evidence; stale or unavailable inputs fail closed |
| `LifecyclePreparation.prepareSelection` | exact private SelectionCandidate, moduleId, OperationContext | prospective desired snapshot, validated graph/check report and immutable EffectPlan; never commits selection |
| `RuntimeLifecycle.stopModule` | moduleId, exact owned resource IDs, expected fingerprints, context | per-resource stop/detach receipts; must preserve shared and unrelated services; `StopAll` is not an implementation |
| `ModulePurge.plan/execute` | removed module, typed logical artifact IDs with revisions/digests, reference/recovery evidence, approval | exact owner-dispatched deletion receipts and durable recovery result; no path or shell deletion input |
| `ModuleUpdate.plan/execute` | module-owned installed artifact IDs/digests, trusted pinned target, compatibility evidence, approval | scoped update receipts retaining prior version; unsupported if the module owns no update target |
| `ForeignLifecycle.observe` | registered exact identity | state/restart-policy evidence; drift blocks silent takeover |
| `ForeignLifecycle.transition` | admitted start/stop/autostart action, approved identity, epoch | bounded receipt; native config/certs remain foreign-owned |
| `ForeignDiscovery.list` | fixed local Docker observer, Budget | bounded redacted candidate summaries; immutable IDs only, no inferred module ownership |
| `ForeignQualification.inspect` | exact immutable resource ID, exact-inspect facts, Budget | typed evidence bound to structural fingerprint; unsupported/missing protocol, config, mount or restart facts block |
| `RegistryAdoption.adopt` | exact qualification, opaque `AssociationApproval`, expected registry revision | journalled `Ownership::Associated` entry with minimal lifecycle rights; no port mutation authority |
| `ForeignRemap.preflight` | exact registered ResourceIdentity set, expected fingerprint, requested HostEndpoint + TCP/UDP bindings, Budget | fresh `ForeignEvidence`, destination `PortConflict[]`, supported-recreation evidence and plan fingerprint; read-only, does not confer association or consent |
| `ForeignRemap.snapshot` | exact identities + current fingerprint, OperationContext | protected `ForeignSnapshotHandle` including writable layer/config identity, create parameters, port bindings, networks, mounts, restart policies and runtime state; secret bytes opaque |
| `ForeignRemap.execute` | exact `RemapPlan`, snapshot handle, resource-scoped `ApprovalEnvelope`, OperationContext | `ForeignRemapReceipt`; rechecks source fingerprint, changes only declared host port bindings, verifies native identity/state and preserves restart behavior |
| `ForeignRemap.restore` | exact receipt, snapshot handle, expected current fingerprint, OperationContext | exact inverse receipt or durable `recoveryRequired`; refuses drift |
| `HostDocker.remapPublishedPorts` | immutable container ID, exact before fingerprint, typed `TransportPortSet`, snapshot handle, bounded deadline | replacement immutable ID + inspected launch receipt; no raw argv/image/command, and never discards writable-layer data |
| `NativeAction.run` | admitted service ID, fixed action enum, artifact handles, timeout | bounded redacted process receipt; runtime only |

Prefer upstream engines; renderer adapters serialize their native formats, not
implement protocol clients/servers. Concrete Xray/Caddy/HAProxy/Amnezia adapters
are separate file owners. Shared engine consumers are assembled once per bundle.
A foreign adapter cannot claim an unsupported action merely to satisfy the interface.

The production install path is `scripts/install` → fixed `install.reconcile` →
`cmd.install.collectInput` → `InstallationCoordinator.preflight/execute`; the bounded
shell adapter collects non-secret inputs through TTY prompts and passes typed values to
the trusted Core entry. Explicit typed inputs are direct consent to those exact values
and use narrow CAS commit. OR-9 supplies `example.com` as a first-publication Core
suggestion for this instance, never as a baked config default. The coordinator reads
`core.edge.domains`, `core.edge.listen`, `core.certificates` and persisted VPN
allocations from coherent stored configuration. `core.port-allocation` generates
missing VPN backend ports in 5000–6000 and commits them through each existing
service settings owner; TCP/UDP listeners of one service share a port. For an
unconfigured first publication, the guided input suggests public TCP/443; TCP/4443
remains a development/prepublication example. Typed site values are explicit consent
to those exact values and are committed with revision-bound config CAS; saved VPN
port changes require separate exact local approval. ACME credentials come from the existing
protected `core.certificates.credentials.*.secretRef`. Native Amnezia in-container
ports are independently observed and preserved. The coordinator then uses CertificateCoordinator and the existing plan/
render/apply path. Missing values in non-TTY mode return `inputRequired` before effects. Initial config is
committed only after conflict-free preflight and exact typed/menu input recheck; `--yes` cannot replace
the separate foreign-remap consent gate. It does not call `publication.switch` or duplicate
HAProxy/Caddy configuration logic.

Only a descriptor-declared external host listener can enter allocation. The current
catalogue has one allocation mapping, for `foreign-amnezia`; VLESS is intentionally
multiplexed through the shared TCP/443 edge and has no separate host-published
listener. Other standalone VPN modules must add their own typed settings/transport
mapping before selection or enablement. The present implementation therefore does
not yet cover future Hysteria/TUIC or other standalone backend modules, and Amnezia's
mapping alone is not evidence that every VPN service has a durable random port.

Preflight checks the full requested graph and all TCP/UDP reservations first.
Outline's observed host TCP/UDP 8443 is outside the allocation range and remains
untouched unless explicitly registered as an HTON service and included in a
separately reviewed compatible remap. Discovery alone does not establish that
association. Foreign remaps still require exact consent, protected snapshot and
recovery receipt. After transient site input preflight and durable candidate
journal, Core commits allowlisted input and allocation settings through per-document
CAS. OR-10 directly authorizes a first random assignment after fresh collision
checks; changing a previously saved port requires exact local approval. Core then
issues or reuses the site certificate via DNS-01 InternetBS, applies the normal
admitted ingress bundle, and verifies local TLS/SNI, HTTP 200 and the exact
`/var/www/index.html` body, with source identity, digest and bytes unchanged across
the probe. Read-only preflight binds either the safe index absence/parent identity
or its content/filesystem identity into `InstallPlan`; mutation rechecks it before
creating a missing placeholder. `RenderInputs` carries the exact bounded `SiteIndex`
bytes and digest into an immutable `site-index.html` bundle artifact. Caddy's fixed
route rewrites site requests to that artifact; no host webroot directory is mounted
and unrelated `/var/www` files are not served. Content and route roll back in the
same bundle. `ready:true` reports `servedState:verified-local` and
`externalDns:unverified`; no public DNS or identity probe is implied. Foreign
remap remains blocked until qualified snapshot/restore support exists.

Install may use authenticated DNS-provider API calls required by explicit ACME
DNS-01 issuance, but it must not run public identity A/IPv4 probes. Public DNS
readiness remains unverified until the owner explicitly invokes
`identity.whoami` (OR-8). `publication.switch` applies the saved declarative edge
through the shared admitted effect path; an optional port asserts the saved value
and does not edit YAML. Read-only validation cannot mint public consent. Complete
fresh host TCP/UDP conflict evidence precedes consent writes, and unknown/foreign
holders block without mutation. Qualified foreign remap remains a distinct owner;
local served readiness does not claim foreign Amnezia activation.

## 6. Independent runtime contract

| Record / interface | Fields / behavior |
|---|---|
| `CandidateBundle` | bundleId, contractVersion, immutable artifact paths/digests, pinned images, exact services/claims/secret versions, native-check evidence, optional staging attestation |
| `AdmissionProof` | expected runtime epoch/accepted ID, actual resource fingerprints, approval envelope, fallback compatibility; locally protected caller/artifact identity |
| `AdmittedBundle` | validated exact artifact handles and fixed action set; cannot be manufactured by frontend JSON |
| `RuntimeState` | schemaVersion, revision, runIntent, runEpoch, accepted/previousCompatible/trial IDs, lease, current fences, known resource receipts |
| `Lease` | leaseId, candidate/fallback, runEpoch, deadline, state; an older epoch cannot restart a later-stopped server |
| `RollbackAccepted` | exact `previousCompatible` bundle ID; runtime resolves the digest, inverse actions and approvals only from the committed lease and protected bundle authority |
| `RestartAccepted` | exact accepted `BundleId`, planned run epoch, nonempty explicit `ServiceId` set; Core resolves omitted CLI selection from enabled optional/data services, Runtime rejects empty selection and stale bundle/epoch |
| manifest.admit | bundleId, proof, current fences + bounded artifact/resource reads -> AdmittedBundle or rejection; required native evidence, optional staging evidence |
| state.read/write | read(instanceId); write(expected revision/epoch, typed transition) -> durable state; no state solely in /run |
| executor.execute | action enum, bundleId?, AdmissionProof?, expectedRunEpoch, OperationContext -> RuntimeResult; no compiler callback |
| actions.apply/restore | AdmittedBundle/action set or compatible fallback, run intent/epoch, context -> resource receipts; fixed native operations only |
| actions.restartAccepted | current admitted accepted bundle, selected service IDs, exact run epoch -> restart receipts; every target is a bundle-declared Owned service under the effect lock |
| guardian.arm | admitted candidate, fallback target, deadline, runEpoch -> durable LeaseId before activation |
| guardian.monitor | LeaseId, state/clock/resource ports -> commit observation, compensation, stopped-with-artifacts or recovery-required |
| guardian.rollbackAccepted | previous-compatible ID + current lease/state -> inverse-action restore and requalified accepted result; exact epoch, certificate and credential fences required |
| probes.verify | exact admitted bundle identity, Budget -> route/auth/resource evidence, not just open sockets |

Accepted-service restart is a Core journaled lifecycle intent. Core validates the
catalogue IDs, installed Owned registry resources, current Running intent, accepted
resource receipts, and absence of trial/maintenance conflict. Runtime independently
admits the current accepted bundle and rejects missing or Associated targets, stale
epoch, maintenance, trial or any lease. Core resolves omitted CLI selection from
the saved enabled optional/data-service registry set and excludes core ingress.
Runtime rejects an empty selection and restarts only the exact admitted service
resource, records its receipt before returning, and checks post-restart Running
status plus exact admitted launch identity. It then runs declared semantic probes.
Unsupported/unknown probes for the selected service set produce Partial with restart
receipts/survivors retained; unrelated accepted members are outside this result.
They never become a synthetic health pass. No arbitrary locator, shell/native
command, or StopAll fallback is accepted.

`stopAll` first persists stopped epoch and prevents new promotions, then reconciles
all associated restart owners and stops members in dependency order. It attempts
all members with individual deadlines. Accepted boot checks stopped intent before
Docker startup and never downloads images. Stop intent outranks a previous lease.

Own certificate delivery validates chain/SAN/key/expiry and changes only affected
consumers. Amnezia native certificate handling is outside this issuer interface.
Revocation fences are monotonic; future strict-family fences stay future scope.
Pre-install rollback is an explicit target, not an invented previous accepted ID.
Explicit previous-compatible rollback consumes only the fallback proof retained by
the committed activation lease, replays that candidate's declared inverse actions,
and re-admits the target before terminal acceptance. An expired certificate or stale
credential fence blocks acceptance; client-supplied digests/actions grant no restore
authority. It requires Running intent and the current epoch; down's stopped intent
fences restart.

## 7. Deferred configurator interface

`ConfigRequest` variants: ReadObjects, Diagnose, PrepareChanges, CommitChanges,
MigrateFormats, ImportLegacy. Target object IDs resolve through a known inventory;
no raw file paths. The tool's own auth/diagnostics context does not load target YAML.

`ParameterChange = {objectId, stablePath, action(set/remove), typedValue?, baseline}`.
`baseline` distinguishes missing, null and actual previous value. A protected draft
stores changes, read dependencies, actor, creation time and redacted preview.
`prepare` does not publish canonical files. `commit` merges independent current
changes, checks touched values/operation preconditions, validates the whole result
and saves all affected documents recoverably. Conflict retains draft and current files.
New effective plan is produced after rebase; old effect plans stay invalidated.

`MigrationStep` declares source/target format versions, named transforms and
preservation destinations. An explicit strategy exists for every rename/reset/removal.
Deprecated records retain original path/version/value/transaction provenance and
are secret-protected. No migration for unchanged format; a required conversion
without a defined strategy blocks supported activation, preserving originals.

Publication coordination is a hard future gate: multi-file atomic visibility is
logical through a manifest/coherent-reader protocol; filesystem operations are not
a universal multi-file transaction. Arbitrary external editors do not honor flock.
Before implementation select an editor-lease/conditional publication protocol and
state its guarantees; no post-hoc hash check can undo an overwritten external write.
See sdd-configurator.md. This limitation cannot be removed by optimistic wording.

## 8. Optional management API and web

The full management API implements these typed operation families over authenticated
transport, reusing Core and configurator. A local Unix transport and future public
HTTP transport have identical domain requests/results. Proposed HTTP path prefix
`/api/v1` and operation names are engineering bindings; token scopes are to be frozen
before implementing that future module. API daemon absence never affects CLI.

| Operation family | Request / result |
|---|---|
| status / modules.describe / config.read / logs | Known IDs/sections, bounded cursor/limit -> redacted metadata/evidence; invalid target remains diagnosable |
| modules.add/enable/disable/remove/purge/update | Typed lifecycle request + requestId -> operation reference/result, possibly needsLocalApproval |
| server.up/down / runtime.startAccepted | Whole-server request or explicit accepted ID -> operation result; run-intent rules identical to CLI |
| config.prepareChanges / commitChanges | Parameter batch/preconditions or draftId -> draft/commit/conflict result; never whole-file replacement |
| config.validate / plan / preparePlan / apply | Object/draft IDs, resulting snapshot or plan ID -> report/immutable plan/job; no altered candidate smuggled into apply |
| devices/credentials | Known identity/profile IDs and grants -> metadata/explicit issuance/revoke result; disclosure policy versioned |
| certs | Configured names + approval -> bounded operations; no arbitrary paths/force defaults |
| operations.get / events.subscribe | OperationId or cursor, Budget -> durable redacted status/events, resync marker after retention gap |

`ApiRequest = {apiVersion, requestId, operation, payload}`; actor derives from OS peer
or authenticated token/session. `ApiResponse = {apiVersion, requestId, operationId?,
status, result?, error?}`. Reused requestId with changed normalized payload fails;
identical retry returns durable metadata/status, never repeats uncertain effects.
Secret-bearing results are explicit separately scoped disclosure and excluded from
ordinary durable response caches; future one-time policy cannot be bypassed by retries.

Web `api.request(operation,payload,preconditions)` returns typed result. A draft
holds parameter changes and baselines, not a replacement YAML upload. `draft.edit`
updates that batch; `preview` asks API to prepare/plan. `PlanReview.apply` submits
only the plan ID. UI routes, account tables and SSE contain metadata; never raw
store/terminal/Docker access. Raw YAML repair, if later offered, needs a dedicated
protected diagnostic/edit workflow, not a bypass around parameter-batch safety.

## 9. Deferred accounts, publication and client assistance

Accounts records: ownerId, login, profile grants, credentialId/name/state, disclosure
state and backend capability evidence. `authorize(actor,resource,operation)` returns
a grant/denial; `rename` changes metadata only. `create(owner,profiles,name,requestId)`
uses independent scoped keys and an ephemeral one-time response; `revoke` updates
native bindings and compatible fallbacks before reporting success. Family selection
is a separate far-future capability and not required by initial accounts.

Publication records: publicationId, owner, exact host/path claim, private target,
access mode and scoped agent binding. `planPublication` checks reserved routes and
ownership; `configure` returns an existing qualified tunnel implementation's config.
No new reverse-tunnel protocol, public management endpoint or implicit NAT authority.

Local profile tester records: already-provisioned profile refs, approved probe target,
Budget and timestamped evidence/limitations. Bash/PowerShell entries only guide
selection. Ranking cannot imply external-client route ownership or auto-installation.
Autonomous agent/path-switch interfaces remain exploratory pending a separate scope
decision; IPv6 and related enforcement are far future.

## 10. Completion and final testing order

A file plan is complete when its map names an owner, phase, methods/arguments/results,
state/effects, callers/dependencies and relevant structured failure behavior. Source
packages also declare composition/dependency injection and capability limits above.
Data/build/docs files instead name consumers and validation obligations. Future
interfaces are not promises that their implementation ships in R1.

Complete all selected implementation waves before running product tests. Then unit
checks exercise public module interfaces and pure transitions; functional checks
exercise completed command/API/runtime workflows. Define further environmental,
load, security and deployment acceptance after those results. Planning may check
links/schema examples/graph consistency; those checks are not product acceptance.

## 11. Narrow composition and setup writers

`host.systemd-installation.install` delivers only the five compiled platform
templates to protected work/systemd and exact system-unit links, with a journaled
template digest and repeatable interrupted completion. Conflicting destination
links/files or staged template drift block without overwrite. Host-support accepts
only the fixed link set and daemon-reload; installation never starts/enables these
units. Runtime owns guardian start, accepted boot and durable shutdown semantics.

`selection.prepare(snapshot, moduleIds, enabled, expectedRootRevision)` returns a
bounded root-selection candidate or conflict; `selection.commit(candidate,
expectedRootRevision, OperationContext)` persists only that selection recoverably.
It is the R1 lifecycle writer, not a general parameter API. An external editor
race causes refusal under the supported coordination policy; no claim of arbitrary
writer safety. Full semantic merging remains deferred to configurator.

`setup.prepare(moduleId, RegistryEntry?, artifactInventory, suppliedInputs)` returns
SetupPlan with fixed steps and explicit missing operator actions. `setup.resume(plan,
OperationContext)` executes approved steps/checkpoints, seeds only missing inputs,
records installed resources, and returns incomplete/complete. A new module stays
disabled. `ModuleUpdate` stays module-owned; setup is not another image-update path.

### Module membership effects

`remove` prepares an effect plan against the exact prospective selection candidate
before committing the disabled membership. The apply path rechecks that candidate's
read set and journals the selection and effect transition. Remove retains config,
data, secrets and recovery evidence. Runtime stopping is limited to the module's
exact owned resources; shared services remain while another consumer exists. A
missing prospective-selection adapter or per-module runtime stop port blocks
resource-bearing removal. `effects.prepare_selection` binds the exact opaque
candidate to ordinary assembly, semantic validation, rendering and effect planning.
The resource-free Owned VLESS feature uses `execute_resource_free_remove`: the
ordinary Remove planner, durable intent, selection CAS and exact effect apply run
before RegistryChange::Remove. Its physical engines stay Core-owned. Associated or
nonempty resource sets remain unsupported until exact stop authority exists.
`StopAll` cannot substitute for that authority.

`purge` is a separate operation after a verified remove. Its plan names typed logical
config/artifact/secret/resource IDs, revisions or before-digests, owning deletion
ports, consumer/reference and recovery checks, and an approval bound to that exact
set. SecretStore deletes secret versions, descriptor-anchored storage deletes exact
files, and Runtime handles native resources. Any active/shared/foreign/recovery
reference refuses purge. Durable intent and per-action receipts allow absence or
remaining state to be reconciled after interruption; callers never pass raw paths
or shell commands. The supported R1 VLESS config binding uses typed `ModulePurge`
intent (module ID, registry revision, exact entry digest, ContentRevision and local
approval digest) plus protected originals. It deletes only `config.vless.v1` through
the fixed config owner after opaque root-TTY approval. Recovery accepts exact-old
or absent bytes, refuses re-add/drift, and reconciles only its exact nested registry
purge child. Shared credentials/images are not module-owned purge targets.

`update` accepts only artifacts/resources owned by the target module and a trusted,
pinned target digest. Its plan binds the installed digest, target, compatibility and
config/data-format evidence, retained prior version, approval and exact actions.
Missing ownership, capability, target or evidence returns unsupported/invalid before
effects. A leaf module cannot update a shared Xray/Unbound engine through its own
update command. `up` never invokes this path or pulls images.

For down, DesiredReadResult may contain errors: lifecycle uses durable registry and
runtime resources, not the broken target. For restore, archived running intent is
never automatically reinstated; restore result is stopped until explicit up.
Down suspends automatic HTON jobs; manual certificate preparation is separate.

## 12. Execution objects, permission profile and stop requests

### CLI deployment boundary

`hton deploy [--source-dir ABSOLUTE_DIR]` is the fixed public deployment command;
the default source is the command's working tree. It dispatches only to
`scripts/deploy`, copies current source files (including uncommitted changes), and
excludes `.git`, `docs/_lang/ru/tasks/`, generated `target/`, `var/`, credentials and mutable
state (including generated `etc/unbound/root.key`).
It computes and verifies a bounded copy inventory, stages a content-addressed source
snapshot under `/opt/hton/deployments`, and atomically switches `/opt/hton/current`.
It owns group `hton` and code permissions; it never builds, downloads, starts
services or changes config/work/log state. It does not require `BUILD-PROVENANCE.json`
or a clean commit; hashes verify copy integrity only, not publisher identity.
`scripts/install` from a source tree outside the active snapshot invokes that same deploy
owner, then re-enters `/opt/hton/bin/hton install` once. `--docker-only` remains an
independent bootstrap path. Stable `/opt/hton` keeps private state outside the
switched source tree. See D103/OR-14 for the active source-copy contract; D99/D100
are retained as superseded method history while their permission/state guarantees
remain.

When invoked without root, guided install elevates the full operation with `sudo`,
while standalone deploy elevates only copy/promotion. Both verify the local source
tree before requesting sudo. Dry-run does not elevate. Deploy copies prebuilt
`libexec` helpers if present; it never compiles them on the target.

### I1 filesystem mechanics binding

`hton-storage` (`fs-support/`) is independent infrastructure shared by Core,
runtime and logging. It owns descriptor-relative, no-symlink bounded reads,
private directory creation, bounded advisory locking and fsync/rename publication.
Domain revisions, journal ordering, intent/epoch transitions and secret grants
remain in their existing owners. No dependency on either domain contract package
is introduced. The directory capability receives an installer/trusted-entry path,
never a path selected by a remote request. File methods accept one validated
component; nested traversal uses separately opened directory capabilities.

`SecureDir.open/open_private/create(path)`, `child/create_child(name)`,
`try_clone()`, `read(name,limit)`, `list(limit)` return typed storage results.
Only explicit `create/create_child/lock` mutate. `lock(Instant deadline)` returns
`LockedDir` with `read/list`, `directory`, `compare_exchange(name,expectedBytes?,
replacement)` and `remove(name,expectedBytes)`. Publication uses protected temporary files
(0640 `root:hton` when the operator group exists, else 0600), fsync, atomic
rename and directory fsync; lock/create operations require euid-owned private
directories. Expected bytes implement cooperation-based
CAS under the same advisory lock, not atomicity against arbitrary external editors.
An error after rename may mean publication is visible but durability is uncertain;
callers must reread/reconcile, never blindly repeat an effect. No generic shell,
arbitrary filesystem API, secret serialization or business transitions live here.

Stateful objects own resources: RegistryStore, JournalStore, RuntimeStateStore,
RuntimeExecutor, RuntimeGuardian, SetupCoordinator and LogSink. Pure planning,
normalization and engine serialization can remain functions over immutable records;
class wrappers without state add no interface value. The map records each file's
structure and injected dependencies. Shared record definitions live in contracts;
consumers do not copy DTOs into every crate or manually redefine them in web forms.

R1 creates an `hton` OS group and adds the invoking sudo user. Release directories
are root-owned with group `hton`, directories mode 0750, ordinary files 0640 and
entry executables 0750. Group members may read/execute everything under `/opt/hton`, not change it; group
membership grants no sudo privilege. OR-46 makes the mutable `config`, `work`,
retained `host-access` and copied `.secrets` trees `root:hton` 0750/0640 too, so
`hton` membership is credential-equivalent for VPN material. Deploy preserves
these state roots; `scripts/install` normalizes their modes and group. CLI mutations still use existing OS/sudo authorization; no
separate R1 login/role system exists. Runtime privilege is scoped to fixed admitted
actions; future web/API processes do not inherit OS administrator authority or mount
secrets. Host root remains outside the code protection guarantee.

`cmd.deploy` accepts an optional canonical local source directory (default: the current command's working tree), inventories the bounded product allowlist, and copies current files including uncommitted edits. It excludes `.git`, `docs/_lang/ru/tasks/`, generated `target/`, credentials, and mutable config/work/log state. It stages a content-addressed snapshot under `/opt/hton/deployments/<tree-digest>` and atomically switches `/opt/hton/current`; prior snapshots are retained. No release archive, provenance manifest or clean commit is required. Copy hashes verify transfer integrity, not publisher identity. Deploy never downloads, builds, starts services or modifies foreign resources. Existing `libexec` helpers are copied if present; missing required helpers remain an install blocker. `scripts/install` deploys the current tree then re-enters the stable entry point. Group setup, permissions, private state preservation, and internal sudo behavior remain as specified above. See OR-13/OR-14 and D102–D103.

Whole-server down can be requested through the future API, including stopping that
API. Persist operation ID/stop intent before asynchronous effect execution by a
runtime worker independent of the requesting API process. The HTTP request may
return accepted before its own listener disappears. Durable final result remains
available to local CLI and after explicit restart; a lost HTTP connection must not
trigger re-execution or be reported as proof of complete shutdown. The short-lived
local execution helper is not the daemon it stops and writes the final result
before exit. Suspend automatic timers/watchers with the associated composition.

`purge` requires the module already removed/stopped and no live/shared consumers;
failed cleanup blocks destructive deletion of originals needed for recovery.
Re-adding an already enabled existing module resumes/observes it and does not
silently disable it; only a newly registered module starts disabled.

Lifecycle execute takes the immutable LifecyclePlan value, not a hidden persisted
ID from a read-only plan. It rechecks inputs, journals intent and explicitly stages
any nested EffectPlan for apply. Public persisted effect IDs arise only from stage/
prepare operations. Dry-run uses transient stderr/in-memory logging and creates no
log files, directories or rotation metadata.

## I1 concrete call-boundary decisions

Core config owns pure schema/module/global validation aggregation through injected
ModuleCheck ports; checks owns bounded observations/identity evidence. Neither
config.validate nor plan.plan starts observations, resolves secret material or
writes journals. Registry read is zero-write; transition prepares journal intent
before CAS publication. Journal receives runtime intent/observations as values.
`Condition.Autostart(resourceId,enabled)` expresses restart-owner postconditions.

Local HTTPS publication readiness requires fresh `certificates.native` evidence
bound to the exact root ContentRevision.digest, with pass, nonfuture timestamp and
five-minute freshness. Public DNS-A identity is external evidence and is never
queried automatically or used as an activation prerequisite; it remains unverified
until explicit `identity.whoami` (OR-8). Certificate evidence must come from its
qualified local owner; its omission never implies certificate success. Plans additionally
bind synthetic state.registry/state.runtime/state.catalogue/plan.targetArtifacts/
plan.priorArtifacts read dependencies; apply reproduces these from their owning
ports, never treats them as user-selected filesystem paths.

Install preflight may call the catalogue's structural resolver to prepare port
candidates before certificate issuance. `InstallPlan.certificate_requirements`
remain explicit, but a missing certificate does not block safe earlier install
phases. The ordinary resolver with genuine fresh local evidence remains mandatory
immediately before ApplyIngress; the structural graph never grants activation
authority (D95).

Runtime has an independent library coordinator, and an I3 installed host entry.
NativeAction verifies exact prior resource identity and action.before, limits each
call, then independently re-observes receipt identity/fingerprint/status/policy and
persists it. Owned recreation can replace native identity only under admitted
launch+after fingerprint and effective bundle proof; associated resources cannot
be recreated. Future native bindings must implement these contracts rather than
inventing a generic Docker command API. The guardian supervisor acknowledges
independent monitoring after durable lease arm and before activation. Stopped
intent/epoch still outrank old trial recovery. D80 changes only the approved
Amnezia REALITY certificate scope, not these lifecycle/admission obligations.

## I2 concrete contract completions

Native render ports receive complete immutable inputs plus scoped resolved secrets
and return protected artifact bytes and manifest claims. Metadata-only NativeArtifact
is not an executable artifact handle. Preview remains in memory; staged publication
requires an existing journal intent. Engine implementations are injected into render;
no UI/module YAML selects executable code. The shared wire package owns canonical
bundle serialization/fingerprints (SHA-256); renderer and independent admission use
the same encoding. This pure serialization helper imports no Core/runtime effects.
Private Unbound is an explicit engine, never relabelled as Xray.

Before staging/approval, effect planning receives trusted native transition bindings:
exact before/after fingerprints and declared inverse actions. EffectPlan has separate
forward actions and compensations; no inverse is silently appended by apply. The
candidate digest binds them all. Missing or underivable native transition evidence
blocks the plan rather than inventing a resource fingerprint from an artifact hash.

The journal owns named Lifecycle, Setup, Selection and Update intents plus named
certificate/identity request-digest intents. It persists durable
OperationResult alongside terminal receipts through finish_operation; callers do
not create separate ad-hoc idempotency caches. Terminal reads are zero-write. Setup
registers an owned incomplete member before acquisition; SetupProgress appends exact
verified resources/versions/checkpoints and completes only after the coordinator
has verified every declared step and resolved missing/operator actions. Existing
versions are changed only by the separate update operation.


### I2 identity versus observed native state

Management ResourceIdentity.fingerprint identifies structural installed launch
metadata; its exact locator carries the native object ID. It does not encode
running/autostart or active service config. Runtime ResourceIdentity.fingerprint
and Action ResourceFingerprint conditions encode mutable native status/config.
StepReceipt.state_fingerprints explicitly carries those observed native hashes,
separately from structural identities. EffectPlan.resource_before binds structural
registry facts; NativeTransitionBinding supplies fresh mutable before/after hashes
and chains them per resource. RuntimeServiceBinding carries both structural hash
and independently observed runtime identity; renderer never equates their hashes.

Verified recreation of an owned resource may change the native locator. Registry
RebindOwned checks prior exact structural identity, same module/service/resource,
verified receipt and fresh exact observation before recording replacement. It
never changes installed image versions or transfers foreign ownership. Apply's
required AcceptedBindingPort reconciles these facts through RegistryStore before
terminal publication; failure remains recovery-required with runtime acceptance
and originals retained. There is no silent discovery/adoption or optional no-op
writer. The installed I3 binding must supply this cross-boundary adapter.


### I2 bootstrap, stopped composition and restore

SecretStore.import owns bounded opaque input publication; current_version returns
metadata only. Rotation compares the exact previous selector and cannot rewind a
newer independently selected version. Immutable accepted bundles retain their
explicit grants. Installers and certificate ports never write SecretRecord directly.

Issuing a UUID for an installed disabled profile persists Pending metadata without
starting a listener. IdentityCoordinator.confirm_accepted requires fresh Running
accepted-bundle proof before activation; exports derive from that actual bundle.

CandidateBundle.retired_services grants only owned Stop/DisableAutostart and their
explicit inverses, outside active rendering/probe/start closure. Restore reserves
maintenance only after durable stopped/restart-disabled observations; each write
checks the reservation epoch. Up/admission/trial refuse that reservation. Release
requires the same operation and exact fence, with no automatic expiry.

Docker logging is disabled in emitted engine services. Production composition must
attach the independent bounded redacting native log collector before start; queue
pressure drops logs without blocking VPN. Caddy's reload endpoint is a private Unix
socket and uses the admitted stable current artifact reference.

### I3-A config.check helper transport

Short-lived libexec/hton-core takes fixed config.check plus bounded argv containing
--config-root PATH, --json, --quiet, --dry-run. The local shell binding supplies
its protected distribution's config root. This internal local transport grants no
remote caller path authority. FsConfigSource/load/normalize own all YAML checks;
entry emits issues and validation scope only, never normalized setting bytes.
Until validation ports are composed, registry/module semantics, secret-version and
native observations are explicit unverified items. Exit 0 covers available syntax/
normalization checks only; status=unverified cannot be used as apply evidence.

### I3-A read-only module inventory

RegistryStore.inspect(directory, readOnlyContext) -> optional RegistrySnapshot
shares registry decode/invariants and creates no journal. None is known absent file;
errors are unavailable/corrupt, never an empty installation. Expected-instance
checks remain mandatory for mutations. Local inventory context derives from euid
and carries no mutation scopes. Lifecycle.list(catalogue, desired?, RegistryInventory)
uses explicit Available/Absent/Unavailable registry states and projects only bounded metadata; desired participation, registry membership/setup/
ownership and native observation remain separate. Unreadable desired settings do
not hide registry facts. Raw registry checkpoint/diagnostic payloads never leave the
CLI metadata projection. modules.list uses fixed config/work roots supplied by the
installed shell binding, no user-selected executable or remote path authority.

### I3 independent runtime status

RuntimeStateStore.inspect(directory) -> Option<RuntimeState>/ErrorCode reuses bounded
state decode/invariants; absent is not an initialized stopped state. Independent
libexec/hton-runtime status accepts fixed --state-root plus JSON/quiet/dry-run, emits
only metadata/historical resource statuses, and labels native observation and bundle
admissibility unverified. No Core/config/registry is required. Missing state returns
an absent result; unreadable/corrupt returns 1, usage 2. The local shell always supplies
its distribution's work/runtime path. Status never creates state, locks or logs.

### I3 bounded native observation transport

hton-host (host-support/) owns Linux subprocess mechanics and typed read-only Docker/
systemd observations, independent of Core and runtime contracts. It is not a generic
shell/host-authority API. A private process module accepts only command descriptions
constructed by fixed driver methods. Absolute /usr/bin/docker and /usr/bin/systemctl,
cleared environment, fixed local Docker socket/config, no stdin, no shell, bounded
concurrency/output/deadline/cancellation and nonblocking pipe drains are mandatory.
Errors contain closed codes, never stderr/native credential/config bytes. Output
budget counts stdout and stderr together; overflow aborts instead of truncated success.
Timeout/cancellation kills the child process group; unreaped children retain bounded
runner slots while a reaper waits, so repeated failures cannot create unlimited threads.

DockerObserver.inspect_container requires a full 64-character container ID and verifies
that the returned Id matches; inspect_image requires an immutable repository digest
and checks RepoDigests. No listing/discovery grants ownership; no pull/start/update
operation is available. Parsed launch/env/mount/label facts are protected records
without Serialize and with redacted Debug; native stderr and unknown JSON fields
never become ordinary logs. SystemdObserver.inspect accepts a literal UnitName,
never patterns/options/paths, calls show with fixed properties and rejects unknown/
missing/duplicate fields or unit mismatch. Observations do not grant action/adoption
or semantic health; domain owners derive fingerprints/authority from these facts.

`DockerObserver.observe_container_network_listeners` accepts one exact immutable
container ID and expected structural fingerprint. It reads only the fixed Linux
`/proc/<docker-init-pid>/net/{tcp,tcp6,udp,udp6}` tables, with the same strict
byte/row caps as host port observation. The Docker init process must be running;
PID start time and network-namespace inode are stable across the read; a second
exact Docker inspect must reproduce the ID, PID, start timestamp and structural
fingerprint. It returns local namespace socket protocol/address/port rows without
PIDs or socket inodes. This proves only that a socket is present in the exact
container network namespace at that point in time; it does not attribute the
socket to Xray or any other process and cannot alone set native-listener
qualification coverage true.

For the foreign-adapter protocol-evidence subset, `DockerObserver.read_amnezia_xray_server_config`
takes one exact immutable `ContainerId`, its expected structural fingerprint, a
Budget and Cancellation, and returns a protected non-serializable file observation
with exact ID, matching before/after structural fingerprints, stable content digest
and bounded bytes accessible only to the trusted parser. It constructs only fixed
`docker container cp <exact-id>:/opt/amnezia/xray/server.json -` argv; no path,
command, shell, name or `--follow-link` is caller supplied. The fixed archive parser
accepts exactly one regular member matching the selected file, rejects links,
path escape, extension records, duplicate members, malformed/truncated archives and
oversized contents. The operation performs inspect → copy → inspect → copy → inspect;
all structural fingerprints and both file digests must match. It refuses a copy
whose selected path intersects an unclassified mount. This point-in-time observation
does not grant qualification or mutation authority. Raw bytes and secret-bearing
JSON remain protected; only borrowed, bounded nonsecret facts can enter Core evidence.

For Core VPN port preflight only, DockerObserver may enumerate a bounded local
inventory of all container IDs and published TCP/UDP host bindings. This is
reservation evidence only: it does not associate, inspect configuration for,
authorize or mutate an unregistered resource. The inventory is complete only when
the container ID set and published-binding digest are stable across two consecutive
bounded reads and every container inspection succeeds; otherwise allocation blocks.
Host socket reservations are also read from fixed Linux `/proc/net` tables under a
strict byte/row cap. The observation result is point-in-time evidence, never a lease;
Docker binding records carry the immutable container ID and structural fingerprint
from the same exact inspect beside each TCP/UDP port. Core can map them to a managed
service only when both match an exact RegistrySnapshot resource locator and identity.
`/proc/net` claims carry no holder identity or fingerprint. The install coordinator
must refresh the point-in-time evidence immediately before effects.
If an unknown procfs socket shares a port with an identified Docker binding, both
records are retained; the unknown claim blocks reuse rather than being attributed to
Docker by coincidence.

Host support may optionally attribute that duplicate procfs row to Docker only with
per-socket proof: a narrow fixed `ss --listening --numeric --processes --tcp --udp
--extended --no-header` record supplies the same protocol, local endpoint, inode,
PID and FD; `/proc/PID/fd/FD` resolves to that inode; PID start time is unchanged
across reads; `/proc/PID/exe` is exactly root-owned, non-writable `/usr/bin/docker-proxy`;
bounded NUL-separated argv matches only Moby's known `-proto`, `-host-ip`,
`-host-port`, `-container-ip`, `-container-port` flags (optional `-use-listen-fd`);
the local endpoint and argv match one exact Docker host binding; and the container
endpoint matches exactly one inspected network IP/port whose structural fingerprint
matches the exact registered resource. Any absent, duplicate,
unsupported or racing fact leaves the procfs claim unknown. `ss` output is a narrow
recognized text grammar, not a general parser: an unavailable tool, changed format or
exceeded output budget disables attribution while preserving complete base port
reservations. Unknown claims still block reuse; there is no readiness guarantee when
proxy attribution is unavailable. The accepted Docker-proxy flag shape follows the
[Moby Linux proxy entrypoint](https://github.com/moby/moby/blob/master/scripts/docker-proxy/main_linux.go);
the `ss` process and inode fields follow the
[iproute2 ss reference](https://github.com/iproute2/iproute2/blob/main/man/man8/ss.8).

### I3 protected Core application composition

core.host owns installed paths/context/port composition, not duplicate lifecycle,
selection, setup, planner or apply rules. Local actor derives euid; mutations require
OS root/sudo and the fixed /opt/hton/config + /opt/hton/work boundary. Common --yes
is ordinary prompt metadata only. Bounded operation argv crosses the short-lived
helper after --; no actor/scopes/foreign approvals can be asserted by payload.
Explicit dry-run opens no mutable dirs, locks, secrets, logs or journals.

Installation begins with private directory capabilities then Installation request-
digest journal intent before seed/config/instance writes. Missing inputs are seeded
only by compare_exchange(None); existing edited config is retained. A protected
installation record gives a stable random instance identity independent of hostname/
root display name. Runtime missing-state DTO uses wire.initial_state as the single
canonical pure construction; only runtime state repository persists transitions.
The registry remains the sole installed resource owner, setup owns resumable steps,
secret material only through SecretStore, selection only through SelectionWriter.

I3 native fingerprint correction: concrete Docker object ID belongs only to the
independently verified identity_digest/locator. Mutable native state fingerprint
binds structural launch + status + restart policy + admitted active artifact digest,
without new object ID, so an exact Recreate after-condition can be derived before
Docker allocates a new ID. hton-host owns canonical hashing shared by Core/runtime;
no caller invents mutable hashes from configuration bytes alone.

Native collector retains only bounded count/severity-class metadata in closed
NativeOutput events; raw messages are discarded, not heuristically redacted or
stored in journald. It must acknowledge attach readiness before start. Systemd
observation fingerprints include actual loaded launch properties and protected unit
fragment bytes; unexpected drop-ins or unavailable launch proof remain unknown.

### Fixed foreign Xray version port

`HostDocker.observeAmneziaXrayVersion(exact ContainerId, structuralFingerprint,
native IPv4/port, expectedExecutableDigest, Budget, Cancellation)` returns bounded
parsed Xray build/version evidence after the native process/socket attribution
checks. Only `/usr/bin/xray version` may run inside that exact container, as
unprivileged 65534:65534, fixed workdir and sanitized non-secret locale/PATH.
No arbitrary exec/input/path is exposed. Native process/executable and Docker
identity are rechecked before/after; unknown output, drift, cancellation and expired
budget fail closed. Raw output remains protected and at most 4096 bytes. Installer
version, native client authentication, external restart/data ownership and remap
snapshot are separate evidence requirements.

`HostForeignSnapshot.capture(exact ContainerId, structuralFingerprint, unique
64-hex SnapshotId, Budget, Cancellation)` is a privileged, consented and journaled
effect: private fixed-store raw inspect custody, pause/commit, immutable image-save
stream <=2 GiB, SHA-256 and fsync, completion manifest. It rejects every mount,
volume declaration, bind source and tmpfs before commit. Repeated completed reads
revalidate protected archive/manifest digests; incomplete captures require explicit
recovery and cannot rerun commit or authorize remap. Returned metadata exposes
no raw launch/config/key bytes. It is snapshot custody, not permission or proof
for recreation, restore, image pruning or external-controller takeover.

Foreign qualification/adoption are read-only with respect to foreign resources:
`ForeignEvidence`/`QualifiedForeign` carry current native Xray version and
`SnapshotCapabilityEvidence` bound to exact structural fingerprint, not a stored
snapshot. Historical installer/client provenance may remain unknown when exact
current native version, executable/runtime layout and protocol compatibility are
proven. `plan_association` retains no invented snapshot artifact. Only separately
consented snapshot capture produces `ForeignSnapshotHandle`; remap planning now
requires that exact protected handle, source fingerprint and artifact identity.
The actual snapshot precedes every foreign recreation/change, preserving OR-12.

## Planned managed access and resolved SNI contract — OR-39/D137

Design extension only; do not add live descriptor fields before typed consumers.
`ResolvedNameInventory` is a pure Core validation result from one ConfigReadSet:
module/object ID, canonical name, purpose, exact route owner, certificate binding,
private fallback and compatibility classification. Render/export/probe consumers
receive it; they cannot invent or hard-code names. Errors: InvalidName,
RouteCollision, CertificateCoverageMissing, FallbackLoop, CompatibilityRequired.

`HostAccess.plan(configured user/access/sshPort, managed ownership, local account
and effective sshd facts)` is read-only and returns a revision-bound access plan.
`HostAccess.reconcile(plan, SecretRef, OperationContext)` requires the planned
`ManageProvisionedHostAccess` privilege and durable journal intent; it returns
ManagedHostAccessReceipt(user, UID, owned paths, public-key digest, secretRef,
state) or AccountConflict, PolicyBlocked, Drift, RecoveryRequired. Private key
bytes never enter the receipt. It owns account/key/sudoers effects only, not Docker.
Pure full-access encoding receives the receipt's configured user and protected key
handle; default output is protected artifact metadata. Explicit interactive reveal
is separate from logging/JSON. Uninstall consumes exact receipts and refuses drift.

The planned `amnezia.provision` binding is distinct from association/lifecycle;
managed state includes nativeOwnership and resource receipts. Install result carries
adminAccess (ready | disabledByOperator | blocked) and secret-free share paths.
No new runtime functionality is claimed by these contracts.

## Planned host response and DNS evidence contracts — D139–D142

Extend `ResolvedNameInventory` with closed roles: Site, Protocol, Legacy,
RealityBrowserDenied, OwnedWildcardRedirect. A separate typed `HostResponsePolicy`
binds configured apex URL/wildcard zone, HTTP denial (403), redirect (308 to fixed
root) and Host/SNI mismatch (421). Core validates coverage, canonical names and
exact-before-wildcard precedence; existing HAProxy and Caddy adapters render one
shared policy. No external target URLs derived from a request. REALITY receives
only the private fallback endpoint of the existing Caddy, never an HTTP response
interception hook. Unknown non-covered SNI closes before any backend. Separate
UDP listener ownership is mandatory; TCP classifiers cannot attest QUIC policy.

Planned `DnsPrivacyEvidence` binds module/profile, client mode/app/OS/version,
clientDNS (tunneled | remoteApplicationOnly | directException | unverified),
serverDNS (authenticatedTlsForward | recursivePlaintext | unverified), bootstrap
exceptions, tunnel-loss/IPv6 evidence and observed time. Config and server render
alone cannot mint client protection evidence. DNS policy owner is sdd-dns-dot;
validation emits UnsupportedClientDns, DnsPlaintextFallback, UpstreamTrustMissing
or DnsCoverageUnverified as applicable. Effects/captures remain explicit, scoped
and bounded; no general background external network probe is added.

## Frozen M1 execution records and input policy — D143–D147, D154, D157

The current methods/args/call edges are in sdd/INSTALLATION-FILE-GRAPH.md; owning
config, managed-amnezia, proxy, dns-dot and client-access-page SDDs define semantics.
These are future typed Python records, not R1 Rust DTOs imported at runtime.

| Record | Required fields / authority |
|---|---|
| ConfigSnapshot | coherent ReadSet, root/module docs, normalized scalar provenance; config owner |
| NormalizedInstallConfig | accepted mode, module selections, ResolvedNameInventory, HostResponsePolicy, DnsPolicy, PageSettings, InputPolicy |
| InputPolicy | Interactive or NoPrompt; quiet/json/non-TTY select NoPrompt; dry-run forbids input writes; approvals never synthesized |
| ManagedAmneziaPlan | config/pin/observation revisions, native ports, private mappings, desired effect list, ownership and maintenance preconditions |
| NativeReceipt | exact ID/image/launch/config/key-file digests, clientSetRevision, created flags, previous accepted receipt |
| ClientCredential | client-only kind, stable credential/profile IDs, name, active status, source revision, bounded typed payload; no SSH/server-admin fields |
| ClientCredentialInventory | accepted generation/native clientSetRevision, entries, per-profile coverage, observed/generated timestamps |
| ClientPagePlan/Receipt | opaque route ID, inventory and source revisions, artifact hash, coverage, expected base host; no URL/private payload in Debug/log/JSON |
| HostAccessReceipt | configured username, UID, home/sudoers/authorized-key owned paths, grant state, key secretRef/digest, created flags |
| DnsPlan/Receipt | typed upstream/auth/bootstrap policy, private endpoint, consumer capabilities, artifact/image/trust refs and coverage |
| InstallResult | per-phase outcomes, site/protocol/adminAccess/page coverage, inputRequired/policyBlocked, accepted generation; no secret URLs/bytes |
| SecretShareReceipt | safe artifact file names, coverage/status and destination classes; never the bearer URL path or credential bytes |

NoPrompt never invokes a callback that reads stdin or an elevating command that
may ask a password. Optional absent inputs retain defaults/disable; mandatory
absence is a typed result. Partial operations return durable receipts and bounded
retry context; an exit code cannot claim an unverified native/client protection.
Pure ClientPage.render accepts only ClientCredentialInventory, never AdminArtifact.
Read-only native observations are supplied explicitly; rendering cannot execute
Docker or fetch DNS. Current native client edits are tolerated only in the declared
client-set scope and cannot erase topology/revocation fences during rollback.

M1 client inventories have explicit producer ports: managed_amnezia.client_inventory
for native REALITY clients and protocols.client_inventory for optional protocols.
Both consume accepted revisions only. access_links.encode discriminates Guest and
Admin request records; only Admin receives a HostAccessReceipt/private SSH handle.
The AmneziaWG2 `amneziawg` export is Guest-only and structurally validated by
`protocols.awg_client_text`: it may contain the expected client `PrivateKey`, but
never the server `ListenPort`, hooks or `SaveConfig` fields.
Coordinator merges client inventories and labels before handing them to client_page.
