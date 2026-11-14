# core/src/plan.rs

Stable ID: `core.plan` · Phase: `R1` · Kind: source

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Revision-bound effect planning

Implementation wave: `I1-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- config
- transactions

## Methods, arguments and result

- `plan` — arguments: NormalizedSnapshot, DependencyGraph, RuntimeObservation, RegistrySnapshot, ReadSet, approvals; result: EffectPlan with immutable inputs and scoped actions.

## Structures and injected dependencies

EffectPlan/Action/ReadSet; pure effect calculation

Dependencies: normalized snapshot, dependency graph, supplied observations, approval verifier.

## Logic

- Plan includes hashes, secret versions, approvals, costs and interruption.
- Read-only planning never pulls images or allocates containers.
- Core-disable and stale revision fail before effects.
- Apply batch toggles to one candidate before dependency validation; reject core changes and incomplete dependent removal.
- Plan records effective defaults, catalogue version, reason for every retained service, owned removals and upgrade default changes.
- Disabled settings and secret bindings remain retained; no allocation, rotation, revocation or purge is implied by a toggle.
- Bind coherent root/module/Docker read set, registry revision, secret/image/catalogue versions and durable run intent.
- A saved effect plan invalidates when its inputs change; semantic rebase belongs to configurator before replanning.

## Calls / reads

- No observation call: immutable RuntimeObservation is supplied by the trusted caller.
- Consumes a precomputed `PortAllocationCandidate` from [core/src/port_allocation.rs](port_allocation.rs.md) when the install/add/enable caller plans VPN backend effects; never generates entropy, allocates ports or probes listeners.

## Called / read by

- [scripts/install](../../scripts/install.md) — `run` -> `plan`.
- [scripts/config](../../scripts/config.md) — `run` -> `plan`.
- [scripts/update](../../scripts/update.md) — `run` -> `plan`.
- [scripts/switch](../../scripts/switch.md) — `run` -> `plan`.
- [scripts/doctor](../../scripts/doctor.md) — planned `--fix` only; Core has no bound repair planner yet.
- [core/src/main.rs](main.rs.md) — `run` -> `plan`.
- [core/src/certificates.rs](certificates.rs.md) — `issue` -> `plan`.
- [core/src/backup.rs](backup.rs.md) — `backup` -> `plan`.
- [adapters/foreign](../../adapters/foreign.rs.md) — `planRemap` -> `plan`.
- [adapters/ipv6](../../adapters/ipv6.md) — `qualify` -> `plan`.
- [controller/src/api.rs](../../controller/src/api.rs.md) — `handle` -> `plan`.
- [publication/src/model.rs](../../publication/src/model.rs.md) — `planPublication` -> `plan`.
- [scripts/uninstall](../../scripts/uninstall.md) — `run` -> `plan`.
- [tests/unit/config-contracts.rs](../../tests/unit/config-contracts.rs.md) — `verify` -> `plan`.
- [core/src/lifecycle.rs](lifecycle.rs.md) — `plan` -> `plan`.
- [configurator/src/changes.rs](../../configurator/src/changes.rs.md) — `commit` -> `plan`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I1 concrete implementation binding (2026-10-31)

`plan(snapshot, graph, observations, registry, readSet, approvals, PlanInputs)` is
pure. Additional trusted inputs bind now/expiry, durable intent/epoch/revision,
pinned target images, prior and target native artifact hashes and fallback target.
A synthetic read dependency binds the registry/runtime/target artifact records;
these identities participate in the same immutable plan hash. No persisted PlanId
is minted by preview. Only fixed owned-resource reload/recreate/start/stop actions
are emitted; associated foreign changes require dedicated lifecycle approval and
are never inferred from editable YAML. Stopped run intent produces no traffic
start/reload action. Active unknown/stale resource observations or ungranted public
listeners/privileges block mutation planning. Missing pinned images or exact target
artifact identity is an explicit failure, never a guessed download/latest tag.

Last-consumer owned-service deactivation emits DisableAutostart before Stop;
already stopped resources still need verified inactive autostart. Dependent service
stops precede dependency deactivation. Postconditions bind both stopped state and
autostart=false. Per-step compensationRef is absent when recovery uses the explicit
whole-plan accepted/pre-install fallback; there are no dangling synthetic compensation
actions. Prior policies remain bound in registry originals and fallback admission;
a later stopped epoch must prevent their restoration from starting traffic.

I2 exact native transition correction supersedes the earlier whole-plan-only
compensation note: `PlanInputs.nativeTransitions` supplies a transition for every
forward action from a trusted native observation/transition planner. Each binding
contains exact before/after fingerprints and a complete inverse `Action`. Before
and after must differ, sequential effects on one resource must chain fingerprints,
and inverse conditions must reverse that exact pair. Inverse IDs/dependencies are
separate from forwards and fully bound in `EffectPlan.compensations`. No apply or
renderer may add/change actions after local scope review. If the native planner
cannot derive exact reversible identities, effect planning refuses the operation.

`prepare_actions` is a pure internal requirements pass over the same planner
calculation. It returns unapproved action requirements only, never EffectPlan or
PlanId. A trusted native transition owner observes/derives exact reversible
before/after bindings for those IDs; `plan` then runs the same single derivation
and validates all bindings/inverses into a reviewable complete EffectPlan. This
prevents callers duplicating action IDs or business rules and avoids requiring
opaque action IDs before the planner has exposed its requirements.


Native transition before/after hashes represent mutable state/config, not the
structural registry fingerprint in resource_before. The first native hash comes
from the trusted bounded native planner; subsequent steps must chain exactly.
Final rendering also compares each first before hash with its supplied observed
runtime binding. Neither an artifact digest nor a structural hash substitutes for
this evidence. Full reviewed plan equality is rechecked before effects.
