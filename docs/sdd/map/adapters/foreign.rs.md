# adapters/foreign.rs

Stable ID: `adapter.foreign` · Phase: `R1` · Kind: source

Partial source implementation. Purpose: read-only foreign discovery/qualification,
conservative Amnezia compatibility policy, and scoped remap planning. The exact-ID observer contract is implemented here; the concrete host supplies
native config/process/version and no-mount snapshot-capability observations.
External restart ownership, loop exclusion and complete recreation compatibility
still lack qualified producers, so no candidate is reported qualified.

Implementation wave: `I2-B` (policy source written; concrete evidence binding and
remap executor remain incomplete).

## Features and responsibility

- coexistence

## Methods, arguments and result

- `discoverCandidates` — arguments: bounded Budget, OperationContext; result:
  complete fresh redacted `ForeignCandidateInventory`; exact IDs are display/selection
  only and carry no module or mutation authority.
- `qualifyCandidate` — arguments: one exact immutable `DockerContainerId`, reserved
  SNI set, Budget, OperationContext; result: expiring `ForeignQualificationEvidence`
  bound to that ID and structural fingerprint, or typed unsupported/drift blocker.
- `planRemap` — arguments: qualified resource, protected snapshot, exact host bindings; result: typed
  plan only; consent and execution remain separate.
- `restore` — arguments: snapshot, observed fingerprint; result: restored / recoveryRequired.
- `observe` — arguments: registered resource identity, Budget; result: ForeignResourceEvidence or drift.
- `planAssociation` — arguments: detected evidence, exact approved scope, originalPolicies; result: RegistryChange and lifecycle capability evidence.

## Structures and injected dependencies

`ForeignCandidateSummary`, `ForeignCandidateInventory`, `QualificationCoverage`,
`ForeignEvidence`, `ForeignQualificationEvidence`/`QualifiedForeign`, association and
remap plans; foreign compatibility and drift rules.

Dependencies: injected read-only `ForeignObservePort` for candidate listing, exact-ID
inspection and registered-resource observation; actor-bound approval scope; verified
native version/protocol/restart evidence. Discovery and inspection use explicit
bounded `Budget`/`OperationContext`; no ambient network or Docker client is owned here.

## Logic

- Preserve all supported launch metadata, networks/mounts/policy.
- Candidate inventory must be complete, fresh, bounded, de-duplicated and limited to
  immutable full container IDs, image digests, runtime state, exact published TCP/UDP
  bindings and structural fingerprints; it never guesses module identity.
- Qualification re-inspects the selected ID independently; exact locator and full
  fingerprint must agree. Missing coverage for image, launch, restart owner, host
  bindings, native listener, networks, mounts/data owners or protocol facts blocks.
- Default discovery/qualification port methods return typed unsupported errors.
  Until a concrete host/native observer is bound, no success-shaped candidate or
  compatibility evidence is synthesized.
- Refuse unsupported recreation semantics and external drift.
- Register only explicitly associated resource IDs and original restart policies; own Amnezia config/cert logic remains native.
- D80 accepts native REALITY passthrough with genuine own-domain HTTPS on the ordinary site; qualify exact installed identity/SNI/target/restart compatibility before integration. No custom certificate layer.
- Whole-server up/down includes adopted Amnezia; unassociated components stay untouched.
- Remap/restore plans require approvals; independent runtime performs admitted native effects. Association includes no undeclared mutation.

## Calls / reads



## Called / read by

- [scripts/switch](../scripts/switch.md) — `run` -> `planRemap`.
- [scripts/remap-undo](../scripts/remap-undo.md) — `run` -> `restore`.
- [scripts/uninstall](../scripts/uninstall.md) — `run` -> `restore`.
- [core/src/lifecycle.rs](../core/src/lifecycle.rs.md) — `plan` -> `observe`.
- [core/src/lifecycle.rs](../core/src/lifecycle.rs.md) — `plan` -> `planAssociation`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I2 concrete binding

Binding extension changes to Rust `.rs`; stable adapter.foreign remains.
`ForeignEvidence` carries exact resources, native version/transport/SNI/target,
fresh observation and verified lifecycle/renewal owners, all supplied by a bounded
read-only ForeignObservePort. `qualify` is pure conservative policy: D80 REALITY
requires distinct exact SNI routing without edge/target loop, transparent TLS,
complete restart authority and explicitly qualified native version evidence.
Unknown installed facts never confer readiness. `plan_association` requires exact
resource-scoped local approval and preserved original restart policies, returns
RegistryChange::Adopt only; it mutates nothing. `plan_remap` and `restore` accept
only typed compatible transition evidence and refuse unsupported recreation.
Native config/certificate ownership remains external; no issuer or Docker executor
is introduced. Actual installed Amnezia integration remains gated by evidence.

