# Requirements and document authority

- D180 — Independent backends use explicit public endpoint bindings; shared
  identity pools are only for replicas. Native AWG uses distinct UDP IP/port
  pairs. Optional Styx requires paired encapsulation endpoints and does not
  replace official-client access. Trace: OR-72; engineering design in Tartarus
  `tasks/styx-multi-backend.md`. Live failure remains undiagnosed.
- D181 — Evaluate transport camouflage against observable behavior and active
  probing; distinguish protocol detection from IP blocking. Key substitution
  alone is not an obfuscation mechanism. Trace: OR-73. This records criteria,
  not a verified anti-classification result or permission to deploy.
- D182 — Start Styx as an optional D6ton adapter over a bounded established
  transport. Use one public TLS name per Triodos edge, keep native profiles and
  inner VPN credentials intact, and make protocol/backend attribution explicit.
  UDP wrapping, official-client support, user metrics, public-edge integration
  and camouflage claims remain outside the initial slice. Trace: OR-74; task:
  Tartarus `tasks/styx-encapsulation.md`.
- D183 — Unified edge measurements are keyed by authenticated user, protocol
  and backend. The Styx selector exposes the latter two only after a user is
  authenticated and authorized; byte counters and Omma integration are a later
  slice. Trace: OR-75; task: Tartarus `tasks/styx-encapsulation.md`.

Updated 2026-10-31. Tracked summary of owner requirements from this conversation;
local docs/_lang/ru/tasks/00-decisions.md preserves historical context and private details.
This summary is not a claim that suggested implementation details were owner quotes.

| ID | Owner requirement | Design owner |
|---|---|---|
| D24 | Thin dispatcher and Bash-first commands; complex logic can use an appropriate language | ARCHITECTURE.md; file-map logical contracts survive language/path changes |
| D25 | Bootstrap a fresh Ubuntu VPS, including Docker, with scripts/install | sdd-installer.md |
| D21 | Persistent state in host bind mounts; disposable containers | sdd-runtime-safety.md; recreation may interrupt connections |
| D26/D28 | TV/router support; SSH tunnels session-scoped, no autossh | sdd-router.md; sdd-smart-connect.md |
| D29 | Integrated optional toolbox, declarative toggles, self-sufficient CLI and optional later web administration | sdd-config.md; sdd-capabilities.md |
| D30 | Post-release personal accounts; login -> protocols -> named tokens; one-time display, Amnezia defaults | sdd-identity.md |
| D31 | Publish local site through final project domain | sdd-web-publish.md, scheduled after release |
| D32 | Plan device-first onboarding, local diagnostics, controlled failover and IPv6 over IPv4 | sdd-smart-connect.md; sdd-client-ipv6.md |
| D33 | Select IPv4 or IPv6 Internet egress during token creation | sdd-identity.md; sdd-client-ipv6.md, post-release |
| D34 | Evaluate/reuse existing engines, panels, UI and useful code; record provenance | REUSE-ASSESSMENT.md; powered.md; ADMIN-UX.md |
| D35 | Per-file SDD and project-shaped map; stable semantics despite language changes | sdd/map/README.md; sdd/file-index.json |
| D36 | Management failure must preserve VPN and ordinary site; Docker staging is optional; retain working fallback | sdd-runtime-safety.md, highest architectural invariant |
| D37 | Console-access users are HTON admins; R1 has no web admin; CLI uses always-available HTON Core independently of the management API | ARCHITECTURE.md; CLI.md; sdd-control.md |
| D38 | Management API is a full, self-sufficient optional module; future web admin is a separate Docker module and always uses that API | sdd-control.md; sdd-admin.md; module/file maps |
| D39 | Each module owns declarative configuration and a checker; common validation detects cross-setting contradictions; API edits preserve the same semantics | sdd-config.md; module contracts and schemas |
| D40 | Prefer ready-made third-party modules unchanged; HTON custom configuration/API/validation/tests run natively or in separate containers | ARCHITECTURE.md; REUSE-ASSESSMENT.md |
| D41 | R1 includes Xray and the already-installed Amnezia VPN behind TCP/443 with a genuine domain certificate; first prefer Amnezia's native domain setup, with HTON checking compatibility/port conflicts and no custom certificate-management layer absent a new decision | OWNER-REQUIREMENTS.md; sdd-foreign-adapters.md; sdd-installer.md |
| D42 | No traffic accounting in R1 or R2; defer to long-term plans | ROADMAP.md; remove accounting from release requirements |
| D43 | Installer checks/configures domain, offers generic free-domain guidance and an example when absent; installation may finish without a domain with an applicable server-ban risk warning | sdd-installer.md; cert and ingress contracts |
| D44 | Module add/remove differs from enable/disable; remove retains data, purge is explicit, image updates are separate; up launches saved composition without image updates | CLI.md; sdd-config.md |
| D45 | OpenWrt client installation is already planned; device rollout remains incremental/optional; health-check already exists; autonomous self-healing is not a product gap | OWNER-REQUIREMENTS.md, product clarifications |
| D46 | Build a simple local PowerShell/Bash protocol/profile tester to guide client choice using speed and reliability evidence; consider mobile expansion later | OWNER-REQUIREMENTS.md, local client selection clarification |
| D47 | Detailed HTON logs support fast AI-assisted repair; config controls verbosity; installer configures the log directory, default `/var/logs/hton` | OWNER-REQUIREMENTS.md, logging clarification |
| D48 | Every HTON module must follow SDD, DRY, SOLID and GRASP; contracts and boundaries are documented with implementation | OWNER-REQUIREMENTS.md, module development canons |
| D49 | External VPN uplink and HTON server chains remain compatibility goals; HTON does not own external client configuration, and traffic/route integration requires a separate decision | OWNER-REQUIREMENTS.md; sdd-foreign-adapters.md |
| D50 | Owner prefers Amnezia's native domain/certificate setup over a custom HTON certificate layer, subject to technical verification before implementation | OWNER-REQUIREMENTS.md; sdd-foreign-adapters.md |

Earlier exclusions remain: no unsolicited public IPsec/bare-WireGuard/PPTP/plain
SOCKS; no silent changes to pre-existing foreign stacks. The broad toolbox request
does not automatically cancel these. Tailscale deployment remains policy-blocked.
Core is included and has no feature off switch; it is NOT a runtime dependency
of the data plane. Data-plane services use accepted native configs without admin.

Precedence: current explicit owner instructions -> this traced requirements summary
-> area-owning SDD -> architecture/roadmap views -> per-file implementation bindings.
If an inconsistency appears, correct derived documents; do not invent a new owner
requirement. Research reports and archived/local historical copies are not competing
normative specifications. CLI.md alone declares public CLI syntax. Config schemas
own syntactic field shape; sdd-config.md owns normalization/semantic rules.

## Current owner clarifications

- D74 (Q34): Configurator API accepts atomic sets of parameter changes rather than whole-file replacement. Preserve concurrent manual edits and prepared changes on conflicts; account for touched values and operation preconditions rather than blanket file-revision rejection. Source: OWNER-REQUIREMENTS.md, atomic API changes clarification. Design owners: sdd-config.md and sdd-control.md. This supersedes whole-file replacement/revision-only rejection assumptions.

- D73 (Q33): R1 retains manual YAML editing, CLI/Core validation and safe apply; advanced configuration editing/merge/migration/maintenance is deferred to the standalone module, callable by CLI without web administration. Public API uses tokens. Manual YAML edits must not break the configurator itself; invalid target configuration remains diagnosable/repairable and cannot be activated. Source: OWNER-REQUIREMENTS.md, Q33 clarification. Design owners: sdd-config.md and sdd-control.md. This settles the R1/deferred-module boundary raised in D71.

- D71: Configuration updating, maintenance and editing belong to a standalone module with its own API and possible integration into web administration API. Defer implementation to web administration/public service API with token access. This supersedes placement of the whole configurator/migrator in mandatory Core; CLI/Core/validation/apply boundaries require clarification without silently cancelling standalone CLI. Source: OWNER-REQUIREMENTS.md, standalone configuration maintenance module clarification. Design owners: sdd-config.md, sdd-control.md and sdd-admin.md.
- D72 (Q32): Backing up durable working state with configuration is acceptable, but configuration and working files remain distinct categories with distinct access rights; caches/temporary state are excluded. Source: same owner clarification. Design owners: sdd-config.md and backup contract. Superseded for R1 scope by D159 (OR-54): no separate backup feature; the private `.secrets` repository is the durable copy.

- D69 (Q30): Explicitly register associated components, including adopted Amnezia; discovery alone does not confer management. Keep the registry and related runtime bookkeeping in a dedicated Git-ignored working directory. Code is read-only except during updates; owner suggests privileged updates, with permission mechanics to be designed. Source: OWNER-REQUIREMENTS.md, working directory/code protection clarification. Design owners: sdd-control.md, sdd-config.md and sdd-installer.md.
- D70 (Q31): Configuration archive restoration checks format versions, secrets and module availability and does not start services; startup is a separate `up`. Source: same owner clarification. Design owners: sdd-config.md and sdd-control.md. Superseded for R1 scope by D159 (OR-54): recovery is reinstall from the private `.secrets` repository, without a separate restore mode.

- D67 (Q28): `modules add` obtains resources/configuration and missing inputs but leaves a new module disabled; `enable` checks readiness and enables it. Source: OWNER-REQUIREMENTS.md, Q28–Q29 clarification. Design owners: CLI.md and module contracts.
- D68 (Q29): Incomplete module setup retains resources/settings, exposes incomplete status and resumes on repeated `add`; it cannot start while incomplete. Source: same owner clarification. Design owners: CLI.md and module contracts.

- D65 (Q26): Failed module shutdown does not prevent remaining stop attempts; `down` preserves disabled autostart, reports an error and lists remaining running components. Only confirmed complete shutdown may be reported as server off. Source: OWNER-REQUIREMENTS.md, Q26–Q27 clarification. Design owner: sdd-control.md.
- D66 (Q27): Partial startup retains healthy components and reports failures; repeated `up` starts missing components without restarting healthy ones. Source: same owner clarification. Design owner: sdd-control.md.

- D63 (Q15): Invalid configuration makes `scripts/up` report an error and offer explicit last-known-good startup as a separate action; no silent fallback. Source: OWNER-REQUIREMENTS.md, Q15–Q16 clarification. Design owners: CLI.md and sdd-control.md.
- D64 (Q16): `scripts/down` persists the off state across VPS reboot, including associated Amnezia; `scripts/up` enables the server and boot autostart again. Source: same owner clarification. Design owners: CLI.md, sdd-control.md and sdd-foreign-adapters.md.

