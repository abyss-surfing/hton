# docs/INTERFACES.md

Current M1 planning is governed by [the tracked plan](../../../plans/INSTALLATION-IMPLEMENTATION.md)
and the canonical document. Older wording below is baseline/history where it
conflicts; OR-37–OR-45 and D143–D148 prevail.

The canonical shared interface defines `hton deploy [--source-dir]`, current
working-tree copy (excluding `.git` and `docs/_lang/ru/tasks/`), group access, and
`scripts/install` handoff through `/opt/hton/bin/hton`; deploy does not build, download
or start services.
Unprivileged install/deploy invoke sudo internally; install elevates the full
guided operation, and deploy elevates only delivery. Dry-run remains unprivileged.
Both inventory the local working tree before invoking sudo; missing Core/runtime
helpers remain explicit and are never built on the target.
The final OR-12 interface goal is an HTTPS site plus byte-preserving qualified
Amnezia REALITY SNI routing, with local TLS and native recovery evidence.

Stable ID: `doc.docs-INTERFACES-md` · Phase: `design` · Kind: documentation

Planned file binding; no implemented functionality is claimed. Purpose: Canonical planning document docs/INTERFACES.md

Implementation wave: `P0/I3-C` (planned ownership, not completion).

## Features and responsibility

- doc

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Own the stated requirements/contract area; reference other owners instead of duplicating semantics.
- Planned interfaces and orchestration are not product implementation or acceptance evidence.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


InstallPlan binds public edge TCP/443 and a PortAllocationCandidate with one unique persisted 5000–6000 port per selected VPN backend; a service using TCP and UDP shares its number. Fresh exact foreign identities/fingerprints and observed native ports remain bound; native ports are preserved. Site and ACME values remain under `core.edge`/`core.certificates`.

`catalog.profileSuggestions(ProfileId)` returns only trusted first-run hints. The
index covers the fixed minimal/home/compat/lab profile IDs and may name only
implemented R1 descriptors. A hint never updates desired selection or grants
installation/association authority.

`InstallSelectionOffer` is a read-only trusted-catalogue projection of supported
module choices, saved optional selection, first-run profile suggestions and the
exact root revision. `cmd.install.collectSelection` may render these choices but
does not own module metadata or write config. A profile applies only as a first-run
suggestion; repeated install preserves saved choices unless `--menu` explicitly
revisits them. `installation.prepareSelection` validates the exact offer and returns
a transient candidate bound to the current root revision or expected absence. The
install plan uses its prospective desired snapshot; execution commits the selection
only with the reviewed install transaction after all blockers clear. Selection does
not associate a foreign resource or grant allocation/remap authority. The allocation
coordinator consumes only the selected, qualified graph; its committed service
settings preserve the same ports across repeated installs.

Foreign integration has separate typed boundaries: `ForeignDiscovery.list` returns
redacted exact candidate IDs only; `ForeignQualification.inspect` checks one chosen
immutable ID and binds supported protocol/launch facts to its current structural
fingerprint; local `AssociationApproval` permits journalled lifecycle adoption only.
An adopted service can enter allocation only after a fresh registry/config read.
The `ForeignRemap` owner and its separate approval/snapshot/recovery remain
independent; discovery, qualification, adoption and allocation do not authorize
container recreation.

The single allocation policy/commit owner is `core.port-allocation`, specified in
[`sdd-port-allocation.md`](specs/sdd-port-allocation.md.md) and mapped to
[`core/src/port_allocation.rs`](../core/src/port_allocation.rs.md). Planning receives
explicit entropy and complete reservation evidence; commit uses descriptor-scoped
CAS after journal intent.

The commit path requires an opaque crate-private `AllocationApproval` from
`host.approvals.authorizeAllocation`. Its exact scope binds the full candidate and
candidate digest (service/port/path/transport, prior values, reservation evidence,
read-set revisions and expiry), instance, actor and operation ID. The journal intent
is `Intent::PortAllocation { candidate }`; recovery reuses that candidate and cannot
reroll. A candidate whose fields already match saved settings returns a no-write
receipt with no transaction ID, approval or journal entry. Changed values are
published by per-document CAS after protected originals are journaled. Recovery
allows only exact old→new or exact-new states; drift is retained for manual repair.
First-install module settings absence is explicit in `ReadSet.absentObjects`; the
catalogue plans from trusted descriptor defaults, while the journaled allocation
writer creates that settings document with the selected port only after full
preflight and consent. `settingsDocument` is canonical object ID `module.<moduleId>`.
Same-operation/same-candidate commit retries reconcile their exact child journal;
a different unfinished allocation digest for that operation conflicts.

Host support obtains reservation evidence by bounded `/proc/net` parsing and a
two-pass stable Docker published-binding inventory. Discovery is collision-only and
never establishes foreign identity, association or mutation permission. Churn or an
unavailable source makes evidence incomplete and blocks planning.
`host.docker.observeContainerNetworkListeners` reads exact TCP LISTEN/UDP bind rows
from a selected container network namespace, bracketed by Docker identity/fingerprint
and stable init PID start-time/namespace identity. It proves namespace-level socket
presence only; process attribution remains required before setting native-listener
qualification coverage.
`host.observations.reservationEvidenceFor` accepts an exact caller-supplied registry
snapshot and `OperationControl` without opening or initializing `HostApplication`,
so first-install virtual preflight uses the same evidence path before bootstrap.
The candidate binds the exact registry revision independently.
If the same port also appears as a procfs socket, retain an unknown claim unless
the optional exact docker-proxy proof succeeds; port coincidence alone never permits
attribution.

