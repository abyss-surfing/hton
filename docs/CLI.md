# Public CLI contract

Public syntax and bounded shell adapters are written; host/domain completion and
acceptance are tracked in IMPLEMENTATION-STATUS.md. This is the
single declaration of CLI arguments; per-file SDDs explain, rather than redefine it.
Internal native helpers/guardian have a separate fixed trusted interface.

## Common rules

hton <command> [subcommand] [arguments] [flags]. Long value options accept
--key value or --key=value. Unknown/conflicting inputs return 2. Every command has
-h/--help (0, no side effects). Flags can appear before/after the command.

| Flag | Meaning |
|---|---|
| -y / --yes | Ordinary prompts only; never foreign/host-policy/443 consent |
| --dry-run | Read-only plan; no config, secrets, journal, packages, images or containers |
| --json | Structured result on listed commands; logs stay on stderr |
| -q / --quiet | Installation: no questions; use saved/default values, noninteractive sudo, suppress routine/secret output; retain redacted errors and exit status |

Exit codes: 0 success, 1 failure, 2 usage, 3 missing dependency, 4 declined/missing
required consent. whoami informational warnings may return 0. Non-TTY missing
local consent fails before effect. Dry-run reports unavailable checks as unverified.
Service IDs/aliases come from the catalogue, not a separate hardcoded CLI list.
Scoped service actions target enabled qualified owned resources; whole-server lifecycle also includes explicitly associated foreign resources. Public service operations
cannot target arbitrary host units or control-plane infrastructure.

## Installation configuration

`platform/etc/install.json` declares non-secret initial installation.directory and optional
site domain/account email. Installed `<directory>/config/hton.yaml` takes priority;
missing site values are prompted and saved. `scripts/install` needs no repeated site
flags when configuration supplies them. The minimum directory is a canonical
direct child of `/opt`, default `/opt/hton`; source target changes do not migrate
existing installations. See MINIMUM-INSTALL.md and OR-16.

## R1 commands

| Command | Arguments / behavior |
|---|---|
| help | Print declared commands and implementation availability |
| install | [--profile minimal\|home\|compat\|lab] [--site-domain DOMAIN] [--certificate-email EMAIL] [--amnezia-container NAME_OR_FULL_ID] [--dns-credentials PRIVATE_JSON] [--with mtproto,hysteria,outline,amneziawg] [--without mtproto,hysteria,outline,amneziawg] [--mtproto-camouflage DOMAIN] [--outline-binary PATH] [--chain-sni DOMAIN] [--hysteria-port PORT] [--test-sni DOMAIN --test-tunnel-port PORT] [--no-test-tunnel] [--rotate-client-page] [--fresh] [--docker-only] [--menu] [--skip-remap] [--skip-start] |
| setup | Same configuration/native and minimum-protocol options as install; physical CWD must equal configured target; collect missing inputs and perform setup |
| update | Fast-forward `main` from `origin/main`, then reinstall the HTON-owned composition; foreign containers untouched; `--dry-run` reports the plan without network/Docker effects; the installer enables a daily check that updates only after a week without install/update |
| deploy | [--source-dir DIR]; copy the minimum runtime closure into `/opt/hton`, excluding `.git`, `docs/_lang/ru/tasks/`, generated build output, full-R1 sources, build-only scripts and mutable state; delivers the Python minimum installer, shell entrypoints, operator docs and image inventory; no target build/download/service start |
| usage | [--json] [--collect]; print per-user Outline WSS traffic from the loopback metrics collector; `--collect` stores a sample without printing; run with sudo |
| modules | list [--json]; add ID; enable ID... [--save-only]; disable ID... [--save-only]; remove ID; purge ID; update ID; see lifecycle rules below |
| up | [--json]; validate and start missing members of saved enabled composition; enable boot autostart, no pull/update |
| down | [--json]; persist stopped intent, disable autostart and stop all associated members; continue after errors and list survivors |
| config check | [--json]; read-only schema and available semantic validation |
| config plan | [--file FILE] [--json]; read-only redacted preview/fingerprint; no stored PlanId |
| config stage | [--file FILE] [--json]; explicit mutating prepare, native checks and protected persisted PlanId; no activation |
| config apply | [--plan ID] [--json]; current desired or exact staged plan, rechecked under lock |


