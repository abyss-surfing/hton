# SDD control — plans, transactions and bounded administration

> Authority: [owner requirements](../OWNER-REQUIREMENTS.md). R1 is standalone CLI/Core with manual YAML validation and safe apply; API/configurator/web implementation is deferred. Native checks are required, Docker staging optional, IPv6 far future. See [final audit](../FINAL-REVIEW.md) for unresolved design details.

## Purpose and authority

Shared implementation design introduced by the owner's architecture-improvement
request on 2026-10-31. Preserves D21/D22/D24/D25 and foreign-change stop-points.
The controller, API and journal schema are engineering choices, not owner quotes.

## Requirements

### Owner-defined whole-server lifecycle (Q14–Q16, Q26–Q27)

`scripts/up` starts the saved enabled composition, including associated external
Amnezia, without reinstalling or upgrading images. It enables boot autostart.
Invalid configuration reports an error and offers separate explicit startup of
the last-known-good configuration; no automatic substitution. Partial startup
keeps healthy components running and reports failures. Repeated `up` starts
missing components without restarting healthy ones.

`scripts/down` stops the entire associated service composition, including external
Amnezia, preserving settings/data and disabling startup across host reboot.
One stop failure does not prevent other stop attempts. Report failure and list
components still running; only report server off after verified full shutdown.
Disabling autostart remains required even when stopping a component fails.

These requirements supersede any assumption that external configuration
ownership exempts Amnezia from whole-server start/stop. Detailed state transitions are owned by sdd-lifecycle.md and mirrored in the file map.

- CTL-1 CLI, optional controller, certificate reconciliation and doctor share configuration,
  service registry, planning, journal, apply and checks. Bash remains orchestration;
  externally container-built Rust helpers handle complex structured logic when needed.
- CTL-2 Config lock serializes desired edits/preparation; runtime owns a separate
  short-held effect lock and durable trial lease. Release management's lock before
  waiting for trial verification; a hung web/controller cannot block guardian recovery.
  Recheck revisions/fingerprints and reject competing promotion while a lease is active.
- CTL-3 Dry-run performs read-only planning: no files, images, containers, packages
  or journal writes. Missing validation tools produce explicit unverified checks.
- CTL-4 Persist originals and intent before effects. Record schema version, ID,
  actor, resource IDs, fingerprints, preserved-original references and timestamps. Steps are
  planned/prepared/applied/verified/compensated; transaction results are committed,
  rolled-back or recovery-required. Flush/fsync intent before destructive steps.
- CTL-5 Recovery reconciles observed effects when a prepared step was interrupted
  before its applied record. Resolve unfinished transactions before new mutations.
- CTL-6 Render immutable candidates and validate affected native configurations.
  Keep editable desired inputs distinct from immutable generated artifacts; preserve original values and accepted generation. Failed manual edits remain for repair. Commit only after checks.
- CTL-7 Compensate completed effects in reverse dependency order. Check for drift;
  never overwrite unrelated external changes. Failed rollback retains originals
  and reports recovery-required. No executable shell strings or eval in journals.
- CTL-8 Registry declares enablement, dependencies, probes and change action:
  no-op/reload/restart/recreate. Aliases: naive -> caddy, dns -> unbound, vpn -> xray.
- CTL-9 Plans/logs redact secrets; protected originals may contain them. Audit
  records actor, operation, resource, result and transaction ID, not credentials.
- CTL-10 All HTON component/module logs go to the installer-selected log directory
  (default `/var/logs/hton`); the desired config selects verbosity. Debug/trace
  output must still redact secrets and credential-bearing URLs. Include timestamps,
  component/module, severity, operation/transaction correlation ID and structured
  error context so a support/AI workflow can assemble an incident bundle quickly.
  Automatic rotation is mandatory; retention is one week — logs older than seven days are deleted, and configuration may shorten but not extend that window. Configurable size limits and bounded exports must prevent logging pressure from stopping VPN.

## Controller interface

### Explicit saved-target public cutover

