# HTON implementation gaps

> Historical analysis snapshot. Current reconciliation: [FINAL-REVIEW](FINAL-REVIEW.md), [architecture](ARCHITECTURE.md), [interfaces](INTERFACES.md) and [orchestrator](IMPLEMENTATION-ORCHESTRATOR.md). Earlier gaps/schedules are not current acceptance criteria.


Research snapshot: 2026-10-31. This report compares the current repository with the
owner requirements in [OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md). The owner
requirements are authoritative; roadmap and design text are evidence of planned
work, not proof of implementation. Repository files and code are the primary
sources for the claims below. No third-party technical claims are needed here.

## Current state

HTON is still a scaffold and design, not an integrated VPN release. The README
explicitly says that the target YAML/compiler/UI/runtime guardian and most commands
are unimplemented; the existing Bash scripts use legacy `.env`
([README.md](../README.md#L7-L10)). In the checkout, the executable CLI consists of
the dispatcher, help, install helpers, uninstall stub, and transitional `whoami`;
the install helper stops after dependency setup and says the menu/Compose work is
not implemented ([bin/hton](../bin/hton), [scripts/help](../scripts/help#L9-L18),
[scripts/install-ubuntu](../scripts/install-ubuntu#L30-L39), [scripts/uninstall](../scripts/uninstall)).
There is no current integrated data-plane implementation to qualify.

## Material gaps to close

### 1. Reconcile the public CLI with the required module lifecycle

**Owner requirement.** All operations must be self-sufficient through `scripts/*`.
`scripts/modules` owns add/remove and module resource/config setup; enable/disable
controls whether an installed module participates at startup. `remove` retains
data, `purge` is explicit, image updates are separate, and `scripts/up` launches the
saved platform composition without pulling/updating images
([OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L57-L65)).

**Repository gap.** The public CLI instead declares `feature list/enable/disable`,
`update`, and `service start|stop|restart|status`; it does not declare `modules`
or `up`, nor give `update` the required separate image-update meaning
([CLI.md](CLI.md#L33-L59)). The legacy help also describes `install` as a one-shot
menu/Compose setup, not the saved-composition lifecycle
([scripts/help](../scripts/help#L9-L18)). This is a contract mismatch, not just missing
code: reconcile `docs/CLI.md`, help, command SDDs, schemas/file map, and roadmap
before implementation binds to the current syntax.

### 2. Build the actual standalone Core and stable management contract

**Owner requirement.** HTON Core is required and is used by CLI independently of
the management API. The API is a distinct optional, full service/module; it need
not run as a daemon in R1. Later web, GUI, app, and MCP clients must use that API,
with compatible configuration and operation semantics. Each module owns
declarative configuration and contributes semantic checks to common validation
([OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L17-L23)).

**Repository gap.** No Core, API server, or module checker implementation exists
in the checkout; schemas and detailed API design are artifacts only. The control
SDD describes a versioned API and operations, while architecture calls the runtime
and controller proposed boundaries
([sdd-control.md](specs/sdd-control.md#L39-L72),
[ARCHITECTURE.md](ARCHITECTURE.md#L165-L186),
[README.md](../README.md#L7-L10)). Implement the shared model/checker contracts and
Core-backed CLI first, then implement the optional API against those same
semantics. Preserve current owner requirement that UI never writes config directly.

### 3. Implement installation under `/opt/hton` and domain-aware setup

**Owner requirement.** Install the platform under `/opt/hton`, keep runtime state
under `/run/hton`, avoid unnecessary global system configuration changes before
package delivery, and allow service lifetime independent of a login/home directory.
Installer must check a configured domain or explain how to start obtaining a free
one; it may complete without a domain but must warn about applicable server-block
risk ([OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L29-L31),
[OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L39-L41)).

**Repository gap.** Current scripts resolve and operate from the checkout root;
they do not install the application tree or units. The Ubuntu helper writes a
Docker apt source and enables Docker, then exits before platform setup
([bin/hton](../bin/hton), [scripts/install-ubuntu](../scripts/install-ubuntu#L11-L39)).
The installer SDD already describes `/opt/hton` and `/run/hton`, but its
implementation is absent and it needs reconciliation with the owner's specific
domain/no-domain flow ([sdd-installer.md](specs/sdd-installer.md#L38-L46)).
Implement a repeatable installer/reconciler, explicit dependency/bootstrap effects,
domain guidance and a degraded no-domain result; avoid silently broadening global
host changes.

### 4. Deliver and prove the R1 TCP/443 site + VPN baseline

**Owner requirement.** R1 requires IPv4, a genuine HTTPS certificate for owned
domains, ready VPN services on the shared TCP/443 ingress, and a regular site that
remains available when management is absent or fails. A plausible site placeholder
is allowed. IPv6 is deferred; UDP-specific modules retain their stated limits
([OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L33-L41)).

**Repository gap.** There are no HAProxy/Caddy, VPN engine, certificate, systemd,
or accepted-bundle runtime implementations in source; only design/templates maps
and the README's stated plan are present
([README.md](../README.md#L7-L15),
[sdd-runtime-safety.md](specs/sdd-runtime-safety.md#L59-L82)). The roadmap makes
the baseline and independent boot/recovery conditional on future gates G1–G4
([ROADMAP.md](ROADMAP.md#L7-L34)). Build the selected R1 stack and demonstrate
cold boot, management-offline operation, valid cert issuance/delivery, regular-site
routing, and real client connectivity before calling it release-ready.

### 5. Reconcile owner scope with stale derived requirements and maps

The owner explicitly supersedes derived specifications and file maps on conflict
([OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L1-L3)). Several docs acknowledge
reconciliation is pending. In particular, older installer/config/test designs
describe mandatory Docker staging, while owner OR-6 says candidate Docker
qualification is optional and must not be a gate for every apply
([sdd-installer.md](specs/sdd-installer.md#L3),
[sdd-config.md](specs/sdd-config.md#L90-L101),
[sdd-runtime-safety.md](specs/sdd-runtime-safety.md#L59-L82),
[OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L49-L51)). The file index, SDD map,
schemas and roadmap still describe extensive planned files and behaviors; the map
itself says bindings are proposed rather than evidence of existence
([ROADMAP.md](ROADMAP.md#L83-L117), [sdd/map/README.md](sdd/map/README.md)).
Reconcile normative docs first, then synchronize index and mirrored map before
implementation, as roadmap sequencing already prescribes.

Other directly material scope corrections to preserve during reconciliation:

- Do not make traffic accounting an R1/R2 criterion, and do not pull IPv6 into R1;
  both are explicitly deferred by the owner
  ([OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L25-L27)).
- Preserve the owner's boundary around upstream VPN clients: HTON must work with
  an already configured external uplink/chain, but configuration and operation of
  that client (including `scripts/up`/`scripts/down`) belongs to a separate repository.
  The integration point and tests are not yet made concrete in the current code
  ([OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L43-L47),
  [ARCHITECTURE.md](ARCHITECTURE.md#L158-L160)).

### 6. Qualify the low-resource target with measurements

**Owner requirement.** Economical operation is architectural; the planning target
is 1–2 GB RAM, around 50 devices and tens of simultaneous connections. This is a
target, not a tested hard limit; exact capacity estimates are not a blocker
([OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L5-L7)).

**Repository gap.** There is no measurement evidence or integrated stack yet; the
roadmap explicitly schedules actual CPU/RAM/throughput evidence for G4
([ROADMAP.md](ROADMAP.md#L17-L23)). Once the selected baseline runs, record memory
and CPU under representative connection/load cases and use the results to keep
expensive modules independently optional. Do not convert the target into a
hard acceptance ceiling without owner agreement.

## Owner decisions and recommendations kept separate

**Binding owner decisions:** self-sufficient console and Core; optional separate
management API and future Docker web module; module-owned configs/checkers; platform
placement and domain-aware install; resilient TCP/443 site/VPN ingress; external
uplink compatibility with client management kept outside HTON; optional Docker
candidate checks; module lifecycle semantics; R1 IPv4-only with no traffic
accounting through R2. See [OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md) for
complete wording and scope.

**Agent recommendations:** close gaps 1–5 in dependency order, then gather
resource evidence; keep the site placeholder minimal for the first integrated
baseline. These recommendations do not change owner scope.

**Open decisions already identified by the owner:** exact R1 protocol minimum,
free-domain instructions/example, authorization model for OS console users, and
whether/how to idle-stop the optional web module. None should be silently resolved
by treating a proposed SDD detail as an owner decision
([OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L67-L69),
[OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L9-L13),
[OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md#L53-L55)).