| validate | BUNDLE_ID [--json]; explicit isolated Docker qualification, not dry-run |
| runtime | status\|start\|rollback [BUNDLE_ID] [--json]; fixed local accepted-runtime actions |
| diag | [--json] [--verbose] [--section SECTION]...; read-only evidence |
| keys | [kinds...] [--device ID] [--show] [--force] [--json]; scoped release credentials |
| share | [identity] [--kind K]... [--target TARGET] [--client CLIENT] [--format text\|json\|qr] [--out FILE] [--json] |
| certs issue | [--domain D]... [--force] [--json]; default certs subcommand, configured names only |
| certs status | [--json]; SAN/expiry/served state; below 14 days is unhealthy |
| certs list | [--json]; configured inventory |
| restart | [services...] [--wait N]; all enabled optional/data services if omitted; wait default 10s |
| switch | [PORT] [--validate-only] [--json]; publishing cutover, may disconnect clients |
| remap-undo | [last\|all\|ID]; drift-aware restore; missing selector requires TTY |

| doctor | [--json]; read-only local observations for services, runtime and Docker |
| uninstall | [--keep-certs] [--keep-env] [--foreign] [--purge]; local ownership-aware full maintenance |
| whoami | [--json]; read-only identity/DNS/local-vs-external evidence |

install-ubuntu/install-unknown are internal dispatcher helpers, hidden in public
help. They obey the same dry-run/consent limits; accepted runtime boot does not
execute them or load the config compiler. runtime start defaults to last-known-good;
rollback defaults to the exact previous-compatible bundle retained in the current
committed lease. An optional rollback ID must equal that protected target; arbitrary
old IDs cannot bypass current policy/certificate fences. Rollback is available only
while run intent is Running; down wins by epoch. Successful rollback consumes this
one-step fallback until a later qualified commit supplies another.

## Config and selection

Install selection is owned by Core and the trusted module catalogue. `--profile`
selects a first-install suggestion set only; it is not itself consent to install,
associate, enable, or mutate a module. On a new installation, Core presents the
suggestions for explicit selection, with optional modules otherwise off. The chosen
module IDs are persisted in canonical `hton.yaml`. On an initialized installation,
rerunning `install --profile ...` never replaces saved choices. `--menu` asks Core
to reopen the current selection for review/editing; it does not reset it to profile
defaults. Invalid or unavailable catalogue entries fail before bootstrap effects.
The installer passes the resulting saved/approved selection to the same Core
catalogue, graph and allocation owners used by module lifecycle operations. VPN
allocation therefore sees only explicitly selected, qualified service bindings;
repeated install loads their saved service ports and does not choose replacements.
Optional modules default off; core is installed/protected, systemd enabled and PATH
shim off by default.

