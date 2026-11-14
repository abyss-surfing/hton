# Final architecture review and plan reconciliation

Current 2026-11-06 planning authority: [M1 implementation plan](plans/INSTALLATION-IMPLEMENTATION.md).
It freezes OR-37–OR-45 and supersedes conflicting older design/default descriptions
below; historical source/build/test observations remain historical evidence. No
new M1 product implementation, build or acceptance is claimed.

Date: 2026-10-31. Subject: repository plans and scaffold. Product code had not been
written, and product tests and acceptance had not been run at the time of this
review. This is a document audit and architecture clarification, not evidence that
VPN works or that the product is production-ready.

Sources: [owner requirements](OWNER-REQUIREMENTS.md), [decisions](DECISIONS.md),
[architecture](ARCHITECTURE.md), [structures/interfaces](INTERFACES.md),
[file map](sdd/map/README.md), [future orchestrator](IMPLEMENTATION-ORCHESTRATOR.md).
The owner's new development-order instruction is recorded separately from
engineering decisions and applies to all later waves.

## 1. Main result

The architecture is organized around an independent Core for the CLI, an explicit
composition registry, a small independent runtime and a separate future
configurator. API and web are not required in the CLI or VPN/site execution path.
Editable configuration, protected distribution, working state and accepted runtime
snapshots have separate owners. This reduces coupling, duplicated rules and the
number of persistent processes.

Shared structures, methods/arguments/results, errors, passed dependencies, effects
and phase ownership have been detailed. The map contains 223 logical file contracts
and 389 directed edges with caller and callee methods. This is the count of planned
contracts, including documents/artifacts and future capabilities; it is not the
amount of product already written or the number of R1 source files required.

## 2. Resolved contradictions

