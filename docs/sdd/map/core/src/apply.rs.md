# core/src/apply.rs

Stable ID: `core.apply` · Phase: `R1` · Kind: source

I2 coordinator implementation binding; native host drivers and final acceptance remain pending. Purpose: Shared CLI/web apply coordinator

Implementation wave: `I2-C` (coordinator source present; final acceptance pending).

## Features and responsibility

- transactions
- safety

## Methods, arguments and result

- `apply` — arguments: PlanId, expectedInputs, OperationContext; result: OperationResult with receipts and retained state.

## Structures and injected dependencies

ApplyCoordinator; durable operation workflow

Dependencies: plan store, snapshot rechecker, renderer, native checks, runtime port, journal, clock, log sink.

## Logic

- Acquire lock and persist intent.
- Compile candidate and request independent validation/admission.
- Commit desired revision only if still current; never lie about observed readiness.
- Release management preparation lock before waiting for runtime trial; recovery must never depend on this lock.
- Allocate missing secrets/reservations only under apply intent, bind exact versions into the tested bundle; retain valid existing bindings and reject revoked ones.
- CLI invokes directly; controller is an optional transport adapter.
- Required native checks and optional Docker staging are distinct evidence; no universal staging gate.
- Do not restart a server with durable stopped intent while applying participation/cert updates.
- Ordinary config apply replans current desired inputs; staged PlanId lookup is protected and rechecked, never silently recreated from client content.

## Calls / reads

- [core/src/lock.rs](lock.rs.md) — `apply` -> `acquire`.
- [core/src/journal.rs](journal.rs.md) — `apply` -> `prepare`.
- [core/src/render.rs](render.rs.md) — `apply` -> `compile`.
- [validation/run-bundle](../../validation/run-bundle.md) — `apply` -> `testBundle`.
- [runtime/src/main.rs](../../runtime/src/main.rs.md) — `apply` -> `execute`.
- [core/src/journal.rs](journal.rs.md) — `apply` -> `finish`.
- [logging/src/lib.rs](../../logging/src/lib.rs.md) — `apply` -> `emit`.
- [core/src/journal.rs](journal.rs.md) — `apply` -> `readPlan`.

## Called / read by

- [scripts/install](../../scripts/install.md) — `run` -> `apply`.
- [scripts/config](../../scripts/config.md) — `run` -> `apply`.
- [scripts/update](../../scripts/update.md) — `run` -> `apply`.
- [scripts/certs](../../scripts/certs.md) — `run` -> `apply`.
- [scripts/switch](../../scripts/switch.md) — `run` -> `apply`.
- [scripts/doctor](../../scripts/doctor.md) — planned `--fix` only; Core has no bound repair action path yet.
- [core/src/main.rs](main.rs.md) — `run` -> `apply`.
- [core/src/identities.rs](identities.rs.md) — `issue` -> `apply`.
- [controller/src/api.rs](../../controller/src/api.rs.md) — `handle` -> `apply`.
- [accounts/src/issuance.rs](../../accounts/src/issuance.rs.md) — `create` -> `apply`.
- [publication/src/adapter.rs](../../publication/src/adapter.rs.md) — `configure` -> `apply`.
- [scripts/uninstall](../../scripts/uninstall.md) — `run` -> `apply`.
- [core/src/lifecycle.rs](lifecycle.rs.md) — `execute` -> `apply`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


## I2 concrete binding

Typed coordinators own sequencing and validate invariants; trusted injected ports
own bounded native processes/transport and protected logical inventories. Ports
must fail unavailable/unsupported explicitly and never return synthetic success.
Mutation requires OperationContext Mutating, live deadline/cancellation and durable
journal intent before any secret, certificate, archive or runtime effect. Secret
payload wrappers redact Debug and are never cached with ordinary operation status.
Product tests remain deferred until I4; compilation is integration, not acceptance.

ApplyCoordinator::apply(PlanId,ExpectedInputs,ctx) reads only a protected stored plan,
rechecks an independently reconstructed fresh EffectPlan including all five synthetic
read dependencies (state.registry, state.runtime, state.catalogue, plan.targetArtifacts,
plan.priorArtifacts), versions/resources/approvals, and refuses any changed input.
ApplyRecheck owns observation/composition and calls the existing plan::plan; the
coordinator does not copy its synthetic record serialization. Preparation runs under
a bounded Core config lock, after Effects intent; the lock is dropped before trial.
Preparation/qualification ports consume render::compile and publish_qualified, using
pinned native validators always and optional staging by explicit protected policy.
Unfinished intents are reconciled through a typed recovery port before new effects;
uncertain effects are never replayed by an ordinary retry. Durable result metadata
contains status/IDs only. Stopped intent returns saved pending artifacts, never starts.
EffectApplyPort::stage_and_apply accepts an immutable EffectPlan value, refreshes it,
stages a deterministic protected PlanId then invokes the same apply path.


AcceptedBindingPort.reconcile(plan,RuntimeApplyResult,ctx) is required after a final
runtime outcome and before the terminal result. It verifies actual accepted/fallback
native bindings and uses RegistryStore.RebindOwned for proven recreations. It
cannot perform new native effects or rediscover/adopt arbitrary resources. Failed
metadata reconciliation leaves the Effects intent pending/recovery-required.