The minimum profile adds `mtproto`, `hysteria`, `outline-wss` (alias
`outline`) and `amneziawg` (aliases `amneziawg2`, `awg`, `awg2`). On a first
selection with no optional-protocol keys they are all enabled by default, so no
`--with` argument is required. `--with`/`--without` remain optional bounded
overrides, and a saved choice is reused on later installs. AmneziaWG2 uses the
pinned AmneziaWG 3.1 userspace images, its own public UDP port from 5000-6000
and one client `.conf` per native user. The shared REALITY/MTProto SNI defaults to the configured site mask
domain; `--chain-sni` is an optional subdomain override. MTProto camouflage
defaults to the same SNI; `--mtproto-camouflage` is optional. Own Outline
defaults to the pinned `outline-ss-server` v1.9.2 path; `--outline-binary` is
optional. Hysteria defaults to UDP/8443 and accepts `--hysteria-port`. With the neutral chain, install keeps the native REALITY listener unchanged by
default and serves the own SNI through the copied-clients listener. Moving the
native listener to the own SNI requires the explicit `DROP-LEGACY-SNI` phrase; an
already migrated install offers to restore the previous native configuration. A
subdomain override requires MTProto and a certificate SAN.
These selectors are minimum-profile only. Own Outline runs
Shadowsocks-over-WebSockets on a loopback listener and Caddy reverse-proxies its
secret paths under the site certificate; HAProxy still owns TCP/443.
Generated MTProto/Hysteria/Outline/AmneziaWG credentials remain protected; share links and
the Outline dynamic access key are written 0640 `root:hton` under the minimum
work tree (readable by the operator group, never by others) and never into JSON
or logs. Disabled protocols remove their listeners and retain
settings.
Guided install from a working source tree first invokes the same privileged deploy
owner used by `deploy`, then resumes through `/opt/hton/bin/hton`. Deploy copies
current source files, including uncommitted changes, without requiring a release
manifest or clean commit; `.git`, `docs/_lang/ru/tasks/`, generated build output and mutable
state are excluded. The `hton` group can read/execute installed code but cannot
modify it or gain sudo; config/work and private secret state remain root-private.
Deployment stages a content-addressed source snapshot under
`/opt/hton/deployments`, atomically switches `current`, and preserves prior snapshots
and all mutable state. It verifies the copied file set and hashes but does not
authenticate the source publisher. It does not download code or build on the VPS.
Missing prebuilt Core/runtime helpers are reported; deploy never compiles them.
--docker-only performs bootstrap only; --fresh foreign installs are explicitly
selected and journaled. --skip-remap leaves foreign resources untouched; --skip-start
can stage a plan but never starts own services; any explicitly selected foreign
mutation still requires consent and is reported, not implied harmless.

On first unconfigured publication, `install` guides the operator through the site
domain and public edge endpoint. VPN backend ports are allocated by Core from
5000–6000, persisted in the existing settings owner for each selected service, and
reused on repeated install; they are never requested as TTY input. Public edge
defaults to TCP/443 only for that first publication. These site values are prompts
only; they are not public install flags.
Before bootstrap or package effects, the shell calls the pure read-only
`install.reconcile --input-preflight` mode. Core returns
`{"status":"inputRequired","operation":"install.reconcile","requiredInputs":[...]}`
with exit 0 and no effects only when saved config lacks values. The shell prompts
only for those named fields (site domain, edge port and/or certificate account
email), with three attempts and a 60-second read limit, then re-runs the pure
preflight once with those typed values. Domain has no default; the public edge port suggests 443. VPN
port candidates are generated after fresh reservation evidence and displayed in the
exact plan for review. Values are non-secret. Bootstrap and final reconciliation proceed only after Core returns
a conflict-free `ready` plan. Dry-run never prompts. Non-TTY and `--json` input
returns the structured `inputRequired` result unchanged (exit 0); it does not guess
values or continue with effects. `example.com` is the current instance's saved domain,
not a default; Outline's existing 8443 mapping is historical host state outside the
new VPN allocation range, not an Amnezia input target.
Mutating Core reconciliation persists the exact typed input and VPN allocation,
completes supported owned setup, issues or reuses the DNS-01 certificate, applies
the normal ingress bundle and verifies local HTTPS with the exact site body. If
InternetBS credentials are missing, it returns a resumable partial result and the
TTY flow imports them from protected files through SecretStore. The current host
still has no trusted `example.com` certificate, and Amnezia remap remains blocked
until its listener and snapshot/restore are qualified; source implementation does
not prove live deployment. A saved non-public edge such as loopback TCP/4443 also
stops for exact review; Enter/default never accepts that change. Guided install
deploys the current working tree to `/opt/hton` before publication reconciliation.
If the copied tree lacks `libexec/hton-core`, it returns dependency exit 3 before
publication prompts or configuration effects; it does not compile on the host.
The explicit `--docker-only` dependency bootstrap remains independent of Core.
Ready means HAProxy is active from the `/opt/hton` deployment, the configured site
SNI returns the site over HTTPS with a trusted certificate valid for that domain,
and the qualified Amnezia SNI route reaches its native backend without REALITY
termination. Install verifies these outcomes before reporting ready; missing
environmental or Amnezia evidence leaves an explicit degraded/blocked result.
DNS provider credentials stay in the protected SecretStore and saved `secretRef`; no
secret is accepted in argv. Repeated install reuses saved values. For saved changes,
show before/after and require digest-bound explicit review; `--yes` cannot provide that
review. A coherent-read/CAS conflict returns without overwriting. Non-TTY missing values return `inputRequired` before effects.
Public TCP/443 and foreign listener remaps require separate exact local consent;
`--yes` cannot bypass either. A port conflict is reported during read-only
preflight before config write, journal/snapshot writes, packages, certificate issuance
or native effects. Initial desired values are transient until conflict-free consent. Missing non-interactive consent fails before effects.

