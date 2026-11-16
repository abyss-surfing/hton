# contracts/src/operations.rs

Stable ID: `contract.operations` · Phase: `R1` · Kind: contract

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Typed operation/context/errors and identity-safe records

Implementation wave: `I0-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Data-only tagged records from INTERFACES sections 2–4; no I/O, framework, Core or runtime logic.
- Distinct validated IDs; explicit partial outcomes and redacted errors; API version separate from format version.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/lib.rs](../../core/src/lib.rs.md) — declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I0 concrete Rust binding (2026-10-31)

`operations` owns private-string validated ID newtypes (1..128 ASCII alphanumeric,
`-`, `_`, `.`; initial alphanumeric), SHA-256/image digest newtypes, distinct
numeric versions, typed errors/outcomes, actor/operation metadata, approvals,
fixed lifecycle/effect/setup/update/registry transition DTOs and capability ports.
Serde deserialization uses the same ID constructors; unknown record fields fail.
Epoch/deadline timestamps use Unix milliseconds. Wire context contains cancellation
and sink IDs; actual cancellation/event capabilities are injected trusted ports.
String fields in errors/evidence are approved redacted prose, never raw subprocess
output; the package has no logger or process authority. Runtime wire types remain
independent; Core converts at the runtime boundary. No test execution in I0.

Digest wire encoding is uniformly `sha256:<64 lowercase hex>` for SHA-256 and
image digest records. IDs serialize as scalar strings; operation/condition/setup
variants use internally tagged camelCase fields. `Budget` is a request record;
trusted operation boundaries must enforce deadlines and validate/clamp numeric
limits against local policy before issuing observations. Its default helper does
not authorize an unbounded caller budget. Free-form safe error/repair prose must
be produced by trusted code, never copied from backend stderr or client settings;
ordinary structured logging accepts only the closed `SafeFields` vocabulary.

I1 condition vocabulary adds `Autostart{resourceId,enabled}` so deactivation plans
can verify restart-policy/boot owner state independently of observed running state.
It is a fixed typed predicate, not a command or a success inferred from stop alone.

I2 exact-effect binding: `EffectPlan.compensations: Vec<Action>` contains explicit
inverse action bodies, separate from forward actions; every forward compensationRef
resolves into this collection. Inverse IDs and exact before/after fingerprints are
reviewed with forwards, never invented during render/apply.
`RegistryChange::SetupProgress` appends proved resources, installed artifact
versions, setup status and optional receipt to an already registered incomplete
module. It does not replace installed images or infer activation from progress.

`LifecycleRequest::RollbackAccepted` carries a validated BundleId only. Core lifecycle
planning requires Running intent and exact equality with `previousCompatible` and
the committed lease fallback; Core cannot supply runtime digests or inverse actions.

`LifecyclePlan` carries the exact optional `PortAllocationCandidate` for managed VPN
backend settings during add/enable. The candidate is data-only and immutable plan
content; it contains neither random entropy nor approval capability. `AllocationApproval`
remains a separate host-issued, non-serializable capability and is never replaced by
generic lifecycle/effect approval or `--yes`. The optional field is additive and
defaults absent when reading older serialized plans. A legacy add/enable whose
current trusted graph requires a candidate is stale/recovery-required; execute may
not silently generate one while resuming that old plan.

`InstallSelectionCandidate` binds the exact selected optional-module set to the
catalogue version, offer digest, previous selection and exact root revision or
expected absence. It grants no association/remap authority. `InstallPlan` binds its
stable `instance_id` (including virtual first-install identity), `ExpectedInputs`,
the exact selection/input candidates, prospective dependency graph,
public edge, mandatory `InstallPlan.host_reservation_evidence` (`ReservationEvidence`)
for complete fresh TCP/UDP host listeners even without a VPN allocation, optional persistent port-allocation
candidate bound to the same reservation digest/time basis, fingerprint-bound
foreign identities with prior/proposed host bindings and preserved native endpoints,
certificate claims, blockers, typed approval requirements, a fixed effect-phase list
and expiry. It contains no approval capability, secret bytes, Docker argv or shell
command. `InstallResult` wraps `OperationResult` with typed phase outcomes,
certificate status, foreign remap transaction references and explicit unverified
external DNS state. These records are transport data only; coordinator policy and
effects remain in `core.host-installation` and their owning modules.

`LifecycleRequest::RestartServices` carries explicit service IDs, exact accepted
BundleId and bounded waitSeconds. Core lifecycle planning binds the saved enabled
service closure, installed Owned registry resources, accepted receipts and run epoch;
the request never carries a native locator or command.


StepReceipt.state_fingerprints maps exact resource IDs to observed mutable native
state/config hashes. ResourceIdentity.fingerprint in receipt.resources stays the
structural launch fingerprint. Resource postconditions check state_fingerprints,
not structural identity. RegistryChange.RebindOwned explicitly journals verified
owned native identity replacement; it does not imply image update or adoption.

`BundleValidationAttestation` is a data-only protected result binding one exact
BundleId/candidate digest, staged artifact and immutable image digests, runner
version, bounded scenario evidence, and cleanup receipt/time. It contains no Docker
IDs, arbitrary paths, commands, raw output or authority; Core must load the persisted
record from its protected owner and revalidate expiry/digests before consumption.

RegistryChange::UninstallOwned carries module ID, exact removed resource identities
and Verified receipt from the internal ownership-bounded uninstall composition.
The registry enforces complete Owned scope and fresh exact identity/digest coverage;
CLI/API input cannot supply this receipt or directly invoke registry changes.
