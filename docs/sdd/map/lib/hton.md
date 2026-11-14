# lib/hton

Stable ID: `shell.project` · Selected package: `M1-E` · Status: planned M1 contract

Common parsing/elevation/fixed minimum dispatch. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `hton_cli(command, bounded args) -> fixed helper invocation or usage error`.

- `hton_source_update(root, best-effort|required) -> bounded fast-forward status` — install runs it best-effort before delivery; update requires success. Uses `python/minimum/updates.py fast-forward`.

- `update` dispatch -> `updates.py due` for `--auto`, `updates.py fast-forward` (required), optional `scripts/deploy`, then the target minimum `install.py update` with `HTON_SOURCE_UPDATED=1` and `HTON_INSTALL_SOURCE` set to the invoking checkout.

- Install re-entry as non-root runs `sudo -- env HTON_SOURCE_UPDATED=1 <source>/bin/hton ...`, so the already performed source fast-forward is not repeated. Quiet, JSON and non-TTY invocations add `sudo -n`. The environment assignment precedes the command.

## Calls

- `hton_cli` -> `cmd.deploy.run`.

- `hton_cli` -> `core.minimum.install.main`.

## Called by

- `cmd.install.run` -> `hton_cli`.

- `cmd.uninstall.run` -> `hton_cli`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.

## Historical baseline contract (superseded where M1 differs)

# lib/hton

Stable ID: `shell.project` · Phase: `R1` · Kind: source

Source binding includes source-snapshot and stable-state resolution; full R1 completion is not claimed. Purpose: Tiny shared Bash initialization

Implementation wave: `I3-A` (planned ownership, not completion).

## Features and responsibility

- Bash-first

## Methods, arguments and result

- `projectRoot` — arguments: executable path; result: absolute root.
- `invokeHelper` — arguments: fixed operation, argv; result: structured helper result.
- `hton_install_prompt_value` — arguments: label, optional Core-suggested default, domain/port/email validator; result: bounded non-secret TTY value or usage failure.
- `hton_install_preflight_kind` — arguments: bounded Core JSON response; result: `inputRequired` plus only validated required field names (`siteDomain`, `edgePort`, `certificateEmail`) and optional exact Core suggestions (`edgePort:443`, first-publication `siteDomain:example.com`), `recoveryPending`, `preflight`, or `ready` after validating plan shape and no conflicts/approvals.
- `hton_install_input_preflight` — arguments: Core-owned saved install state and trusted TTY input; result: ready, one bounded prompt/retry for Core-named missing site/domain/edge/email fields, unchanged `inputRequired`, or structured blocked plan with nonzero exit.
- `hton_install_selection_offer` — arguments: fixed Core selection-offer JSON, optional trusted profile/menu intent, TTY; result: revision-bound offer digest and Core-approved selected IDs, unchanged structured offer when prompting is unavailable, or a bounded `recoveryPending` state that recovers before any selection/input prompt.

## Structures and injected dependencies

Argument decoder and request adapter; no duplicated domain classes

Dependencies: fixed command dispatch, Core typed helper, trusted OS/TTY consent context.

## Logic

- Default install/explicit minimal profile delegates to core/minimum/install.py
  after the same protected deploy/self-sudo handoff. New typed options are
  site-domain, certificate-email, amnezia-container and dns-credentials.
- The minimum profile rejects fresh/menu/skip-remap/skip-start rather than silently
  weakening its two-route goal. Docker-only bootstrap remains separate.
- Full-R1 helper operations refuse an installation with minimum's ownership marker;
  no two configuration/runtime controllers act on the same resources.

- Quote arguments and validate fixed operation IDs.
- No sourcing deployment data or executable journal text.
- Validate only bounded install response shape; `preflight` prints the original JSON and exits blocked before bootstrap. Never infer readiness from a substring or accept ready when the immutable plan has conflicts or required approvals.
- `inputRequired` prompts only the exact validated fields returned by Core and retries once; non-TTY/JSON preserves the original structured response without prompting.

## Calls / reads

- [core/src/main.rs](../core/src/main.rs.md) — `invokeHelper` -> `run`.

## Called / read by

