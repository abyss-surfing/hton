# Architecture review and proposed evolution

> Historical analysis snapshot. Current reconciliation: [FINAL-REVIEW](FINAL-REVIEW.md), [architecture](ARCHITECTURE.md), [interfaces](INTERFACES.md) and [orchestrator](IMPLEMENTATION-ORCHESTRATOR.md). Earlier gaps/schedules are not current acceptance criteria.


Date: 2026-10-31. Status: historical review. The subsequent owner request to improve architecture is implemented in ARCHITECTURE.md, CLI.md and specs/sdd-control.md; use those for current design.

## Scope and baseline

Reviewed README.md, docs/ARCHITECTURE.md, both archived Russian documents,
all 14 files under docs/specs/, all five local docs/_lang/ru/tasks/ files, .env.example,
and the current dispatcher, shared helpers and command scripts.
The local decision log was the historical source; current authority and precedence
are defined by DECISIONS.md. The .env, Bash-library layout and edge proposals below
are historical recommendations, superseded by the current architecture and SDDs.

The repository is a scaffold: Compose, service templates, admin and most CLI
commands are not implemented. Recommendations below concern the design;
they do not imply that a running deployment has been inspected or tested.

Preserve the owner's decisions: bash-first scripts/* and a thin dispatcher,
containerized services, host bind-mounted persistent state, HAProxy at the
edge, acme.sh DNS-01, and explicit consent for foreign mutations and cutover.

## P0: resolve before service implementation

| Issue | Evidence | Proposed resolution |
|---|---|---|
| Container loopback is confused with host loopback | XRY-1, DOT-1, ADP-1 and architecture section 3 | Own backends listen on their container network interfaces, without public port publishing. For host-loopback foreign services, choose an explicit host relay or a reviewed alternative topology; host-gateway does not reach host 127.0.0.1. |
| TLS routing is ambiguous | PRX-2/4/5; architecture section 4; IP mask acceptance | Specify a complete ordered ClientHello routing table, including absent SNI, absent ALPN, unknown SNI, malformed and fragmented hello, timeouts, and disabled backends. Separate TCP classification from the internal TLS/HTTP frontend. Define whether IP-only requests get the mask or are rejected. |
| No ALPN is treated as protocol identity | PRX-4, NMP-3/4 | Absence of ALPN is only a dispatch heuristic. Ordinary TLS can lack ALPN. Prove compatibility with actual MTProto and foreign clients, including camouflage SNI collisions; do not claim definitive protocol recognition. |
| HAProxy reload contract is incorrect | PRX-7, CERT-5, CLI switch contract | Use a documented master-worker reload mechanism for the pinned image, such as SIGUSR2 to the master, after configuration validation. Ensure the master is the signal recipient. SIGHUP is not the documented configuration reload mechanism. |
| Public port switching ignores Docker publishing | CLI switch contract, TEST-3 | Changing a container listener does not change Docker port mappings. Declare whether switch recreates the edge container or uses a different binding topology. Define the expected interruption; config reload alone cannot guarantee a published-port cutover. |
| ACME hook requires unavailable authority | CERT-5 versus ADM-7 | ACME writes a certificate candidate and a completion marker. The common apply path validates and installs it, then reloads the edge. ACME receives neither docker.sock nor general host execution. |
| Application whitelist is described as socket isolation | ADM-7 acceptance | A process holding the raw Docker socket retains Docker authority regardless of wrapper methods. Put a narrowly scoped controller between admin and Docker, or explicitly accept the broader trust boundary. |
| fakeTLS secret encoding is wrong | NMP-3/5, .env.example | mtg v2 generates fakeTLS secrets with an ee prefix in hexadecimal form, or its supported Base64 equivalent. Generate through the pinned backend and test Telegram import. Do not label dd as fakeTLS. |

Primary references:

- HAProxy master-worker reload: https://docs.haproxy.org/3.2/management.html
- Docker published ports and loopback scope: https://docs.docker.com/engine/network/port-publishing/
- Docker host networking semantics: https://docs.docker.com/engine/network/drivers/host/
- mtg secret generation: https://github.com/9seconds/mtg#generate-secret

NaiveProxy CONNECT over the chosen HAProxy HTTP/2 and Caddy h2c path is an
integration hypothesis until a real pinned client/backend combination passes.
Likewise WS and XHTTP need independent client tests; they are not interchangeable
wire formats merely because their path and endpoint are the same.

## Proposed control flow

Keep Bash as orchestration. Introduce one shared mutation path used by CLI,
admin requests, certificate installation and doctor repairs:

```text
read desired configuration + read observed state
  -> build plan with preconditions and required consent
  -> acquire project lock and recheck preconditions
  -> persist original state and transaction intent
  -> render candidate generation and validate all affected services
  -> apply ordered steps
  -> verify local health and end-to-end behavior
  -> commit generation, or compensate completed steps
```

Use flock for concurrent mutations. A dry run follows the planning and
validation path without writes or mutations, including package changes.
Never interpret journal contents as shell commands with eval.

Use a structured journal with transaction ID, schema version, resource identity,
before/after fingerprints, backup paths, action type and step status.
Persist intent before mutation; record applied, verified, committed or
compensated state. Resume/recover incomplete transactions after interruption.
This is a compensating workflow, not an atomic transaction across Docker and
the host. Rollback may itself fail; report that explicitly and retain backups.

For foreign containers, inspect output is a snapshot, not a guaranteed original
launch command. Preserve supported mounts, networks, labels, capabilities,
restart policy and other relevant settings; refuse unsupported configurations.
Check for external drift before restoring. Compare stable configuration and
client behavior, rather than requiring byte-identical docker inspect output
whose IDs and timestamps necessarily change on recreation.

