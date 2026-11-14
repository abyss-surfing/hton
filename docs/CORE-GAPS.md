# HTON Core system gaps

> Historical analysis snapshot. Current reconciliation: [FINAL-REVIEW](FINAL-REVIEW.md), [architecture](ARCHITECTURE.md), [interfaces](INTERFACES.md) and [orchestrator](IMPLEMENTATION-ORCHESTRATOR.md). Earlier gaps and schedules are not current acceptance criteria.

Snapshot as of 2026-10-31. This audits the project foundation; it is not a list of
new protocols or user features. Mandatory owner requirements are in
[OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md); the roadmap and SDDs mostly describe
a system that was planned but not yet implemented at the time of this snapshot.

## Conclusion

The main missing piece in HTON is a working, verified vertical slice of the system.
The repository contains a detailed target design, but Core, the unified configuration
model, module lifecycle, apply/recovery and data plane are not yet connected into a
working product. Adding protocols does not close this gap.

## What must be completed for a working core

1. **One authoritative configuration contract.** YAML examples, schemas and
   normalization rules exist, but there is no executable Core that reads the config,
   applies migrations, resolves module dependencies and gives CLI/API a consistent
   view. Implement actual schema validation, semantic checks, deterministic
   normalization and revisions. The foundation is described in
   [sdd-config.md](specs/sdd-config.md); implementation was absent at this snapshot.
2. **Complete module lifecycle.** One working contract is needed for the catalogue,
   `add/remove`, `enable/disable`, `purge`, pinned image/update operations and `up` of
   the entire saved composition. It must distinguish protocol, module and shared
   service, preserve data on remove, and never update images during `up`. At this
   snapshot CLI and documents still needed reconciliation with the owner's latest
   clarification.
3. **Shared change planner and bounded execution point.** Configuration, modules,
   listener/port allocation, dependencies and system privileges must resolve into
   one previewable plan. Define explicit preconditions, revision conflicts, local
   approval for privileged effects and a fixed runtime executor interface. These
   were architecture contracts, not implemented code at this snapshot.
4. **Apply, interruption and recovery as one verifiable transaction model.** Provide
   a durable journal, locks/revision checks, immutable generations, bounded
   activation, an independent guardian and compensation for interrupted steps. The
   specification already recognizes this as a compensating workflow rather than an
   atomic transaction; implement and verify every transition, including failed
   rollback. See [runtime safety](specs/sdd-runtime-safety.md).
5. **Ownership of host resources.** The platform must know exactly which packages,
   files, units, ports, firewall rules, Docker resources and data belong to HTON.
   Install/remove/update/restore must change only those resources; a foreign stack
   must never be treated as HTON-owned. Runtime/installer SDDs describe this, but an
   implemented ownership journal and reconciliation were missing at this snapshot.
6. **Platform update and recovery.** Provide a supported upgrade path: Core/schema/
   module-manifest versions, forward migrations, saved-data compatibility checks and
   manual/emergency rollback. Images are pinned and `up` does not update them; a
   separate update operation needs a clear strategy for Core, modules, config and
   rollback.
7. **Unified module-check contract.** Health checks are already planned. The gap is
   not another health check, but ensuring every module has a native syntax check,
   cross-setting checker, readiness probe and verifiable post-apply behavior. Core
   must aggregate errors and explain which checks passed, were skipped or are
   unsupported.
8. **Build and delivery with artifact provenance.** Repeatable installation needs
   supported release artifacts: image versions/digests, signatures/provenance checks,
   SBOM/licenses, Core/schema compatibility and a verified update channel. `powered.md`
   records that upstream code has not yet been accepted; the roadmap includes
   pins/attestations, but a complete release and upgrade chain was missing at this
   snapshot.
9. **Narrow administrative security boundary.** Console commands should run as an
   OS-authorized user, but the precise privilege model was open. Define the
   unprivileged portion, narrow root executor, secret transfer and audit trail;
   API/web must not receive the Docker socket or arbitrary command execution. This
   boundary precedes web administration and must be captured in an implementable
   interface.
10. **Central diagnostic log.** The owner requires detailed HTON logs under
    installer-configured `/var/logs/hton` and a verbosity level in `hton.yaml` for
    rapid repair with AI. Logging was mentioned only in fragments in the current
    architecture; a unified event/correlation model for CLI, modules, apply, guardian
    and health checks, plus debug/trace redaction, was still missing.

## What is already planned and should not be treated as absent

- Health-check/doctor is already present in the CLI plan and SDD.
- OpenWrt client installation and gateway support are planned; device onboarding is
  incremental.
- Journal, guardian, module checkers, API and dependency planner
  are already described in architecture/specifications. Their main problem is
  missing implementation and evidence, not missing design.
- Protocol testing tools, mobile client selection and expanded product modules are
  long-term plans and should not compete with Core in R1.

## Recommended system work order

1. Resolve the public CLI conflict with owner lifecycle requirements and approve one
   minimal release config/catalogue contract.
2. Implement minimal Core: read -> normalize -> validate -> plan -> render, without
   a public API daemon or web UI.
3. Connect one vertical configuration: install -> first accepted generation -> up
   -> health-check -> change -> failure recovery -> reboot from accepted generation.
4. Implement a separate versioned update operation and safe removal, with data
   retention and purge as distinct actions.
5. Then connect only selected R1 data-plane modules and stabilize their API/config/
   checker contracts; add later modules one at a time.

These are agent recommendations. They do not add owner requirements or cancel
existing requirements for R1, IPv4, `/opt/hton`, `/run/hton`, external uplink or
keeping the data plane available when management fails.
