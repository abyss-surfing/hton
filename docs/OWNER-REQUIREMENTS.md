# Binding Owner Clarifications

## Current phase: authorized M1 implementation slices (2026-11-07)

The owner explicitly authorized the OR-49/OR-52 client-page and private-share
implementation, then instructed action on the existing task list, builds and
one-week log rotation. Those M1 slices are implemented in the minimum Python path
and the Rust workspace build is recorded in `IMPLEMENTATION-STATUS.md`. This does
not authorize full-R1 integration, launching implementation agents, deployment to
an external server, service startup or image publication. Full R1 remains deferred.
The earlier 2026-11-06 “planning only” record and the 2026-10-31 implementation
authorization are historical, not erased.

On 2026-11-12 the owner authorized the **design** of a project split: the public
ingress moves into a separate `triodos` project and HTON remains the VPN server
(OR-68). This pass is limited to requirements, architecture, scaffold and a
migration plan. It does not authorize runtime cutover, replacing the current
stand wiring, deploying either project or launching implementation agents. The
current HTON monolith remains the working data plane until a separate owner phase
change authorizes the migration slices.

Source: owner instructions dated 2026-11-07 (OR-49/50/52–54) and the current
“actualize tasks / build / one-week log rotation” instruction.

Source: owner’s detailed response dated 2026-10-31. These are requirements, not agent recommendations. Changing a decision requires the owner’s explicit consent. This document clarifies and, in case of conflict, supersedes earlier wording in DECISIONS, SDDs, schemas and file maps. Technical options below are clearly distinguished from approved requirements.

## OR-1. Resource-efficient operation

Target a VPS with 1–2 GB RAM, about 50 devices and dozens of concurrent connections. This is a target scenario, not a test-verified limit. Resource efficiency is a required architectural property; a strict load estimate does not block the project. A more powerful server or two or three smaller servers may be used if needed. The latter does not imply a requirement to provision an automatic cluster. Resource-intensive protocols are disabled as separate modules. Follow the Unix way: simple, atomic, self-contained solutions and ready-made VPN engines.

## OR-2. Management and iterations

The first iteration provides self-sufficient console management, available to any properly authorized OS user; the specific permission model remains to be decided. The console does not depend on the web admin. Declarative configuration retains all configurability.

In the first iteration, design and implement the API interfaces and configuration contracts needed for the later web admin and user accounts, so adding a UI does not require rewriting core logic. This does not require implementing all future account functionality in advance. User accounts are needed and must be part of the later web admin. The web admin is an optional, disableable module; its absence does not limit console management.

Traffic accounting is not needed in either the first or second iteration; it is a long-term item. Do not add it to R1/R2 readiness criteria.

### API, Core and web admin clarification (2026-10-31)

HTON Core is a required part of the project and is used by console commands; commands work independently of the management API. The management API is a separate, self-sufficient, full-featured service and an optional module that does not need to run as a daemon in the first iteration. The future web admin always calls this API; the UI does not bypass it or write configuration directly. The web admin is a separate Docker module. Configuration and operations must be available through the API to different clients (web, GUI, application, MCP) without semantic changes.

Each module has its own declarative configuration. Configuration changes must preserve API compatibility. Checks must detect both syntax/schema invalidity and contradictions between settings: each module provides its own check, and a shared mechanism runs the checks and consolidates the results. Avoid configurations that break automation.

Use ready-made third-party modules unchanged where possible (for example, a VPN application Docker image). Place HTON-owned configuration, API logic, validators, tests and other logic natively or in a separate Docker container.

### First and second iteration clarification (2026-10-31)

R1 has no web admin. A user with console access is considered an HTON administrator. Traffic accounting is not part of R1 or R2. IPv6 is also deferred to the long term; IPv4 is sufficient for R1.

## OR-3. Installation location

Install the entire platform under `/opt/hton/`; do not distribute it across `/var/lib` or a home directory. `/run/hton` is suitable for runtime state and sockets. Until package delivery through apt or similar exists, do not change global system configuration without need. Changing the user, ending their session or deleting their home directory must not determine the platform lifecycle.

## OR-4. Entry port and website

The platform combines ready-made VPN services behind a single TCP/443 port with a genuine HTTPS certificate for its own domains. HAProxy or its future replacement must keep both VPN and the ordinary website working when management fails. The website and web admin are separate destinations. A plausible placeholder ordinary website is allowed during implementation. This does not authorize changing an existing external server without established access context.

Disabling the web admin, cold start, or absence of its DNS/backend must not break startup or reload of the entry proxy or the website/VPN. UDP and protocols with special requirements retain their explicitly described limitations; a shared HTTPS port does not turn every VPN protocol into HTTPS.

### Domain and installation clarification (2026-10-31)

The installer must check whether a domain is already configured or help start its setup. If no domain exists, it offers text instructions for obtaining a free domain and an example; third- or fourth-level domains are acceptable. Installation may finish without a domain/publication, but must report a high server-blocking risk if that risk applies to the chosen publication method. IPv4 is required in the first iteration; IPv6 is a long-term item.

### R1 and Amnezia clarification (2026-10-31)

R1 includes Xray and the already-installed Amnezia VPN; Amnezia must work with a genuine domain certificate. Prefer reinstalling/configuring Amnezia with a domain through its native process if its installer provides the certificate and compatibility. Do not develop an additional Amnezia certificate-management layer without a separate decision. Amnezia remains an independent external component: HTON checks compatibility and port conflicts and offers safe remapping if needed. Verify the native scenario and its limitations before implementation.

### Accepted change to certificate scope for Amnezia REALITY (2026-10-31)

After researching primary sources, a specific option was proposed to the owner:
if the installed native VPN uses the researched REALITY protocol, preserve it
through transparent TCP passthrough, and require a publicly trusted certificate
for the owner’s domain on the ordinary HTON HTTPS site. The owner replied “+” to
this recommendation; this accepts that proposal and adds no unstated requirements.

In this scenario, the earlier requirement for a publicly issued certificate
directly in the Amnezia REALITY handshake is replaced by preserving the native
REALITY semantics of the temporary authentication certificate. This is not
ordinary TLS termination and does not promise support for every native Amnezia
protocol. The exact version/resources, serverNames/target, passthrough compatibility,
absence of routing loops and restart owners must be established before implementing
a specific integration. This acceptance does not remove association/approval for
third-party resources or authorize a new certificate manager. Research: [AMNEZIA-NATIVE-EVIDENCE](AMNEZIA-NATIVE-EVIDENCE.md).

### External uplink boundary clarification (2026-10-31)

Compatibility with a separately configured VPN client and HTON server chains remains a goal. HTON does not own the external client configuration; the specific traffic handoff and route selection require a separate decision before integration.

### Requirement precedence during document reconciliation (2026-10-31)

When derived documents conflict, owner requirements apply: Docker candidate checks are optional, IPv6 is deferred long term, and the CLI follows the agreed module lifecycle (`add/remove`, `enable/disable`, separate `purge` and image update, `up` starts the saved composition). Derived specifications must be reconciled before claiming readiness.

## OR-5. External uplink and server chains

The hton VPN server must fully use a separately installed VPN client, such as sing-box, as an uplink to another server/region. Chaining hton servers must work. Configuring sing-box itself is outside this project: hton integrates with a ready-made external uplink and does not become a second owner of its configuration.

Configuration and operation of the VPN client, including `scripts/up`, `scripts/down` and selecting an ordinary or VPN route, belong to a separate repository and separate logic. This is not part of the hton VPN server; do not add uplink integration to hton without a new owner decision.

## OR-6. Changes and checks

The architecture may be revised; the expected primary user operation is enabling/disabling modules. Docker candidate checks are optional, allowing complex changes to be checked in advance and reducing downtime. They are not a required gate for every configuration application. Native validation, preserving working state, bounded application and recovery remain required safety mechanisms.

## OR-7. On-demand web admin

To save resources, the owner proposed stopping the admin container after an hour of inactivity and starting it on access. This is an option authorized for investigation, not an already approved implementation. The key constraint is that the mechanism must not disrupt HAProxy, the website or VPN. Determine separately which requests start the container, what counts as activity and how unfinished operations are handled. The frontend cannot start its own stopped backend; an independent, bounded wake-up mechanism is required.

## OR-8. Explicit external diagnostics

Do not make public IPv4/DNS requests automatically in general diagnostics,
startup or other background operations. External identity observation is allowed
only when the owner explicitly invokes `identity.whoami`; local `diag.observe`
makes no network requests. Unverified external fields must be labeled as such,
without presenting assumptions as facts.

Source: direct owner clarification during implementation of `diag.observe` and
`identity.whoami` (2026-11-02). This restriction preserves WHOAMI as an explicit
owner-facing command and does not extend it to other operations.

## Console management clarification

All settings and operations must be self-sufficient through `scripts/*` commands, without the web admin. `scripts/modules` manages module lifecycles: it adds and removes modules, installs/obtains required Docker images and resources, creates configs from placeholder templates, and requests missing values or user actions. For a complex step, the CLI explains it in text and asks the owner to perform it manually or assign it to an AI; the CLI runs shell commands from the instructions itself when appropriate.

`scripts/up` starts the entire HTON daemon and all connected modules according to the saved platform composition. It does not select and start one individual module. A removed module is excluded from the startup composition. Exact rules for add/remove, data preservation and image updates remain to be defined.

## Module lifecycle clarification (2026-10-31)

`add/remove` manage the installed composition, while `enable/disable` control whether an installed module participates in startup. `remove` preserves data; irreversible data deletion requires the separate, explicit `purge` action. Image update is a separate operation. `up` starts the entire saved composition and does not update images automatically.

## Product capability clarification (2026-10-31)

Client installation for OWRT is already planned; consider it an existing part of router support, not a new idea. Do not propose installing on all user devices as one mandatory complex operation: device onboarding must remain incremental and optional.

Health checks are already part of the functionality. Do not describe them as a missing capability. Automatic self-repair without AI is unlikely; the owner considers that AI will make repair available anyway. Therefore, do not treat autonomous self-repair as a product gap or priority recommendation. Any future AI integrations require a separate design and are not implied by this clarification.

## Local client protocol selection clarification (2026-10-31)

The owner wants a simple PowerShell/Bash tool, run on the user’s device, that
quickly checks available protocols/profiles from the current network and helps
select a suitable client with a good balance of speed and reliability. The approach
may later expand to Android, iOS and other platforms. This does not require
immediate support for every OS or automatic client installation. Results must
reflect measured conditions and test limitations rather than promise a universally
“fastest” option.

## Logging clarification (2026-10-31)

For fast analysis and repair, including with AI assistance, HTON must keep
detailed logs. Logging/debug level is set in configuration. All HTON logs are
stored in `/var/logs/hton`; the installer configures the path.

## Module development principles clarification (2026-10-31)

Each HTON module must be developed according to SDD (Software Design
Description), DRY, SOLID and GRASP. The contract, responsibility boundaries,
dependencies, checks, operational effects and relationship to the overall
architecture are documented before or alongside implementation; a module must not
create duplicate business logic or bypass shared configuration and application mechanisms.

