# SDD: Core VPN port allocation

Authority: [owner requirements](../OWNER-REQUIREMENTS.md) OR-10 and
[decisions](../DECISIONS.md) D91–D93. The pure planner, descriptor mapping,
reservation evidence path, candidate approval and narrow settings writer are
implemented in source and connected to install preflight/persistence. Build and
product acceptance are deferred until the selected implementation scope is complete.
Foreign remap remains blocked on Amnezia qualification and complete snapshot/restore.

## Responsibility and boundary

`core.port-allocation` owns the single policy for selecting, validating, persisting
and reusing external host ports for distinct managed VPN listener/backend services.
It assigns one distinct number per selected managed VPN service from inclusive
5000–6000. One service that binds TCP and UDP uses the same number on both. The
public HAProxy TCP/443 edge is outside this allocator; logical protocols multiplexed
on that shared edge do not become separate public listeners.

The allocator is not the effect planner, config validator, host probe, foreign
resource discovery tool, or generic config editor. It accepts explicit trusted
inputs and returns a typed candidate. It has no ambient filesystem, network, Docker,
shell, or randomness access. It never associates a discovered resource, authorizes
foreign mutation, or infers a listener from an unregistered container.

## Inputs and persisted mapping

`AllocationRequest` contains the stable IDs of managed VPN services whose external
backend listeners participate in the current install/add/enable operation, the
coherent desired-settings snapshot and registry snapshot, and catalogue-declared
`ServicePortBinding` mappings. Each mapping identifies the service ID, owning module
settings document, exact allowlisted field(s) representing its host allocation, and
bound transports. Examples: Amnezia maps one port to both
`foreign-amnezia.settings.hostPorts.tcp/udp`; a standalone Hysteria or TUIC listener
maps to its declared `settings.listen.port`. These mappings are declared by the
canonical descriptor/schema owner, never supplied as arbitrary paths by a caller.
`catalog.allocationBindingsFor(graph, snapshot)` is the only method that resolves
those descriptor paths to current saved numeric values for `AllocationRequest`; the
installer and lifecycle callers do not traverse module settings themselves.
`catalog.retainedAllocationsFor(selected, registeredServices, snapshot)` supplies
previous allocations from other installed disabled backends to the same uniqueness
set; missing or mismatched TCP/UDP saved fields are blockers.
For a selected first-install module that is not already registered and whose
descriptor settings document is absent, the catalogue reads the descriptor's trusted
defaults in memory and represents its known document ID in `ReadSet.absentObjects`.
An installed/associated/retained service with a missing settings document is a
`moduleIncomplete` blocker; defaults cannot replace its saved state. Absence is valid only when the config
inventory read was coherent and did not list that document; malformed, unreadable or
drifting inventories remain blockers. It does not seed a file during preflight.
The request and candidate carry the exact `RegistrySnapshot.revision` separately
from the config `ReadSet`; candidate digest and approval bind both revisions.
The install/lifecycle coordinator must re-read and compare that registry revision
immediately before committing or applying the candidate; the settings writer only
verifies the config-document CAS read set and cannot claim runtime/registry freshness.

`ModuleDescriptor.portAllocation` is the authoritative binding declaration:
`allocationId`, descriptor-owned `serviceId`, `settingsDocument`, `bindingGroup`, a
typed Host listener template (`namespace`, `exposure`, `addresses`), and an exact
list of `{path, transport}` assignments. Every path is relative to that
module settings document, belongs to its known schema, names a numeric external
host-binding setting, and is unique. All paths in one binding group receive the
same candidate number; the declared transport set must cover every external mapping
the service claims. A descriptor may declare `portAllocation: null` when its VPN
protocol is multiplexed through the shared public edge or it has no separately
reachable backend. Catalogue validation rejects unknown services, paths, transports,
duplicate fields, or settings outside the owning module document. The allocator
consumes this validated typed mapping and contains no module-ID/path switch table.
`settingsDocument` uses the canonical config object ID `module.<moduleId>`, matching
the key in `DesiredReadResult.readSet`; the file component is independently resolved
from the descriptor's trusted `modules/` location.

`catalog.bindAllocation(graph, candidate, snapshot)` is the only operation that
materializes the descriptor template as `ListenerClaim`s. It reads the numeric
setting field(s) through validated paths, verifies they equal the candidate, and
emits one host claim per declared transport with the candidate port, namespace,
exposure and address template. `catalog.resolve` returns typed allocation templates
for selected descriptors and does not invent duplicate static port claims. The
resulting graph is then given to pure `core.plan`. This keeps the descriptor mapping
as the single source for both persistence targets and collision/runtime listener
claims.