- D62 (Q25): Store named configuration objects as mappings keyed by stable identifiers; use lists where order matters, with module-defined update semantics. This supports recursive merging without positional identity. Source: OWNER-REQUIREMENTS.md, named objects/lists clarification. Design owner: sdd-config.md.

- D61 (Q22–Q24): Configurator/migrator/API implementation is deferred; define architecture and a nested configuration layout now. YAML is the HTON settings format without a parallel `.env` source. Editable Docker customization is retained. Version-aware validation moves unsupported/unused keys to `*.deprecated.yml`; every transformation has an explicit strategy and must preserve original values, including secrets. This replaces the Q24 proposal of requiring a user decision for every such parameter. Source: OWNER-REQUIREMENTS.md, deferred configurator/value preservation clarification. Design owners: sdd-config.md, sdd-control.md and module contracts.

- D59: Keep shipped Docker configuration/templates in Git and editable deployed configuration/secrets together under `/opt/hton/config/` (subdirectory details delegated), independently archivable without the source tree. API must support editing this configuration without using `/etc/***`; upgrades preserve custom Docker settings via transformations. Source: OWNER-REQUIREMENTS.md, unified editable configuration clarification. Design owners: sdd-config.md, sdd-control.md and module contracts. Older `var/secrets/` and single-root-file layouts require reconciliation.
- D60 (Q21): Chain migrations by configuration format version for the platform/modules, not by software release. Missing or empty migrators alone must not fail the script. Source: same owner clarification. Design owner: sdd-config.md; full upgrade implementation deferred.

- D58: Current work prioritizes configuration storage architecture and future migration seams; full upgrade implementation is deferred. Owner proposed chained version-to-version configuration migrations for skipped versions; the mechanism remains under discussion. Source: OWNER-REQUIREMENTS.md, storage/migration priority clarification. Design owner: sdd-config.md.

- D56 (Q14): `up` after `down` starts the saved enabled composition, including Amnezia, preserving settings/data and performing no reinstall/image upgrade. Source: OWNER-REQUIREMENTS.md, general upgrade/restart clarification. Design owners: CLI.md and sdd-control.md.
- D57: `scripts/upgrade` performs `git pull`, preserves secrets while migrating configuration, and invokes `scripts/modules update XXX`; each module owns its update logic. Migration/failure/module-selection policies remain design questions. Source: OWNER-REQUIREMENTS.md, general upgrade/restart clarification. Design owners: CLI.md, sdd-config.md and module contracts.

- D54 (Q12): Module participation changes apply after checks by default and show expected connection interruptions; a separate save-without-apply mode permits batching. Source: OWNER-REQUIREMENTS.md, Q12–Q13 clarification. Design owners: CLI.md and sdd-config.md.
- D55 (Q13): `scripts/up` turns the HTON server on; `scripts/down` stops everything associated with the service, including external Amnezia. Configuration ownership does not exempt Amnezia from server-wide shutdown. Invalid configuration handling is settled in D63; pending-candidate semantics remain to be reconciled. Source: OWNER-REQUIREMENTS.md, Q12–Q13 clarification. Design owners: CLI.md, sdd-control.md and sdd-foreign-adapters.md.