## Log and resumable installation clarification (2026-10-31, Q9–Q11)

At every debug level, logs hide private keys, passwords, access tokens and full connection links. For repair, including when logs are shared with AI, retain the object ID, action, result and error reason.

Automatic log rotation is required; total size and retention limits are configurable, with specific values chosen during implementation. A full log must not stop a working VPN, especially on the target small VPS.

When installation partially succeeds, completed steps are retained and a rerun can resume installation. The CLI clearly reports what is installed, what is running, what is externally reachable and which step is blocked; for example, certificate issuance failure must not require repeating completed Docker and HTON installation.

## Confirmed architecture clarifications

### Named objects and lists (2026-10-31, Q25)

Named configuration objects, such as profiles and connections, are stored as mappings with stable identifiers. Use lists where ordering matters; the module defines their update semantics. This allows recursively merging changes to a specific object regardless of its position in a list.

### Deferred configurator and parameter preservation (2026-10-31, Q22–Q24)

The configurator, migrator and API will be implemented later; for now, their architecture and configuration structure must be established. Configuration should be sufficiently nested for convenient recursive merging, adding new parameters and explicit renames. With a full configurator/migrator, `.env` is not needed as an additional HTON settings format; avoid multiplying parallel formats and related problems.

Q23 is confirmed: Docker configs in use can be edited directly or through the API; customizations are preserved and checked against module constraints. This is an already discussed requirement, not a new choice.

The accepted alternative to proposal Q24 is that a versioned validator determines supported parameters. Unsupported/unused keys are retained in the corresponding `*.deprecated.yml`; the user decides whether they are needed. A key may be a typo or a secret that must not be irreversibly deleted. Changes through the configurator/migrator must, by definition, not lose original values. For every transformed parameter, the migrator has a specific strategy: remove it from active configuration, rename it, clear it, and so on. Removing/clearing the active value does not mean destroying its preserved original. Protection of archived values must account for possible secrets.

### Unified storage for editable configuration (2026-10-31)

The owner requires one storage strategy: shipped Docker configs/templates are in Git; in-use and updated configs are stored together, separately from the distribution and accessible through the API. Docker config customizations must be preserved; an update reads the active config and applies necessary transformations, such as adding or renaming fields. A complete architecture is required for API-based editing and safe software upgrades. The API must not access `/etc/***` for these configs.

Mutable configuration and secrets must be placed so that one directory can be archived separately from the project’s Git sources. Preferred root: `/opt/hton/config/`, for example with `secrets/` and `docker/` inside; the agent is tasked with the detailed structure. Do not use a separate absolute `/var/secrets`. A configuration archive must not automatically be treated as a backup of all service data.

Q21 is accepted: the common config and module configs have format versions; sequential migrations apply to format changes, not every software release. A missing or empty migrator alone is not a reason for a script to fail. This does not remove required configuration validity checks. Full upgrade implementation remains a long-term item; for now, the storage and configuration change architecture must be established.

### Storage and migration architecture priority (2026-10-31)

The current priority is an architecture that avoids later rework and a decision on configuration storage: where and what to store, and whether to use `.env` or YAML. Full upgrade implementation is a long-term item. The owner proposed sequential config migrations between versions, including adding fields and renaming variables: when skipping multiple versions, run the full transformation chain. This is a mechanism proposal for design, not a requirement to implement a set of scripts immediately. The goal is to preserve configuration and secrets when the format changes.

### General upgrade and restart (2026-10-31, Q14)

After `scripts/down`, `scripts/up` starts the entire saved enabled composition, including Amnezia, without reinstalling or updating images. Stopping preserves settings and data.

`scripts/upgrade` runs `git pull`, updates configs without losing secrets, invokes `scripts/modules update XXX` and performs the remaining necessary platform update logic. Each module contains its own update logic. The owner asked for a proposal for safely updating configs; exact migration, failure-handling and module-selection rules remain to be decided.

### Applying composition and global start/stop (2026-10-31, Q12–Q13)

By default, module participation changes are applied immediately after checks, showing expected connection interruptions. A separate mode saves changes without applying them, allowing several changes to be batched into one operation (Q12).

`scripts/up` turns the HTON server on. `scripts/down` turns off everything associated with the service, including the independent external Amnezia. Therefore, independent ownership of Amnezia configuration does not exclude it from global shutdown. This owner clarification replaces the assumption that Amnezia’s external status alone forbids global shutdown. Invalid configuration handling is clarified in Q15 below.

### Starting with invalid config and persisting shutdown (2026-10-31, Q15–Q16)

With invalid configuration, `scripts/up` reports an error and offers a separate, explicit start using the last working configuration; it must not automatically replace the configuration with the working copy.

After `scripts/down`, HTON and associated Amnezia remain off after a VPS reboot. `scripts/up` turns the server on and restores autostart at OS boot. Shutdown preserves configuration and data.

## Open details

### Atomic parameter changes through the API (2026-10-31, Q34)

Q34 is accepted with a clarification: the configurator API atomically saves a specific set of parameter changes rather than accepting a whole file as a replacement. Concurrent manual edits must not be silently overwritten. Independent changes are preserved; conflict detection must consider affected values and operation conditions rather than unconditionally rejecting every file change. On conflict, prepared edits are retained for reapplication after rereading the current configuration. The specific update and check semantics must ensure atomicity of the entire parameter set.

### Self-sufficient CLI and configurator resilience (2026-10-31, Q33)

Q33 is accepted: R1 retains manual YAML editing, checks and safe application through CLI/Core. Automated editing, merging, migrations and maintenance will be implemented later in a standalone module that works without the web admin. The CLI may call it directly; the public API provides token-based access.

The owner separately requires that manual YAML edits must not break the configurator itself. An error in editable configuration must remain diagnosable and fixable through the configurator rather than disabling it. This does not authorize applying invalid settings to a running service.

### Standalone configuration maintenance module (2026-10-31)

Everything related to config updates, maintenance and editing is moved into a separate, independently running module with its own API; this API may integrate with the web admin API. Implementation is deferred until the web admin and public service API with access tokens are implemented. This clarification replaces the previous assumption that all configurator/migrator logic lives in mandatory Core. The boundary with the self-sufficient CLI, native validation and safe application requires explicit reconciliation; the requirement that the CLI be independent of the web admin is not itself cancelled.

Q32 is accepted as an allowable option: a backup may include the durable portion of the working directory together with configs, excluding temporary files and regenerable cache. Configuration is not working data: these categories must be stored separately and have their corresponding distinct access rights. Archiving them together does not mean shared permissions or a single data type.

### Working directory, code protection and restoration (2026-10-31, Q30–Q31)

The composition of HTON-associated components is explicitly stored in a registry. Existing Amnezia is added to it when connected to HTON; merely discovering a foreign container does not transfer it into management. The registry and similar working state require an explicit working directory excluded from Git.

Code is effectively read-only, with no edit permission except during update. The owner proposed privileged updates (for example, through sudo); the architecture must determine the specific permission mechanism. Mutable configs and working data must be separated from protected code.

After restoring a `config/` archive, check format versions, secrets and module availability. Restoring files does not itself start services; start them through `scripts/up`.

### Module installation and incomplete setup (2026-10-31, Q28–Q29)

`scripts/modules add` obtains resources, creates a config and requests missing values, leaving the new module disabled. A separate `enable` checks readiness and enables the module. Installation and participation in operation are distinct actions.

If setup is incomplete, prepared resources and settings are retained, and the state is explicitly shown as “setup incomplete.” A repeated `add` resumes from that point; an incomplete module does not start.

### Partial shutdown and startup (2026-10-31, Q26–Q27)

If one module cannot be stopped, `scripts/down` continues stopping the others, preserves the autostart-disabled state and exits with an error listing components that remain running. Report “server is off” only after confirming that the entire composition has stopped.

During partial startup, healthy components continue running; `scripts/up` reports partial startup and specific errors. A repeated `scripts/up` starts missing components without restarting healthy ones.

Choose a specific free-domain example during implementation based on current conditions; installation may finish without a domain/publication. The admin lifecycle may be optimized based on actual practice; automatic shutdown on inactivity is not required.

## Standing planning and implementation principles (2026-10-31)

Source: owner’s detailed instruction after requesting a final plan review.
The owner explicitly began implementation in the instruction dated 2026-10-31 at
the start of this document; the previous “planning only” phase has ended. Preserve
this order: first analyze contradictions, inconsistencies and issues; update the
affected architecture and contracts; then write product code within approved
boundaries. Resolve issues independently where possible without changing previously
accepted requirements without consent. A question must include a worked-through
problem statement, options and a recommendation.

After reconciliation, update the detailed plan of relationships, file/class
structures, methods/functions, arguments and APIs. Decomposition must follow SDD,
SOLID, GRASP and DRY and be designed for production rather than later accumulation
of workarounds. Record these principles as standing rules in AGENTS.md.

Prepare an orchestrator for future parallel multi-agent implementation: waves,
specific goals, dependencies and file assignments. At this stage it is a
documented execution plan, not a launch of implementation or product code.

Testing and acceptance take place at the end, after the project is implemented and production-level decomposition is reached: unit testing first, then functional testing, after which further checks are determined. This replaces the earlier interleaving of implementation and test gates. Planning test interfaces and scenarios now is allowed; product tests and acceptance should not be performed now. Runtime configuration validation remains product behavior, not an early development/acceptance stage.

## HTTPS/HAProxy/Amnezia trial (2026-11-02)

The owner instructed the team to begin a trial of an HTTPS daemon with a
placeholder website from `/var/www`, HAProxy and Amnezia integration. This is an
instruction to start a trial; it does not change OR-4’s general requirements for
an own public HTTPS site or remove existing restrictions on changing external
resources/public port 443. The actual local test environment, its listener/certificate
and checks are engineering decisions recorded separately in `docs/DECISIONS.md`
and `docs/IMPLEMENTATION-STATUS.md`.

The goal is to deliver the finished product in one coordinated pass of file implementation
and acceptance, avoiding numerous iterative polish passes and spaghetti code.
Plan first, then implement: correcting the architecture plan is cheaper and faster
than correcting code. This is the target way of working, not permission to hide
errors or claim quality without an actual final check.

## Development and build organization clarification (2026-11-01)

Owner: “your iterations are too short. Continue development,” followed by:
“continue parallel multi-agent development. Build after all plans are fully ready.”
Continue in large, coherent packages with parallel implementers when available.
Do not run intermediate builds; build after all agreed plans are fully ready. The
coordinator’s working interpretation is: do not run cargo check/build or release
build until implementation of all selected plans is complete. This clarification
supersedes the earlier permission for intermediate static compilation; Markdown,
graph and source syntax checks are not builds. The order after implementation
remains: build, then U0, then F0. Report implementers’ unavailability honestly;
do not claim parallel work that did not occur.

## Publishing example.com through HAProxy and installation (2026-11-02)

The owner instructed the team to configure its own `example.com` website on public
TCP/443 through HAProxy, preserve Amnezia operation, move its external listener to
TCP/UDP 8443 and obtain a trusted certificate from Let’s Encrypt or a compatible
issuer. This specific request authorizes preparing and performing the agreed public
TCP/443 cutover and reconfiguring Amnezia to the specified port after checking
resources, preserving the original state and preparing recovery. The new route must
preserve native Amnezia REALITY without TLS termination.