config plan/check and --plan-only never write. Local --file accepts a bounded
candidate path; web accepts schema data, not arbitrary filesystem paths.
--no-reload stores a protected pending candidate without changing accepted runtime
or overwriting live files. apply regenerates/revalidates stale candidate evidence.
Manual invalid desired bytes remain for repair; accepted runtime continues working.

Diag sections: config, docker, ports, services, certs, journal, runtime, tunnel, admin.
Legacy env section spelling is not a new source of truth. Public HTTP ready status
requires trusted certs; local bootstrap/NO-CERT is degraded.

## Credentials and device exports

keys kinds: all (default), uuid, vpn-path, mtg-secret, naive, admin, socks, plus
qualified module kinds. Missing values are generated; --force is replacement with
ordinary confirmation, not --yes. --device always means an existing identity ID.
--show/share intentionally expose release-owner credentials; R2 changes managed-user
semantics explicitly as described below. Logs/plans/metadata never expose them.

share identity defaults to default. --target is android-tv|quest|apple-tv|router|
gateway|other-tv; --client is flclash|hiddify|amnezia|sing-box|passwall2|native. Reject
unqualified client/version/transport rather than substitute. Kind IDs follow module
IDs plus admin-url; mtg is a legacy alias of mtproto, tailscale of tailscale-exit.
Default kinds are selected supported endpoints; no disabled module output.
--json aliases --format json; contradictory flags fail. --out creates a protected
0600 file and also writes stdout only if TTY; dry-run never writes the output file.

Router recipe options: --router (alias --target router), --no-dot, --extra, --ssh.
Conflicting target selectors fail. --extra appends selected Hysteria only.
--ssh uses selected session-only D1/D2 profile, never autossh. SSH -D alone does not
carry arbitrary game UDP; client ServerAlive* and cleanup are explicit. Direct
fallback follows selected policy; it cannot override an R2 strict-family token.

## Operations and boundaries

`switch [PORT]` applies the saved declarative edge configuration through normal
admitted apply and public-listener consent. An optional port must equal the saved
target; edit desired YAML before changing that target. Complete fresh host listener
checks reject unknown or foreign holders before consent or native effects. The
command does not remap a discovered foreign container. `--validate-only` is
read-only; missing protected consent remains an explicit approval error. Actual
pending/partial apply results are retained, without claiming served-site or native
Amnezia readiness. Docker port changes recreate the edge. Release the target
binding before restoring foreign originals. If implemented, doctor --fix stays bounded to approved
own-resource repair, never automatic foreign modification or protocol enablement.
runtime rollback is subject to revocation/family/cert compatibility, not blind
restoration of old files.

`doctor` currently wires only the default observation action. `doctor --fix` and
`doctor --watch N` are parsed but return the structured `module.incomplete` error
(exit 1); no repair or scheduler action is performed. `doctor --json` preserves the
same bounded status/counts/issues/unverified fields as `diag`, with operation
`doctor.observe`. This route reads registered local services/runtime/Docker only;
it performs no external probes. Use explicit `whoami` for permitted public identity
probes.

Backup covers YAML, secret versions, identities, ACME, accepted/fallback bundles,
fences and journal; decryption identity must be available outside the archive.
Restore checks ownership/paths/config before admission and never blindly replays
another machine's foreign changes. Purge waits for required foreign restoration;
--keep-certs retains certificate state; --keep-env preserves legacy .env only.
Canonical config/secrets need separate explicit archive/delete choices. Core
non-toggleable does not prevent local maintenance stop or uninstall.

## Post-release syntax reservation