`publication.switch` uses the saved declarative HAProxy edge target. An optional
CLI port asserts that exact saved target; R1 manual YAML editing remains the owner
of target changes. A mismatch fails before consent, and the root revision is bound
again after preparation. Shared effect preparation checks complete fresh host
TCP/UDP reservations before any local public-listener grant writes. Unknown or
co-tenant holders block without stopping/remapping them. The normal apply owner
performs native admission, bounded cutover and compensation. `--validate-only`
and dry-run remain read-only and cannot manufacture missing public consent.
The operation reports actual pending/accepted/partial state, with external DNS and
served-route readiness unverified; it is not the deferred publication module.

Read-only `diag.observe` uses bounded registered-resource and durable-runtime
facts only; it does not issue public network queries. External identity probes are
permitted only during an explicit `identity.whoami` invocation (owner requirement
OR-8). Missing evidence remains `unknown`/`unverified`, not inferred success.
R1 `doctor.observe` is a thin read-only entry to the same bounded registered local
diagnostics as `diag.observe`, limited to services/runtime/Docker by default. It
does not repair, schedule work or issue public network probes. `doctor.fix` remains
unwired until an exact owned-resource repair planner and journaled action path exist;
`doctor.watch` remains unwired until the optional scheduler has one explicit owner.

Core exposes a versioned in-process/CLI operation interface in R1. CLI invokes
Core directly and never depends on a running controller. The later optional full
management API is a separate module; web, GUI, app and MCP use its typed operations.
A private Unix socket is a possible local transport, not an R1 daemon requirement.
The later public interface uses token authorization and bounded operation scopes.
Mount the socket parent directory when used; no admin/ACME Docker socket.

Requests carry apiVersion=1, requestId and a fixed operation schema:

| Operation | Validated inputs | Effect |
|---|---|---|
| status | Optional fixed section enum | Redacted normalized checks |
| logs | Fixed source enum, bounded line count | Redacted tail |
| share | Existing user ID, enabled kind enum | Deliberate credential output |
| revoke | Existing user ID | Locked users update, render, Xray restart |
| restart | Enabled own service ID from the catalogue, within approved scope | Journaled restart and health verification |
| config.read | No file path | Redacted normalized config and revision |
| config.validate | Known configuration object IDs / protected draft ID | Core aggregated syntax, module and cross-module checks |
| config.plan | Prepared atomic parameter batch, touched-value/operation preconditions | Rebase independent edits through configurator; validate merged snapshot; produce immutable effect plan |
| config.apply | Plan ID, expected revision, request ID | Exact approved own-resource plan through shared apply |
| keys.generate / keys.rotate | Existing device ID and allowed credential kind | Scoped generation/rotation; one-time explicit secret output |
| devices.upsert | Validated device fields and expected revision | Managed identity/profile update |

`services.restart` is a bounded Core lifecycle intent. Explicit IDs must resolve to
catalogue services with exact installed Owned registry resources and receipts in the
current accepted bundle. Omitted IDs select enabled optional/data services only;
core ingress is excluded. Runtime rejects empty, missing, Associated or stale targets,
active trial/maintenance state and epoch drift. It restarts the exact admitted resource
without converting drift into Recreate or rewriting artifacts. It verifies Running
status and admitted launch identity, then runs declared semantic probes. Unsupported
probes produce a journaled Partial result with completed receipts/survivors preserved.
The default wait deadline is 10 seconds; `--wait` accepts 1..300 and narrows it.

Resolve all paths, project and container IDs internally; reject unknown fields,
operations and cross-project resources. No shell fragments, arbitrary arguments,
exec, API proxy or arbitrary file reads. Repeated requestId with identical
payload returns the durable result; a different payload is rejected. Reconcile
uncertain pending effects before retry. Responses include version, request ID,
status, transaction ID where relevant, redacted result and structured error.
Authorize and audit before effects. Qualified own-module installation/configuration is allowed through config.plan/apply within locally approved listener/privilege envelopes (sdd-config.md). OS bootstrap, foreign mutations, public-port switching, policy exceptions and new host-network privileges require local CLI approval. No UI request can create or broaden that approval.

VPN port allocation is reviewed by the local `host.approvals.authorizeAllocation`
owner after a complete fresh allocation candidate exists. The confirmation shows
each service, chosen port, setting path and transport, and requires a digest-bound
TTY challenge. Its opaque capability binds the complete candidate, coherent read-set
revisions, instance, local actor, operation ID and expiry; a caller-supplied
`ApprovalId`, `AllocationReview` or `--yes` is insufficient. The allocator journals
the same immutable candidate as `Intent::PortAllocation` before any settings CAS.