The same sequence must be supported automatically by `scripts/install`: domain and port checks, certificate preparation, HAProxy and backend checks, coordinated application, served-certificate/route checks and resumable recovery. Initial domain/ports are provided during installation and persisted through a narrow typed writer; manual YAML editing is not required for this sequence. Reinstallation uses the saved choice; changed existing values must not be silently overwritten. Domain/issuer credentials are entered only through a protected secret store; they must not be guessed, placed in YAML/logs or requested in chat. Installation remains idempotent and explicitly reports ready/degraded/unverified.

This authorization does not allow stopping or reconfiguring other containers
and processes. If the specified port is occupied by a co-tenant resource,
`scripts/install` must stop before effects and request a separate owner decision;
`--yes` does not bypass this restriction. On the current server, Outline already
uses `8443` over TCP and UDP, so the requested Amnezia remap cannot be performed
until the owner selects another free Amnezia port or explicitly authorizes moving Outline.

Source: direct owner instruction dated 2026-11-02: “move Amnezia to external
port 8443, bring up HAProxy on 443 with domain example.com and obtain a Let’s Encrypt
certificate or something else if better. Important clarification: everything must
work automatically through scripts/install.” This requirement applies to the current
host and future reproducible installations; the port conflict was identified in a
subsequent read-only check and does not authorize changing Outline.

## OR-10. Random VPN service ports with persistence (2026-11-02)

The owner’s new clarification replaces the specific Amnezia 8443/8443 target for
this installation: Amnezia and other VPN services selected for management/integration
in the HTON installation must receive distinct random ports from the inclusive
5000–6000 range. The selected numbers are persisted as state of the corresponding
service and reused on every `scripts/install`, including after process exit or host
reboot; reinstall and resume must not choose new numbers. The source of truth is
the service owner’s durable configuration, not process memory or only a temporary
operation record. A new value is permitted only on initial assignment or a separately
approved change. HAProxy retains public TCP/443 for `example.com` and VPN routing.

Selecting this range satisfies the earlier OR-9 condition to choose an Amnezia
port different from Outline’s occupied 8443. This clarification does not reconfigure
Outline itself: it may be included in the HTON composition only through separate
service selection and after compatibility checks, an exact snapshot and an agreed remap.

Random assignment does not mean random changes on every run. Choose a new value
during initial service setup or an explicitly requested change, after checking
availability; if no value is free, installation reports a blocker and does not
persist a candidate. Read already-saved values from the service owner’s configuration.
Discovery of a foreign container alone does not connect it to HTON or authorize
changing its configuration; exact registration, compatibility, snapshot and consent
requirements still apply to foreign resources.

Source: direct owner clarification dated 2026-11-02: “let Amnezia and other VPN
services use a different random port in the 5000–6000 range,” followed by
confirmation: “but remember these as state—reuse this port throughout installation.”


## OR-11. Code deployment and administrator group (2026-11-02)

`scripts/install` configures the system `hton` group, permissions for the distribution
and starts code delivery to `/opt/hton`. A separate `scripts/deploy` performs delivery
and can also update already installed code. The operation requires explicit local
`sudo`; deployment copies current product source files from the working directory
and does not compile on the target host. The newer OR-14 clarification defines the
source-tree inventory and exclusions; no prepared release artifact is required.
When run without root, `scripts/install` and standalone `scripts/deploy` invoke `sudo`
themselves for operations requiring root; the user runs the commands without a
`sudo` prefix.

Source: owner clarification dated 2026-11-02: “scripts/install configures groups and
permissions and launches deployment. Deployment delivers code changes to /opt.”
The previously confirmed requirement to restrict code mutability and keep working
data separate remains in force. The `hton` group does not receive permission to
change protected code or sudoers membership; its exact modes and atomic switching
order are recorded in the agreed SDD and traced in DECISIONS.

## OR-12. Working site with trusted SSL and traffic separation (2026-11-02)

Bring the project commands to full usability so the owner gets a working
`example.com` site with trusted SSL and exact-SNI separation of inbound TCP/443
between the site’s HTTPS and the already-installed Amnezia. Preserve the accepted
D80 architecture: site TLS terminates at HTON/Caddy with a trusted certificate;
Amnezia REALITY remains independent and receives bytes without TLS termination at
the edge. The goal is achieved only after completing the selected implementation,
build and final checks in the D81 order, installing the release, successful DNS-01
issuance/local site verification, and qualified Amnezia remap/handshake with
recovery verification. Do not bypass fail-closed Amnezia checks or claim external
readiness based only on source code or local rendering.

For stages that depend on the owner/external environment, do not request secrets
in chat; import DNS credentials through the provided SecretStore path. Facts about
the exact installed Amnezia, its SNI/target, listener, restart/data owners and
authorization to change it must come from qualified local observation and an
explicit operator decision. This is a required outcome, not confirmation that a
certificate has already been issued or that the server is currently accessible.

The successful installation outcome is the running deployment under `/opt/hton`:
HAProxy on public TCP/443 routes the configured website SNI to its HTTPS site
backend and qualified Amnezia traffic to the Amnezia backend without terminating
its REALITY protocol. The website presents a publicly trusted certificate valid
for the configured domain, with no browser certificate warning. `scripts/install`
must verify both routes and the served certificate before it reports ready; if an
environmental prerequisite is missing, it reports the precise blocker/degraded
state rather than claiming success. This restates the required operational outcome
of OR-12 and does not waive Amnezia qualification or change authorization.

## OR-13. English source language and deferred product localization (2026-11-02)

All product source code, executable scripts, CLI help, prompts, diagnostics,
errors, logs and other product messages use English. The only permitted non-English
product text is a future localization layer; product localization is explicitly
deferred and must not be started as part of the current implementation. Russian
text is allowed only in `docs/_lang/ru/`; other documentation remains English. This is a
content/language rule and does not prescribe a programming language.

Source: direct owner clarification dated 2026-11-02: “always English in code;
the only exception is product localization, but we are not doing it yet.”

## OR-14. Deploy the current working source tree (2026-11-02)

`scripts/deploy` delivers the current product files from the local working directory
into `/opt/hton`, including current uncommitted edits. Deployment does not require
a release archive, provenance manifest or clean Git state. It must exclude `.git`
and `docs/_lang/ru/tasks/*`; generated build output, credentials and mutable runtime/config/
work/log data are also excluded. `/opt/hton` contains only the deployed product
snapshot and stable HTON entry points, never Git history or task planning files.
`scripts/install` configures the `hton` group and starts this same deploy operation.
Deployment copies existing build helpers if present but does not compile them;
missing required helpers remain a clear install blocker. The development/build
sequence in D81 remains in force.