- [scripts/install](../scripts/install.md) — `run` -> `invokeHelper`.
- [scripts/deploy](../scripts/deploy.md) — `run` -> privileged current-source inventory and activation.
- [scripts/install-ubuntu](../scripts/install-ubuntu.md) — `run` -> `invokeHelper`.
- [scripts/install-unknown](../scripts/install-unknown.md) — `run` -> `invokeHelper`.
- [scripts/config](../scripts/config.md) — `run` -> `invokeHelper`.
- [scripts/diag](../scripts/diag.md) — `run` -> `invokeHelper`.
- [scripts/keys](../scripts/keys.md) — `run` -> `invokeHelper`.
- [scripts/share](../scripts/share.md) — `run` -> `invokeHelper`.
- [scripts/certs](../scripts/certs.md) — `run` -> `invokeHelper`.
- [scripts/restart](../scripts/restart.md) — `run` -> `invokeHelper`.
- [scripts/switch](../scripts/switch.md) — `run` -> `invokeHelper`.
- [scripts/remap-undo](../scripts/remap-undo.md) — `run` -> `invokeHelper`.
- [scripts/doctor](../scripts/doctor.md) — `run` -> `invokeHelper`.
- [scripts/whoami](../scripts/whoami.md) — `run` -> `invokeHelper`.
- [scripts/validate](../scripts/validate.md) — `run` -> `invokeHelper`.
- [scripts/runtime](../scripts/runtime.md) — `run` -> `invokeHelper`.
- [scripts/uninstall](../scripts/uninstall.md) — `run` -> `invokeHelper`.
- [scripts/tunnel](../scripts/tunnel.md) — `run` -> `invokeHelper`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


I3-A: hton_public_command(name) is the single fixed public command-name allowlist,
shared by bin/hton and scripts/help. Sourcing defines functions only and never loads
.env automatically. Remaining legacy functions still await replacement and must
not be called by new target commands to bypass Core state/authorization.

Called by bin/hton.dispatch and scripts/help.run for hton_public_command only.

invokeHelper(operation, argv) now selects config.check only, resolves the fixed
libexec/hton-core binary beneath its own distribution root and passes config root
as an explicit argument. It never executes a binary supplied by PATH/environment,
builds a helper, sources managed YAML/.env or falls back to legacy operations.

invokeRuntime(status, argv) selects fixed libexec/hton-runtime and work/runtime state
root without consulting Core/compiler/config. Missing binary returns dependency 3;
no dynamic build/download or fallback. Other internal operation IDs are refused.


## I3 CLI consistency binding (2026-11-01)

Shared shell.project owns hton_bounded_arguments (128 argv, 4096 bytes each,
64 KiB total), hton_parse_common (canonical idempotent common flags, separate
positionals/help/yes metadata), hton_valid_id and fixed command availability.
No argument is evaluated or sourced. Known unfinished bindings return 3 even if
an unrelated executable happens to exist at scripts/<name>. Each declared command
has command-specific side-effect-free help; Core composition includes config,
modules, runtime and read-only diag/whoami/doctor observation. Other known
unfinished operations return their explicit Core incomplete result. Direct cmd
scripts share the same common parser.
Config accepts documented --file/--plan syntax and enforces operation applicability
before unavailable outcomes; modules/runtime enforce ID/cardinality constraints.
Repeated boolean common flags are normalized, not forwarded repeatedly to strict
helpers. --yes remains ordinary-prompt metadata and grants no consent. Legacy
functions remain defined for unmigrated direct scripts but target commands do not
call them. Static Bash syntax checks are not product U0/F0 acceptance.


## I3 final shell binding (2026-11-01)

The bounded shell adapter owns syntax/ordinary prompts only. It calls fixed Core
operation IDs with --config-root/--work-root, canonical common flags, -- delimiter
and literal operation argv; no actor JSON, YAML/env sourcing, generic shell or
domain state writer. --yes is ordinary prompt metadata, never foreign/443/purge
consent. All actual module/config/cert/native effects remain under Core/
runtime owners and typed journals. Missing installed helper returns 3, usage 2,
consent 4, native failure propagates. Dry-run never bootstraps dependencies or
writes a journal/distribution/config. Runtime status directly uses independent
runtime; public start/rollback ask Core to compose an admitted typed runtime request.

Bootstrap owns only OS package/distribution mechanics. It accepts Ubuntu/Debian
with literal validated os-release fields; no sourced os-release or pipe-to-shell.
Existing supported Docker is preserved; package conflicts are not auto-removed.
Automatic Docker repo setup uses distro-correct HTTPS origin and signed apt metadata;
unknown OS prints primary-source instructions. `scripts/deploy` copies current working-tree
product files using its bounded allowlist, excluding `.git`, `docs/_lang/ru/tasks/`, generated
output and mutable state; no release manifest is required. `lib/bootstrap` installs
OS dependencies only. No unverified download, VPS compilation, registry/secret writes
or implicit service startup occurs. Installer reconciliation is the fixed
install.reconcile Core operation.