The current Amnezia descriptor maps TCP and UDP host mappings to one binding group.
VLESS declares no separate allocation because its protocol route uses the shared
HAProxy/Caddy public edge; it is reached through that shared TCP/443 entry and is
not an independent host-published listener. This is the current architecture's
boundary for OR-10: each distinct managed VPN backend with its own external host
listener receives its own persisted random port; logical protocols multiplexed on
the shared edge do not. The present module descriptors provide an allocation mapping
only for `foreign-amnezia`. Hysteria/TUIC and other standalone listener modules must
add descriptor-owned settings paths, transports and listener claims before they can
be selected or enabled. Until those modules and mappings exist, OR-10 coverage for
future standalone backends is not implemented; do not claim all VPN services have
working durable random ports based on the Amnezia mapping alone.

The desired set includes existing managed VPN allocations, including disabled
installed services, for uniqueness. It excludes public-edge protocol routes that
share HAProxy TCP/443. Foreign services enter the managed set only through explicit
registration and selection. A registry association is evidence of scope, not proof
of compatibility or permission to mutate; foreign compatibility, exact identity,
snapshot and consent remain owned by the foreign adapter/coordinator.

`ReservationEvidence` is a fresh, complete, read-only host TCP/UDP reservation
snapshot supplied by the trusted observation owner. Host support reads fixed Linux
`/proc/net` socket tables and enumerates Docker container IDs plus published
bindings only for collision evidence. Docker inventory completeness requires two
consecutive equal container-ID sets and equal binding digests, with successful
inspection of every listed container under fixed limits; Docker bindings retain
their immutable container ID and structural fingerprint from the same exact inspect.
Any error, churn, truncation or unsupported binding is incomplete evidence. Core maps
an ID to a managed service only when both ID and fingerprint match the exact
RegistrySnapshot resource identity; `/proc/net` socket rows have no holder or
fingerprint and remain unknown. Discovery in this path never associates a resource or grants
configuration/lifecycle/remap authority. A point-in-time snapshot is not a port
lease, so the coordinator refreshes it immediately before effects. Duplicate
observations with one unknown procfs holder are retained as an unknown conflict;
they are never merged into a Docker identity by matching port alone.

`host.observations.reservationEvidenceFor(registry, control, ctx)` accepts an exact
caller-supplied `RegistrySnapshot` and injected `OperationControl`. It performs the
same bounded read-only host observation as the installed-application wrapper but
does not open `HostApplication`, initialize a store, or reread registry state. This
allows first-install virtual preflight to use an empty expected registry before
bootstrap writes. The caller binds `registry.revision` separately into the
allocation candidate and rechecks it before effects. The empty registry conveys no
foreign qualification, association, or remap authority.

An optional strict `ss` attribution seam may identify that duplicate only when one
bounded `ss --listening --numeric --processes --tcp --udp --extended --no-header`
record agrees on protocol, local endpoint and inode; its single PID/FD resolves to
that socket inode under `/proc`; PID start time is stable before/after; the exact
`/proc/PID/exe` is root-owned, non-writable `/usr/bin/docker-proxy`; bounded NUL argv
contains only the recognized Moby proxy flags (`proto`, `host-ip`, `host-port`,
`container-ip`, `container-port`, and optional `use-listen-fd`); and all endpoint
fields match exactly one inspected Docker published binding and exactly one address
on a container whose structural fingerprint matches its exact registered resource.
The local `ss` text parser accepts one narrow grammar and fails
closed on format drift, missing process visibility, duplicate owners or races.
Attribution is optional: if `ss` is missing, unsupported, over budget, or any identity
fact is uncertain, complete Docker+procfs reservations remain available with the
procfs holder unknown. The unknown claim blocks reuse; the installer makes no
readiness promise for such a saved port. The allocator itself does not probe.

`EntropyBytes` are supplied explicitly by an OS cryptographic random source. The
planner is deterministic for identical request, evidence and entropy. It uses
unbiased bounded selection (rejection sampling, not modulo reduction), discards
occupied candidates, and samples without replacement across the candidate service
set. Entropy is neither persisted nor returned in display output.

## Methods and results

| Method | Arguments | Result / effects |
|---|---|---|
| `plan_graph` | trusted catalogue, selected graph, validated snapshot, exact diagnostic-free registry snapshot, fresh reservation evidence, explicit entropy/time/budget | candidate plus graph with materialized host claims; composes selected bindings and retained installed allocations, pure; rejects registry diagnostics |
| `plan` | `AllocationRequest`, `ReservationEvidence`, `EntropyBytes`, `Budget` | `PortAllocationCandidate` or typed blocker; deterministic and side-effect free |
| `host.approvals.authorizeAllocation` | exact `PortAllocationCandidate`, local `OperationContext` | crate-private `AllocationApproval` capability; requires exact local TTY confirmation, persists a protected actor/instance/operation/candidate-scoped grant; `--yes` cannot create it |
| `commit` | exact candidate, expected coherent read set, optional opaque `AllocationApproval`, `PortSettingsWriter`, `OperationContext` | `PortAllocationReceipt` or typed failure; exact no-op returns before approval/journal; first assignment into fields with no saved port is allowed from the exact fresh candidate; changing any saved port verifies capability, then journals candidate and originals before descriptor-allowlisted CAS |
| `recover` | transaction ID and `PortSettingsWriter`/`OperationContext` | completed/recovery-required receipt; reconciles exact candidate, never generates replacement entropy |