The adapter contains read-only orchestration. `core.host-foreign-observations`
now binds candidate discovery to a stable, bounded Docker inventory of full IDs,
image digests, runtime state, published TCP/UDP bindings and structural
fingerprints. This display inventory is not qualification evidence. For the exact
selected container, the host binding must still provide image version evidence;
effective launch and restart facts; native listener endpoints; networks, mounts and
data-owner classification; Amnezia protocol/REALITY SNI/serverNames/target facts;
private endpoint and route-loop evidence; and protected snapshot identity/digest.
Every category must be explicit and fresh or qualification returns a typed blocker.
Current producers supply config, process/socket attribution, binary-reported
version and the no-mount snapshot capability subset. Missing external restart
owners, resolved loop exclusion and recreation compatibility still block native
Amnezia integration.

### Planned exact listener remap owner

Extend the policy boundary with pure exact-binding preflight, remap and restore
verification. The Core installation coordinator owns sequencing, local consent,
journal and snapshot lifecycle; the native effect is an injected fixed
`ForeignRemapPort` implemented by `host-support/src/docker.rs`. Its request binds
immutable container/resource IDs, current fingerprint, exact TCP/UDP host binding
set, snapshot digest and approved target. Its receipt binds before/after identities,
ports, restart policy and fingerprints. Restore is inverse-bound and drift-fenced.
No arbitrary Docker args, image substitution, configuration editing, or Amnezia
certificate management is part of this adapter.

The superseded 8443 target is outside the allocated 5000–6000 range; current Outline
TCP/UDP 8443 remains untouched unless explicitly registered as a selected VPN
service and separately qualified, snapshotted and consented for remap. This is not a
current Amnezia target or automatic remap instruction.

### Typed receipt policy implementation

`ForeignRemapPlan`, `ForeignSnapshotHandle` and `ForeignRemapReceipt` now carry
exact resources, before/after fingerprints, TCP/UDP host bindings and protected
snapshot/manifest digests. `validate_remap_receipt` independently checks that
these values agree, that replacement IDs are full immutable IDs, and that the
receipt claims preservation of the exact protected manifest. This is pure
validation only; it does not execute or authorize a Docker replacement. The contradictory legacy native-endpoint `RemapPlan` has been removed; all
planning uses exact host publication bindings and the actual protected snapshot. Each binding includes the native container
port. Validation requires identical protocol/container-port/host-address
multisets before and after, with at least one changed host port; host IP and
native listener port cannot change.

### Read-only qualification and actual snapshot sequencing

`native_version` is exact current native Xray version/runtime compatibility, not
unknown creator-client provenance. `SnapshotCapabilityEvidence` binds implemented
no-mount Docker commit/image-save capability to the current structural fingerprint.
Read-only qualification/adoption never captures foreign bytes and never fabricates
a stored artifact. `ForeignSnapshotHandle` adds explicit artifact identity;
`plan_remap` requires a separately consented actual protected snapshot matching
qualified resources/source fingerprint. Association retains original policies but
no nonexistent snapshot. Actual remap still requires its exact approval and
complete protected custody before any recreation or stop.

### Unified host-publication remap policy

`QualifiedForeign.host_bindings` retains exact observed protocol/native-port/host-IP
publication. `plan_remap(qualified, actualSnapshot, targetBindings, approval, actor,
now, context)` returns `ForeignRemapPlan` only when native ports/host IPs are
unchanged, at least one host port changes, target ports are nonzero, and fresh
module-bound `amnezia.remap-compatible` plus distinct `amnezia.remap-consent`
checks bind the qualified identity, exact targets and actual snapshot. The legacy
private-native-endpoint `RemapPlan` is removed because it did not implement the
owner's host-port-only request. Wildcard host bindings remain valid if preserved.

`validate_remap_receipt` additionally binds prior IDs to exact original Docker
locators and requires one distinct replacement per original. `restore` plans from
that validated receipt, actual fresh replacement identities and separate exact
restore consent; it returns `ForeignRestorePlan` without claiming native effects.

## OR-39/OR-40 planning amendment

Read canonical sdd-config, sdd-installer, sdd-foreign-adapters and INTERFACES:
all SNI are config-owned and resolved once; administrative username comes from
`foreign-amnezia.settings.admin.user`. Managed provisioning is separate from
foreign association, starts on private bindings and delegates account effects to
HostAccess. Exact future file ownership and schema changes remain implementation
prerequisites in TASK-AMNEZIA-PROVISION; this mirror does not change live parsers.