Source: direct owner clarification dated 2026-11-02: “scripts/deploy copies not a
‘release’ but literally the current version of scripts from the working folder.
There is no .git and docs/_lang/ru/tasks/* in /opt/hton.”

## OR-15. Minimum working installation (2026-11-03)

The owner narrows immediate delivery to `scripts/install`: a publicly trusted site
through HAProxy, remapping the existing Amnezia Docker publication away from
TCP/443, and forwarding matching Amnezia traffic from HAProxy TCP/443 through
`127.0.0.1:<persisted-port>`. The backend host binding is explicitly loopback,
replacing older derived wildcard/private-container-endpoint bindings for this path.
Preserve the saved random 5000–6000 policy, native credentials/config and recovery
originals. Other programs remain outside the authorized mutation set.

Unfinished unrelated R1 management features no longer block this selected minimum
once its own implementation is complete. Build, unit, then functional verification
still follow implementation. Do not claim full R1 completion. The accepted Caddy
site TLS / HAProxy passthrough architecture remains until another TLS owner is
explicitly accepted. Proposed HTTP-01 instead of DNS-01 is an unresolved question,
not an accepted requirement change.

Source: owner: “finish to a minimum state: scripts/install makes the signed site at
HAProxy level, moves the Amnezia Docker port, forwards Amnezia traffic from HAProxy
443 to the Amnezia service (127.0.0.1:xxx).”

## OR-16. Configured installation inputs and deployment directory (2026-11-03)

The website domain and ACME account email belong in configuration. The owner
provides instance-specific domain and ACME account email values. Existing protected installed
configuration is authoritative; if either value is absent from configuration,
scripts/install asks locally and saves the validated value in the configuration under
the installation root (default /opt/hton). Deployment directory is also a
configuration field, not a hardcoded operational choice. Source deployment must
preserve installed settings rather than resetting them on every code delivery.

Source: direct owner clarification that domain/email must be configured, followed
by clarification that missing values must be prompted and written under /opt/hton,
and the deployment path must likewise be configured. This does not authorize
implicit relocation of an existing installation or overwriting its saved state.

## OR-17. Install, deploy, setup and update responsibilities (2026-11-03)

scripts/install compares the physical current working directory with the configured
target directory. Outside the target it invokes scripts/deploy for directory/group/
permission preparation and replacing current product source, then changes into
the target. In both cases it runs scripts/setup. scripts/setup fails if its current
working directory is not the configured target; in the target it obtains and
persists missing parameters and secrets and performs setup. Keep install as
orchestration and deploy as delivery, rather than duplicating setup.

The owner also requests scripts/update to perform git pull first, then update all
Docker containers. This replaces the older derived compatibility alias to config
apply. The exact meaning of all (HTON-owned versus all server containers including
independent VPNs) is an unresolved consequential clarification, requested because
older owner scope preserves unrelated components. Do not silently reinterpret
that scope or execute unqualified foreign upgrades while it is unresolved.

Source: direct owner flow description dated 2026-11-03. Existing lossless state,
secret handling, approval and independent data-plane requirements still apply.

## OR-18. Understandable installer interaction (2026-11-03)

The owner reports that the existing long remap confirmation is inconvenient and
unclear about whether input is required. Installer prompts must explain the next
action and make the required response understandable. This does not remove local
foreign-resource approval or authorize automatic acceptance. Product messages stay
English under OR-13; Russian localization is still deferred.

## OR-19. Publication DNS preflight and provider-neutral assistance (2026-11-03)

During explicit installation/setup, check the domain through a public resolver
such as Google 8.8.8.8 or Cloudflare 1.1.1.1 and compare it with the server's public
IPv4. On mismatch, show a short provider-neutral manual DNS instruction and allow
retry, with DNS API automation as an alternative rather than forcing InternetBS.

This is a narrow exception to OR-8 for explicitly requested setup/publication
preflight. It does not authorize background, startup or ordinary diagnostic
identity probes. A resolver supplies domain records; server external IPv4 requires
a separate observed or operator-configured value. An observed outbound IP alone
must not be presented as proof of inbound routing behind NAT or a proxy.

Source: owner proposes checking public DNS against the external IP first, then
asking for manual configuration with a universal instruction or API access.

## OR-20. Certificate lifecycle reliability and explained choices (2026-11-03)

The owner wants the most stable and reliable practical arrangement, including
unattended certificate renewal. Explain the actual operational difference between
HTTP-01/DNS-01 and whether another publicly trusted CA has meaningful advantages.
Do not make the owner select unexplained protocol names or imply that InternetBS
credentials are inherently necessary for trusted HTTPS.

This states the desired outcome and information request; it is not an acceptance
of a particular alternative CA or permission to create third-party accounts.

## OR-21. Public product and fleets of VPS/domains (2026-11-03)

HTON is a public, reusable project. It may be used with a fleet of VPS instances
and domains. Do not design or recommend its architecture as a bespoke one-domain
installation for the owner. example.com and the supplied email are instance-specific
configuration, not mandatory public product defaults or certificate policy.
Certificate validation/renewal, DNS providers, installation settings and state must
support independent deployments and different domain/publication topologies.

Source: explicit owner correction that this is a public project and may have a
fleet of VPS and domains. This does not itself request automatic remote VPS
provisioning, an always-running fleet controller or shared private keys. Preserve
those as separate consequential design choices rather than inventing scope.

## OR-22. Existing port 80 service (2026-11-04)

The owner questions any attempt to take over port 80: an occupied, working HTTP
service is valid existing infrastructure. Do not stop, restart, reconfigure or
force release of that listener as an install prerequisite. An HTTP validation
strategy may use an already-serving, verified challenge URL without changing the
service; unavailable routes require a configured alternative, not a takeover.

Source: direct owner objection that the existing port works and may remain busy.

## OR-23. Plain, clearly marked operational outcomes (2026-11-04)

Errors must be visually and textually obvious. Use simple English, clearly marked
ERROR and OK outcomes and understandable failed-step/reason messages. Avoid words
such as withheld and vague native-operation failures. Keep secret values private
without hiding the useful explanation. Successful code delivery must not imply
setup succeeded or confuse users with unrelated helper-availability false flags.

Source: owner reports that the failed scripts/install looked ambiguous and asks for
simpler language and visible errors. OR-13 English product language still applies.

## OR-24. Literal file deployment, no symlinks or snapshot directories (2026-11-04)

`scripts/deploy` must place the delivered product files directly under `/opt/hton`
as ordinary files and directories. It must not create a `current` symlink,
per-name symlinks, or content-addressed `deployments/` (or `releases/`)
subdirectories; repeated deployments must overwrite the same literal paths, not
accumulate snapshot directories. The administrator's OS user must be able to
read the deployed files and traverse the deployed directories through membership
in the `hton` group; `/opt/hton` itself and the delivered product tree are owned
`root:hton` with group read/traverse. `scripts/install` must create the `hton` group
and add the invoking user if absent, and must tell the user to activate the new
membership (`newgrp hton` or a fresh login), because Unix group membership only
applies to a new session. This refines OR-11 and OR-14, whose "literally the current
version" wording already intended plain files, not a symlinked snapshot layout.

Source: direct owner instruction dated 2026-11-04: deploy should literally
replace files in /opt/hton rather than make symlinks and proliferate deployment
directories, and the administrator user must see the files/folders through the
`hton` group.

## OR-25. Minimum MTProto, Hysteria and foreign-Outline support (2026-11-04)

> Outline portion superseded by OR-29: the minimum now runs an HTON-owned
> Outline WSS instead of preserving a foreign container.

The minimum profile must support three additional opt-in protocols without
weakening existing safety: MTProto through a private `mtg` backend selected by an
exact camouflage SNI at the HAProxy edge; Hysteria2 through an explicit public UDP
endpoint authenticated by a generated password and the existing trusted site
certificate; and coexistence with an already-installed Outline server by exact
container association and preservation only. MTProto/Hysteria settings, generated
credentials, share links and immutable native artifacts belong to the protected
minimum state. Outline is never remapped, stopped or reconfigured by this support;
it remains foreign and independently managed. Protocol activation is explicit,
disabled modules have no listener, and a failed cutover restores the previous
services and credentials. `--yes` does not authorize changing a saved protocol
association or camouflage name; a local readable confirmation is required.

Source: direct owner instruction dated 2026-11-04: “add support for MTProto,
Hysteria and Outline, all per docs/tasks”. This expands OR-15's authorized minimum
mutation set by exactly those three capabilities. It does not complete full R1, the
31-item catalogue, Outline WSS migration or any public VPS cutover.

## OR-26. Rename `cmd/` to `scripts/` (2026-11-04)

The public command entrypoint directory is `scripts/`, not `cmd/`. Existing
`scripts/` build/release files and the command entrypoints share that directory.
Documentation, file maps, CI paths and runtime error text must use `scripts/`.
Document stable IDs such as `cmd.install` may be retained for continuity; they are
identifiers, not filesystem paths.

Source: direct owner instruction dated 2026-11-04: “rename the cmd folder to
scripts and do not forget the docs”.

## OR-27. Third-party MTProto camouflage stays third-party (2026-11-04)

The MTProto `mtg` camouflage name (third-party, currently
`www.microsoft.com`) must remain third-party. Do not replace it with the owner's
site domain or a subdomain of it; MTProto keeps a third-party fakeTLS camouflage.
REALITY is governed by OR-28.

Source: owner answer dated 2026-11-04 to the SNI review: keep third-party MTProto
camouflage.

## OR-28. REALITY camouflage uses the owner's real site (2026-11-04)

> Superseded for the selected single-SNI chain by OR-31. The self-steal concept
> remains, but the REALITY listener is now an HTON-owned container with the native
> keys/clients and the neutral SNI `www.example.com`.

Amnezia REALITY must use the owner's real site `example.com` as camouflage instead
of `www.googletagmanager.com`. The native `serverNames` must include `example.com`;
unauthenticated probes must receive the real site served from `/var/www/html`
with its trusted Let's Encrypt certificate; native `dest` must point at the local
site backend, never at public `example.com:443`, to avoid a routing loop.
Authenticated REALITY clients continue to receive the VPN. The site must remain
reachable if the REALITY backend is unhealthy, so the edge routes the site SNI to
REALITY with the site backend as a health-checked fallback. Existing client
profiles migrate to `example.com`; the old third-party name is removed only after
migration. MTProto remains third-party under OR-27.

Source: owner statement dated 2026-11-04: the site served from `/var/www/html`
has no relation to Google, and the owner wants to use the real site for
camouflage. This supersedes OR-27 for REALITY only. It authorizes the self-steal
migration as a separate, explicitly consented requalification; it does not make
an implicit change to native `serverNames`/`dest` part of ordinary remap.

## OR-29. Single 443 entrypoint and HTON-owned Outline (2026-11-04)

TCP/443 carries only VLESS/HTTPS traffic: the ordinary site, REALITY, MTProto
fakeTLS and the HTON-owned Outline WSS, all through HAProxy. Hysteria2 is not
VLESS/HTTPS and uses UDP separately because HAProxy does not proxy UDP. The
original UDP/8443 choice is superseded by OR-42: selected QUIC uses UDP/443. The existing `outline-operator-two` container must be removed and not used;
HTON runs its own Outline (Shadowsocks-over-WebSockets) behind the site TLS
terminator so it shares TCP/443. The desired default composition enables MTProto,
Hysteria2 and the HTON Outline.

Source: owner instructions dated 2026-11-04: enable MTProto, Hysteria and
Outline through HAProxy on 443; delete `outline-operator-two`, including its Docker
container; install our own. Follow-up clarification: only VLESS/HTTPS traffic may
use 443; Hysteria2, being different, goes to 8443.

## OR-30. Minimum delivery closure (2026-11-04)

The minimum profile must not deliver files it does not use to `/opt/hton`.
Full-R1 Rust sources, module descriptors, schemas, compose/docker build inputs,
tests, build-only scripts and unused reference docs are excluded from a minimum
delivery. The minimum closure is the Python minimum installer, its shell
entrypoints and shared helpers, the pinned image inventory, protected
installation input, CLI/minimum operator docs, README and LICENSE. Deploy always
delivers this minimum closure; full-R1 sources are supplied by a separate
build/release path, not by deploy. A minimum deploy must also prune
managed top-level paths left by an older full delivery, while preserving
mutable `config/`, `work/`, independently built `libexec/` and deployment
markers.

Source: owner instruction dated 2026-11-04: reduce the number of files delivered
to `/opt/hton`; do not deliver what is not used there.

## OR-31. One neutral site SNI for REALITY and MTProto (2026-11-04)

Use exactly one neutral, non-VPN-looking SNI for REALITY and MTProto on TCP/443.
By default that SNI is the configured site mask domain itself (`example.com`); if
the site domain is specified, no separate SNI/camouflage argument is required.
`--chain-sni` may override it with a subdomain, but is optional. Do not use
`vpn.*`, `mtp.*`, or cloud-provider names such as Microsoft/Google/Amazon; they
are implausible on this VPS and obvious. A new HTON-owned Xray container copies
the native REALITY private key, short IDs and clients, uses
`serverNames: [<chain-sni>]` and sends unauthenticated bytes to `mtg`; `mtg`
serves MTProto clients and forwards everything else to Caddy, which serves the
real `/var/www/html` site. The old native `www.googletagmanager.com` SNI remains
only until existing clients are reissued. The MTProto share link advertises the
same chain SNI. Generated MTProto/Outline/Hysteria material stays in the
protected secret store; the mask SNI is derived from the configured domain, not
passed on the command line. The pinned Outline binary path and Hysteria UDP/8443
are defaults, with optional overrides.

Source: owner instructions dated 2026-11-04: no obvious `vpn`/`mtp` names; use
something legitimate and unobvious; cloud-provider SNI on a cheap VPS is not
plausible; by default use the mask domain if a domain is configured; avoid
arguments by keeping generated values in secrets. The earlier `www.example.com`
selection is an optional subdomain override, not the default.

## OR-32. Argument-free minimum defaults (2026-11-04)

A minimum installation with no protocol-selection flags enables MTProto,
Hysteria2 and HTON-owned Outline WSS until the operator has explicitly chosen a
selection. An old saved all-false feature map does not count as that choice; a
successful install records the choice once. OR-38/39 replace the shared REALITY/MTProto name with configured separate
protocol names, and OR-42 replaces UDP/8443 with UDP/443; the pinned Outline binary
path is built in. Generated MTProto/Outline secrets and paths stay in the
protected minimum secret store. Existing saved selections remain authoritative,
so `--with`/`--without` are optional overrides rather than required arguments.

Source: owner instruction dated 2026-11-04: no arguments; put the generated
values in secrets; by default the SNI is the mask domain when a domain is
configured.

## OR-33. Per-user Outline keys and traffic accounting (2026-11-05)

Every Outline WSS user must have a unique access key so usage can be attributed
per person. The default user names are derived from the existing Amnezia client
UUID suffixes (`amnezia-<suffix>`); the protected user list may be edited before
installation to set real names. The Outline server config contains one key per
user, and each user receives a separate dynamic access key YAML. The local
Outline Prometheus endpoint is enabled and a collector stores per-user totals
over time, including across service restarts; an operator command reports total
and current-session bytes per user. Per-user revocation is removing that key
without changing other users. Outline Manager is not required.

Source: owner statement dated 2026-11-05: each user must have a unique key to see
who spends how much over time.

## OR-34. `scripts/install` is the rollout command (2026-11-05)

`scripts/install` is the single command for deploying and applying fixes. It must
be run from the cloned project root. It deploys the current source tree whenever
the source root differs from the configured target, then runs the installed
setup as root with `SUDO_USER` preserved. Deploy creates the `hton` group, adds
the invoking user and sets `root:hton` modes. Every run applies incremental
changes: stale files are pruned, obsolete runtime material is removed,
permissions are normalized and registered migrations run once in order.
Running the installed copy from the target skips delivery and applies
migrations/setup only. No separate deploy invocation is required for updates.

Source: owner instruction dated 2026-11-05: install must run from the current
cloned project directory; it should deploy and then run the inner install as the
needed user with the needed permissions and groups; updates are incremental,
including deletions and permission changes.

## OR-35. Native REALITY uses the own SNI (2026-11-05)

The installed Amnezia REALITY listener itself must present the deployment's own
SNI (the site domain by default, or the configured `--chain-sni` subdomain), not a
third-party name. Consequently, a full-access `vpn://` key exported by the Amnezia
app must carry the own SNI. `scripts/install` must perform this migration as an
explicitly consented requalification (the local `y/yes` prompt; `--yes` is not
sufficient):

- add the own name to native `serverNames` and keep the legacy third-party name
  until existing clients are reissued;
- point native `dest` at an internal target reachable from the bridged native
  container: the `mtg` chain listener on the Docker bridge gateway when the
  neutral MTProto chain is enabled, otherwise the site TLS backend on that
  gateway (never public `example.com:443`, to avoid a routing loop);
- preserve the native private key, short IDs, flow, clients and listener port;
- route the own SNI through HAProxy to the native REALITY backend, with the site
  backend as a health-checked fallback;
- retain the original container and its exact configuration for recovery, and
  stop using the separate self-steal container after migration;
- perform no migration from unattended renewal: renewal keeps the last accepted
  configuration until an interactive install obtains consent.

Source: owner instruction dated 2026-11-05: the SNI must be `example.com` or a
subdomain, and the migration must be part of `scripts/install`; earlier OR-31
selected a separate container, which this requirement replaces for the native
listener.

## OR-36. Backward-compatible SNI is the default (2026-11-05)

Existing Amnezia profiles must keep working until the owner explicitly approves
dropping backward compatibility. Because REALITY binds the fallback certificate
and target to a single `dest`, adding the own name to `serverNames` does not by
itself keep legacy-SNI clients working: verified empirically, a client using the
old third-party SNI fails after native `dest` moves to the internal site chain.
Therefore:

- the default install keeps the native listener unchanged (legacy SNI and
  `dest`) and serves the own SNI through the separate copied-clients listener;
- switching the native listener to the own SNI requires the operator to type the
  exact phrase `DROP-LEGACY-SNI` (a plain `y` is not approval);
- an installation already migrated under the earlier draft offers to restore the
  saved previous native configuration on the next interactive install;
  unattended renewal never restores or migrates;
- the Amnezia full-access `vpn://` key continues to use the legacy SNI, so
  profiles created through it keep working; own-SNI profiles are served by the
  copied listener until the explicit cutover.

Source: owner instruction dated 2026-11-05: make the old Amnezia VPN connections
keep working at least until explicit approval to abandon backward compatibility.

## OR-37. Managed Amnezia XRay provisioning (2026-11-06)

On a fresh host, `scripts/install` must provision the supported Amnezia XRay
REALITY service instead of requiring a pre-existing container. Once provisioned,
hton owns installation, update and uninstall of that container and its host
access. A fresh installation must emit both:

- a full-access `vpn://` key for the AmneziaVPN application, containing a
  dedicated `hton-amnezia` SSH user and a generated Ed25519 private key;
- a guest `vless://` client link.

The dedicated user has `NOPASSWD` sudo. The private key is stored only in the
protected store and embedded in the full-access `vpn://`; the root password is
never stored. Full-access `vpn://` is root-equivalent and operator documentation
must say so. The configuration selects between `associate` (current behavior) and
`provision`; existing installations remain associated unless the operator
explicitly changes the mode. The image must be a pinned hton build derived from
Amnezia 5.0.3's XRay core and must pass the existing exact container
qualification. Product code and tests remain blocked until the owner explicitly
changes phase.

Source: owner decisions dated 2026-11-06 in the managed-Amnezia provisioning
review: emit full-access `vpn://` and guest `vless://`; use a dedicated
`hton-amnezia` sudo user with an Ed25519 key; make Amnezia hton-managed.

## OR-38. Per-protocol SNI and anti-detection defaults (2026-11-06)

The three public protocol names are the accepted instance values (configurable under OR-39):

- Amnezia REALITY: `static.example.com`;
- Outline WSS: `assets.example.com`;
- MTProto fakeTLS: `media.example.com`.

They are subdomains of `example.com`. The earlier `packet.aexample.com` was a typo.
Use one wildcard certificate for `*.example.com` plus the apex `example.com`, and a
wildcard DNS record for `*.example.com` plus the apex, so Certificate Transparency
and ordinary DNS enumeration do not expose the per-protocol names. Each name must
serve the same real, coherent HTTPS site with the wildcard certificate, except
for the later OR-44 custom browser-denial policy on the REALITY name.
Unauthenticated or wrongly authenticated probes must fall back to that site, not
to a default page, an error page, a self-signed certificate or a third-party
certificate. OR-44 explicitly replaces the site/error-page restriction for the
REALITY browser host with a custom 403; the trusted-certificate requirement remains.

This supersedes OR-27 (third-party MTProto camouflage) and the single neutral SNI
selection in OR-31. For new keys it changes the default REALITY name in OR-35 from
`example.com` to `static.example.com`. Existing profiles may keep a prior own-domain
SNI while the wildcard certificate covers it; third-party legacy names are
removed only with explicit operator approval. New full-access `vpn://` keys use
`static.example.com`.

The install must not retain defaults that are easy to fingerprint: no
non-standard public port, stale uTLS fingerprint, long high-entropy WSS path,
third-party SNI unrelated to the host, or protocol-specific name that is not
served as a normal site. REALITY keeps `xtls-rprx-vision` and a real-site
fallback. MTProto must domain-front to the real site. Hysteria, if enabled, must
not keep UDP/8443 as an unreviewed default; its obfuscation and port contract must
be reviewed before remaining enabled. These measures reduce but cannot guarantee
avoidance of IP/ASN blocking or active probing; a previously blocked address or
provider must be replaced.

Source: owner instruction dated 2026-11-06: use `static.example.com` for Amnezia,
`assets.example.com` for Outline and `media.example.com` for MTProto; improve
obfuscation everywhere, avoid non-standard parameters that are quickly detected,
and prevent a repeat of the previous installation's ban.

## OR-39. Configurable SNI and scripted administrative account (2026-11-06)

Every SNI must be easily editable in declarative configuration, including protocol,
site, legacy compatibility and fallback names. OR-38's three names are the accepted
instance values, not hard-coded product constants; its wildcard certificate,
real-site fallback and compatibility constraints still apply when names change.
Administrative access such as Amnezia SSH must use a separate account created by
the installation script, with the username read from a configuration string.
The owner specifically raised the risk of SSH access. This clarification does not
revoke OR-37's accepted full-access export or NOPASSWD sudo; those credentials
remain root-equivalent and must be identified as administrative credentials.

Source: owner instruction to update the task and SDD, move all SNI into config,
and create the administrative user by script with its name from config.

## OR-40. Review masking after the previous server ban (2026-11-06)

Review and improve the provisioning task and related SDDs with particular attention
to anonymity/detectability: the owner reports the previous server was banned and
asks for a stronger design this time. The cause of that ban is not established.
Do not present masking settings as proof of anonymity or future availability.
This requests documentation/design work in the current planning phase, not a live
server change or permission to execute implementation/acceptance.

## OR-41. Broader masking and code-backed DNS review (2026-11-06)

Improve VPN/proxy security and obfuscation wherever feasible after the reported
ban. Inspect actual code and put concrete findings/remedies into the plans. The
owner specifically asks whether DNS is tunneled, visible to the ISP, and covered
for every VPN/proxy protocol. Do not claim this is already guaranteed; distinguish
client/application DNS, endpoint bootstrap and server-side DNS, and document actual
coverage. This is design/review work under the current planning-only instruction.
Source: owner asks to obfuscate everything feasible and to check code, not only SDD.

## OR-42. No public 8443; browser QUIC port (2026-11-06)

Strictly no public 8443 in the selected HTON installation. Public VPN/HTTPS ingress
uses TCP/443; if a QUIC/UDP VPN is selected it uses UDP/443, the browser QUIC port.
This supersedes the former Hysteria UDP/8443 default. Preserve private backend port
allocation; matching the port does not by itself make UDP traffic browser QUIC.
Existing unassociated third-party 8443 services are not authorized for mutation:
report the conflict with this installation goal and require a separately reviewed
transition rather than claim complete host compliance.
Source: owner explicitly prohibits public 8443 and selects the browser QUIC port.

## OR-43. Unknown SNI rejection with owned-wildcard exception (2026-11-06)

The owner first requires an error for SNI absent from configuration, so unknown
names cannot reach Amnezia and its old Google/API fallback. The subsequent explicit
exception is: undefined wildcard subdomains of `example.com` redirect to `example.com`.
Thus configured exact protocol/site names retain their routes; an unassigned
single-label subdomain covered by the configured owned wildcard certificate goes
to the ordinary HTTPS server and redirects to `https://example.com/`. Other unknown
names are rejected before reaching a VPN backend. All names/redirect target remain
configurable under OR-39. The exception does not admit arbitrary third-party names
or names outside certificate coverage. Legacy known profiles retain OR-36 consent.
Source: consecutive owner instructions on unknown SNI and wildcard redirects.

## OR-44. Simple REALITY browser response and one shared Caddy (2026-11-06)

For the configured Amnezia name (currently `static.example.com`), an ordinary browser
request should receive a simple custom `Access denied` response. This replaces
OR-38's same-site/HTTP-200 requirement for this name only; valid VPN authentication
must continue to work. Reuse the existing Caddy serving `/var/www/html`; no second
Caddy/container/HTTP proxy just for this response. Other configured protocol names
retain their accepted site behavior and unknown wildcard names follow OR-43.
Source: owner asks for a simple custom denial on browser probes, then asks to
optimize away an extra Caddy layer and reuse the existing website server.

## OR-45. Secret client-key page and quiet install (2026-11-06)

Generate an optional page on the base site domain that shows all client VPN/proxy
keys with their names, without a password. Never include administrative keys. The
page path and settings belong in the main configuration. The owner's current
instance URL is `https://example.com/client-access`; this is an explicitly supplied
instance value, not the product default. The main example defaults to an empty
path. Interactive scripts/install asks for a secret URL path when it is empty,
saves the answer, and an empty answer means no page/file is needed.

`scripts/install -q` asks absolutely no questions and uses saved settings/defaults;
an empty page path stays disabled. Full administration remains TODO/deferred.
This authorizes a read-only passwordless client-key page, not a web administration
service, password database or public management API. A secret URL is bearer access
to the displayed keys; no stronger authentication is implied by this requirement.
Source: owner's detailed client-key-page/config/install instruction dated 2026-11-06.

Later clarification: OR-49/OR-52 supersede the empty-by-default interaction above.
An absent `path` now publishes automatically with a generated CSPRNG URL, an
explicit empty `path` is the opt-out, and the successful install also writes the
per-user guest files and URL into the private `.secrets/share` repository.

## OR-46. Operator-group read access across /opt/hton (2026-11-07)

Everything the installer keeps under `/opt/hton` must be readable through the
`hton` group: deployed code, `config/`, `work/`, retained `host-access/`, a copied
`.secrets/` and generated artifacts such as share files. Directories are `0750`
and files `0640`, owned `root:hton`; that is the purpose of the group, and a group
member must be able to `cat` a share file without sudo. Runtime stores, migrations
and deploy must not clamp these trees back to root-only `0700`/`0600`. Transient
lock files are not data and may stay root-only.

Consequence accepted by the owner: VPN credentials and secrets under `/opt/hton`
(REALITY and Hysteria keys, issued certificate keys, DNS API keys, and the managed
Amnezia admin SSH key when provisioned) become readable to every member of the
`hton` group. Group membership is therefore credential-equivalent for VPN
material, though still not root-equivalent. This supersedes the "mutable
`config/`/`work/` root:root 0700" wording in D120 and the corresponding premise in
D149; the separate `host-access/` subtree decision stands.

Source: owner report that `cat /opt/hton/work/minimum/share/amnezia-guest.txt`
fails for a group member and that the group exists precisely to read
`/opt/hton`, dated 2026-11-07.

## OR-47. Never replace an existing website (2026-11-07)

`scripts/install` must serve the website that already exists on the host instead
of publishing a placeholder. The document root is an explicit profile setting
(`site.root`), imported into the runtime config as `core.site.root`. When it is
empty, install checks `/var/www/html` then `/var/www` for an `index.*` and asks
once whether to use that site as the default site on the domain (default yes);
the answer is saved (`core.site.root` when accepted, `core.site.useExisting:
false` when declined) so later runs do not ask again. Quiet, JSON and renewal
runs never prompt and adopt the detected site. A configured but unusable root
fails the install instead of silently substituting the placeholder, and with no
detected or configured root the placeholder is served. Nothing under the root is
ever written, moved or deleted: Caddy mounts it read-only at `/site`, serves it
with `file_server` and hides `.git`. `https://<domain>/` must return that site.

Source: owner report that the domain root showed the installer placeholder
rather than the pre-existing git project (`~/html`, a GitHub Pages clone with its
`index.html`), dated 2026-11-07.

## OR-48. The public repository is anonymous (2026-11-07)

hton is an anonymous public project. No tracked file may identify the owner or
the owner's infrastructure: personal domains, names, e-mail addresses, host
names or home-directory paths. Requirement provenance records the date and the
substance of a report, never the concrete domain. Real values live only in
`.secrets/` (a separate repository, outside the published tree) and in runtime
state on the host. A tracked-file guard (`tests/anonymity`) fails the build when
an identifier appears.

Source: owner report that an anonymous public project referenced the owner by a
personal domain, dated 2026-11-07.

## OR-49. VPN share credentials are published on the service site (2026-11-07)

`scripts/install` must publish the current guest VPN credentials (Amnezia REALITY,
MTProto, Hysteria2, Outline) on the service's own HTTPS site under a secret,
unguessable path (`https://<domain>/<secret-path>`), so a client can fetch its
share file in a browser. The path is generated with a CSPRNG on first enable
(at least 128 bits, one safe segment) and is rotatable; it is never derived from
the domain, host name or client name. Publication must not write into the site
document root (OR-47): the page is served from its own read-only root. Responses
carry `no-store` and `noindex` headers, unknown paths return 404, the page lists
no credentials for disabled or incomplete modules, and no credential bytes appear
in logs, plans or JSON receipts.

Source: owner report that share/credential publication on the service site was
expected but is absent, dated 2026-11-07.

## OR-50. Active development deploys origin/main (2026-11-07)

While the project is under active development, the deployable release is the
current `origin/main` source tree. `scripts/install`/`scripts/deploy` must accept
a clean checkout at `HEAD`; a `BUILD-PROVENANCE.json` gate, signed artifacts,
SBOM and publisher attestations describe a later release phase and must not block
the current milestone.

Source: owner clarification that the release is the current main branch during
active development, dated 2026-11-07.

## OR-51. Native remap is deferred (2026-11-07)

Association, remap and restore between the original native Amnezia container and
the HAProxy-managed edge, including port handover, are not required for the
current milestone. They move to far-future scope ("maybe"). No acceptance
criterion may depend on them, and install must keep working with the native
listener untouched.

Source: owner clarification that remap is no longer needed, dated 2026-11-07.

## OR-52. Guest share files follow each user into the private secrets repository (2026-11-07)

`scripts/install` must write the per-user guest share artifacts and the generated
secret client-page URL into the private `.secrets` repository, not only into the
runtime work tree. After a successful install:

- `/opt/hton/.secrets/share/` receives `amnezia-<name>.md`, `mtproto.md`,
  `hysteria.md`, `outline-<name>.yaml` (all `0640`, readable by the operator
  group) and `client-page.md` containing `https://<domain>/<secret-path>`.
- The source checkout `.secrets/share/` receives the same files when `.secrets`
  exists there, so the operator can commit them to the separate private Git
  repository.
- Only active guest credentials are written; admin keys, host SSH keys and
  private server keys never appear.
- Nothing is written on a failed install, and files from a previous successful
  publication that are no longer current are removed.
- The generated URL is absent from logs, plans and JSON receipts; the page itself
  continues to be served from `work/client-page`, never from the site document
  root or `.secrets`.

The owner's phrase “по юзерам их секретам и публикации урл в .secrets и в вэб дир
с секреным урл” collects these outcomes: per-user secrets, the URL in `.secrets`,
and the secret-URL web directory. The web page remains the client-access page
defined by OR-45/OR-49, not a new administration interface.

Later clarification (OR-58): the per-user artifact layout is one Markdown bundle
per user containing all that user's protocols, not one file per protocol. The
private-repository, only-active-credentials and no-secrets-in-logs requirements
above still apply.

Source: owner instruction to implement the per-user, secret and secret-URL
publication tasks, dated 2026-11-07.

## OR-53. Log rotation forgets material older than one week (2026-11-07)

HTON must rotate its logs automatically and delete log material older than seven
days. The retention window is one week; configuration may shorten it but must not
extend it past seven days. Total-size limits stay configurable and bounded, and
rotation or a full log must never stop a working VPN or block safety recovery.
This applies to the minimum installer log (`/var/logs/hton/minimum.log*`) and to
the bounded Core/runtime log ring. Secret redaction rules are unchanged.

Source: owner instruction “сделай ротацию логирования с забытием логов старше
недели”, dated 2026-11-07.

## OR-54. The private `.secrets` repository is the durable copy; no separate backup/restore (2026-11-07)

The owner does not see a need for a separate HTON backup/restore feature:
`.secrets` is the substance of the backup, and it is kept in a separate private
Git repository rather than inside a backup archive. A separate encrypted archive
duplicates the same non-regenerable material and was not understood as useful.

Consequences:

- The durable material is the private `.secrets` repository: `profile.yml`,
  `images.lock.json` and `share/` (OR-52). The operator commits and pushes it.
- Disaster recovery is: restore/checkout that private repository, run
  `scripts/install` (which imports the profile), then `scripts/up`. There is no
  separate restore mode.
- Runtime state under `/opt/hton/work/` (registry, journal, accepted bundles,
  certificates, ACME state) and cached images is regenerable or reconstructible;
  it is not a required backup target.
- R1 must not require, schedule, advertise or implement a separate
  backup/restore command, encrypted archive, retention policy or `/opt/hton/storage/`
  backup store. The earlier backup/restore planning in D70, D72 and Q32 is
  superseded for R1 scope.
- `config/` is materialized from `.secrets`; it does not need a second archive.
- Point-in-time runtime snapshots for a different problem require a new owner
  decision; they are not implied by this requirement.

Source: owner statement “зачем нужен бэкап рестор я вообще не понял .secrets и
есть суть бэкапа а оно будет лежать в новом приватном репо а не в бэкапе”, dated
2026-11-07.

## OR-55. Install refreshes the release and unattended update runs weekly (2026-11-07)

`scripts/install` is also an update operation: on every explicit install it
fast-forwards the source checkout's `main` to `origin/main` before delivery and
then re-pulls the pinned images, recreating owned containers whose image changed.
Local commits, uncommitted work, a detached HEAD or a non-main branch are never
rewritten. If the source checkout is not a Git working tree or the fast-forward is
impossible (offline, diverged, dirty conflict), install reports the reason and
continues with the source already present; an offline or partially updated host
must remain installable and its VPN must keep running.

`scripts/update` performs the same fast-forward first and then reinstalls the
HTON-owned composition. Its scope is the HTON-owned containers and configuration
only; independently installed VPNs and unrelated host containers are not updated.
A failed fast-forward stops the update before any container effect. The former
`update` config-apply alias stays removed.

The installer enables an unattended update timer. It checks daily but performs an
update only when the last successful install or update is at least seven days old,
so an installation refreshed by `scripts/install` is not updated again inside the
same week. A successful install/update records the protected timestamp. The timer
runs without prompts or public DNS/identity observation, requires a committed
installation, and cannot perform the first install, native migration/remap or
approval-bearing changes. Failures retain the last working composition and are
retried on a later daily check; no partial candidate may replace a working VPN.

`--dry-run` reports the planned fast-forward/update without network, Docker or
container effects.

Source: owner instruction “доделай всё что кассется обновления при каждом инсталл,
автообновление раз в неделю если ни разу не делался инсталл за неделю”, dated
2026-11-07.

## OR-56. Native client names accept Russian letters and uppercase (2026-11-07)

Native Amnezia client names (`scripts/clients add` / `rm`) must accept letters in
any case, both Latin and Russian Cyrillic, not only lowercase ASCII. The accepted
form is: a leading letter or digit, then letters (Latin/Cyrillic, any case),
digits, `_`, `.` or `-`, up to 32 characters. Names stay case-sensitive and are
used verbatim for the native `email` field, the share artifact title and the
`vless://` link fragment; the file-safe slug lowercases them and maps other
characters to `-`.

The same form applies to the configured initial guest `provision.clientName` in
the `foreign-amnezia` module (and therefore to the installed `#<name>` link,
`amnezia-<name>.*` artifacts and client page), so an install can create `Гость`
or `Alice` rather than only lowercase ASCII.

Source: owner interjection “нужны русские буквы и заглавные” during the
`scripts/clients` work, dated 2026-11-07.

## OR-57. The app connection name shows the country, not the username (2026-11-07)

The generated native VLESS link's display fragment — the connection name shown in
the Amnezia app — must be a configured server label, normally the country, followed
by the protocol name (`XRay`; see OR-60), not the credential's username. The username remains the xray `email` and the credential
identity: it still names the client on the client page and in the share artifact.

- The label is `instance.label` in the root configuration/profile, a bounded
  printable string of 1–64 characters; spaces and flag/other emoji are allowed.
- A `core.clientAccessPage.names` entry for a credential overrides the instance
  label for that link; with neither configured, the fragment falls back to the
  client name.
- The installer regenerates the ready-summary link, the protected share artifact
  and the client-page payload with this fragment; `scripts/clients add` uses the
  same precedence for its immediate link.

Source: owner instruction “нафигша в приложении имя пользователя - нужна страна в
названии соединения”, dated 2026-11-07.

## OR-58. One share bundle per user; operator group can read it (2026-11-07)

Share output is organized by user, not by protocol. Each user (native Amnezia
credential, and a matching Outline identity when the names coincide) gets one
Markdown file under the private `.secrets/share` containing every credential that
user can use: their REALITY link, their Outline key when present, and the shared
MTProto/Hysteria server links. The client-page URL is included in each bundle;
`client-page.md` may remain as the operator copy. The former
`amnezia-<name>.md` / `mtproto.md` / `hysteria.md` / `outline-<name>.yaml` layout
is superseded.

Every file the product writes under `/opt/hton/.secrets/share` must be readable
by the `hton` group: directories `0750`, files `0640`, owned `root:hton` for the
installed repository. A script-created share file must take the private
repository's owner/group, never remain `root:root 0640`. In the source checkout
`.secrets` the operator's own ownership is preserved. `scripts/clients add/rm`
writes/removes the same per-user bundle in both the installed and source-checkout
`.secrets/share` when both exist.

Source: owner report `cat /opt/hton/.secrets/share/amnezia-d6.md: Permission
denied` (file was `root:root`) and the instruction “где разделение по юзерам у
каждого юзера должен быть свой share файл. фокус на пользователей а не 1 файл 1
пользователь внутри все его протоколы-программы”, dated 2026-11-07.

## OR-59. Protocol-selection prompt with MTProto and AmneziaWG2 defaults (2026-11-07)

The installation/user workflow must ask which protocols to add for a new user or
server instead of silently enabling a fixed set. MTProto and AmneziaWG2 are the
requested defaults; the user may choose others.

AmneziaWG2 is a separate VPN endpoint, not a client-side toggle of the existing
REALITY connection: it needs its own UDP listener, userspace/kernel AWG2 engine,
per-peer keys and an Amnezia-compatible `.conf`/`vpn://` export. It is not present
in the current codebase (`amneziawg` module absent, no AWG image, and the SDD
marks AWG unsupported for the HTTPS/SNI stack); existing foreign `amnezia-awg*`
containers are preserved, never managed. Implementing it therefore needs an
explicit module scope, pinned image/build, UDP port allocation, key/peer lifecycle,
config export and deployment authorization before the prompt can default to it.

Source: owner instruction “и скрипт должен спрашивать какие протоколы добавлять по
умолчанию mtproto, amnezia wg2”, dated 2026-11-07.

## OR-60. No HTON brand in client-facing connection names (2026-11-07)

Client-facing connection/config names must not contain the word `hton`. For the
native Amnezia protocols, the name identifies the protocol: `XRay` for the current
REALITY endpoint and `AmneziaWG` for a future AWG2 endpoint. The optional
`instance.label` (country) may prefix the protocol name, producing e.g.
`🇱🇻 Латвия XRay`; without a label the fragment is just `XRay`. Hysteria uses
`Hysteria2`; Outline uses the user identity; MTProto has no link fragment. The
username remains the credential identity, never the connection name by default.
`instance.name` stays the internal logical ID and is not a client-facing label.

Source: owner instruction “слово хтонь в клиенте не нуж но если это амнезия то
хрей или амнезиявг”, dated 2026-11-07.

## OR-61. One admin client-access URL, shared by all users (2026-11-07)

The credential page must be a single administrative URL, one for all users; it is
not a per-user page. Its secret path is a bearer admin credential: the operator
distributes that one URL deliberately, and every per-user share bundle keeps
referring to the same URL (OR-58). The page lists the accepted client inventory
and must not be disabled by an unrelated configuration diagnostic, such as an
inconsistent certificate wildcard/challenge pair; page availability is owned by
the page's own path/refresh validation.

Guardrails: the page contains every published client credential, so it must stay
protected (no-store/noindex headers, bearer URL outside the site and page roots),
and per-user URLs must not be minted to satisfy this requirement. Hosting the
page still requires the install transaction to create its Caddy route and the
`/client-page` mount; a refresh of the artifact alone does not publish it.

Source: owner instruction “url не работает. более того - урл с паролями должен
быть один на всех юзеров. админский”, dated 2026-11-07.

## OR-62. Plain HTTP redirects to HTTPS; ACME keeps TCP/80 (2026-11-08)

`http://<site>` must not fail or serve plaintext. With a real site root, TCP/80
answers only the HTTP-01 challenge path `/.well-known/acme-challenge/*` from that
webroot and returns a `308` redirect to `https://<request-host><request-uri>` for
everything else. The site, credential page, API and VPN traffic remain HTTPS-only;
HAProxy keeps public TCP/443. A placeholder install with no site root may leave
TCP/80 free so the standalone ACME flow still works.

Consequence: once the permanent HTTP listener owns TCP/80, HTTP-01 issuance and
renewal must use the verified webroot served by that listener, not acme.sh
standalone binding the port. The certificate lifecycle must switch modes only
through its existing select/prove path and must not bind TCP/80 while the listener
is live.

Source: owner instruction “http://example.com/ - не открывается (сделай редирект на
https)”, dated 2026-11-08.

## OR-63. Apex SNI for services, www alias and wildcard certificate (2026-11-08)

For Amnezia REALITY and the other public services, stop using dedicated
third-level names: their SNI is the site apex `example.com`. The browser host of
the REALITY name therefore receives either the ordinary site through the nested
fallback (apex) or the fixed `Access denied` stub for any remaining REALITY
subdomain. Nested SNI within SNI is the accepted mechanism.

An interactive `scripts/install` migrates saved protocol subdomain names of the
site domain to the apex automatically; renewal and unattended update never
change the accepted names. Client links must be reissued after that migration.
Keep exactly one third-level alias, `www.example.com`, redirecting to
`https://example.com/` (308, path and query preserved). Services must not add
protocol-specific third-level names for new deployments; existing legacy names
may be retained only while old client profiles are being reissued.

The certificate is a wildcard `*.example.com` plus the apex, so the alias and any
subdomain verify against a trusted chain; the owner uses `www` to confirm that
the wildcard certificate is valid. Wildcard issuance requires DNS-01 with real
DNS API credentials; `www` must not rely on a self-signed or mismatched leaf.

Source: owner instruction “может для амнезии и других сервисов вообще откажемся
от домена 3 уровня? но оставим домен 3 уровня: www.example.com с редиректом на
EXAMPLE.COM; серт через вавйлдкард так я буду проверять что сертификат рабочий”,
dated 2026-11-08.

## OR-64. Install console shows the client page URL and the share directory (2026-11-08)

After a successful `scripts/install`, the local operator console must show the
client-access page URL (`https://<domain>/<secret-path>`) so the browser page
with all client keys is immediately usable. It must also show the directory that
holds the generated share files instead of printing every credential link.
Individual VPN/proxy links are not printed to the console; the files under
`/opt/hton/.secrets/share` (per-user bundles, `0640`, readable by the `hton`
group) and `work/minimum/share` hold them.

This supersedes the OR-52/OR-45 statement that the generated URL is never
printed: it is printed to the local operator console after a successful install,
but remains absent from JSON output, receipts, plans and logs. Quiet and
machine-readable runs still never print it.

Source: owner instruction “а где ссылка в браузер где можно подсмотреть все
ключи? ее должно быть видно после инсталл и зачем вообще вконсоль публикуется
[links]... достаточно указать папку где лежат файлы”, dated 2026-11-08.

## OR-65. Deployment mounts the product closure instead of publishing a branch (2026-11-09)

`scripts/hephaestus/deploy-taenarum` must carry the current product closure to the server by mounting it
read-only over the existing SSH connection (reverse SSHFS) and running
`scripts/install` from that mount, instead of committing a temporary branch and
making the server clone or fetch it from the hosting provider.

Rationale: the installer copies the source into `/opt/hton` and spends its time
on Docker, so the mount replaces only the transport. Removing the temporary
branch also removes the repository round-trip, server-side Git, server
repository credentials and the pre-commit audit from the deploy path. Only the
product closure is exported: `bin`, `lib`, `scripts`, `python/minimum`, the
install and image descriptors, the operator docs and the public
`.secrets.example` templates, each as its own read-only mount. `.git`,
`.secrets`, `.env`, `.deploy` and the rest of the tree are not mounted at all.
Deployment uses a separate server-only key that cannot reach the repository.

The mount is the default transport and the routine deploy path:
`scripts/hephaestus/deploy-taenarum`, with `scripts/hephaestus/deploy` as its alias. The
earlier temporary-branch path stays available as a separate, explicit fallback
`scripts/hephaestus/deploy-git` for hosts without `sshfs` or FUSE access; it is not
the default and must not be used while the mount works.

OR-34 and OR-50 remain in force: `scripts/install` still runs from a source tree
that contains `scripts/install`, still delivers to the configured target and
still accepts a clean checkout at HEAD. The mount changes only how that tree
reaches the server and adds no provenance gate.

Source: owner statements dated 2026-11-09: "отказ от временных бранчей и сложной
системы пуш-пул через гитхаб внутри докера хоруса"; the same message notes the
lower secret-leak risk and the time spent on commit and push checks.

## OR-66. AmneziaWG2 endpoint in the minimum profile (2026-11-09)

HTON must support the AmneziaWG protocol at its newest practical release. The
minimum profile adds a dedicated `amneziawg` protocol module (aliases
`amneziawg2`, `awg`, `awg2`) built on the pinned AmneziaWG 3.1 userspace engine:
its own public UDP listener, one X25519 and preshared key per peer, generated
obfuscation parameters (`Jc/Jmin/Jmax`, `S1-S4`, non-overlapping `H1-H4`), a
server-side `/32` route per peer, and one complete client `.conf` per native
Amnezia user. The `.conf` must remain compatible with D6ton's native
`amneziawg` backend (no server `ListenPort`, hooks or `SaveConfig`).

The endpoint is an additional public UDP surface with the same lifecycle,
ownership, drift checks and rollback as the other optional protocols. It is
included in the default first selection, may be disabled with
`--without amneziawg`, retains its keys when disabled, and must be exercised
locally through D6ton before the feature is reported ready.

Source: owner instruction dated 2026-11-09: "добавь в hton поддержку протокола
AmneziaWG самую свежую версию, протестируй локально через d6ton".

This supersedes the "AmneziaWG2 absent" prerequisite note in OR-59/D165: the
module and its pinned image now exist; the remaining interactive per-user
protocol prompt is still a separate deliverable.

## OR-67. Reverse-tunnel test ingress for VPN-blocking tests (2026-11-12)

For manual VPN-blocking tests, Tartarus must be able to build this path:

`abyss-echo -> real HTON server -> taenarum reverse SSH forward -> local abyss-hton`.

The HTON edge on the real server routes a dedicated **test SNI** (separate from
every production site, REALITY, MTProto, Outline and Hysteria name) to a
loopback port on that server. The port is supplied only by an SSH reverse
forward opened from the Tartarus checkout through the taenarum bridge. The forward
connects that port to the local Tartarus `abyss-hton` listener. The local
`abyss-echo` Docker client connects to the real server address with the
normal VLESS/QUIC-style client traffic and the test SNI; the traffic loops back
to the local HTON endpoint for the test.

Requirements:

- the test route is opt-in and off by default; no production install enables it;
- the test SNI is a valid configured DNS name and must not collide with a
  production SNI;
- the forwarded listener is loopback-only (`127.0.0.1`), adds no new public
  listener and is not a production protocol;
- the local stand accepts the same test SNI in REALITY `serverNames` while
  keeping its normal client URI working;
- the test client URI reuses the local HTON credentials but points at the real
  server address and test SNI;
- `scripts/echo` owns the Tartarus side: configure, up, status, uri and
  down; it starts the reverse forward and the `abyss-echo` client;
- normal stand operation, `scripts/verify.sh` and the local client/server Docker
  path remain unchanged.

The SSH reverse forward carries TCP only. VLESS/REALITY, Trojan or WSS-style
test traffic works through it; QUIC/UDP is not carried by this route and would
need a separately designed UDP forward. This limitation is reported rather than
represented as full QUIC coverage.

Source: owner instruction dated 2026-11-12: “добавь ... обратный тунель ...
тартарус соединяется к хтони через стикс по ссш-проброс портов ... хтонь через
ха прокси перенаправляет весь ТЕСТОВЫЙ (отдельный SNI) трафик в порт обратного
проброса ... обратный проброс подключает тунель к локальной хтони ... клиент
d6ton локальный подключается к хтони серверной как обычный vless/quic ...
это нужно для тестирования блокировок впн трафика”. The follow-up clarification
dated the same day fixes the manual-test path as
`<tartarus: <d6ton-docker>-<hton-server>-<taenarum-docker>-<hton-docker>>`.

## OR-68. Split the public ingress into Triodos (2026-11-12; VPN owner superseded by OR-69)

The current monolith is split into two independent projects with separate
repositories, Compose projects, installers, lifecycle and deployment:

- **Triodos** owns the entire public front: the HAProxy ingress and load
  balancer, exact SNI and request classification, Caddy own-domain TLS, the
  ordinary site and its masking policies, ACME certificate issuance and renewal,
  public TCP/80, TCP/443 and TCP/853, backend pools and health checks. It decides
  which road a public name takes.
- **HTON** was assigned the VPN server in the original split decision. OR-69
  supersedes that service ownership: Onomat owns VPN engines and data plane;
  HTON remains the server platform. The Triodos boundary remains: VPN is reached
  by SNI through Triodos, and HTON never owns the public site, certificates or
  ingress.

The split must keep the following properties:

- Triodos and the server-side platform/services run on the same host with
  independent repositories, configuration stores and lifecycle boundaries;
  neither imports the other's internal modules.
- Triodos is authoritative for the public route map. The Onomat VPN service
  consumes via HTON a versioned,
  validated **grant** describing the names, paths, certificate hosts and ingress
  fallbacks assigned to it; HTON never publishes a name by assumption.
- Load balancing is allowed only across Onomat VPN backends that share an identical
  credential/identity set (the same REALITY keypair, MTProto secret, Outline
  user keys and AmneziaWG parameters). Triodos records and compares an identity
  digest per pool member; balancing across different identity sets is a hard
  error, because per-server VPN credentials are not transparently portable.
- No duplicate business logic or long-lived compatibility branch: the extracted
  ingress code leaves HTON rather than remaining as a second implementation.
- The current Dokimasia stand, `scripts/verify.sh` and the existing hton/d6ton
  path keep working until the migration cutover. The cutover itself is a separate
  owner-authorized phase with an explicit maintenance window and rollback target.
- Open decisions that this requirement does not settle: public UDP/443 ownership
  and the UDP balancing mechanism; the DoT/Unbound boundary; the exact
  client-page artifact handoff; and credential replication across pool members.

This pass authorizes design, scaffold, contract, example configuration, schema
and a phased migration plan only. It does not authorize changing runtime code,
the stand wiring, deployment, service startup or publication.

Source: owner instruction dated 2026-11-12: “давай разделим хтонь на 2 отдельных
проекта: Triodos (ха прокси ингресс балансировщик на входе) и отдельно по sni:
хтонь для впн сервера”, followed by owner choices: Triodos owns the whole public
front (HAProxy + Caddy + certificates + site); several HTON backends are balanced;
both live on one host as different Compose projects; this pass delivers design,
`hton/services/triodos` scaffold and a migration plan.

## OR-69. Server/client platforms; Onomat owns VPN (2026-11-13)

The product roles are distinct: **HTON is the platform for a server**, **D6ton is the platform for a client**, and **Tartarus is the development-environment platform for deployment and maintaining connectivity**. **Onomat owns the VPN service and Unbound** as server-side services under `hton/services/`. Move VPN engines, protocol configuration, private listeners, client/key application and data-plane lifecycle into Onomat's service boundary. Onomat owns their operator settings and enablement. HTON provides server-platform mechanisms to install, configure, validate, start and maintain services, and coordinates Onomat through declared interfaces. Do not describe HTON itself as the VPN service or place VPN business logic in the platform core.

Preserve the Unbound work already underway in Onomat; do not relocate or duplicate it. Detailed interfaces among Onomat, Triodos, Daedalus, Hecate and Omma must be reconciled in their service contracts. Existing monolithic HTON implementation and stand remain operative until a separately authorized migration phase; this requirement records target ownership and does not itself authorize runtime changes.

Source: owner instructions dated 2026-11-13: “onomat - впн unbound(уже делается в другой сессии). надо вынести vpn из hton как отдельный сервис. hton теперь платформа для сервера, d6ton платформа для клиента, тартарус платформа для дев енва деплоя и обслуживания связи” and “перенеси впн в ономат!”. The latter clarifies that the VPN service belongs to Onomat.

## OR-70. DNS runtime moves from HTON to Onomat (2026-11-13)

The Unbound DNS runtime moves out of HTON into the Onomat repository: the Python
policy/render/lifecycle owner (`dns.py`), the Rust Unbound renderer
(`unbound.rs`), the Unbound configuration template and their contract tests now
live under `hton/services/onomat/runtime/dns/`. HTON keeps the platform role: it
consumes the moved renderer, renders the accepted bundle and applies the private
Unbound service; it does not keep a second DNS implementation. The running
Onomat console stays optional, and the moved runtime does not gain a Docker
socket, direct engine writes or canonical state.

Source: owner instruction dated 2026-11-13: “перенеси днс из хтони в onomat”.

This is the separately authorized DNS migration step. VPN and DNS both belong to
Onomat per OR-69; their implementation and contracts remain separate services
within Onomat's ownership boundary.

## OR-73. Traffic camouflage is a primary requirement (2026-11-13)

The owner reports a prior HTON server was blocked and prioritizes resistance to
TSPU traffic classification in the new version. Evaluate optional encapsulation
and normal-looking transport behavior, including observable handshakes, packet
characteristics and active probing. The blocking mechanism is not diagnosed.
Preserve OR-72 official-client compatibility. Do not promise that changing keys
alone improves camouflage or that wrapping bypasses an IP-level block. Criteria
are recorded in Tartarus's `tasks/styx-multi-backend.md`.

## OR-72. Independent servers behind one Triodos (2026-11-13)

One Triodos may front multiple independent VPN servers, including multiple
AmneziaWG servers. Preserve official-client compatibility for the corresponding
protocol/version. The owner reports the current connection is broken and asks
for an engineering solution. Do not conflate independent servers with replicas
sharing credentials. The failure cause is not yet established by live evidence.
The proposed binding and optional Styx design is recorded in
`tasks/styx-multi-backend.md` in the Tartarus workspace. This requirement does not
authorize deploying to an external server or starting a new public edge.

## OR-71. D6ton is a full multi-node VPN platform (2026-11-13)

D6ton is a complete VPN platform intended to run on user devices and on
server/relay nodes. A D6ton node may act as a VPN client relative to the next
node while forwarding traffic through further tunnels; this is the same product
and profile model, not a reduced server fork. D6ton owns routing decisions among
sky, direct-earth and chthonic VPN paths. Charon is the logical ferry/outbound
for traffic D6ton selects for the Styx transport to Triodos; Styx carries that
selected traffic, while Triodos owns public ingress classification.

This platform role does not reassign HTON's server-platform lifecycle or
Onomat's ownership of HTON's VPN and Unbound services. D6ton server/relay nodes
are a distinct deployment of the D6ton platform. This clarification sets target
architecture and naming; it does not authorize a new deployment, runtime
cutover or external service startup.

Source: owner's clarification dated 2026-11-13: D6ton should be the VPN
"client" relative to a route while remaining a standalone platform like HTON
that can run on client and server nodes and form multi-hop metro routes.

## OR-74. Start with optional Styx encapsulation in D6ton (2026-11-14)

Begin by implementing an optional Styx transport adapter for D6ton. Keep native
profiles working and preserve the inner VPN protocol configuration. For one
Triodos edge, Styx profiles use its one shared public TLS name while selecting
independent backend/protocol bindings behind the edge. Do not assume that
official Amnezia, Hysteria, Happ or other clients support this wrapper; their
compatibility is a later investigation. Start with an established D6ton engine
transport and explicitly limit the first supported slice. Do not claim
obfuscation success without traffic-shape and active-probing evidence. Future
control messages are optional and are not part of the first data-plane slice.

This authorizes the D6ton adapter and local, non-started pairing artifacts. It
does not authorize public-edge startup, deployment, Dokimasia rewiring or an
external service migration. The implementation boundary and acceptance limits
are in Tartarus `tasks/styx-encapsulation.md`.

Source: owner instruction dated 2026-11-14: “давай тогда для начала соберём
через стикс инкапсулятор для д6тон потом будем думать что с клиентами возможно
они будут использовать собственные SNI но не хотелось бы(риск бана)”.

## OR-75. Unified per-user, per-protocol edge measurements (2026-11-14)

The public edge must be able to measure traffic independently for each user and
VPN protocol, regardless of whether the VPN backend itself provides counters.
The long-term measurement identity is `(userId, protocolId, backendId)` and
must feed the common measurement system. Styx must authenticate the user and
carry an authorized protocol/backend selector so the edge can attribute the
stream without inspecting or changing the inner VPN protocol. This does not
require the measurement implementation in the first D6ton encapsulation slice;
receiver counters and Omma integration remain a later delivery step.

Source: owner instruction dated 2026-11-14: the edge should count traffic “для
каждого пользователя на каждый протокол в единой системе измерений”, including
protocol servers that do not provide their own counters.