`PortAllocationCandidate` binds service-to-port assignments, exact previous and
proposed values, settings-owner mappings, the trusted descriptor revision for each
assignment, selected service set, reservation-evidence digest, config read-set
revisions/known absences, exact registry revision, creation time and expiry. It does not carry raw entropy,
unbounded host details, foreign credentials, or arbitrary config patches. The
canonical `candidateDigest` is computed from the ordered assignments (service,
settings document, binding group, descriptor revision, port, exact field
paths/transports and prior values), retained allocations, coherent config read set,
including exact known absent documents, exact registry revision,
reservation digest and candidate time bounds. `host.approvals.authorizeAllocation` independently scopes its capability
over the complete candidate plus candidateDigest, instance, local actor and operation
ID. It displays exact service/port/path/transport values and requires a TTY challenge;
the resulting capability has private fields, is not deserializable and is accepted
only by the exact candidate/context that was reviewed. Existing saved allocations
that require no change do not acquire a new allocation grant.

The candidate is immutable and is attached to the caller's install/effect plan
before exact review. `Intent::PortAllocation { candidate }` durably stores the exact
candidate before any settings CAS. Protected originals and per-document CAS receipts
are retained; recovery reconciles only that candidate and never generates replacement
entropy. A changed candidate or actor/instance/operation scope needs a fresh grant;
`--yes` cannot manufacture approval of a changed saved allocation or foreign remap.
OR-10 explicitly authorizes the first random port assignment in the configured
range after fresh collision checks; only replacing an existing saved port requires
the additional exact allocation approval. Repeated install reloads the saved
value and cannot reroll it.

For lifecycle add/enable, the caller first persists `Intent::Lifecycle` containing
the candidate, then commits through this owner. The allocation journal uses the same
operation ID and binds the complete candidate digest. `commit` is idempotent for
that pair: a matching unfinished intent is reconciled through its exact transaction;
a matching completed intent returns its recorded receipt; an intent with a different
candidate digest is a conflict. This closes the crash window between settings CAS
and the parent lifecycle phase receipt without rerolling entropy or inferring a new
candidate. The settings writer remains the only owner of allocation-field CAS.

`PortAllocationReceipt` contains an optional transaction ID (absent only for a
true no-write no-op), per-document resulting revisions, service allocation values
and write/recovery state. It contains no raw configuration bytes or secret material.

Recovery may be invoked by a later local-root CLI operation with a fresh operation
ID. The writer first loads the exact protected transaction and candidate, then uses
the transaction's original operation ID only to publish its terminal record. It
does not reuse the old approval capability or accept a caller-supplied replacement;
recovery remains limited to the original protected before/after bytes and exact
descriptor mapping. Install checks this recovery path before starting another
mutating reconcile, and returns after recovering one candidate so the next run
reloads the newly persisted service setting. More than one unfinished allocation
blocks rather than choosing an arbitrary candidate.
When an unfinished allocation is bound by a parent lifecycle intent, install must
not close that child transaction out of order; lifecycle remains its recovery owner.

The narrow writer loads the owning module path only from the trusted descriptor,
requires the candidate read-set state and every `prior_fields` value to match the
current YAML. For an existing document it mutates only the descriptor-declared
leaves and preserves all other decoded values. For a descriptor-known absent
document, it starts from the trusted descriptor's exact module defaults and uses
one CAS create (`expected=None`) to publish the defaults with the allocated port;
no default-only file is seeded before conflict preflight. Both paths refuse
malformed/intermediate scalar paths. If every
saved field already equals the candidate port it returns before approval, journal,
or write. For first assignment, every prior field is absent and the exact candidate
read set/reservation digest remain mandatory. For a changed saved candidate it verifies the opaque host approval, records
`Intent::PortAllocation { candidate }` with protected original document bytes, then
CAS-publishes each changed document. Partial writes remain recoverable from the same
intent.

Recovery reloads only the exact journaled candidate and protected originals, including
whether each original document existed. For each changed document it accepts exact
expected-old, exact expected-absence, or exact-new; expected-old/absence is CAS-written
to the precomputed exact-new bytes and exact-new is durability-confirmed by same-value
CAS. Missing files when an original was present, mismatched documents, or any other
current bytes return recovery-required without overwriting drift. It never reparses
user paths or generates entropy. If the trusted descriptor revision changed since
the candidate, recovery fails closed for operator repair rather than regenerating
defaults from a different catalogue version.