| Earlier plan issue | Current resolution and owner |
|---|---|
| CLI required a controller | CLI calls Core directly; controller is a separate future optional API; architecture / sdd-control |
| A migrator/editor was required inside Core in R1 | R1 Core reads, validates and applies manual YAML; full editing/merge/migration is a separate future configurator |
| All module settings were only in hton.yaml | Root enabled composition plus separate versioned config/modules/<id>.yaml; module schema/examples aligned |
| User Docker customization was said to be absent | config/docker holds editable preserved inputs; a bundle is the immutable validated result; mounting a live editable file is excluded |
| Secrets, state and code were mixed under var/* | config/secrets and deprecated originals; separate Git-ignored work/registry/runtime/bundles/journal/data; archive storage |
| Catalogue, installed and enabled were treated as one state | Catalogue availability, registry membership/setup, desired enabled state, server run intent and observed health are distinct |
| No add/remove/up/down or resumable setup | Canonical lifecycle: add leaves a new module disabled and preserves incomplete state; enable is separate; remove preserves data; purge is separate |
| up could update/reinstall or restart healthy modules | Start missing members of the saved composition; image updates are a separate module update operation |
| External Amnezia was excluded from whole-server shutdown | After explicit association it participates in up/down while retaining native config/certificate ownership; discovery grants no authority |
| down could be lost after reboot or reversed by guardian rollback | Durable stopped intent/runEpoch precedes effects; stale trials, restart owners and periodic jobs are blocked; an older lease cannot start the server |
| Partial shutdown/start was reported as success | Aggregate result with per-member errors/survivors; continue other operations and preserve healthy components |
| Invalid YAML blocked configurator or shutdown | Tool bootstrap is independent of target; down uses registry/runtime; up with invalid desired state requires separate explicit accepted-start |
| Restore could start services or restore running autostart | Restore checks versions/secrets/modules and leaves stopped intent; up is separate |
| Docker staging was a universal apply gate | Native validation is required; Docker staging is optional; real-client qualification of shipped protocols is a separate obligation |
| Read-only plan created a hidden persisted PlanId | plan returns preview/fingerprint; explicit stage/preparePlan persists a PlanId |
| Any file revision change rejected API edits | Configurator compares affected values and operation dependencies, preserves independent edits, then builds a new runtime plan |
| An old plan could apply after semantic rebase | Every effect plan is bound to exact inputs; merge creates a new snapshot/plan |
| Idempotent response could disclose a secret again | Ordinary durable cache stores metadata/status; secret disclosure uses a separate explicit channel; future one-time policy is not bypassed by retry |
| Logging depended on Core and broke runtime independence | Shared independent logging package with SafeFields/rotation; Core/runtime use it without cyclic imports |
| Verbose logs could fill VPS storage or expose credential URLs | Bounded rotation/redaction is required at every layer; logging failure must not stop a running VPN |
| R1/R2 silently included web, IPv6, accounting and full upgrade | R1 excludes web/API daemon/full configurator; IPv6/family is far future; accounting excluded from R1/R2; upgrade is separately deferred |
| Files belonged to multiple implementers | Orchestrator assigns one writer per file; descriptor is defined in I1-A, adapter implemented in I2-B; graph records implementationWave |
| Test gates alternated with feature implementation | Under the new instruction: implement the full selected product, then unit tests, then functional tests; define further acceptance from results |

The decisions above derive from accepted requirements or are explicitly identified
engineering choices. They do not change owner requirements or claim to quote the
owner.

## 3. Boundaries that a plan edit alone cannot close

| Question | Decision now / condition for closure |
|---|---|
| Native Amnezia identity/SNI/target/restart workflow | Primary sources are researched in AMNEZIA-NATIVE-EVIDENCE.md; owner D80 accepted native REALITY passthrough with a certificate for the ordinary HTTPS site. Exact installed facts and final F0 qualification remain necessary. Do not add a certificate manager or silently exclude Amnezia from R1. |
| Atomicity against arbitrary non-protocol manual editors | flock plus hash-check does not guarantee safety against check/rename races. A future configurator needs explicit editor coordination/conditional publication. Retained draft, coherent reads and local repair session are planned; no strong guarantee against every root editor is claimed. |
| Multi-file persistence | The selected design is a logical manifest/journal/coherent-reader protocol, not one atomic rename for multiple files. Physical format/coordination with external editors must be fixed before configurator implementation. |
| Free domain and selected DNS-01 provider | A name alone does not prove that required DNS automation is available. Installer checks domain/challenge/credentials and provides guidance; without supported conditions it finishes unpublished and does not claim trusted publication. Choose a current example during implementation. |
| Production and external environment | Plans do not prove DPI behavior, capacity, security, real TV/Quest compatibility or port availability. Define further acceptance after unit and functional testing. |

OS permissions received a proposed binding in INTERFACES section 12: root-owned
code, separate config/work permissions, existing OS/sudo authorization for CLI and
a narrow privileged runtime; future web does not inherit host-admin authority.
This is an engineering policy, not a claim that permissions are already configured.
Full upgrade/configurator remains later scope, so its open details are not made R1
dependencies.

## 4. Decomposition and component relationships

- Data and typed operation/config contracts are separate from business logic; the
  runtime wire package builds independently of Core/admin/configurator.
- Core config/catalog/plan and engine serializers own pure transformations. Module
  checks return one CheckReport; shared conflicts are aggregated once, without
  duplicating parsers and feature dictionaries in each CLI/UI.
- RegistryStore, JournalStore, RuntimeStateStore and LogSink own real resources.
  Interface wrappers without behavior are not declared mandatory daemons/classes
  merely to satisfy SOLID.
- Lifecycle coordinates setup/selection/registry/runtime; apply coordinates the
  immutable candidate and independent admission. Setup creates only missing inputs;
  the small R1 selection writer persists enabled state without requiring the full
  configurator.
- Capability-specific adapter interfaces are split into check/observe/render/
  setup/update/lifecycle rather than one enormous required interface.
- API and UI are input adapters. They do not own merge, rendering, secrets, native
  Docker execution or alternative policies/defaults.

Each file contract includes phase, wave, structures, dependencies, methods,
arguments, results, logic and both sides of its edges. Generated/runtime artifacts
have declared producers/consumers rather than invented executable methods.

## 5. Orchestrator and work order

The [Markdown orchestrator](IMPLEMENTATION-ORCHESTRATOR.md) is ready: coordinator
and up to three implementers, exclusive write sets, assignment package, I0–I4 waves,
integration criteria and exact file families. It has not been launched at the time
of this historical review.

P0 ends with documents. After implementation is authorized, complete all selected
I-waves, then U0, then F0; determine A0 from the results. Written-code completeness
differs from test and operational readiness. A one-pass target does not permit
hiding defects or refusing to fix them in their owning module.

## 6. Plan-artifact checks

This phase checks document consistency only: JSON syntax, Draft 2020-12 schemas,
structural validity of examples, uniqueness of file IDs/paths, existence of mirrored
map pages, methods at edge endpoints and reciprocal edges, local links and
whitespace. These are not product unit/functional tests or acceptance. Record the
results of each actual pass below; do not present old historical figures as new
checks.

The document pass completed: 223 unique IDs/paths and 389 edges with reciprocal
projections were consistent; each contract had a map page, structure and wave. The
runtime closure contained 9 contracts and excluded Core/API/web/configurator. Four
schemas were valid Draft 2020-12; both root examples and seven module examples
structurally matched. Shared v1/v2 fields were aligned. Local Markdown links existed
and `git diff --check` passed. These results apply only to planning artifacts; no
product unit/functional tests, Amnezia lab, load test or deployment was performed.

## M1 preparation consistency review — 2026-11-06

Selected scope and source: [M1 plan](plans/INSTALLATION-IMPLEMENTATION.md),
[Markdown file graph](sdd/INSTALLATION-FILE-GRAPH.md), config/managed/page/DNS/proxy
area SDDs. Phase at review time was planning only; the owner later authorized the
OR-49/OR-52 client-page and private-share implementation slice (unit evidence
only). Owner requirements OR-37–OR-45 are traced by
D135–D148; full-R1 execution remains deferred. No agents/code/build/product tests
or live deployment were launched in this review.

Document checks: 29 unique planned file IDs/paths and
42 callable edges have declared endpoint methods and both
mirror projections. New/current local-link targets exist; fenced YAML examples
parse (five target documents), main page path is absent (first-install generation),
three protocol names are separate and public TCP/UDP443/DNS01 invariants agree. Four existing JSON schemas
pass Draft202012 structural checks only; target M1 fields are frozen in Markdown,
not misrepresented as supported by unchanged live schemas. git diff --check passes.

Reconciled: current planning phase; OR-32 default composition; own wildcard and
unknown-SNI ordering; custom REALITY403 in one Caddy; no owned public8443; separate
UDP owner; private DNS versus client coverage; app key files/client edits; retained
remove versus purge; named client-only bearer page and no-prompt quiet install.
Ignored tasks are pointers to the tracked plan, not hidden sources of truth.

Remaining execution evidence: actual image pins/build outputs, host admission,
real app/native clients, per-client DNS/IPv6/tunnel-loss coverage and optional external
A1 observations. No unresolved owner architecture decision is needed to start M1
after explicit phase change. A1/production readiness, universal leak freedom and
resistance to a repeat ban are not claimed. Runtime schema/example/index changes
are assigned exact future owners, not performed early during Markdown planning.
