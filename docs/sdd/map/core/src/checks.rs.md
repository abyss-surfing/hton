# core/src/checks.rs

Stable ID: `core.checks` · Phase: `R1` · Kind: source

I2 coordinator implementation binding; native host drivers and final acceptance remain pending. Purpose: Normalized bounded health and identity evidence

Implementation wave: `I2-C` (coordinator source present; final acceptance pending).

## Features and responsibility

- diagnostics

## Methods, arguments and result

- `observe` — arguments: sections, probeBudget; result: Evidence.
- `identityEvidence` — arguments: domains, capabilities; result: identity report.
- Pure module/global validation aggregation belongs to config.validate; this owner gathers bounded observations for supplied evidence.

## Structures and injected dependencies

CheckReport/Evidence/ProbeBudget; explicit bounded observation aggregation

Dependencies: bounded host/network observation ports, runtime state reader, clock. Pure checks are invoked by config.validate.

## Logic

- Separate local engine/host state from external reachability.
- Label age/vantage/unknown; no repair side effects.
- Supply age/vantage/input-bound evidence to config.validate; never duplicate module semantic rules.
- Health checking is existing planned functionality, not a missing autonomous self-repair feature.

## Calls / reads

- [runtime/src/state.rs](../../runtime/src/state.rs.md) — `observe` -> `read`.

## Called / read by

- [scripts/diag](../../scripts/diag.md) — `run` -> `observe`.
- [scripts/doctor](../../scripts/doctor.md) — `run` -> `observe`.
- [scripts/whoami](../../scripts/whoami.md) — `run` -> `identityEvidence`.
- Core config receives observations as values; it never initiates probes.
- Planner receives observations as values; it never initiates probes.
- [adapters/ipv6](../../adapters/ipv6.md) — `qualify` -> `observe`.
- [controller/src/api.rs](../../controller/src/api.rs.md) — `handle` -> `observe`.

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

CheckCoordinator.observe(ObservationQuery[],Budget,ctx) validates unique check IDs,
logical targets, vantage, input fingerprints, measured bytes, timestamps/age and
resource identity/freshness. Native observation ports enforce subprocess/network
read bounds; aggregate caps are 4096 observations, 4 MiB response bytes and at most
8 concurrent native observations (current coordinator iterates sequentially).
Unknown/stale/skipped/failed required evidence remains blocking. Observation failure
codes remain structured; no timeout becomes a claim that a provider blocks traffic.
Runtime observation values feed config.validate/planner without initiating hidden
probes there. identity_evidence deduplicates wildcard/base DNS host queries and adds
local host evidence. The only production R1 caller for external DNS evidence is the
explicit `identity.whoami` operation; ordinary `diag.observe` is local-only.
