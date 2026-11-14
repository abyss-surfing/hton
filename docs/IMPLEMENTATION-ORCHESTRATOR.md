# HTON Multi-Agent Implementation Orchestrator

Current phase is planning only (2026-11-06). The next selected run is M1 minimum
installation under [the frozen plan](plans/INSTALLATION-IMPLEMENTATION.md) and
[exact file/method graph](sdd/INSTALLATION-FILE-GRAPH.md). Older full-R1 waves below
remain a future independent run, not the next scope or permission to launch agents.
OR-37/40/41 and current AGENTS.md supersede the historical D79 implementation phase.
No implementation agents have been launched by this preparation.

## 1. Goal, Scope, and Authority

After the owner explicitly moves the project into implementation, produce a
coherent product for the selected release in one coordinated pass across files,
then perform final testing. The baseline scope is R1: CLI/Core, module lifecycle,
independent runtime, IPv4/site/Xray, qualified Amnezia, installation, logs, and
recovery. R1 does not absorb deferred API/configurator/web/accounts, IPv6, or client
uplink. The entire catalogue of 31 items does not have to be implemented for R1.

First implement the full selected scope with production decomposition. Then run
unit tests, followed by functional tests. Decide on further checks based on those
results. Scenario plans exist in advance; there are no early product-test gates.
User configuration validation and runtime checks are required product behavior,
not permission to perform early acceptance.

The requirement to reject invalid runtime candidates remains part of the product
regardless of the late development-testing sequence. The historical D79 implementation permission is superseded for this planning pass;
assign agents only after an explicit new phase change and selected run authorization.

## 2. Roles and Maximum Parallelism

With four available slots, use one coordinator and up to three implementers at a
time. With fewer slots, the coordinator reduces the number of implementers while
preserving dependencies.

- The coordinator owns scope, interface registry, schedule, conflicts, dependencies,
  and integration. The coordinator does not rewrite files owned by an active implementer.
- Each implementer receives one bounded package with exclusive file ownership.
  They use the frozen interfaces; proposed changes to another package's contract go
  to the coordinator rather than being silently introduced by changing callers.
- During the final phase, unit and functional implementers receive separate
  scenarios/files; they do not expand release scope just to obtain a passing result.

A shared tree or separate worktrees are both allowed. A shared directory has a
strict one-writer-per-file rule. Git commits, publication, external messages, and
deployment are not implied by this workflow. Secrets and production VPS access are
not shared sandbox resources.

## 3. Assignment Package

Each assignment contains:

```text
Package: <wave/package>; scope: <release + selected modules>
Goal: <observable module responsibility>
Owned paths: <exact file-map list, including new files>
Contracts: <area SDD + INTERFACES sections + stable IDs>
Inputs: <completed predecessor packages and contract versions>
Outputs: <specific files/structures/methods and error behavior>
Allowed effects: <workspace only; no deployment>
Boundary: <responsibilities owned by other packages>
Completion: <complete contract implementation with no stubs>
Checks: <no product tests during this phase>
Report: <files, methods, deviations, unresolved blockers, integration notes>
```

Do not assign vague tasks such as “build the backend” or “implement everything in
place.” The package includes all relevant map pages; if the contract is incomplete,
update the Markdown plan first. Interface changes go through the coordinator and
then all consumers.

## 4. Full-R1 waves (deferred; M1 override below)

This is the order for writing the selected product; it is not interleaved with
acceptance.

| Wave | Implementer A | Implementer B | Implementer C | Result for next wave |
|---|---|---|---|---|
| P0, current | Document contracts | — | — | Resolve tractable inconsistencies; record explicit external blockers; code is not yet authorized by the owner |
| I0 | Independent common/runtime wire records | Build/dependency bindings | Read-only map/packages and artifacts | Types/versions/errors and file owners are fixed; future implementations stay outside R1 |
| I1 | config/catalog/check ports/pure plan | journal/locks/secrets/logging/registry | runtime manifest/state/actions/guardian/probes | Pure rules and independent state mechanisms are implemented; interfaces match |
| I2 | engine adapters/render/native templates | lifecycle/setup/adoption/update adapters | apply/certificates/identities/exports/check aggregation | All R1 domain operations are implemented through their single owners |
| I3 | scripts/lib/bootstrap/installer | Compose/systemd/validation tool/build artifacts | README/operational instructions/provenance/final scope reconciliation | All user and system entry points are connected to completed domain logic |
| I4 | Coordinator integrates the full scope; up to three implementers fix only their owned files | — | — | Complete product implementation; no TODO stubs in the promised scope, duplicated business rules, or unresolved internal links |
| U0 | Unit: config/catalog/plan | Unit: registry/lifecycle/journal/secrets | Unit: runtime/adapters/logging | First product-test phase after I4; report results and defects |
| F0, after U0 | Functional CLI/install/modules | Functional runtime/site/Xray/certs | Functional Amnezia/shutdown | Test the built product after the unit phase completes |
| A0, after F0 | Coordinator prepares follow-up acceptance based on results | — | — | Separate decision on load, security, real environment, and deployment; do not claim readiness in advance |

Within I2, resolve package dependencies through I0 interfaces, then integrate them
through the coordinator. If an interface is incomplete, fix it in the contract and
all consumers before the wave ends. An implementer must not replace an unfinished
dependency with a permanent stub. Under D81, intermediate build/link steps are
deferred until all selected plans are complete; product scenarios and acceptance
remain after I4.

## 5. File Ownership by Package

The current exact list is in the rows of the [file map](sdd/map/README.md); the
assignments below define the sole owner of each family. Before an assignment starts,
the coordinator materializes the exact paths; a wildcard does not grant permission
to edit other files.

