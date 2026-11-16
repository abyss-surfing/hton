# core/src/lifecycle.rs

Stable ID: `core.lifecycle` · Phase: `R1` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Single owner of module and whole-server lifecycle policy

Implementation wave: `I2-B` (planned ownership, not completion).

## Features and responsibility

- core

## Methods, arguments and result

- `plan` — arguments: LifecycleRequest, optional DesiredReadResult, RegistrySnapshot, RuntimeState, ActorContext; result: LifecyclePlan with blockers, affected resources and interruption.
- `execute` — arguments: LifecyclePlan, expectedInputs, OperationContext; result: OperationResult with per-member states and survivors.

## Structures and injected dependencies

LifecycleService/LifecycleRequest/LifecyclePlan; composition policy and transition coordinator

Dependencies: registry, desired reader, planner/apply, setup, selection writer, typed port-allocation commit port, runtime port, clock/log sink.

## Logic

- Separate membership/setup/enabled/runIntent/observed health dimensions.
- add obtains resources and seeds missing config; new module remains disabled; repeat resumes.
- enable/disable apply by default with explicit save-only; stopped server remains stopped.
- remove retains owned data; purge is separate explicit destruction; dependencies and shared ownership are enforced.
- up validates desired inputs, starts missing healthy-enabled closure without pulls/updates and enables autostart.
- down persists stopped intent first, fences trial recovery, disables all restart owners and attempts every registered stop even on failure.
- `services.restart` is a journaled lifecycle request: accept only catalogue service IDs backed by installed Owned registry resources and exact current accepted receipts; require Running intent and reject trial/active lease/maintenance conflicts.
- Omitted selectors resolve by intersecting the saved enabled service closure, current accepted receipts and installed Owned registry; shared services follow enabled consumers, while core-owned ingress is excluded. Bundle ID, planned epoch and wait bound are journaled.
- Invalid desired config blocks ordinary up; explicit accepted start is separate; down does not require valid desired YAML.
- Module update delegates to module-owned trusted adapter and preserves prior image/compatibility evidence.
- Use explicit injected storage/clock/adapter ports and typed results in INTERFACES; do not duplicate another owner's business rule.
- execute consumes the immutable in-memory LifecyclePlan, rechecks inputs, persists intent and stages any nested EffectPlan before shared apply; dry-run stops at plan.

## Calls / reads

- [core/src/registry.rs](registry.rs.md) — `plan` -> `read`.
- [core/src/registry.rs](registry.rs.md) — `execute` -> `transition`.
- [core/src/config.rs](config.rs.md) — `plan` -> `load`.
- [core/src/catalog.rs](catalog.rs.md) — `plan` -> `resolve`.
- [core/src/plan.rs](plan.rs.md) — `plan` -> `plan`.
- [core/src/port_allocation.rs](port_allocation.rs.md) — add/enable planning -> `plan/commit` with coherent service config and fresh reservation evidence; no second allocation rule.
- [core/src/catalog.rs](catalog.rs.md) — add/enable planning -> `bindAllocation` to materialize host ListenerClaims from descriptor mapping and candidate.
- [core/src/apply.rs](apply.rs.md) — `execute` -> `apply`.
- [runtime/src/main.rs](../../runtime/src/main.rs.md) — `execute` -> `execute`.
- [core/src/setup.rs](setup.rs.md) — `plan` -> `prepare`.
- [core/src/setup.rs](setup.rs.md) — `execute` -> `resume`.
- [core/src/selection.rs](selection.rs.md) — `plan` -> `prepare`.
- [core/src/selection.rs](selection.rs.md) — `execute` -> `commit`.
- [logging/src/lib.rs](../../logging/src/lib.rs.md) — `execute` -> `emit`.
- [adapters/foreign](../../adapters/foreign.rs.md) — `plan` -> `observe`.
- [adapters/foreign](../../adapters/foreign.rs.md) — `plan` -> `planAssociation`.
- [core/src/journal.rs](journal.rs.md) — `execute` -> `prepare`.
- [core/src/journal.rs](journal.rs.md) — `execute` -> `stagePlan`.

## Called / read by

- [scripts/feature](../../scripts/feature.md) — `run` -> `plan`.
- [scripts/feature](../../scripts/feature.md) — `run` -> `execute`.
- [scripts/service](../../scripts/service.md) — `run` -> `plan`.
- [scripts/service](../../scripts/service.md) — `run` -> `execute`.
- [scripts/modules](../../scripts/modules.md) — `run` -> `plan`.
- [scripts/modules](../../scripts/modules.md) — `run` -> `execute`.
- [scripts/up](../../scripts/up.md) — `run` -> `plan`.
- [scripts/up](../../scripts/up.md) — `run` -> `execute`.
- [scripts/down](../../scripts/down.md) — `run` -> `plan`.
- [scripts/down](../../scripts/down.md) — `run` -> `execute`.
- [scripts/upgrade](../../scripts/upgrade.md) — `run` -> `execute`.
- [tests/integration/lifecycle.sh](../../tests/integration/lifecycle.sh.md) — `run` -> `plan`.
- [tests/integration/lifecycle.sh](../../tests/integration/lifecycle.sh.md) — `run` -> `execute`.
- [tests/unit/lifecycle-contracts.rs](../../tests/unit/lifecycle-contracts.rs.md) — `verify` -> `plan`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I2 concrete binding