Plan IDs bind candidate, secret versions, catalogue version, current resource fingerprints and approvals. Unknown/stale/expired plans fail; changes to plan inputs invalidate that effect plan. Independent edits can be merged by the configurator before replanning; a file revision change alone is not a parameter-batch conflict (Q34). The web never writes hton.yaml directly. Core cannot be disabled by a feature request.

ACME writes candidates and a completion marker. A host timer consumes fixed
candidate paths through shared apply; marker contents are never executed.
When implemented, doctor --fix is limited to bounded own-service repairs and cannot
bypass consent; it currently remains unwired (see [implementation status](../IMPLEMENTATION-STATUS.md)).

## Acceptance

- Concurrent CLI/admin/renewal operations serialize without lost user updates.
- Kill at every transition, including between effect and record: recovery
  succeeds or reports recovery-required without overwriting external drift.
- Invalid config/certificate preserves working runtime and desired state.
- Repeat apply has no diff/reload; retry requests do not duplicate effects.
- Malicious service names, paths, shell text and raw Docker requests are rejected.
- Foreign remap/cutover is unreachable through the controller.
- Backup restore preserves protected originals and verifies permissions.

## Deferred operation variants

Post-release sdd-identity.md narrows all managed-user credential operations to
single-disclosure issuance. Its idempotent retries return metadata/status only,
never replay a secret result. No existing share/keys/config/QR operation may bypass
that future policy after migration. Account ownership and scoped publication/agent
operations are added only with F01–F08, not as part of the release controller.

## Runtime separation (overriding architectural boundary)

Controller prepares proposals; runtime admission/guardian are independent and admit
only exact validated bundles/fixed accepted-resource actions (sdd-runtime-safety.md).
No VPN traffic or accepted cold boot calls this API. Kill/upgrade/OOM of controller
or admin does not stop data-plane Compose. Failed manual desired config stays for
repair while accepted runtime remains intact; a web draft commits only against its
source revision. Docker bundle tests are explicit mutating qualification, not dry-run.

### Explicit bundle validation

`hton validate BUNDLE_ID` resolves only a BundleId in the protected work bundle
store. The owner reloads `candidate.json`, recomputes its canonical candidate digest,
checks every staged artifact hash and verifies images by immutable digest before
creating any test resource. It derives a private Compose project and private test
network from that candidate; test-only port mappings are generated from the bundle's
declared listener claims and cannot bind host/public addresses. No foreign mount,
arbitrary image, shell command or client-supplied Compose is accepted. Run fixed
native validators first, then the selected pinned server/client scenarios. Every
created resource is journaled by exact immutable identity before start; cleanup
deletes only those identities plus the operation-owned network/project. Interrupted
or uncertain cleanup returns `recoveryRequired` and cannot issue a passing result.

`validation.bundle.testBundle` returns a protected `BundleValidationAttestation`
binding the BundleId, candidate digest, artifact/image digests, runner version,
per-scenario evidence, operation/time and exact cleanup receipt. Public DNS,
external reachability, real devices, censorship and broad client compatibility are
reported unverified. Ordinary apply does not implicitly run these semantic tests;
apply may consume only a successful nonexpired attestation for the exact candidate.

## Deferred configurator and lifecycle integration

Automatic editing/merging/migration belongs to the independent configurator, not
mandatory Core. R1 manual YAML check/plan/apply stays in Core. Management API
parameter batches delegate to sdd-configurator; no whole-file replacement endpoint.
Multi-file publication and arbitrary manual writers require a proven coordination
contract before advertising atomicity; locks alone do not cover external editors.
Module and whole-server operations delegate to sdd-lifecycle, including adopted
Amnezia. `down` persists a stop fence before effects, and recovery cannot undo it.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

Read-only plan previews have no persisted PlanId. Explicit `config stage` (CLI)
or `config.preparePlan` (future API) persists the exact plan through the journal;
apply looks it up and rechecks all inputs. `config.plan` is a preview. Secret-bearing
responses are excluded from ordinary durable idempotency caches; retries return
metadata/status, with issuance semantics narrowed further by the accounts policy.
See INTERFACES.md sections 8 and 12 for API-triggered self-shutdown behavior.