| Package | Stable IDs / files |
|---|---|
| I0-A | contract.*, runtime-contract.* — shared types and a separate independent runtime wire package |
| I0-B | build.workspace, build.*-manifest, build.rust-lock; future packages are excluded from the R1 execution closure |
| I1-A | core.config, core.catalog, core.plan; schema.v1/schema.module; module.index/service.index and selected module.* |
| I1-B | core.journal, core.lock, core.secrets, infra.logging, core.registry; schema.registry and state.registry structure |
| I1-C | runtime.executor/manifest/state/actions/guardian/probes; no Core/configurator/admin imports |
| I2-A | core.render; adapter.engine-*; template.*; site.mask; image.lock, selected scope only |
| I2-B | core.lifecycle/core.setup/core.selection; adapter.foreign/adapter.resources; setup/update/lifecycle capability implementations |
| I2-C | core.apply/core.certificates/core.identities/core.exports/core.checks |
| I3-A | cli.entry, R1 cmd.* and selected compatibility delegates, shell.*, core.entry/core.library; deferred commands remain honestly unavailable |
| I3-B | compose.data, systemd.data/guard/acme/acme-timer; validation.bundle/compose; build.core/build.caddy; ci.workflow/release |
| I3-C | notices, README, operational docs/examples; update adoption claims only when supported by facts |
| U0 | Final tests/unit/* plus module scenarios; complete all unit results first |
| F0 | test.lifecycle/recovery/protocols/migration and other selected functional scenarios only |

`state.registry` is a generated runtime artifact: the implementer builds its
producer and schema, not real committed state. Document unit bindings do not
require recursively generating documentation about documentation. Compose/control,
systemd/control, admin/web/controller/configurator/accounts/publication/client
IPv6 are outside R1. Future branches receive a separate I0–I4/U0/F0 sequence after
scope selection.

## 6. Conflict Resolution and Integration

1. Before assigning work, the coordinator checks write sets for overlap, confirms
   inputs are ready, and verifies each ID's phase. Do not launch a blocked package
   “just in case.”
2. Freeze interfaces by package version. Describe a proposed change as old/new,
   with the reason, affected consumers, saved-data migration, and Markdown source.
3. The coordinator makes engineering decisions within owner requirements; a
   contradiction with an owner requirement requires the owner's explicit approval
   of a replacement.
4. Return assignment mistakes to the plan; do not create a compatibility layer just
   to preserve an arbitrary first implementation.
5. Package completion means its responsibility is fully implemented, with a
   concrete report and synchronized contracts. It does not mean product acceptance.
6. Start U0 after I4. Fix discovered defects in their owning module; complete the
   unit phase before F0. One pass is an organizational goal, not a ban on correcting
   defects or permission to hide failures.

## 7. Preliminary Blockers and Independent Decisions

- Native Amnezia has been researched from pinned primary sources in
  AMNEZIA-NATIVE-EVIDENCE.md; under D80 the owner accepted REALITY passthrough with
  a real certificate for the ordinary site. Before the concrete I2-B integration,
  establish the exact installed identity/SNI/target/restart semantics. Then qualify
  the selected scenario on a safe stand during final F0. A certificate manager is
  not authorized; do not hide incompatibility or silently exclude Amnezia from R1.
- OS permissions: the proposed binding is a root-owned distribution, separate
  config/work permissions, privileged runtime, and existing OS/sudo CLI
  authorization; a web token does not grant host admin. Fix the exact install
  permission profile before I3.
- Future configurator: atomic merging against an arbitrary non-cooperating editor
  cannot be promised based on flock/hash. Its implementation is blocked until a
  supported editor-coordination contract is selected; this does not block R1.
- Production deployment, load/DPI/real-device testing, and follow-up checks are not
  presumed to have passed. Define A0 after U0/F0.

## 8. Coordinator Final Report

State selected scope, implemented files and interfaces, resolved contradictions,
MD deviations, actual U0/F0 results, remaining defects, and A0 questions. Claim
“production-ready” only for an explicitly defined and completed acceptance scope.
Record actual execution in IMPLEMENTATION-STATUS.md.

Under D81, CI performs only document/syntax reconciliation until plans are complete.
Build jobs remain gated by the completion marker until I4; product-test jobs are
assigned by the coordinator to final U0/F0 rather than every intermediate commit.
The functional job follows the completed unit job. Neither a written workflow nor
a successful build counts as product acceptance.

## OR-41–OR-44 security plan reconciliation

Before assigning implementation, reconcile [code-backed security review](SECURITY-MASKING-REVIEW.md),
TASK-MASKING-REVIEW and TASK-AMNEZIA-PROVISION with each owning SDD/mirror. Existing
owners: minimum protocols/ingress and Rust engine adapters; shared DNS policy belongs
to sdd-dns-dot, root host response policy to sdd-config, routing to sdd-proxy.
Resolve exact new helper file ownership/mirrors, typed config/schema/example changes,
managed qualification shape and Amnezia-vs-HTON mutation authority before launch.
Do not launch agents or implement code under this planning-only request. Final unit
then functional evidence must include DNS scope/leaks, no public 8443, UDP/443
ownership/HTTP3, REALITY 403, wildcard 308 and foreign unknown-SNI rejection.

## Next-run package override: M1

Use packages M1-A–M1-E, then B1/U1/F1/A1 from the tracked implementation plan;
materialize exact paths from the Markdown graph before any assignment. Keep one
writer per file. M1-A freezes shared types first; B/C may then proceed independently;
D consumes their receipts; E integrates all entry/lifecycle/refresh paths. Builds
are B1 after integration; final unit then functional testing follows, with defects
returned to owning packages. Full-R1 Rust/schema descriptor work is not imported
into the active minimum controller. Preparation is complete as a design activity,
not a phase change or permission to deploy/publish.