Personal-account/token/publication/agent commands are not assigned public syntax
until R2 delivery. Their contracts live in sdd-identity, sdd-client-ipv6,
sdd-smart-connect and sdd-web-publish. One-time creation alone returns reusable
secret/config/QR material; later share/keys --show/API/TG/retry cannot redisclose it.
Token creation initially takes permitted protocol profiles and editable date-default name; strict egressFamily ipv4|ipv6 is far future. Rename is metadata-only; changing secret/family is
revoke+create, with policy-clean fallback. Existing release clients migrate explicitly.

## Module and compatibility rules

`modules add ID` obtains resources, seeds missing config and prompts/guides setup;
a new member remains disabled. Incomplete setup is saved and repeated add resumes.
`enable/disable` validate and apply immediately with a disruption preview;
`--save-only` saves participation for a later combined `config apply`/`up`. While
the server is stopped, these operations never start it. `remove` preserves data;
`purge` explicitly deletes retained owned data with local confirmation. `update ID`
uses that module's update logic and is the only module image-update operation.
See [lifecycle](specs/sdd-lifecycle.md) for dependencies and partial failure.

`up` with malformed desired config fails and offers an explicit
`runtime start BUNDLE_ID` of an accepted compatible bundle; no automatic fallback.
That explicit action also records running intent and restores boot autostart.
`down` uses registered resources even if desired YAML is broken. Partial start/stop
returns 1 with per-member results. Healthy members are not restarted by repeated up.

`feature` and `service` remain compatibility adapters. `feature enable/disable
--plan-only` maps to `modules enable/disable --save-only`; `service start/stop`
map to whole-server up/down. OR-17's former `update` config-apply alias is
replaced by the OR-55 fast-forward-plus-owned-reinstall operation; the HTON-owned
scope is fixed, so mutation is no longer blocked by an open scope question.
Help must label their availability explicitly. Server IPv6/tunnel commands and
`config migrate-env` are deferred; R1 has manual YAML check/plan/apply. Future
configurator operations use atomic parameter batches and preserve concurrent edits.

## Deferred commands

`upgrade` later coordinates protected source update, preserving configuration
maintenance and module-owned image updates. `config migrate-env` belongs to that
future configurator. Their full implementation is excluded from R1.

The planning `--plan` flow uses an explicitly persisted protected pending plan;
read-only `config plan` only prints a plan preview/fingerprint and creates no ID
in storage. `config apply` without `--plan` replans current desired inputs. A plan
ID may be used only when an earlier explicit mutating prepare/stage operation
persisted it. This prevents a supposedly zero-write plan from creating hidden files.


## Installed helper transport

All R1 command scripts share bounded argv parsing and invoke fixed Core operations
with --config-root/--work-root, canonical common flags, --, then literal operation
arguments. No YAML/env sourcing, dynamic helper selection, code download, compile
or generic shell is allowed. Domain failures propagate; a present wrapper does not
prove native qualification. Runtime status calls the independent runtime helper;
start/rollback use Core to prepare typed admitted requests. No API daemon is needed.

The shell enforces syntax/cardinality; Core enforces install/ownership/approvals,
read sets, secret grants, stopped epochs and native postconditions. --yes carries
ordinary prompt metadata only. --force credential rotation, purge, foreign changes
and public 443 must obey their separate confirmations; a boolean shell flag never
manufactures those grants. Source syntax checks are not implementation acceptance.

Guided minimum install must run from the cloned project root. It copies its
runtime closure into `/opt/hton` through `scripts/deploy` whenever the source
root differs from the configured target, including uncommitted changes, then
runs the installed setup as root with `SUDO_USER` preserved. Deploy always
delivers the minimum closure; the bounded allowlist excludes `.git`,
`docs/_lang/ru/tasks/`, generated `target/`, credentials, mutable state, Rust/full-R1
sources and build-only scripts. Each run also applies registered migrations once
(stale runtime material removal, permission normalization). From an installed
copy whose source equals the target it skips delivery and applies
migrations/setup only. Copy hashes check transfer integrity; no release manifest
or clean commit is required.
No remote artifact download, host compilation or package removal is used.
`--docker-only` may bootstrap missing Ubuntu/Debian dependencies without deploying
HTON; `--dry-run` creates no files or installs packages. Full install delegates
configuration and setup writes to `install.reconcile`.