Install invokes pure `install.reconcile --input-preflight` before bootstrap. It
prompts only after a successful `inputRequired` JSON response naming missing fields,
uses only validated Core-suggested defaults (`edgePort:443` and, on first
publication, `siteDomain:example.com`), and forwards the bounded values only to Core.
Each prompt allows three attempts and
60 seconds per answer. Dry-run, `--json`, and non-TTY never prompt; non-TTY preserves
the structured inputRequired response. The shell does not read YAML/.env or persist
values. Prompts are limited to site domain, public edge, non-secret certificate
account email and explicit first-run module selection; DNS credential values are
never prompted or forwarded. A required credential-import continuation asks only
for bounded file paths and calls `keys.manage dns` with those paths. VPN port
allocation is Core-owned and persisted in service settings. Bootstrap proceeds only after Core
reports a conflict-free ready plan.
A read-only `recoveryPending` response is a separate repair branch: invoke normal
`install.reconcile` once and return its partial exact-allocation recovery or
lifecycle-owner handoff result; do not prompt, bootstrap packages or continue
publication in that invocation.
`--profile` and `--menu` are not shell-only metadata: the normal install flow must
pass their typed intent to Core's catalogue-backed selection workflow. Profile is
first-install guidance and cannot replace saved choices; menu revisits the current
saved selection. Each normal guided install calls read-only
`install.selection-offer` with JSON and validates the bounded IDs/digest through
Python's JSON parser. A first install prompts for explicit module selection even
without `--profile` or `--menu`; a profile only contributes suggestions. An
initialized install preserves saved IDs unless `--menu` requests edits. Prompts are
bounded to three attempts and 60 seconds. It forwards only Core-offered IDs and the
exact offer digest as internal install argv; it never writes YAML. Dry-run/JSON/non-TTY
never prompt or persist selection. `install.selection-offer` is in the fixed helper
allowlist.
The original profile accompanies input-preflight and final reconcile for Core's
offer-digest recheck; `--menu` is offer-only.

When sourced from a deployed snapshot under `/opt/hton/deployments/<id>`, `projectRoot` resolves
the immutable code directory. Core/runtime helpers use the stable `/opt/hton`
config/work roots. Guided install outside the active snapshot invokes that source
tree's `scripts/deploy`, then re-enters the stable installed CLI once; the active
snapshot continues without redeployment. `deploy`
dispatch remains a fixed public command binding and does not use Core.
The read-only `install.selection-offer` may return the exact
`{operation:"install.selection-offer",status:"recoveryPending",writes:false}`
shape when a first-install allocation has a rootless journal intent. Shell then
calls normal no-argument `install.reconcile` once and returns its recovery outcome
before any selection, input prompt, package bootstrap or distribution action.
Dry-run preserves that read-only marker and does not invoke recovery.

During source install, resolving the protected active-source symlink is best
-effort before sudo: a session without refreshed hton group membership may not
traverse it yet. An unreadable target is not an installation failure; the
privileged re-execution resolves it authoritatively. No code/state modes widen.

Resolve configured/deployed root through the shared installation config reader.
Active-link lookup, source re-entry and minimum ownership guard use that root;
no hardcoded /opt/hton handoff. Full R1 custom-root execution is not promised.

install/setup share syntax parsing. install only delivers/CD/delegates; setup
checks physical CWD then enters the active Core path. Sudo re-entry preserves the
chosen command and literal typed options. setup never invokes deployment.

hton_install_target(source, target, original_options...) is the post-privilege
orchestration seam: compare physical CWD, deliver when outside, cd target and call
its setup with literal options/common flags. No configuration or Docker policy.
It is tested with private command fixtures for order, CWD and failure propagation.

`hton_minimum_setup(target, literal_core_options...)` runs the shared host-package
bootstrap before starting minimum Core, including on a fresh server without
PyYAML/cryptography. A bootstrap refusal/failure propagates without invoking Core.
Dry-run skips package bootstrap and forwards the preview to Core unchanged.

Shell refusals use an explicit ERROR prefix under OR-23; ordinary successful
steps and final readiness remain distinct. No error is hidden by quiet output.