## State ownership and filesystem

`.env` is desired installation configuration, not the entire runtime database.
Document ownership explicitly: users.json owns client identities; admin.json
owns the password hash; the journal owns mutation history; ACME owns its account
and renewal metadata. Generated service configs are disposable artifacts.
Admin and CLI must change users through the same locked apply operation.

Suggested layout, subject to approval where it changes existing contracts:

```text
etc/                         versioned templates only
lib/{config,services,plan,apply,journal,checks}
var/config/generations/<id>/  validated generated configurations
var/config/current           active generation reference
var/certs/                   certificate material
var/acme/                    ACME account and renewal metadata
var/data/                    users, admin auth, audit state
var/journal/                 transactions and original snapshots
var/run/                     locks and bounded controller socket
storage/backups/             encrypted recovery archives
```

Mount generation directories rather than individually bind-mounted files
when replacement uses rename; test how running processes observe replacements.
Validate first, switch the generation reference, reload/recreate as required,
then verify. A filesystem pointer change alone does not update running services.

Parse .env as data with a defined quoting/list grammar; do not source it.
The current unquoted PORT_CANDIDATES list already conflicts with shell sourcing.
Use safe serializers for JSON/YAML and explicit template variables instead of
unrestricted substitution. Enforce secret file permissions and redact diffs.

## Admin, certificates, DNS and optional components

- Admin submits bounded operations with fixed service IDs; no arbitrary paths,
  commands or Docker API forwarding. Add session expiry, Secure/HttpOnly/
  SameSite cookies and CSRF protection. A secret URL is an extra concealment
  measure, not authentication. Clarify initial password import and rotation:
  a password retained in .env is still stored in plaintext despite ADM wording.
- Prefer a systemd timer invoking existing Bash commands for automatic apply
  and doctor scheduling; the exact scheduling contract is a proposal. Avoid
  separate implementations of rendering/reload in admin, ACME and CLI.
- Validate certificate SANs, expiry, chain and key match before promotion.
  Retain the previous generation on failure. Dummy certificates represent
  degraded bootstrap state, not production readiness.
- Define unbound TCP ingress from the edge and DNS ingress from xray explicitly.
  Choose bounded public resolver policy and resource limits. SNI matching alone
  does not authorize DNS users. Clarify behavior when INSTALL_DNS=0, since
  XRY-4 currently requires unbound.
- Treat mask serving, Naive, DNS, admin, MTProto, Hysteria and IPv6 as explicit
  capabilities with dependencies. Disabling a capability removes its listener,
  routes, mounts, credentials and share outputs. SITE=0 must have a defined
  relationship with Caddy required by Naive.
- Prefer native IPv6 when available; make HE optional. Do not overwrite an
  existing default route blindly. Separate tunnel-link and routed-prefix
  addressing, check protocol 41 and MTU, journal host network changes.
- SSH router recipes need client ServerAliveInterval/ServerAliveCountMax;
  ClientAlive* belongs to sshd. Detect stalled sessions as well as interface
  disappearance. A second default-route metric does not detect a live but
  unusable tunnel. Preserve the owner's session-only, direct-fallback decision.
- A listening local port is not proof of public reachability. whoami should
  distinguish local observations, external probes and unavailable evidence.

SSH option reference: https://man.openbsd.org/ssh_config

## Documentation consistency

Resolve these without silently changing owner decisions:

1. Publish a reviewable, tracked CLI contract and accepted decision summaries;
   local gitignored files currently carry essential normative content.
2. Remove the obsolete Rust CLI workspace from architecture section 12 and
   reconcile NFR-2 with the explicitly allowed host package bootstrap.
3. Standardize runtime paths: .tmp/, certs/, var/remap.journal and var/* coexist.
4. Resolve CLI disagreements: --purge versus --volumes; --extra versus
   --router-extra; --domain requiring membership versus adding domains;
   global --yes wording versus mandatory stop-points; service aliases.
5. Add router and whoami to the SDD index. Mark archived Russian documents
   clearly at the top and link the current source.
6. Remove HTTP ACME routing unless a separate requirement justifies it;
   the accepted certificate path uses DNS-01.
7. Replace absolute claims about DPI invisibility, harmless deletion and
   zero-risk rollout with conditions and measured acceptance criteria.
8. Separate actual owner quotes from inferred design. In particular, router
   hardware/client choices are elaborations and need accurate traceability.

## Delivery order and acceptance gates

1. Reconcile contracts and implement config parsing, service registry, locking,
   journal recovery and candidate rendering. Gate: dry-run has zero writes;
   repeated apply has no diff; injected failures preserve recoverable state.
2. Build the smallest vertical slice: HAProxy + site + DNS-01 + VLESS on 4443.
   Gate: real TLS validation and client traffic; bad candidate preserves
   service; certificate renewal preserves an established long-lived stream.
3. Prove Naive and MTProto separately, then test the combined classification
   matrix with actual clients, missing ALPN/SNI and camouflage collisions.
4. Implement foreign adapters and 443 cutover only after those gates.
   Gate: interrupt/reboot/drift scenarios and rollback restore client behavior.
5. Add optional DNS, Hysteria, IPv6 and generated router recipes, then admin
   on top of the stable bounded controller. Test backup restore on a clean VM,
   including key availability, file ownership and journal recovery.

Use shell syntax checks and ShellCheck for Bash, service-native validators in
pinned containers, and disposable VMs for apt, systemd, firewall and adapter
tests. Do not use repeated forced production ACME issuance as a routine test;
exercise renewal with staging and controlled certificate fixtures first.