Source os-release fields are parsed as data, never sourced. Existing supported
Docker is preserved. Conflicting existing Docker/containerd packages, repo/key drift
and nonempty orphan Docker data require manual reconciliation. Ubuntu/Debian use
their own official signed apt origins; other OSes receive primary-source guidance.

feature --plan-only uses modules enable/disable --save-only plus dry-run, preserving
zero-write planning. The former update/config-apply alias stays removed; `update`
now follows OR-55. Service start/stop map to whole-server lifecycle.up/down;
service status directly reads runtime; restart uses services.restart. Unsupported
legacy update operands are rejected rather than silently broadening their effects.

## Install/setup/update separation (OR-17)

`install` delivers the working source with `deploy` when physical CWD differs from
the configured target, changes into that target, then runs `setup`. Inside the
target it skips delivery. `setup` accepts the same site/native/secret options as
install but requires target CWD before privilege, prompts or effects. It owns
configuration collection through Core; it never delivers source.

`update` performs the OR-55 fast-forward of `main` to `origin/main` and then
reinstalls the HTON-owned composition (pinned images are re-pulled and changed
containers recreated). The scope is HTON-owned containers and configuration;
independently installed VPNs and unrelated host containers are not updated. A
failed fast-forward stops before container effects. `update --dry-run` reports
the plan without Git/network/Docker effects.

The installer enables `hton-minimum-update.timer`; it checks daily at a
randomized quiet hour but acts only when `work/minimum/update.json` shows no
successful install/update for seven days. `scripts/install` itself performs the
same fast-forward best-effort before delivery, so an explicit install also
updates the installed release; a missing Git checkout, offline network or dirty
divergence is reported and the current source is installed instead. Unattended
update never prompts, performs no public DNS/identity observation and requires a
committed installation. If the installed target is not itself a Git checkout
(for example it was deployed from a separate source directory), the unattended
run refreshes the pinned images but cannot deliver new source; run
`scripts/install` from the source checkout for a code update.

Default minimum certificate mode is HTTP-01 without provider credentials. Existing
HTTP/80 listeners remain running; setup can verify their public challenge webroot.
Only explicit dns-01 uses a configured supported DNS API adapter. An unissued,
uncredentialed legacy minimum attempt migrates to HTTP without changing its saved
Amnezia backend port or native approval. Background renewal reuses the saved HTTP
transport and private account, with no public identity discovery.

## M1 input, client-page and private-share contract

The source implements the minimum profile path. Primary settings are in config;
no new repeated --sni/--admin-user/--page-secret arguments. Legacy --chain-sni,
--mtproto-camouflage and --hysteria-port remain diagnosable compatibility inputs:
conflicts with saved separate names or non-443 public policy require explicit
migration or rejection, never ignored flags. admin.access/user/sshPort are module
settings; page path/names/refreshSeconds are root core.clientAccessPage.

An absent `core.clientAccessPage.path` publishes the guest page at a generated
CSPRNG URL on first install; an explicit `path: ""` deliberately disables it.
`--rotate-client-page` replaces an existing URL immediately. Examples omit `path`,
so a fresh installation publishes automatically. -q/--quiet never prompts and
applies the same absent/empty rule; it uses sudo -n and exits with redacted
nonzero inputRequired or policyBlocked on missing required input/privilege/
ungranted approval. --yes does not relax consent boundaries. JSON/no-prompt and
dry-run never reveal secrets or the generated URL.

After a successful install the per-user guest artifacts and `client-page.md`
(containing the bearer URL) are written to `<installation>/.secrets/share` and to
the source checkout `.secrets/share` when present, all `0640` group-readable.
No output includes private admin link bytes or individual VPN credential links;
the successful interactive install prints only the client-access page URL and
the share directory. JSON, quiet and dry-run output exclude the URL, receipts and
logs never contain it, and explicit local share/reveal remains the existing
privileged disclosure boundary; no web endpoint can request it. Full admin stays
TODO, not a minimum dependency.