## Reuse, conflicts and errors

- An existing saved allocation in range is reused when its number remains unique
  and its reservation is consistent with that same managed service. Managed VPN
  assignments are numerically unique across service IDs regardless of transport;
  host occupancy is a bind conflict only when the observed reservation overlaps a
  transport declared by that service's binding.
- A missing value is generated only for a selected managed VPN service. A saved
  out-of-range value, duplicate assignment, mismatched TCP/UDP pair, or value
  occupied by another resource is returned with exact current/proposed data for
  review/remediation. The allocator never silently rerolls saved state.
- Occupied generated candidates are discarded. If no number remains for the whole
  requested set, planning blocks before config, package, certificate, container,
  or listener effects.
- If an existing resource is bound to its own saved port, evidence must identify
  that exact service/resource; otherwise it is not treated as a self-reservation.
- Port number uniqueness is enforced across managed VPN service IDs even when their
  transport sets differ. A foreign resource may be changed only through its own
  separately registered, qualified, snapshotted, reviewed and consented remap flow.

Typed failures include `invalidServiceBinding`, `invalidSavedAllocation`,
`duplicateSavedAllocation`, `savedAllocationConflict`, `reservationEvidenceMissing`,
`reservationEvidenceStale`, `entropyUnavailable`, `noFreePort`, `candidateExpired`,
`reviewRequired`, `readConflict`, `notAuthorized`, `storageFailure`, and
`recoveryRequired`. Errors expose only bounded identifiers and exact port values
needed for review; they do not leak config contents or secret material.

## Commit ownership and dependencies

`core.port-allocation` owns allocation policy and the narrow commit of generated
port fields. It depends on:

- the trusted config reader for validated settings plus coherent revisions;
- catalogue descriptors for stable service IDs, settings-owner mappings and
  transport shape;
- a host observation owner for fresh TCP/UDP `ReservationEvidence`;
- an injected OS CSPRNG adapter for explicit entropy bytes;
- `core.journal` for durable allocation intent and recovery receipts;
- a protected `SecureDir` handle for the fixed `modules/` inventory; the writer
  resolves filenames only through trusted catalogue descriptors, locks that
  directory, and uses storage CAS on exact document bytes.

It does not write `core.edge`, root module enablement, arbitrary module settings,
Docker YAML, native container config, registry association, or host/runtime state.
Config CAS preserves unrelated decoded fields and checks every expected document
revision. Partial writes are journaled and recovered against the same candidate; no
reroll or best-effort merge occurs. `core.config` remains read/validate-only.

## Callers and graph

- `InstallationCoordinator.preflight/execute` uses `plan_graph` and `commit` for all
  selected managed VPN backends; it binds the candidate into `InstallPlan` and
  commits only after conflict-free preflight, exact review/consent and journal intent.
- `core.lifecycle` module add/enable uses the same owner before activation, so
  separate commands cannot implement a second port policy. Disable/remove preserve
  saved allocations unless the owning removal contract explicitly deletes that
  module's settings; a retained installed module continues reserving its number.
- `core.plan` consumes an already validated `PortAllocationCandidate`; it remains
  pure and performs no randomness, listener probing or allocation.
- `core.config` validates syntax and descriptor semantics but does not choose or
  persist allocations.

See [shared interfaces](../INTERFACES.md), [installer](sdd-installer.md),
[configuration](sdd-config.md), and [foreign adapters](sdd-foreign-adapters.md).

Current implementation status is tracked in
[`IMPLEMENTATION-STATUS.md`](../IMPLEMENTATION-STATUS.md). Install preflight composes
the planner and mutating install persists the exact candidate before root config;
repeated installs reuse values from each service's settings owner. The only current
allocation-bearing descriptor is foreign Amnezia, whose integration remains
unexecutable until qualification and protected remap recovery are complete. Other
standalone VPN listener descriptors remain future implementation work. No build,
unit or functional acceptance is claimed.

## M1 private versus public bindings

OR-42 supersedes the older Hysteria/TUIC random-public-listener example for the
selected QUIC profile. The allocator's 5000–6000 policy applies to managed private
backend host bindings (including Amnezia), not the public UDP/443 listener. A
multiplexed TCP protocol may have a private loopback backend allocation without a
separate public port. Hysteria public UDP443 is an explicit fixed reservation,
checked alongside TCP443 and all retained/foreign reservations, never randomized.
No second service can claim that UDP endpoint. Existing M0 internal 40000–49999
backend ports are retained until a reviewed allocation migration, not rerolled by
this architectural clarification. Do not reinterpret a private backend port as a
public client export port. This is the M1 owner-scope exception; other future
protocol/publication decisions remain separate.
