# docs/IMPLEMENTATION-STATUS.md

Current M1 planning is governed by [the tracked plan](../../../plans/INSTALLATION-IMPLEMENTATION.md)
and the canonical document. Older wording below is baseline/history where it
conflicts; OR-37–OR-45 and D143–D148 prevail.

Stable ID: `doc.docs-IMPLEMENTATION-STATUS-md` · Phase: `design` · Kind: documentation

Implementation wave: `P0/I3-C`; coordinator owns this factual execution ledger.
Records completed packages, compilation evidence, deferred tests, remaining work
and external blockers. It does not override requirements or replace per-file SDD
contracts, and never treats compilation as acceptance. No executable methods,
state effects or runtime consumers. Depends on actual implementation reports and
build results. Read with OWNER-REQUIREMENTS, INTERFACES and the orchestrator.

Current OR-12 status: the HTTPS site install path and exact-SNI HAProxy renderer
exist in source, but no release is built/deployed or trusted certificate issued;
Amnezia native qualification and remap/restore remain blocked. The D81/I4 build
gate is active. Source contracts and local render are not live readiness evidence.
The current install also fences readiness: local site HTTPS yields `siteReady:true`
only; whole-system `ready:true` remains unavailable until qualified Amnezia route
and native-handshake verification exists. `scripts/deploy` copies the current source
tree without requiring a release manifest, excluding `.git`, `docs/_lang/ru/tasks/*` and
generated/mutable state.