Optional `ss` attribution requires exact inode↔PID/FD, stable process start time,
trusted docker-proxy executable and argv, plus one Docker mapping whose structural
fingerprint matches the exact registry resource. Unsupported `ss` output or any
ambiguous fact preserves the full reservation set with the holder unknown.


Lifecycle add/enable carries the exact allocation candidate inside immutable
`LifecyclePlan`; fresh execution evidence must reproduce its config, descriptor,
registry, resource and reservation bindings or fail `planStale`. The lifecycle
intent stores that candidate before settings CAS. Changed allocations need the
separate allocation capability; an exact no-op does not. Port persistence precedes
selection/apply and uses lifecycle operation ID plus candidate digest for exact
cross-journal recovery. Adoption and foreign remap retain their separate gates.

`scripts/install.collectInput` gathers missing non-secret site values and ACME account email only. DNS API bytes are imported from protected input files through the existing SecretStore operation; ordinary install input never carries them. InstallationCoordinator owns site input review/commit. `core.port-allocation` generates missing VPN allocations using OS cryptographic randomness plus fresh TCP/UDP reservation evidence, preserves saved values, and commits through existing per-service settings owners with CAS after journal intent. Public HAProxy TCP/443 remains separate; multiplexed protocols do not get separate public ports. Outline 8443 remains untouched unless explicitly registered and reviewed as a service remap.

Only a descriptor-declared external host listener can enter allocation. The current
catalogue has one allocation mapping, for `foreign-amnezia`; VLESS is multiplexed
through shared TCP/443. Other standalone VPN modules must declare settings,
transports and listener mappings before selection or enablement. Current
implementation does not yet cover future Hysteria/TUIC or other standalone backends.

First install composes a virtual snapshot from trusted defaults, empty registry and
expected-absent root/module objects before `HostApplication` or bootstrap writes.
The coordinator completes graph/reservation/conflict planning and operation-scoped
TTY review, then rechecks absence before effects. Optional VPN modules stay disabled
until explicitly selected; only selected managed services receive allocations.
Interrupted allocations resume their journaled candidate, while repeated completed
installs reuse saved service ports without drawing entropy.
Because persisted approval storage also does not exist yet, first-install TTY review
uses an opaque in-memory operation session bound to the exact candidate and generated
instance; durable journal/approval records begin only after accepted review.

## `install.reconcile --input-preflight` wire contract

This explicit mode is strictly read-only even when the outer operation context is
mutating. Every result carries `operation:"install.reconcile"` and `writes:false`.
`inputRequired` returns `requiredInputs[]` from the fixed enum `siteDomain`,
`edgePort`, `certificateEmail`; the email is non-secret and requested together
with domain/edge when starting first publication. First-publication certificate
configuration uses fixed InternetBS DNS-01 metadata and SecretStore refs
`dns/internetbs/api-key` / `dns/internetbs/api-password`; credential bytes are
imported from protected files through the SecretStore owner. Existing refs are
preserved. `suggestedValues.siteDomain="example.com"` and `edgePort=443` are offered
only when first publication has no saved domain; the domain remains editable and
is not a baked root-config default.
With sufficient values it returns `status:ready` only when conflicts and approvals
are empty and reservations are complete; otherwise it returns `preflight` with an
immutable plan. Initial random allocation is authorized by OR-10; only changing a
saved value requires allocation approval. Normal
no-argument reconcile is separate and cannot fall through from preflight to
seed/setup.

The selection offer may return bounded `status:recoveryPending` before examining
rootless module documents. The shell must invoke normal install recovery before
prompting for a choice. A rootless module settings document is accepted only when
one committed `PortAllocation` history item binds its exact instance/document set
and the settings writer verifies the current values against that candidate.

Mutating reconcile recomputes and rechecks the plan before writes, journals the
exact port candidate before root/installation metadata, and commits via the narrow
settings writer. It configures the fixed certificate SecretRefs, imports no secret
bytes through install argv, issues/reuses DNS-01, and applies through ordinary
effects. The install plan binds `/var/www/index.html` absence/parent identity or
content/filesystem identity; mutating reconcile rechecks it before creating a
missing placeholder. Caddy receives an isolated HTON-owned snapshot containing
only that file. `ready:true` requires local TLS/SNI, HTTP 200 and exact staged
body, with source/staged identities and digests unchanged around the probe. It
reports `externalDns:unverified` and does not claim public DNS or Amnezia
readiness. Unqualified foreign remap remains blocked.

## OR-39/OR-40 planning amendment

Read canonical sdd-config, sdd-installer, sdd-foreign-adapters and INTERFACES:
all SNI are config-owned and resolved once; administrative username comes from
`foreign-amnezia.settings.admin.user`. Managed provisioning is separate from
foreign association, starts on private bindings and delegates account effects to
HostAccess. Exact future file ownership and schema changes remain implementation
prerequisites in TASK-AMNEZIA-PROVISION; this mirror does not change live parsers.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
