# Implementation plan

Current 2026-11-06 planning authority: [M1 implementation plan](plans/INSTALLATION-IMPLEMENTATION.md).
It freezes OR-37–OR-45 and supersedes conflicting older design/default descriptions
below; historical source/build/test observations remain historical evidence. No
new M1 product implementation, build or acceptance is claimed.

Revised 2026-10-31. Planned work, not implementation evidence. Authority:
[owner requirements](OWNER-REQUIREMENTS.md), [architecture](ARCHITECTURE.md),
[area contracts](specs/README.md), [per-file map](sdd/map/README.md).

## R1 deliverables and dependency order

| Gate | Deliverable | Required evidence |
|---|---|---|
| G0 | Root/module config tree, descriptor/registry/lifecycle vocabulary, versioned interfaces | Schema/examples/graph agree; deferred code excluded from R1 execution closure |
| G1 | Core parser/checker/catalogue, registry, plan/render/apply/journal, secrets | Manual YAML supported; zero-write plan; native and cross-module checks; preserved Docker customization; interrupted apply recovery |
| G2 | Module add/resume/enable/disable/remove/purge/update; whole-server up/down | Incomplete add disabled; data retained; partial failure handling; durable off across reboot; no pulls/updates on up |
| G3 | Independent runtime admission/guardian and accepted cold boot | Management death/hang, malformed desired YAML, down vs trial race, missing artifacts, failed compensation; no automatic fallback on invalid manual up |
| G4 | IPv4 HAProxy/Caddy ordinary site, Xray, own DNS-01 certificates | Real selected clients, SNI/path conflict matrix, certificate renew/delivery, site/VPN with API/web absent; domainless install stays explicitly unpublished |
| G5 | Existing Amnezia association/native REALITY passthrough per D80 and consented port integration | Exact installed identity/SNI/target/restart compatibility qualified; resources/policies registered; client behavior pre/post change, undo and whole-server shutdown verified; ordinary site has genuine own-domain certificate |
| G6 | Repeatable installation, logs/rotation with one-week retention, release packaging | Resume partial install; /opt layout/code protection; full disk/log failure containment; recovery by reinstall from the private `.secrets` repository; external builds/provenance |

R1 includes Xray and already-installed Amnezia; G5 is not merely optional catalogue
work. Its exact installed native identity/SNI/target/restart workflow remains a
pre-implementation blocker; the certificate-scope choice is settled by owner D80.
No web UI, API daemon, full configurator/migrator, IPv6 or traffic accounting is
required for R1. Optional Docker candidate staging never becomes a universal apply
gate. Real-client evidence is still required for each shipping protocol.

## Stable backlog reconciliation

Existing IDs remain references, with current scope below replacing older schedules.

| IDs | Current scope |
|---|---|
| T14/T23/T30 | R1 catalogue, root/module schemas, explicit registry, versioned contracts and synchronized file map; full migration deferred |
| T25/T26 | R1 installer and manual YAML validation/safe apply; API merge tests deferred with configurator |
| T27 | R1 independent executor/guardian, run intent, accepted boot, bounded compensation |
| T28 | Optional final-bundle Docker staging tool; shipping-module client qualification is required separately |
| T22/T29 | Per-shipping-module checks, resource evidence, provenance and pinned artifacts |
| T15/T16 | Incremental optional device/OpenWrt/Linux onboarding, existing client support first |
| T17–T20 | Optional compatibility/experimental protocols, no R1 dependency; policy blocks retained |
| T21/T24 | Later optional management API, standalone configurator, Docker web UI and dashboards |
| t02–t13 | Historical work IDs; map by responsibility to G1–G6, never infer old release scope |

## Later web and configuration iteration

Implement full optional management API with tokens and compatible Core operations;
standalone configuration editing/merge/migration/maintenance; separate Docker web
module and personal cabinets. Define all interfaces in R1 so later clients do not
require a second business-logic path. Accounts and single-disclosure credentials
follow sdd-identity; publication/reverse-agent work follows sdd-web-publish.
Future public API is full-function within authorization: operations requiring local
approval return an actionable approval requirement, never silently bypass it.

F01–F09 remain planned accounts/issuance/publication/device UX work. F10 local probes
can begin with the requested bounded PowerShell/Bash profile-selection tool; this
is not approval to own a separately configured VPN client's routes or reconnection.
F11 autonomous route switching requires a separate server/client scope decision.
F12–F14 and IPv6-dependent F15 demonstrations move to far future with IPv6.
Traffic accounting is excluded from both R1 and R2.

## Planned final qualification and readiness

Measure aggregate idle/peak memory, CPU, throughput, handshake latency and long
sessions on representative weak hardware. Target 1–2 GB and about 50 devices is
an orientation, not an unmeasured hard ceiling. Do not build toolchains on that VPS.
Use native validators per candidate and real clients per shipping module. Optional
staging uses isolated resources and exact final artifacts, with explicit test mapping.
A healthy socket does not establish public reachability or device compatibility.

The rows identify product obligations, not interleaved test gates. Their evidence is collected only after complete implementation: unit tests first, functional tests next, further acceptance decided afterward. No schema or review report establishes implementation. Remaining
cross-editor atomicity, permissions, native Amnezia and future upgrade guarantees are
listed in [FINAL-REVIEW](FINAL-REVIEW.md). Restore of config alone never starts services.

## Consolidated implementation sequence: architecture -> map -> code

1. Fix semantic contracts in architecture/area SDD, then schemas/examples and graph.
2. Implement Core pure parsing/checks/plans, registry and resumable setup.
3. Implement exact runtime admission, durable intent, lifecycle and fault recovery.
4. Deliver site/Xray/own certificates, then qualify Amnezia native integration.
5. Complete installer, bounded logging/rotation with one-week retention, packaging and real clients.
6. Implement later API/configurator/web using the existing interfaces, with merge,
   migration and token tests in their own phase.

Per-file methods and Logic are implementation obligations, not existence claims.
Changing a binding requires updating the machine graph and both call directions.

Execution ownership and wave order are in IMPLEMENTATION-ORCHESTRATOR.md. The
owner's later instruction replaces all earlier interleaved testing/acceptance
schedules: complete implementation, then unit, then functional; further acceptance
is planned from results. Runtime validators remain required product behavior.

D81 (2026-11-01) clarifies execution: use larger, coherent packages and parallel
contributors when available; defer intermediate builds until all selected plans are
complete, then proceed with build → U0 → F0. Source/document consistency does not
replace compilation or acceptance; new source changes must not be described as built.