- D51 (Q9): [Owner log requirements](OWNER-REQUIREMENTS.md#log-and-resumable-installation-clarification-2026-10-31-q9q11) require secret redaction at every verbosity, with object/action/result/error context for AI-assisted repair. Design owners: sdd-config.md and sdd-control.md.
- D52 (Q10): The same owner clarification requires configurable log size/age rotation; logging exhaustion must not stop working VPN. Defaults remain implementation choices. Design owner: sdd-config.md.
- D53 (Q11): The same owner clarification requires resumable partial installation and explicit installed/running/externally reachable/blocked status. Design owner: sdd-installer.md.

[OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md), OR-1–OR-7 and subsequent dated clarifications, records binding owner decisions. They supersede earlier mandatory-web, first-iteration UI, traffic-accounting and mandatory-Docker-staging language. Detailed answers are requirements; changing them needs explicit owner acceptance. Installation is outside home, target RAM is 1–2 GB, scale is about 50 devices with tens concurrent, and external uplinks/server chains are required. Traffic accounting is deferred beyond R2. Derived schemas and per-file maps still require reconciliation before implementation readiness.

## Final planning instruction and engineering reconciliation

- D80 — Owner acceptance, 2026-10-31: «+» accepts the explicitly recommended
  native Amnezia REALITY/TCP-passthrough option, conditional on installed-stack
  evidence. Genuine own-domain HTTPS certificate remains required for the ordinary
  HTON site; the earlier requirement of an issued own-domain certificate inside
  the native REALITY VPN handshake is replaced for that scenario. Native identity,
  SNI/target/restart compatibility and foreign approvals remain required. Source:
  OWNER-REQUIREMENTS.md, accepted Amnezia REALITY certificate-scope replacement;
  evidence: AMNEZIA-NATIVE-EVIDENCE.md. No custom issuer or blanket protocol support
  is authorized.

- D79 — Owner instruction, 2026-10-31: “Proceed with implementing the plan in docs/tasks, taking docs/specs and docs/sdd into account.” Explicit phase transition to implementation;
  supersedes the planning-only phase in D75–D76 and AGENTS.md. Preserve the
  architecture-first contracts and D77 final unit/functional sequence. Source:
  OWNER-REQUIREMENTS.md, current implementation phase. Execution follows
  IMPLEMENTATION-ORCHESTRATOR.md; status is recorded in IMPLEMENTATION-STATUS.md.

- D75 — Owner instruction, 2026-10-31: current work is planning only, including
  detailed file/class/function/argument/API contracts and independent resolution
  of design inconsistencies. Record changes in Markdown and permanent principles
  in AGENTS.md. Production SDD/SOLID/GRASP/DRY decomposition precedes coding.
  Source: OWNER-REQUIREMENTS.md, permanent planning principles.
- D76 — Owner instruction, same source: prepare a future parallel multi-agent
  implementation orchestrator with concrete waves/goals. Current authorization
  covers its Markdown plan, not execution of implementation.
- D77 — Owner instruction, same source: complete implementation first, then unit
  testing, then functional testing; plan further acceptance after those results.
  Supersedes interleaved development/test gates. Runtime configuration checks
  remain product behavior; document-consistency checks remain planning work.
- D78 — Owner objective, same source: one coherent implementation/acceptance pass;
  resolve architecture in the plan to avoid repeated polishing and duplicated logic.

Engineering choices implementing existing requirements (not new owner quotations):
`config/` owns editable root/module/Docker/secret inputs; Git-ignored `work/` owns
registry and durable runtime records; immutable bundles isolate live services from
edits. Core directly supports CLI; deferred configurator owns parameter edits and
migrations; optional API delegates rather than duplicating logic. Durable stopped
intent fences boot and guardian rollback. Details: ARCHITECTURE.md,
specs/sdd-configurator.md, specs/sdd-lifecycle.md and INTERFACES.md.

Unresolved guarantees are explicit in FINAL-REVIEW.md, especially publication races
with arbitrary external editors and native Amnezia qualification. No documentation
change is evidence of implementation, tests or production readiness.

- D81 — Owner instruction, 2026-11-01: continue larger coherent implementation
  increments and parallel multi-agent development; postpone builds until all agreed
  plans are fully ready. See OWNER-REQUIREMENTS, “Development and build organization
  clarification”. Coordinator interpretation: no intermediate cargo check/build/release
  compilation until selected implementation is complete; then build → U0 → F0.
  Earlier intermediate-build permission is superseded. Existing prior build results
  remain historical evidence only; new source changes are not claimed compiled.

- D82 — Owner instruction, 2026-11-02: start an HTTPS/HAProxy/Amnezia trial serving
  a placeholder from `/var/www`. This does not change OR-4's public
  domain/HTTPS requirements or existing foreign-resource/public-port boundaries.
  Source: OWNER-REQUIREMENTS, “HTTPS/HAProxy/Amnezia trial”.
- D83 — Engineering decision (not an additional owner requirement): explicit
  `runtime.rollback` may restore only the previous-compatible bundle bound by the
  currently committed activation lease. Runtime uses that lease's protected
  fallback digest/authority and declared inverse actions, compares the exact active
  accepted/previous IDs and resource receipts, and checks credential/certificate
  fences before and after effects. A new `RollingBack` CAS state is distinct from
  guardian recovery and remains fenced by run intent/epoch; a concurrent `down`
  prevents starts and acceptance. Successful rollback consumes the one-step target;
  another target must be qualified by a later commit. The UI/CLI cannot construct
  or broaden fallback authority. Owners: INTERFACES §4/6, sdd-lifecycle,
  sdd-runtime-safety, runtime-contract and runtime guardian/state maps.
- D84 — Historical engineering decision (not an additional owner requirement): the
  initial requested trial was scoped to a local Docker sandbox at
  `127.0.0.1:4443`, with HAProxy SNI routing and a self-signed `hton.local`
  certificate. This described prototype evidence only; it did not settle the later
  production installer target and is superseded for that target by OR-9/OR-10 and
  D88/D91. Historical limitations remain recorded in IMPLEMENTATION-STATUS and
  AMNEZIA-NATIVE-EVIDENCE.
- D85 — Owner clarification, 2026-11-02: public IPv4/DNS probes must not run as
  automatic or background diagnostics; local `diag.observe` is network-free.
  External identity probes are permitted only when the owner explicitly invokes
  `identity.whoami`; unavailable fields remain marked unverified. Source:
  OWNER-REQUIREMENTS, OR-8. Design owners: sdd-control.md, sdd-whoami.md,
  `core/src/checks.rs`, and `core/src/host/observations.rs`.
- D86 — Engineering decision (not an additional owner requirement): a module's
  `update` capability grants authority only over update targets registered as owned
  by that module. VLESS currently consumes shared Xray/Unbound and has no registered
  module-owned image artifact; `xray.update` therefore cannot be bound to VLESS.
  Clear VLESS's update capability until a module-specific updater and owned pinned
  target exist. Shared engine upgrades remain in their owning platform workflow.
  This preserves the owner's requirement that update be distinct from setup and up.
  Design owners: INTERFACES §5/§11, sdd-lifecycle and the VLESS descriptor map.
- D87 — Engineering decision (not an additional owner requirement): Core resolves
  no-argument `services.restart` from the saved enabled service closure, then
  intersects that set with installed Owned registry resources and exact current
  accepted receipts/run epoch. Shared services follow enabled consumers; core ingress
  is excluded. The journal and typed Runtime action carry the selected IDs, accepted
  bundle ID and planned epoch. Runtime verifies only selected-service probes; unknown
  health evidence leaves a Partial terminal with successful receipts/survivors.
  Owners: INTERFACES §6, sdd-control, sdd-runtime-safety, Core lifecycle and Runtime
  action/probe contracts.
- D88 — Owner instruction, 2026-11-02: deploy the owner's site domain `example.com`
  on public TCP/443 through HAProxy, move the existing Amnezia listener to external
  port 8443, preserve Amnezia traffic, and obtain a trusted certificate from Let's
  Encrypt or a suitable issuer. The same setup must be reproducible automatically
  through `scripts/install`, including initial typed domain/port input, protected
  credential entry, safe retries and status reporting, without requiring manual
  YAML editing. Source:
  OWNER-REQUIREMENTS, “Publishing example.com through HAProxy and installation”.
- D89 — Engineering stop-point, not permission to change another service: the live
  `outline-operator-two` container already owns TCP and UDP 8443. Installation and the
  current deployment must refuse the Amnezia remap before effects until the owner
  selects a different free Amnezia port or explicitly authorizes a compatible
  Outline remap. `--yes` cannot supply that authorization. Owners: sdd-installer,
  sdd-foreign-adapters and the installation/foreign adapter contracts.
- D90 — Engineering decision: use the existing certificate lifecycle contract for
  own-site TLS: DNS-01 through the supported InternetBS ACME integration, with
  provider credentials supplied by protected `config/secrets` references. `scripts/install`
  may prompt for missing input through the SecretStore workflow; it must not print,
  log, guess or embed credentials. A trusted cert is required before switching
  public traffic to the new HTTPS site; missing credentials leave a resumable,
  honestly degraded install. Amnezia's native REALITY certificate/config remain
  externally owned. Owners: sdd-certs, sdd-installer, INTERFACES and certificate maps.
- D91 — Owner clarification, 2026-11-02: Amnezia and other VPN services in the
  HTON composition use different random ports from the inclusive range 5000–6000.
  The selected port is durable per-service installation state and every repeated
  or resumed `scripts/install` reuses it, including after process or host restart. The
  service configuration is the source of truth; process memory or an operation
  journal alone is insufficient. A new value is allowed only for first assignment
  or a separately approved change. This replaces D88's specific Amnezia 8443/8443 target; the public HAProxy
  TCP/443 requirement and trusted own-site certificate requirement remain. Source:
  OWNER-REQUIREMENTS OR-10.
- D92 — Engineering decision (not an additional owner requirement): one stable
  random host port is allocated per VPN service ID, unique across the managed VPN
  service set, using OS cryptographic randomness and fresh TCP/UDP reservation
  evidence. A service that binds both transports uses its own allocated number on
  each configured transport. Persist the chosen number in that service's existing
  configuration owner via the installation CAS; unchanged installs reuse it. A
  conflicting saved value or occupied candidate is surfaced and requires the
  existing exact review/remap flow; no automatic reroll of saved state. The
  interval is inclusive at both ends. D89's observation that Outline owns 8443
  remains true, but 8443 is no longer the requested Amnezia target; Outline is not
  moved unless separately registered and included in an explicit compatible remap.
  Owners: sdd-installer, sdd-config, sdd-foreign-adapters, the shared
  `core.port-allocation` policy, and the per-module settings/schema maps. This
  new owner supplies candidate port sets to the pure planner; allocation is not
  embedded as randomness inside `core.plan`.
- D93 — Engineering decision (not an additional owner requirement): lifecycle
  add/enable plans carry the exact `PortAllocationCandidate` inside immutable
  `LifecyclePlan`. Host preparation creates it from the prospective selected graph
  and fresh config/registry/host evidence; execution validates that same candidate
  against fresh evidence and returns `planStale` on any changed dependency instead
  of drawing replacement entropy. After complete preflight, a separate opaque
  `AllocationApproval` is required only when saved port fields would change;
  `--yes` and generic effect approval do not substitute for it. Lifecycle intent
  records the candidate before the port-settings owner journals protected originals
  and performs descriptor-allowlisted CAS. Allocation commit/recovery is keyed by
  lifecycle operation ID plus candidate digest, so interruption resumes the exact
  assignment. Associated foreign resources remain excluded until durable adoption;
  any remap still needs its own snapshot and consent. Owners: `sdd-lifecycle`,
  `sdd-port-allocation`, INTERFACES §2/§4 and the mirrored lifecycle/operation maps.
  The plan field is additive/optional for old serialized records; an old add/enable
  plan that now requires allocation cannot resume by silently generating a port.
- D94 — Engineering interpretation of owner OR-8/D85: catalogue and local HTTPS
  activation must not require a public DNS-A probe, since only explicit
  `identity.whoami` may perform external identity observation. A configured domain
  still requires fresh local `certificates.native` evidence before activation.
  DNS-A/public identity remains explicitly unverified in install results until the
  owner invokes WHOAMI; install must not synthesize a pass. This reconciles the
  older derived catalogue gate with OR-8 while preserving OR-4's certificate/site
  requirement.
- D95 — Engineering decision: `core.catalog.resolve_for_install_preflight` resolves
  the structural graph while deferring local certificate readiness, allowing the
  installer to prepare port candidates and complete safe earlier phases before
  certificate issuance. `certificate_requirements` remain in InstallPlan; absence
  of a certificate is a later phase outcome, not permission to activate ingress.
  Ordinary `resolve` remains the gate immediately before ApplyIngress and must use
  genuine fresh local evidence. This permits honest degraded/partial install without
  fabricating evidence or coupling allocation to ACME success.
  Owners: `core.catalog`, `core.host-installation`, INTERFACES §I1 and their maps.
- D96 — Engineering decision: first-publication input includes a non-secret
  Let's Encrypt account email because a public certificate cannot be issued without
  it. If no certificate configuration exists, the narrow install writer seeds the
  fixed supported InternetBS DNS-01 issuer and SecretStore references
  `dns/internetbs/api-key` / `dns/internetbs/api-password`. Existing issuer and
  reference values are preserved. Credential bytes are never install arguments or
  ordinary prompts; missing values are imported from protected operator-selected
  files through the existing DNS SecretStore owner. Missing credentials leave the
  install resumable and unpublished. Owners: INTERFACES, sdd-installer, sdd-certs,
  `core.host-install-input` and `core.host-install-input-writer`.
- D97 — Engineering decision: install binds `/var/www/index.html` absence or exact
  content/filesystem identity into its read-only plan. After that plan passes, the
  webroot owner may create the default only when absent. Install reports ready only
  after local TLS/SNI, HTTP 200 and exact-body verification, with source identity,
  digest and bytes unchanged around the probe. The bounded probe can validate
  content up to the webroot owner's 1 MiB limit. This is local serving evidence;
  public DNS remains unverified. Owners: `core.host-webroot`, `core.host-installation`,
  `core.render`, INTERFACES and their maps.
- D98 — Engineering decision: the bounded validated UTF-8 `/var/www/index.html`
  becomes a standalone immutable `site-index.html` artifact in the admitted bundle.
  Caddy serves only that artifact through a fixed rewritten route; it does not mount
  `/var/www` or a mutable staged whole-tree path, which could expose protected state
  through source symlinks. Local readiness verifies byte-exact served content and
  stable source identity/digest. Bundle rollback therefore restores the site body
  with the matching Caddy route. This supersedes D84's read-only host-webroot mount
  implementation detail without changing D82's trial intent. Owners:
  `core.host-webroot`, `core.render`, Caddy renderer/template, INTERFACES and maps.

- D99 — Superseded deployment method from owner clarification OR-11: the original
  derived interpretation required `scripts/install` to configure group `hton` and
  invoke `scripts/deploy` with a local prebuilt package/provenance manifest. OR-14/D103
  replaces that package requirement with current working-tree source delivery.
  The invoking sudo user is added to `hton`; group members can read/execute
  immutable deployed code but cannot write it or receive sudo authority. Config/work
  and secret-bearing subtrees remain root-private and preserved. Both install and
  standalone deploy invoke `sudo` themselves when unprivileged; install passes
  bounded flags and arguments in the fully elevated process. The permission and
  self-sudo provisions remain. Owners: OR-11, sdd-installer, INTERFACES, cmd.install,
  cmd.deploy and their file maps.
- D100 — Superseded deployment layout/manifest design: retain immutable versions under
  `/opt/hton/releases/<release-id>` and atomically replace `/opt/hton/current`;
  stable `/opt/hton` entry points resolve the active release while `config/`,
  `work/` and logs remain outside the switched tree. Copy and validate in a private
  staging directory before switching. Keep rollback by retaining the previous
  release. Deploy rejects unsafe paths, modified manifest contents, missing
  Core/runtime helpers, state directories embedded in the package, and unowned
  non-empty `/opt/hton` trees; it never recursively chmod/chowns preserved state.
  This is an engineering choice, not additional owner authority. OR-14/D103
  supersedes the prepared-release path and names the copied snapshots
  `/opt/hton/deployments/<tree-digest>`; current-tree source is the deployment input.
- D101 — Owner requirement OR-12: completion target is an actually working
  `example.com` HTTPS site with trusted SSL and exact-SNI separation of site traffic
  from the existing independent Amnezia REALITY service on TCP/443. Preserve D80's
  TLS termination for the site and byte-preserving passthrough for qualified
  Amnezia. Completion requires the D81 implementation/build/unit/functional order,
  trusted DNS-01 issuance and local HTTPS verification, plus evidence-backed
  Amnezia remap/handshake and recovery. Code/render alone is not deployment or
  external readiness. No chat disclosure of DNS credentials; no bypass of native
  qualification. Owners: OR-12, sdd-installer, sdd-foreign-adapters, INTERFACES,
  installer/foreign module maps and implementation status.
- D102 — Owner requirement OR-13: all product code and product-facing text use
  English; product localization is deferred, and Russian documentation is limited
  to `docs/_lang/ru/`. This constrains content, not programming-language choice. Owners:
  CLI/source SDDs, docs language policy and their file maps.
- D103 — Owner requirement OR-14: `scripts/deploy` copies current working-tree product
  files, including uncommitted changes, into `/opt/hton`; no prepared release,
  provenance manifest or clean Git state is required. Exclude `.git`, `docs/_lang/ru/tasks/*`,
  generated output, secrets and mutable state. `scripts/install` invokes this deploy
  path; deployment does not build missing helpers. D81's build sequence remains.
  This supersedes the deployment method in OR-11/D99–D100 while retaining their
  group, permission, self-sudo, atomic-switch and state-preservation requirements.
- D104 — Owner clarification OR-12: successful `scripts/install` means the active
  deployment under `/opt/hton` has a working public TCP/443 HAProxy split: website
  SNI reaches the HTTPS site backend with a publicly trusted domain certificate,
  and qualified Amnezia traffic reaches its native backend without REALITY
  termination. Install verifies served certificate and both routes before reporting
  ready; blockers remain explicit degraded/unverified states. This operationalizes
  the requested result and does not waive native qualification or external-change
  authorization. Owners: sdd-installer, sdd-foreign-adapters, HAProxy/Caddy adapter
  contracts, install/reconcile interfaces and implementation status.
- D105 — Engineering binding of R1 explicit public cutover: desired YAML remains
  the owner of the edge target. `switch [PORT]` asserts that saved target and uses
  the shared admitted apply path; it does not introduce another config writer.
  Complete fresh host listener evidence is checked before local public-listener
  consent; foreign/unknown holders block without mutation. Validation-only remains
  read-only, and actual pending/partial results are retained. This does not grant
  foreign remap authority or alter OR-12's two-route readiness requirement.
- D106 — Owner scope OR-15 selects the minimum working `scripts/install` delivery
  separately from full R1. Its Amnezia backend is the persisted loopback host
  publication `127.0.0.1:5000–6000`, not a container IP or public wildcard bind.
  Unrelated management operations do not gate this selected scope's build after
  its own implementation is complete. Existing TLS/issuer ownership stays in force
  pending explicit answers to the proposed TLS/HTTP-01 alternatives. Native config,
  keys and original container/rootfs recovery must survive remap.
- D107 — Engineering execution binding for OR-15: the five `core/minimum` Python
  modules implement the selected Core without missing Rust helpers. Deployment
  requires these source files; no compiler runs on the VPS. Docker boot enablement
  is checked before effects, and accepted containers restart independently of Core.
  The broader R1 controller refuses mutations of minimum-owned state. This is a
  minimum delivery, not full R1 acceptance.
- D108 — Correct the minimum remap's incomplete single-network assumption: native
  Amnezia uses default bridge plus an existing local bridge DNS network. Preserve
  both dynamic attachments by immutable ID, aliases and gateway priority. Core
  verifies dependencies read-only; it never alters native networks/DNS or peers.
  Fixed-address, externally managed and unsupported-driver layouts still block.
  Trace: OR-15 native configuration preservation; sdd-minimum-installation.
- D109 — Correct terminal I/O and minimum failure diagnostics: independent ordinary
  TTY read/write streams, stdlib getpass without visible fallback, and actual PTY
  acceptance. Unexpected failures expose only exception kind/owned location/errno,
  never messages, locals or native output. Owns the reported generic-error defect
  in minimum install while preserving OR-12/OR-13 secret handling and protections.
- D110 — OR-16 uses etc/install.json for non-secret source installation inputs:
  installation.directory and optional core.edge.domains/core.certificates.email.
  Canonical installed config/hton.yaml wins over source defaults. Missing values
  are prompted and persisted before remap consent, without granting native change
  authority. The minimum deploy target is a canonical direct child of /opt; a
  deployed source derives its root from its protected snapshot location. Config,
  Docker client paths, units and CLI re-entry use that root. No implicit existing
  state migration or parallel installation is attempted. Full R1 custom-root
  support is outside this selected minimum binding.
- D111 — OR-17 splits entry orchestration from target setup. scripts/install branches
  on physical CWD, calls source deployment when outside target, changes directory
  and invokes scripts/setup. Setup rejects wrong CWD before sudo or effects and uses
  the selected Core path for validation/input/state. Current-tree atomic delivery
  remains scripts/deploy's replacement mechanism. Update's Git-first/container scope
  supersedes its old config-apply alias, but foreign upgrade scope is unresolved;
  do not launch it through the old alias or silently upgrade unrelated services.
- D112 — OR-18 replaces manual copying of a REMAP digest with a short English
  explanation and explicit local [y/N] consent, declining by default. Show native
  name/ID, old/new endpoint, preservation and interruption, and restart ownership.
  Bind approval internally to the same exact scope digest; --yes still cannot
  replace this dedicated prompt. Existing running installers keep their old prompt.
- D113 — OR-19 authorizes public DNS/public IPv4 preflight only during explicit
  setup, with provider-neutral manual instructions and an optional API path.
  OR-20 prioritizes unattended reliable renewal and an explained issuer/challenge
  choice. CERTIFICATE-CHOICE.md records primary-source findings: recommended
  HTTP-01/Let's Encrypt for one reachable public domain; alternative issuers have
  account/EAB requirements and failover is distinct from DNS/port correctness.
  These findings are not a live implementation or an accepted alternative CA.
- D114 — OR-21 corrects the one-instance framing. Certificate policy depends on
  reachability, wildcard use and challenge routing, not the number of owner VPS.
  Separate reusable product defaults, per-node configuration/identity, per-domain
  publication/certificate policy, DNS account references and private issuance state.
  HTTP-01 remains valid across many domains/nodes with correct routing; DNS-01
  suits wildcards/non-HTTP or shared-name cases without coordinated HTTP challenges.
  Fleet renewal must reuse material, respect aggregate limits, stagger retries and
  preserve other concurrent DNS challenge values. No remote provisioning or shared
  certificate key distribution is inferred. The current minimum implementation
  still lacks this breadth; do not label it fleet-ready.
- D115 — Implement the OR-19/20 HTTP-01 default in the existing single-issuer
  acme.sh lifecycle. A bounded standalone TCP/80 job avoids DNS-provider secrets;
  existing certificate delivery/rollback remains authoritative. Migrate only an
  unissued, uncommitted legacy DNS attempt without credentials/job/certificate;
  retain configured/accepted DNS installations. Explicit setup DNS preflight and
  universal manual assistance are owned by core.minimum.publication; no background
  public identity discovery. Public bootstrap removes owner-specific defaults,
  leaving saved private configuration intact. This is a minimum implementation,
  not fleet coordination or universal DNS API support.

- D116 — OR-22 treats an existing HTTP listener as valid infrastructure. D115's
  free-port requirement applies only to standalone mode. For webroot mode, prove
  the exact domain route using an exclusive random public token, then mount only
  its approved challenge subdirectory into the issuer. Preserve service/config,
  existing ownership/modes and unrelated tokens. Save approved webroot owner for
  renewal without SUDO_UID; new public proof data gets explicit readable modes
  even under the timer's private umask. No private keys enter public webroot.
- D117 — Fix owned-receipt false drift on Docker 29 create/start: normalize only
  null/false OomKillDisable through the shared Docker owner, version new launch
  hashes, and verify old hashes under that exact semantic default representation.
  No unknown-resource adoption or general drift bypass. Preserve completed ACME
  output and exercise real receipt validation in native HTTP issuance fixtures.
- D118 — OR-23 adds plain STEP/OK/ERROR outcomes and secret-safe command labels,
  exit codes and closed known-reason messages. Raw process streams stay private.
  Actual failed step was isolated snapshot validation: native root config mode
  0001 needs read bypass. Add DAC_READ_SEARCH only to its no-network/read-only
  validator, preserving original config and keys. The actual snapshot validation
  reproduced exit 23 before this capability and exit 0 with it.
- D119 — Extend D117's OomKillDisable normalization to the foreign replacement
  drift check. `check_foreign` compared the raw observed HostConfig against a
  canonicalized expectation, so Docker 29's create=false / start=null
  OomKillDisable representation tripped "Foreign launch/config drift" right after
  the replacement started, and again inside rollback (blocking recovery, leaving
  the replacement active and the original parked). Canonicalize both the expected
  and observed HostConfig through docker.canonical_host_config before the equality
  check. No broadening of drift detection: a genuine OomKillDisable=true or any
  other field change still refuses overwrite.
- D120 — OR-24 replaces the symlinked content-addressed deployment with literal
  files. `scripts/deploy` now copies each inventoried file directly into `/opt/hton`
  (atomic temp+rename), removes legacy `current`/per-name symlinks, prunes stale
  managed files, and enforces root:hton ownership with group-readable modes
  (0o640 files, 0o750 directories/executables). `install_config.deployment_root`
  drops its `deployments/` special-case, and the minimum installer's source check
  becomes `SOURCE == ROOT`; recovery/renew units run from `/opt/hton` directly.
  `/opt/hton` itself is set to `root:hton 0750`; the deploy creates the `hton`
  group and adds the invoking user if absent, then prints activation guidance
  (`newgrp hton` or a fresh login) because Unix group membership only applies to
  a new session. Mutable `config/`/`work/` (root:root 0700, secrets/state) and
  lock/marker objects are never touched.
- D121 — Fix REALITY authentication false failure during cutover. After a cold
  start the replacement's REALITY endpoint needs several seconds (observed ~7-9 s)
  before it can complete the authenticated tunnel to the camouflage target, but
  `authentication` ran ~1-2 s after `replacement-start`, so the first curl attempt
  timed out (exit 28) and aborted the cutover. Retry the authenticated probe
  against a bounded 30 s readiness budget (catching transient curl/NativeError
  failures, 1 s between attempts) before declaring failure. No weakening of the
  check: an unreachable VPN still fails after the bounded wait. Reproduced against
  the live Docker 29.9.2 socket (cold start: rc 28 at ~2 s; with retry: HTTP 404 at
  ~8.7 s) and confirmed the auth code/keys work when the endpoint is warm.
- D122 — Serve the real website document root, not a placeholder single body.
  `ingress.site_root()` resolves `/var/www/html` then `/var/www` (matching
  `http_challenge.select`), the caddy backend uses `file_server` rooted at the
  read-only `/site` mount (hiding `.git`) when a document root exists, and keeps
  the static placeholder only when none does. `webroot()` therefore reads
  `/var/www/html/index.html`; the install generation binds that index so a changed
  site forces a fresh cutover. Verified live: index, `config.js`, and `chgu.png`
  (1.3 MB) are served and `/.git/config` returns 404.
- D123 — Implement OR-25 in the minimum profile with a separate
  `core.minimum.protocols` owner. MTProto uses a pinned `mtg` container bound to
  127.0.0.1 and an exact camouflage SNI route in HAProxy; Hysteria2 uses a pinned
  server container on the configured public UDP port with the existing site
  certificate, password authentication and site masquerade; foreign Outline is
  associated by exact container name/full ID and only observed/preserved. Module
  settings live in `config/modules/{mtproto,hysteria,foreign-outline}.yaml`, root
  `features.<id>.enabled` selects activation, and generated secrets stay under the
  protected minimum work tree. Share links are written 0600 and printed only for
  explicit human completion; JSON and logs never carry them. Protocol containers
  participate in the same candidate/active ownership receipts, drift checks,
  rollback and restart policy as ingress. Full R1's optional-module descriptors
  remain unimplemented and are not implied by this minimum-profile capability.
- D124 — Implement OR-26 by merging the former `cmd/` entrypoints into the existing
  `scripts/` directory and renaming the mirrored map directory. Command file names,
  behavior and stable documentation IDs (`cmd.install`, etc.) are unchanged; only
  filesystem paths, CI scans, error text and links change.
- D125 — OR-27 keeps the observed third-party camouflage SNI for Amnezia REALITY
  and MTProto instead of moving it to the owner's domain. REALITY borrows a
  third-party certificate and handshake; an own-domain SNI would share TCP/443
  with the site, either forcing site traffic through the REALITY/VPN backend or
  making REALITY unreachable, require native `serverNames`/`dest` edits that the
  remap contract forbids, and invalidate every existing client profile. MTProto
  keeps third-party camouflage because the minimum profile issues one site
  certificate and supports one site name, so an own subdomain would need new
  wildcard/SAN support plus Caddy policy and `[domain-fronting]`; using the site
  apex would make the site depend on `mtg`. Existing exact-SNI routing,
  `DEFAULT_CAMOUFLAGE` and example `serverNames` remain unchanged. Deferred
  defect: `_mtg_secret` validates only the hex prefix, not the camouflage domain
  embedded in the secret, so any future camouflage change must first bind the
  secret to the requested name or regenerate it. Superseded for REALITY by D126;
  the MTProto part remains in force.
- D126 — OR-28 replaces the REALITY third-party camouflage with self-steal on the
  owner's real site `example.com`. Verified in a temporary sandbox with Xray 26.7.28
  on a spare loopback port: `serverNames: ["example.com"]` and
  `dest: 127.0.0.1:9443` forwarded an unauthenticated TLS handshake to the site
  backend and returned the real `CN=example.com` Let's Encrypt certificate and the
  `/var/www/html` body. A second sandbox proved the bridge path: Caddy bound to
  `172.17.0.1:19443` was reachable from `amnezia-xray` and served the same
  certificate/site, and an HAProxy config with REALITY primary plus site backup
  passed `haproxy -c`. Live implications: HAProxy must route `example.com` to a
  REALITY backend and keep the site backend as a health-checked backup; Caddy must
  also listen on the Docker bridge address because the native container is bridged
  and cannot reach host `127.0.0.1`; native `dest` must be that bridge address, not
  public `example.com:443`. The native `serverNames`/`dest` edit changes the native
  digest, so the minimum installer's site/REALITY overlap guard and saved-digest
  check need a consented requalification path; editing state by hand is not a
  supported migration. The 12 existing native clients keep
  `www.googletagmanager.com` until reissued with `example.com`; removing the old
  name is a second explicit step. MTProto stays third-party under OR-27.
- D127 — OR-29 sets the single-443 composition. Live action taken on 2026-11-04:
  the foreign `outline-operator-two` container was force-removed after copying its
  official `outline-ss-server` v1.9.2 binary to
  `/opt/hton/bin/outline-ss-server` (sha256
  `afeca868e0e65d658a53dace9128588e0a8049295b358dbdbe2015ad011513fe`); its
  `/home/operator-two/outline` directory remains because it belongs to another user and
  needs owner/`operator-two` removal. Own Outline design: run the v1.9.2 binary on
  `127.0.0.1:<internal-port>` with `websocket-stream` and `websocket-packet`
  listeners, keep the two WS paths secret, and reverse-proxy them from Caddy under
  the site TLS certificate; HAProxy already routes the site SNI to Caddy, so
  Outline clients v1.15+ use `wss://example.com/<secret-path>` on TCP/443. MTProto
  uses an exact camouflage-SNI route through HAProxy. Hysteria2 cannot use
  HAProxy and, under the owner's follow-up, must not share 443: it binds UDP/8443
  directly while TCP/443 stays VLESS/HTTPS. The minimum profile's
  `foreign-outline` path is replaced by
  an HTON-owned `outline-wss` plan with a pinned Alpine runtime, the verified
  official binary, Caddy WS routes, access-key output and unit tests; the minimum
  implementation exists, but the live install has not been applied. The example configs now
  enable `mtproto`, `hysteria` and `outline-wss`. Live install remains blocked on
  interactive sudo; no HAProxy/Caddy/state mutation was performed.
- D128 — OR-30 makes deployment always minimal and self-sufficient. Audit of the
  pre-change tree: 159 files / 2.54 MB; the minimum closure is 44 files / 304 KB
  (115 fewer files).
  The minimum member set is `bin/hton`; `lib/{hton,bootstrap,install_config.py}`;
  `scripts/install`, `setup`, `deploy`, `help` and the remaining CLI wrappers
  except `build-minimum`, `package-release`, `install-ubuntu`, `install-unknown`;
  `core/minimum/*.py`; `etc/install.json`; `images.lock.json`; `README.md`;
  `LICENSE`; `docs/CLI.md`; `docs/MINIMUM-INSTALL.md`. Excluded: Rust/full-R1
  source trees and Cargo files, `adapters/`, `contracts/`, `runtime*`,
  `host-support/`, `fs-support/`, `logging/`, `schemas/`, `modules/`, `systemd/`,
  `compose/`, `docker/`, `validation/`, `web/`, `hton.example.yaml`, build-only
  scripts and unselected docs. `scripts/deploy` has no profile argument and
  always uses `source_files`' minimum inventory; `prune_stale` removes files
  inside the selected top-level paths and `prune_top_level` removes managed
  top-level paths absent from the minimum closure while preserving `config`,
  `work`, `libexec`, `deployments`, locks and markers. `lib/hton` no longer
  passes a deploy profile and accepts `--outline-binary`; `tooling/build-minimum`
  packages the minimum closure. Verification: minimum unit tests pass, the
  deploy dry-run reports the expected count, and a temporary-tree prune test
  preserved config/work/libexec while removing `cmd/`, `systemd/`, `core/src/`
  and `Cargo.toml`. The live `/opt/hton` has not been redeployed (interactive
  sudo); the next `scripts/install` or `scripts/deploy` applies the prune.
- D129 — OR-31 implements one neutral SNI for REALITY and MTProto. By default it
  is the configured site mask domain; `--chain-sni` is an optional subdomain
  override. The owner selected a new HTON-owned Xray container. A sandbox chain
  (REALITY `serverNames=example.com`, `dest=mtg`; `mtg` domain-fronting to Caddy)
  returned the real `CN=example.com` certificate and `/var/www/html` body, and mtg
  logged parsing the ClientHello, proving the single-SNI chain carries TLS bytes.
  Implementation: `--chain-sni`; `protocols.prepare` creates a `selfsteal` Xray
  container that copies the native private key, short IDs and clients, listens on
  loopback, sets `serverNames` to the chain SNI and `dest` to mtg; mtg's generated
  config embeds the chain SNI and `[domain-fronting] host=127.0.0.1` to Caddy;
  HAProxy routes the chain SNI to the selfsteal container with Caddy as a
  health-checked backup; Caddy TLS policy/hosts include the chain SNI; certificate
  issuance requests the chain SAN; the MTProto share link advertises the chain
  SNI. The native `www.googletagmanager.com` route remains during client
  migration; native `serverNames`/`dest` are unchanged. The default `example.com`
  SNI is already covered by the live certificate; an optional `--chain-sni`
  subdomain would require a SAN certificate. No live deploy/install was performed.
  Real Telegram-client acceptance of the MTProto leg remains required.
- D130 — OR-32 makes the minimum first install argument-free. `protocols.selected`
  enables MTProto, Hysteria2 and Outline WSS when the state has no
  `protocolsChosen` marker, no optional protocol is active and neither `--with`
  nor `--without` is supplied; an old all-false feature map is treated as not yet
  chosen. The marker is written only after a successful install, so a failed
  first attempt retries the defaults. A saved selection remains authoritative
  after that. `chain_sni` defaults to the configured
  site domain when MTProto is enabled, MTProto camouflage defaults to that SNI,
  Hysteria2 defaults to UDP/8443, and the pinned Outline binary path defaults to
  `/opt/hton/bin/outline-ss-server`. Generated MTProto/Outline
  material remains under the protected minimum secret store. `--with`,
  `--without`, `--chain-sni`, `--mtproto-camouflage`, `--hysteria-port` and
  `--outline-binary` remain optional overrides. 52 unit tests pass.
- D131 — OR-33 adds per-user Outline WSS keys and usage accounting. The protected
  `secrets/outline-users.json` holds `{name, secret}` pairs; it is generated from
  native Amnezia client UUID suffixes when absent and can be edited before
  installation. `outline_config` renders one `id`/`secret` entry per user and
  `save_shares` writes one `share/outline-<name>.yaml` per user plus a summary.
  The service runs `outline-ss-server -metrics 127.0.0.1:<internal-port>`; its
  `shadowsocks_data_bytes{access_key,dir}` counters are the source of truth.
  `scripts/usage` scrapes the loopback endpoint, sums `dir=c<p|p>t` per key,
  persists deltas in `work/minimum/usage.json` with counter-reset handling, and
  prints total/session bytes. `hton usage` is a shell binding; a
  `hton-minimum-usage.timer` runs `--collect` every five minutes. This covers
  Outline WSS only; Amnezia/MTProto/Hysteria per-user accounting remains a
  separate future capability. No live install was performed; 52 unit tests pass.
- D132 — OR-34 makes `scripts/install` the rollout entry. The shell requires the
  current directory to equal the cloned source root when the source differs from
  the target, so an earlier skipped deploy from `/opt/hton` cannot recur. Deploy
  runs whenever `source != target` regardless of the caller's CWD; the outer
  privileged run preserves `SUDO_USER`, and deploy creates the `hton` group,
  adds that user and applies `root:hton` modes. The target setup then runs as
  root, which is required for Docker and systemd. Incremental mutations use
  `core/minimum/migrations.py`: a protected `work/minimum/migrations.json`
  ledger applies each registered migration once, in order. The initial
  migrations remove the obsolete single-key `share/outline.yaml` and
  `secrets/outline-secret`, remove the superseded `foreign-outline` module
  document, and normalize runtime/config directory and file permissions. Unit
  tests cover source-different deploy and source-equal setup paths.
- D133 — OR-35 migrates the native Amnezia REALITY listener to the own SNI inside
  `scripts/install`. The replacement container's `/opt/amnezia/xray/server.json`
  is rewritten before/while it is activated: `serverNames` gains the chain name
  (site domain or `--chain-sni`) plus the legacy name, `dest` becomes
  `<docker-bridge-gateway>:<mtg-port>` when the neutral chain is enabled and
  `<docker-bridge-gateway>:<site-port>` otherwise. Keys, short IDs, flow, clients
  and the native listener port are preserved. HAProxy routes the own SNI to the
  native backend with the site backend as a health-checked backup; the separate
  self-steal container is no longer created after migration. The migration needs
  an interactive `y/yes` consent, is validated with pinned `xray -test` through a
  read-only bind mount, stores the previous bytes under `work/minimum/native/`,
  and keeps separate original/replacement native digests so the parked original
  and the replacement can be verified independently. Unattended renewal does not
  migrate. The chain dependencies (mtg, and the legacy self-steal container) are started before HAProxy route verification; the first live attempt failed because the migrated native listener pointed at a not-yet-started mtg and the native backend was healthy, so no site backup was selected. Verification: 61 minimum unit scenarios pass; the rendered migrated
  HAProxy configuration passes `haproxy -c`; the pinned Xray accepts the rewritten
  REALITY config via `run -test`. No live install or container mutation was
  performed.
- D134 — OR-36 makes backward-compatible SNI the default. Isolated REALITY tests
  showed that with native `dest` pointed at the internal site chain, a client
  using `www.googletagmanager.com` is reset even when that name remains in
  `serverNames`; a single REALITY inbound cannot serve both the legacy fallback
  and the own-site fallback. The installer therefore keeps the native listener
  unchanged and the separate copied-clients listener for the own SNI by default.
  Native migration now requires the exact `DROP-LEGACY-SNI` phrase, and an
  installation migrated by the earlier draft is offered a restore from
  `work/minimum/native/previous.json` (or the parked original) on the next
  interactive install. `migrate_native(..., legacy=True)` clears the migration
  markers, records `legacySniRestored`, and keeps the migrated bytes under
  `native/migrated.json` as a rollback artifact. Unattended renewal keeps the
  currently accepted mode. Verification: 61 minimum unit scenarios pass; the
  legacy-SNI failure and the own-SNI/fallback behaviour were reproduced in
  disposable pinned-Xray containers.
- D135 — OR-37 adds a managed Amnezia provisioning mode alongside the existing
  association mode. On a fresh host the installer creates a pinned
  `amnezia-xray` container from an hton build of Amnezia 5.0.3's XRay core,
  generates one REALITY inbound and one guest UUID, runs the existing exact
  qualification/remap transaction, and records `nativeOwnership: managed`.
  The build removes the upstream image's `VOLUME`, uses `/usr/bin/xray` and
  `-config /opt/amnezia/xray/server.json`, and has no mount or Compose/Swarm
  labels. Admin access uses a dedicated `hton-amnezia` system user, an Ed25519
  key stored under `config/secrets/amnezia/admin-ssh-key`, and a validated
  `NOPASSWD: ALL` sudoers entry. The full-access `vpn://` follows Amnezia's
  admin export model (Qt `qCompress` level 8, Base64URL without padding); the
  guest link is a VLESS REALITY URI. Managed uninstall removes only recorded
  resources; a pre-existing user, key or container is never deleted. The
  plan task is `docs/plans/INSTALLATION-IMPLEMENTATION.md`; implementation
  remains blocked until the owner changes phase.
- D136 — OR-38 separates the public names and makes the web identity coherent.
  Amnezia REALITY uses `static.example.com`, Outline WSS `assets.example.com` and
  MTProto fakeTLS `media.example.com`. One `*.example.com` plus apex certificate
  and wildcard DNS record cover all names, so CT does not enumerate protocol
  names. Caddy serves the same site on the apex and all three names; HAProxy
  routes each exact SNI, and invalid authentication falls back to the real site
  rather than a default/error response. This supersedes OR-27 (MTProto stays
  third-party) and the single neutral SNI in OR-31, and changes the new-key
  REALITY default in OR-35 from `example.com` to `static.example.com`. Existing
  own-domain profiles remain valid while the wildcard certificate covers their
  name; third-party legacy names require the explicit `DROP-LEGACY-SNI`
  approval already required by OR-36. Engineering recommendations still
  pending owner acceptance are recorded in `docs/plans/INSTALLATION-IMPLEMENTATION.md`:
  replacing the 49-character hex Outline paths, moving Hysteria off UDP/8443
  and adding salamander obfuscation, reducing REALITY short-ID/metadata
  fingerprints, and removing default server banners. These are not yet binding
  requirements; the binding SNI, certificate and fallback rules are.

- D137 — OR-39 makes every SNI configurable; OR-38 names are saved instance values,
  not constants. Separate administrative SSH account creation reads `admin.user`
  from module config. OR-37 privilege scope remains root-equivalent. Owners:
  sdd-config, sdd-installer, sdd-foreign-adapters and INTERFACES; task:
  [managed provisioning](plans/INSTALLATION-IMPLEMENTATION.md).
- D138 — OR-40 requires a renewed masking review after the reported ban. Engineering
  design separates passive naming/TLS exposure, active probes, backend exposure,
  administrative access and IP reputation. Do not infer a cause from the ban alone.
  Owners: [hardening task](plans/INSTALLATION-IMPLEMENTATION.md), sdd-proxy,
  sdd-naive-mtproto, sdd-config and sdd-foreign-adapters. Private-first provisioning,
  independent protocol routes and final evidence matrix are engineering decisions,
  not additional owner promises of invisibility.

- D139 — OR-41 requires code-backed masking/DNS review. Findings and engineering
  remedies are in [SECURITY-MASKING-REVIEW](SECURITY-MASKING-REVIEW.md), traced to
  the hardening task and DNS/client/proxy SDDs. DNS privacy is not already verified.
- D140 — OR-42 prohibits public 8443; selected QUIC uses UDP/443, independently
  of TCP/443. Do not automatically mutate unassociated services. Resolve one
  UDP/443 owner and reject conflicts; no implicit UDP SNI multiplexing.
- D141 — OR-43 rejects foreign/otherwise uncovered unknown SNI at the edge but
  redirects undefined covered own wildcard subdomains to the configured apex.
  Exact routes precede wildcard. Engineering choice: HTTPS 308 to a fixed root
  URL without carrying untrusted Host/path/query; no arbitrary redirect target.
- D142 — OR-44 uses one shared Caddy for the website and REALITY fallback. The
  configured REALITY host returns HTTP 403 `Access denied` after ordinary TLS;
  Xray selects fallback, HAProxy does not intercept an HTTP error. The same Caddy
  serves `/var/www/html` through the existing admitted content workflow. No second
  server or `REALITY -> MTProto -> Caddy` chain in new provisioning. This custom
  response is an owner choice, not evidence of improved DPI resistance.

- D143 — Reconciliation of OR-32 with OR-38–OR-44: preserve argument-free initial
  MTProto/Hysteria/Outline selection and saved operator choices. Replace unsafe
  parameters, not the accepted composition. Missing prerequisites block activation
  with an actionable result. A recommendation to silently disable default modules
  was not an owner-approved replacement.
- D144 — Engineering freeze: selected delivery is minimum Python Core M1, not all
  R1. [Implementation plan](plans/INSTALLATION-IMPLEMENTATION.md),
  [managed Amnezia SDD](specs/sdd-managed-amnezia.md), config/DNS/proxy contracts
  and [Markdown file graph](sdd/INSTALLATION-FILE-GRAPH.md) define the next run.
  Existing JSON schemas/catalogue/file-index are baseline runtime artifacts; their
  precisely specified updates are implementation work, avoiding early live-parser
  changes in this Markdown-only phase. New image digest is produced by the future
  build, not fabricated as a planning prerequisite.
- D145 — Engineering ownership and removal: Amnezia app owns native client edits;
  HTON owns topology/lifecycle, preserves current clients during reviewed exclusive
  maintenance, and refuses drift. Root-equivalent external writers cannot be fenced
  by HTON's own lock. Remove/uninstall deactivate managed access and retain data;
  destructive purge remains separate. Pin the researched 5.0.3.0 source baseline
  and its Xray v26.7.28 recipe, then qualify actual app compatibility at final F0.

- D146 — OR-45 introduces an optional passwordless client-key page in the existing
  Caddy, configured by `core.clientAccessPage.path` (example default empty). Use a
  typed client-only inventory, exact route, safe static artifact, no admin creds,
  external resources, indexing, caching or secret URL logging. Scope/ownership:
  [client page SDD](specs/sdd-client-access-page.md), config, CLI, installer.
- D147 — OR-45 makes install `-q` fully noninteractive, including sudo (`sudo -n`),
  optional-path input and publication/manual retry prompts. Saved approved settings
  win; empty optional values disable their feature. Missing mandatory inputs,
  unavailable privilege or required ungranted consent return a typed nonzero result,
  not an invented answer or bypass of OR-36/public/foreign approval. Retain real
  errors/exit codes and suppress routine or secret-bearing output.

- D148 — Managed image runtime engineering baseline: bridge container UID0 with
  cap-drop ALL/no-new-privileges, no privileged/NET_ADMIN/host mounts. The app's
  native file-copy ownership must work; do not promise an unverified non-root
  profile. Keep a maintained pinned base and the researched Xray interface, not
  upstream privileged startup/tuning. Non-root optimization needs later evidence.

- D149 — Host-access home storage is a separate retained host-access subtree under
  the installation root, not 0700 config/work. Existing hton-group root traversal
  and dedicated home modes are explicit; deploy preserves this mutable tree.
  This avoids weakening protected Store permissions or depending on /home for
  platform data. Private SSH key authority remains config/secrets.

- D150 — OR-46 makes operator-group read the rule for all `/opt/hton` runtime
  state, replacing the "mutable trees stay root:root 0700" reading of D120/D149.
  `storage.modes()` returns 0750/0640 with group `hton` when that group exists
  and the legacy 0700/0600 otherwise; `Store` creates and admits both forms,
  adopts new entries to the group and falls back to legacy modes when the group
  cannot be applied. `storage.adopt_tree` normalizes an existing tree, and
  `install.main` runs it for `work/`, `config/` and a copied `.secrets/` before
  dispatch while `emit_ready` runs it after every successful install/renew, so
  writers that still emit 0600 artifacts (issuer output, snapshot archive,
  REALITY key) are corrected before the run ends. Migration 0002 applies the
  same modes. Deploy already ships 0640/0750 code; only transient lock files may
  stay root-only. Accepted consequence: `hton` members can read REALITY/Hysteria
  keys, issued certificate keys, DNS API keys and the managed Amnezia admin key,
  so group membership is credential-equivalent for VPN material (not root).

- D151 — OR-47 makes the website document root explicit. `ingress.site_root(configured)`
  validates an absolute, non-symlink directory holding a regular `index.html` or
  `index.htm`; a configured but unusable root is a hard error, never a placeholder
  fallback. `site.root` in `.secrets/profile.yml` is imported into
  `core.site.root`. When that is empty, `install.resolve_site_root` detects
  `/var/www/html` then `/var/www`, asks once on an interactive install whether to
  adopt it as the domain's default site (default yes), and persists the answer as
  `core.site.root` or `core.site.useExisting: false` so the question never repeats.
  Quiet, JSON and renewal runs adopt a detected site without prompting. Both flows
  pass the resolved path to `ingress.webroot` and `ingress.prepare`, which mount it
  read-only as `/site` for the existing `file_server` route. D122's generation
  identity continues to include the index bytes, so an edited site forces a fresh
  cutover while the mount stays read-only and `.git` stays hidden.

- D152 — Managed reinstall uses a real cutover for shared ingress ports. Before
  starting candidates, `run_managed` stops every tracked previous site/edge
  container (active plus `retainedIngress`) and the active protocol containers;
  `rollback_managed` restores suspended ingress, active protocols and a native
  container that was already running, and a committed cutover removes the retired
  ingress/protocol containers and clears `retainedIngress`. This fixes the
  observed failure where HAProxy/Caddy `SO_REUSEPORT` let the 443/9443 probe be
  answered by an older generation serving the placeholder (16 of 20 sampled
  connections), so the first attempt could never be trusted and repeated installs
  accumulated live ingress generations. The probe error now reports served and
  expected body sizes.

- D153 — OR-48 makes repository anonymity a build gate. The personal domain
  previously cited in OR-47's source line is removed; provenance keeps only the
  date and the substance of the report. `.secrets/` remains a separate ignored
  repository and is asserted untracked. `tests/anonymity` scans every
  `git ls-files` entry (text only, bounded size) for owner markers assembled
  from fragments, so the gate does not itself contain the identifiers it bans.
  Runtime domain values stay in `.secrets/profile.yml` and host state, never in
  the published tree.

- D154 — OR-49 uses the existing client-access page as the publication
  mechanism for guest credentials. `core.clientAccessPage.path` stops being
  empty-only: install generates a CSPRNG path when publication is enabled and
  `client_page.plan` keeps validating exactly one safe segment. Caddy serves the
  page from `work/client-page`, never from `site.root`, so OR-47's read-only site
  mount is unchanged. Inventory already covers `outline-wss`, `mtproto`,
  `hysteria` and native Amnezia guests; `work/minimum/share/*` stays the operator
  copy. Rotation replaces the path and the page digest, so a cached URL stops
  resolving immediately.

- D155 — OR-50 makes `origin/main` the deployable artifact during active
  development. The provenance/SBOM/signature pipeline is not a gate for install
  from a checkout; it returns as a release-phase obligation. `scripts/deploy`
  already performs no provenance check, so this only corrects plan and status
  text that claimed the gate was active.
- D156 — OR-51 defers native remap/restore and the associated restart-owner
  qualification. `installation.rs` keeps returning the explicit
  "remap execution is not available" error; no install path may require it, and
  the native listener is used as-is.

- D157 — OR-52 publishes the accepted guest inventory and the bearer URL into
  the private `.secrets` repository. `install.ensure_client_page_path` treats an
  absent `core.clientAccessPage.path` as automatic publication, an explicit
  empty string as the operator's opt-out, and `--rotate-client-page` as explicit
  replacement; `secrets.token_urlsafe(24)` supplies the one safe segment.
  After a successful install `publish_secret_share_artifacts` writes
  `amnezia-<name>.md`, `mtproto.md`, `hysteria.md`, `outline-<name>.yaml` and
  `client-page.md` to the runtime `.secrets/share` and to the source checkout's
  `.secrets/share` when present, preserving owner/mode `0640`, removing stale
  files from the previous published set and updating the profile's `path`.
  Publication happens only after success, the page keeps being served from
  `work/client-page`, and `client-page/receipt.json`/`secret-share.json` record
  only artifact names and coverage, never the URL. The examples omit `path` so a
  fresh install publishes; an explicit empty value remains authoritative.

- D158 — OR-53 fixes the log retention window at seven days. The minimum
  installer prunes `minimum.log*` by file age and rotates the active file when its
  first entry is older than the week, so no entry survives past seven days even
  under steady low-volume writes. The Core/runtime projection defaults
  `logging.retentionDays` to 7, accepts `1..=7`, and the schemas cap the value at
  7; a shorter configured window is still honored. Rotation remains size-bounded
  and log pressure never blocks VPN traffic or recovery. This supersedes the
  earlier “numeric defaults remain implementation choices” wording for retention
  value only, not for the size ceiling.

- D159 — OR-54 makes the private `.secrets` repository the only required
  durability mechanism and removes separate backup/restore from R1. The
  `backup` command binding, `scripts/backup`, its CLI/help row and file-index
  unit are removed; no `/opt/hton/storage/` backup store, archive retention or
  restore mode remains in R1 scope. `work/` is treated as regenerable. This
  supersedes D70, D72 and the Q32 archive portion for R1; the Rust archive
  modules are not exposed as product operations and await removal with the
  deferred full-R1 code reconciliation, not a compatibility shim.

- D160 — OR-55 makes install an updating operation and adds a weekly unattended
  update. `lib/hton` fast-forwards `main` from `origin/main` (`git fetch
  origin main` + `git merge --ff-only`) before delivery; the minimum Python
  `updates.py` owns the redacted status record, the seven-day policy and the
  protected `work/minimum/update.json` stamp. `scripts/install` treats this as
  best effort (offline or dirty/diverged checkout keeps the current source and
  still refreshes images), while `scripts/update` requires the fast-forward and
  then runs the non-interactive minimum `update` action. The update scope is the
  HTON-owned composition; independent VPN/host containers are excluded, which
  resolves the OR-17 scope question in the conservative direction. The installer
  generates and enables `hton-minimum-update.service`/`.timer`; the timer checks
  daily but the action is due only after seven days without a successful
  install/update, so a fresh explicit install suppresses the weekly run. The
  unattended action never public-checks DNS, never prompts, never accepts native
  client-list drift and requires committed state. Unit tests cover the policy,
  the timer text, the stamp and a real fast-forward in a temporary repository.
- D161 — Extend D117's lifecycle normalization to Docker's restore re-encoding of
  `HostConfig.Dns`. After a host reboot the daemon rewrites a nil `Dns` recorded at
  create into `[]`, so every owned site/edge/protocol container created before the
  reboot failed `owned()` with "Owned native launch drifted" although nothing had
  changed. `docker.canonical_host_config(..., normalize_restored_lists=True)` (used
  by `launch_digest`) maps `Dns: []` to null, the create-time form; Go treats both
  as the same zero value. `check_foreign` applies the same normalization to both
  the protected expectation and the observed foreign container. Receipts recorded
  before this fix already captured null, so they still match, and a real `Dns`
  list change still refuses mutation. Verified against the live Docker 29.9.3
  socket: all eight owned containers (active and candidate) validate, and their
  artifact claims still match.
- D162 — Native client names widen from lowercase ASCII to the OR-56 form.
  `clients.NAME_PATTERN` now accepts a leading letter/digit and then Latin or
  Russian Cyrillic letters in any case, digits, `_`, `.` and `-`, still bounded
  to 32 characters and still rejecting spaces, slashes, leading punctuation and
  empty names. The xray `email`, the share title and the link fragment keep the
  exact case; only `slug()` lowercases and replaces non-alphanumerics for the
  file name. `scripts/clients add`/`rm` prompt for a missing name on a TTY and
  `rm` confirms before mutating. `list` needs only the Docker socket group;
  `add`/`rm` still require root because the share artifact must be written
  `root:hton 0640` under the OR-46 rules. No protection rule changes: the
  protected name is still the exact lowercase `guest`. The configured initial
  guest `provision.clientName` uses the same `clients.NAME_PATTERN`; generated
  share artifact names accept Unicode word characters (no separators or leading
  dot), so a Cyrillic guest yields `amnezia-гость.md` without weakening the
  file-name fence.
- D163 — Separate the app connection name from the credential identity (OR-57).
  `managed_amnezia.client_inventory` gains an optional `link_label` used only for
  the `vless://` fragment; `displayName` stays the native `email`. `installation`
  resolves the label from `core.clientAccessPage.names[credential]`, else
  `instance.label`, else the credential id, and passes it to the ready summary,
  share publication and client-page refresh. `scripts/clients` resolves the same
  precedence from the installed `config/hton.yaml`. `instance.label` is validated
  as 1–64 printable non-whitespace-only characters (spaces/emoji allowed) and the
  root v1/v2 schemas accept the optional field. The username never disappears: it
  remains the xray identity, share filename and client-page row identity.
- D164 — Per-user share bundles (OR-58). `secret_share_artifacts` groups native
  credentials and Outline identities by credential name into one `<slug>.md` per
  user, appends the shared MTProto/Hysteria links and the client-page URL, and
  keeps `client-page.md` as the operator copy. Colliding slugs get a credential
  digest suffix; stale old-layout files are removed through `secret-share.json`.
  `scripts/clients add/rm` writes/removes the same bundle in every existing
  `.secrets/share` (installed and source checkout), preserves Outline/client-page
  sections it does not own, and derives file ownership from the private secret
  root instead of leaving `root:root`; the old `amnezia-<slug>.md` is removed.
- D165 — Record the OR-59 protocol prompt as blocked on AmneziaWG2. The existing
  minimum composition already reports and selects MTProto/Outline/Hysteria through
  `protocols.selected` and `--with/--without`; no interactive per-user protocol
  prompt is added yet because its requested default (AmneziaWG2) had no
  implementation. Adding AWG2 as a bare option would have been a dead selection,
  not a protocol. Superseded in part by D174: the `amneziawg` endpoint is now
  implemented and selectable; only the separate interactive per-user prompt
  remains deferred.
- D174 — AmneziaWG2 is an optional minimum-profile protocol (OR-66). The pinned
  `docker.io/amneziavpn/amneziawg-go:3.1.20260828` image carries `awg`,
  `awg-quick` and `amneziawg-go`. `protocols` allocates a persistent public UDP
  port from 5000-6000, generates one server key and per-peer X25519/preshared
  keys, persists `Jc/Jmin/Jmax`, `S1-S4` and non-overlapping `H1-H4`, and renders
  a bridge-network container with `/dev/net/tun`, `NET_ADMIN`, contained NAT and
  a `/32` server peer route. Client keys are exported as complete `.conf` files;
  the server config and `ListenPort` are never exported. `access_links` has a
  dedicated strict client parser because the generic admin-marker scan would
  otherwise reject the expected client `PrivateKey`. The port, keys and peer
  files survive disable/re-enable; normal candidate/active receipts own the
  container. The exact image is amd64-only in the current tag, so arm64 hosts
  fail closed at image pull instead of silently selecting another engine.
- D166 — Remove client-facing `hton` branding and name the native protocol
  (OR-60). The native fragment is `connection_fragment(label)` = `<instance.label>
  XRay` or just `XRay`; a `clientAccessPage.names` override still wins. Hysteria's
  ready-summary link fragment is `Hysteria2`; the Outline fallback identity is
  `guest` instead of `hton`. `instance.name` remains internal. This supersedes the
  bare country-only fragment from D163.
- D167 — One admin client-access URL and page availability independent of
  unrelated diagnostics (OR-61). The page remains one bearer path for every user;
  per-user URLs are not introduced. `configuration.client_page(root)` parses only
  `core.clientAccessPage`, and `refresh_client_page` uses it whenever full
  validation has no `desired` root (for example the live `wildcardDomain` +
  `http-01` mismatch), so the accepted snapshot is refreshed instead of withdrawn.
  Native inventory and the connection label are still taken from the root
  document when `desired` is absent. Publishing the page still requires the
  install transaction to render the Caddy route and mount `/client-page`; the
  timer alone refreshes the artifact. The live outage was a missing route/mount
  plus a receipt withdrawn every minute, not a wrong secret path.
- D168 — Plain HTTP redirect owned by the site listener (OR-62). When a real site
  root exists, `ingress.configs` adds a second Caddy server on `0.0.0.0:80` with
  automatic HTTPS disabled: `/.well-known/acme-challenge/*` is served from `/site`
  and every other request gets `308` to `https://{http.request.host}{http.request.uri}`.
  The server is omitted when the install uses the placeholder, so TCP/80 remains
  free for standalone HTTP-01. The Caddy container already runs with host
  networking and `NET_BIND_SERVICE`. With port 80 permanently owned,
  `http_challenge.select` falls through to the proved webroot on renewal; the
  probe writes into the site root that `/site` exposes. HAProxy stays TCP/443.
- D169 — REALITY names are covered by the trusted site certificate (OR-38,
  OR-44). The managed install requests every own-domain REALITY name
  (`install.reality_certificate_names`, subdomains of the site domain except the
  apex) in the same ACME certificate as the site and loads only
  `/certs/fullchain.pem` into Caddy. The previous `plan.server_names[1:]` slice
  dropped a lone REALITY name, and the generated self-signed
  `reality-fullchain.pem` violated the retained trusted-certificate requirement.
  Third-party legacy names cannot be issued and keep only HAProxy routing. The
  REALITY browser host still returns the OR-44 403 body; it must arrive over a
  trusted certificate.
- D170 — Apex REALITY, www alias, wildcard issuance and a fixed denial body
  (OR-63). When `foreign-amnezia.serverNames` contains the site domain, the
  managed install renders the native listener as a self-steal: HAProxy routes the
  apex to the native backend with the site as a checked backup, the native
  `dest` is the site through the Docker-gateway bind, and no REALITY 403 route is
  emitted for the apex, so browsers reach the normal site through the fallback.
  Other REALITY subdomains keep a fixed `Access denied` 403 body instead of the
  site HTML (`ingress.DENIAL_BODY`). Caddy also serves `www.<domain>` as a 308
  redirect to the apex with path/query preserved and allows it in HAProxy.
  Certificates request `www.<domain>` plus every own-domain REALITY subdomain;
  when `core.certificates.wildcardDomain` equals the site domain and the
  challenge is `dns-01`, the issuer additionally requests `*.<domain>` and
  validates that the issued certificate carries the wildcard SAN. Third-party
  legacy names remain routing-only. Live wildcard issuance stays blocked until
  real InternetBS DNS credentials replace the protected placeholders; until
  then `www` is covered by per-name HTTP-01.
- D171 — Install console shows the page URL and share directory, not credential
  links (OR-64). `emit` prints `Client access page: https://<domain>/<path>` and
  `Client files: <installation>/.secrets/share` (mode note); the per-link list
  and per-file listing are removed. JSON output keeps its existing structured
  `shareFiles` paths without the URL. The URL remains excluded from receipts,
  plans and logs and from quiet/JSON runs; only the local human summary shows it.
- D172 — Interactive install migrates protocol subdomains to the apex (OR-63).
  `apex_server_names(domain, names)` replaces a name list consisting only of
  subdomains of the site domain with `[domain]`; lists containing the apex or a
  third-party legacy name are left unchanged, and renewal/update never rewrite
  names. `run_managed` also recreates the native container when its saved
  `serverNames` differ from the plan, so the apex is actually rendered into the
  native config and client links. The console prints one stderr notice that the
  names were dropped and links must be reissued.
- D173 — Deploy crosses by a reverse SSHFS mount, not a temporary branch
  (OR-65). `scripts/hephaestus/deploy-taenarum` (alias `deploy`) serves `/work` with the
  container's
  `sftp-server` and mounts it read-only on the server through `sshfs -o passive`
  over one SSH connection; `scripts/install` runs from that mount with
  `HTON_SOURCE_UPDATED=1`, so it never fast-forwards the mounted tree. The
  `ferry` service holds only the product closure (allowlisted bind mounts),
  `.deploy/config` and a separate server-only key; `.git`, `.secrets`, `.env`,
  `.deploy` and the rest of the tree are not mounted at all. Server
  prerequisites (`sshfs`, `fuse3`, `user_allow_other`, the authorized ferry key)
  are created by `scripts/hephaestus/bootstrap`. Temporary branches,
  `SERVER_GIT_URL` and server-side Git leave the daily deploy path; `push` still
  audits before publishing. The temporary-branch transport is preserved as the
  explicit fallback `scripts/hephaestus/deploy-git` (owner naming, 2026-11-09), which
  is the only deploy path that still commits and still needs server repository
  access; `deploy` remains an alias of `deploy-taenarum`.
- D175 — Test-only reverse-tunnel ingress (OR-67). `core.edge.testTunnel`
  carries an optional `sni` and loopback `port`; `ingress.configs` adds the test
  SNI to the HAProxy allow-list and routes it to `127.0.0.1:<port>` before the
  production protocol routes. The test SNI is rejected when it collides with the
  site, `www`, a REALITY name, the MTProto camouflage or the chain SNI. The
  route is off unless `scripts/install --test-sni ... --test-tunnel-port ...`
  is explicit; `--no-test-tunnel` removes it. The route adds no public listener
  and is not a VPN protocol. Echo owns the reverse SSH forward, the local
  stand's extra REALITY server name and the local `abyss-echo` client;
  its `scripts/echo configure` passes the two install options through
  the configured Hephaestus deploy transport; both the taenarum and Git transports
  forward the pair and reject a half-set pair. The SSH forward is TCP-only, so
  the route covers VLESS/REALITY/Trojan/WSS-style tests, not QUIC/UDP.
- D176 — Triodos/HTON project split (OR-68). The public ingress is extracted into
  the independent `triodos` project; HTON keeps only VPN engines, account/key
  inventory and private listeners. Triodos owns HAProxy classification, Caddy,
  ACME, the site, public TCP/80/443/853, route policy, pools and health checks.
  Triodos configuration is the single authority for the public route map; it
  exports a versioned grant consumed and cached by HTON, so accepted HTON boot
  does not depend on a live Triodos process. Pools may balance only members with
  the same `identityDigest`; a mismatch is rejected. The split is phased:
  current runtime and stand stay unchanged until a separate phase authorization;
  then ingress extraction, HTON private mode plus grant import, stand cutover,
  deployment/lifecycle, live cutover and cleanup follow in that order. This
  decision authorizes design/scaffold/migration planning, not runtime changes,
  deployment or service startup. Triodos owns its own `OWNER-REQUIREMENTS.md`
  and copies OR-68's boundary there as OR-T1; the two ledgers must agree on the
  boundary while the projects remain separately versioned. Open: public UDP/443
  ownership, DoT/Unbound placement, client-page handoff and cross-member
  credential replication.
- D177 — Product platform and service ownership boundary (OR-69). HTON is the
  server platform, D6ton the client platform, and Tartarus the development-
  environment platform for deployment and connectivity maintenance. Onomat owns
  the VPN service and Unbound under `hton/services/`; VPN engines, protocol
  configuration, private listeners, key application and data-plane lifecycle
  belong to Onomat's service boundary. HTON supplies platform lifecycle,
  configuration and validation mechanisms and coordinates Onomat through
  contracts. Preserve the ongoing Unbound work. Current monolithic runtime and
  stand remain until a separately authorized migration phase. Source: OR-69,
  clarified by the owner's follow-up “перенеси впн в ономат!”.
- D178 — DNS runtime ownership and location (OR-70). The Unbound runtime source
  moves from HTON into the Onomat repository under `runtime/dns/`: the Python
  policy/render/lifecycle module, the Rust engine renderer, the Unbound
  configuration template and the contract tests. HTON remains the consumer and
  applier: `hton-core` includes the moved renderer, the accepted bundle carries
  the rendered `unbound.conf` and `root.key`, and the platform starts the private
  Unbound service. The Onomat console service remains optional and gains no
  Docker socket, engine write path or canonical state. No duplicate DNS renderer
  stays in HTON. This implements the ownership recorded by OR-69 and authorized
  by OR-70; the VPN service extraction and the DoT termination in Triodos are
  unchanged. Source: OR-70.
- D179 — D6ton is a full multi-node VPN platform (OR-71). D6ton is intended for
  client devices and server/relay nodes, using one product/profile model to
  compose multi-hop routes. Its “client” role is relative to the next VPN node;
  no reduced server fork is created. D6ton chooses sky, direct-earth and
  chthonic paths; Charon carries the selected VPN traffic over the future Styx
  transport to Triodos. This does not change HTON's server-platform lifecycle or
  Onomat's ownership of the HTON VPN/Unbound services, and it authorizes no
  deployment or runtime cutover. Source: OR-71.
