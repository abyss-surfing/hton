# docs/specs/sdd-lifecycle.md

Stable ID: `doc.docs-specs-sdd-lifecycle-md` · Phase: `design` · Kind: documentation

Planned file binding; no implemented functionality is claimed. Purpose: Canonical planning document docs/specs/sdd-lifecycle.md

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

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Foreign Amnezia membership boundary

Read-only discovery never adds registry membership. Association requires explicit
selection of one immutable resource ID, fresh qualification bound to its structural
fingerprint, and a separate local approval for journalled `RegistryChange::Adopt`.
The associated record carries exact resource identities, qualification digest,
minimal lifecycle scope and original restart owner/policy. Fingerprint drift blocks
adoption. Associated resources participate in whole-server up/down; unrelated
discovered stacks and the separately managed outbound VPN client do not.

Association, allocation and remap grant separate authority. The allocator reuses a
service's saved host-port state on repeated install. A binding change requires a
separate remap approval bound to exact old/new TCP/UDP mappings and a protected
snapshot. Snapshot and recovery must cover writable-layer state, recreation inputs,
mount data ownership and prior runtime/restart state; incomplete coverage blocks
recreation. D80 REALITY passthrough remains conditional on exact SNI/target, route-loop
and restart qualification; no additional certificate manager is allowed. Details are
owned by [sdd-foreign-adapters](sdd-foreign-adapters.md.md).

D93 adds an exact `PortAllocationCandidate` to immutable lifecycle add/enable plans.
Execution revalidates the same candidate against fresh evidence and never rerolls;
changed values require the distinct allocation approval. Lifecycle intent records
the candidate before the allocation owner journals originals and CAS-writes service
settings. Allocation remains separate from foreign adoption and remap. Current host
preparation does not implement this binding; see INTERFACES and lifecycle map.