`LifecycleService.plan(request,ctx)` calls a read-only SnapshotPort and
PreparationPort then the pure `plan` rule owner to return an immutable LifecyclePlan.
`PreparedOperation` may carry an optional host-prepared VPN `PortAllocationCandidate`;
pure planning copies it into immutable plan data and never generates entropy or
traverses settings paths. Execute re-prepares against the submitted candidate before
exact plan comparison.
`execute(plan,ctx)` reproduces that exact plan from fresh protected inputs before
typed Lifecycle intent, preventing caller-crafted actions. Enable planning rejects
a prepared target graph with managed VPN bindings when its allocation candidate is
absent. For add/enable, the
durable lifecycle intent is followed by committing its exact candidate through
PortAllocationCommitPort before selection commit or activation. The port owns
separate candidate approval, protected allocation CAS and exact allocation-intent
recovery; it cannot reroll entropy. A failure after allocation persistence keeps
the saved assignment while selection and activation remain uncommitted.
RuntimeLifecyclePort
owns whole-server up/down/accepted startup; it preserves durable stopped intent,
partial member results and exact survivors independently of desired YAML.
EffectApplyPort.stage_and_apply owns immutable staging/read-set recheck/apply;
selection commits only already-prepared exact bytes, then apply rechecks the
prospective read set. For member removal, preparation must accept a private exact
SelectionCandidate and derive its prospective snapshot/graph/effect before commit;
the runtime port must stop only exact module-owned resources. Neither adapter is
currently bound in host composition, so remove remains unwired. ModuleUpdatePort
and PurgePort are domain seams only: production adapters must provide the typed,
owner-scoped effects, exact evidence and recovery receipts specified in
[INTERFACES](../../../../INTERFACES.md#module-membership-effects). No generic shell
or arbitrary Docker action port is permitted.

`PreparedOperation` holds pure setup/effect/update/association/selection inputs.
Plan rejects unavailable/incomplete enabled modules, implicit dependents, retained
cleanup conflicts and unsupported foreign qualification. Add starts new modules
disabled and resumes existing ones. Remove disables/stops/excludes before registry
RemovedRetained, retaining data; failed stop records CleanupRequired. Purge requires
removed typed artifacts, fresh stopped/no shared/recovery claims and approval bound
to exact artifact revisions/digests. Current host composition has no safe bound
remove, purge or update adapter; lifecycle domain planning does not prove they are
executable. No pulls occur on up. Journal.finish_operation durably stores final
OperationResult; unknown/partial outcomes never report server off or success.

## D93 lifecycle port-allocation binding (contract target)

For add/enable, trusted host preparation derives the prospective exact graph and
uses `core.port-allocation.plan_graph` only for its descriptor-declared managed VPN
backend services. The resulting `PortAllocationCandidate` is carried in immutable
`LifecyclePlan`, and `catalog.bind_allocation` produces the exact listener claims
used for conflict/effect planning. Execute supplies the submitted candidate as its
baseline, re-reads config/catalogue/registry/resource/reservation evidence and
validates exact equality/freshness; it returns `planStale` rather than rerolling if
any bound input changed.

After all blockers are cleared, changed settings require the distinct local
`AllocationApproval`; exact saved-value no-op does not. `Intent::Lifecycle` retains
the candidate before the allocation owner journals protected originals and commits
descriptor-scoped CAS. Port persistence precedes selection/apply, so failure later
retains a stable, reusable assignment but never reports activation success. The
allocation journal is correlated by lifecycle operation ID and candidate digest
for exact recovery. Foreign discovery/selection cannot enter this path; foreign
adoption is separately qualified/journalled and remap still needs snapshot and
consent. The optional plan field defaults absent in older serialized records; an
older add/enable without a now-required candidate cannot resume by generating a
replacement port. Current host preparation does not implement candidate
construction or bind a PortAllocationCommitPort into production LifecycleService.
The execute seam now requires the typed port and invokes it for add/enable
candidates; end-to-end readset/registry/host freshness and host authorization
remain unavailable until that composition is supplied.

## I3-A inventory projection

list(catalogue, optionalDesiredSnapshot, RegistryInventory) is a pure
metadata projection of union module IDs. ModuleInventoryEntry carries moduleId,
optional phase/catalogueImplemented, optional membership/setup/ownership,
optional desiredEnabled, optional registered (false for known absence, None when registry is unavailable),
and nativeState=unverified. No secret/resource locators, approvals, checkpoint
payloads or installed native observations are exposed. Registry membership and
configured participation are independent; a registry-only entry remains visible.
The entry caller reports whether missing registry means known absent or unavailable;
projection never turns missing evidence into healthy/running/installed status.

I3 execute_runtime_request provides the narrow config-independent Down/StartAccepted/
RollbackAccepted composition without constructing unrelated setup/update/purge ports. It uses the
same pure plan owner, unfinished-operation policy, journal intent and shared terminal
outcome publisher as LifecycleService.execute. Effects are an injected typed callback;
no additional lifecycle rules or direct state transitions are defined in CLI/host.
Rollback planning accepts only the exact previous-compatible bundle retained in the
committed lease and requires a running current epoch; it never reads desired YAML.

## Resource-free owned feature removal

`execute_resource_free_remove` is a narrow composition for an installed Owned
feature with an empty physical resource set. It uses the ordinary pure Remove
planner, rejects pending transactions and rechecks the exact registry snapshot
before durable lifecycle intent. The caller commits the opaque selection and runs
the exact previously prepared effect; only a successful effect and unchanged empty
resource ownership permit RegistryChange::Remove. Partial application retains
ownership through CleanupRequired. Shared engines are never stopped by this seam.

`validate_resource_free_config_purge` owns the supported config-only purge policy:
removed Owned entry, exact retained artifact set, no physical/image targets,
disabled participation and no shared/recovery references.
`validate_purge_participation` applies the same coherent-disabled/no-trial policy
to exact-absence recovery after the registry removal is already durable.
