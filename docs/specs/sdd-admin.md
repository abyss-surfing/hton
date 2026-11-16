# SDD admin — deferred optional web configuration, devices and Telegram

> Authority: [owner requirements](../OWNER-REQUIREMENTS.md). R1 is standalone CLI/Core with manual YAML validation and safe apply; API/configurator/web implementation is deferred. Native checks are required, Docker staging optional, IPv6 far future. See [final audit](../FINAL-REVIEW.md) for unresolved design details.

## Purpose

Optional later-iteration lightweight web admin for configuration and device onboarding;
React assets built outside the VPS, served by axum. Telegram is optional within
the same runtime. Configuration/state semantics are owned by sdd-config.md;
capability choices by sdd-capabilities.md; arguments by ../CLI.md.

## Requirements

- ADM-1 Web/auth is an optional module. CLI and declarative configuration remain fully functional with it disabled. Telegram starts only when selected;
  unavailable Telegram never blocks web or module operations.
- ADM-2 Secret admin route and slash descendants are served behind Caddy HTTPS;
  secret path supplements authentication. Use Argon2 hashes, expiring Secure/
  HttpOnly/SameSite cookies, CSRF protection and bounded login attempts.
- ADM-3 Local CLI or SSH-forwarded one-time bootstrap sets first credentials.
  No public unauthenticated setup wizard. Rotate hash and invalidate sessions.
- ADM-4 Module cards distinguish core, enabled, disabled, unverified and policy-
  blocked features. Core has no off toggle. Display dependencies, public listeners,
  resource evidence and last health result before applying optional changes.
- ADM-5 Toggle changes edit one draft; Apply submits a redacted immutable plan.
  Show desired/pending/actual state and progress; errors explain retained state
  and recovery action. Revision conflicts preserve manual edits rather than
  overwriting them. No restart for every UI click.
- ADM-6 Web can enable/disable/configure qualified own modules within locally
  approved envelopes through the bounded controller. Foreign mutation, public
  443 switching, new host-network privileges and policy exceptions remain local
  consent operations; display needs-local-approval rather than a fake switch.
- ADM-7 Device wizard selects target/client/protocol/transport and outputs private
  links, QR or config plus installation instructions. Generate, rotate and revoke
  scoped device keys. Unsupported combinations are explicit, not claimed working.
- ADM-8 No Docker socket, arbitrary file writes, shell or general Docker API.
  Shared config.plan/apply and scoped key/device APIs own all mutations. Redact
  secret fields; display credentials only in explicit generate/share flows.
- ADM-9 Audit all mutations with actor, request/plan/transaction IDs and result.
  Redacted bounded logs and status are available without leaking credential data.
- ADM-10 Optional Telegram long polling uses configured numeric sender allowlist;
  unauthorized senders receive no responses. It supports existing status/share/
  revoke/restart/logs/doctor actions within the same authority; no stop-point
  bypass. Missing token means no Telegram task or polling.
- ADM-11 Baseline checks belong to core; advanced doctor watch is optional with
  one scheduling owner. Backup/restore and expanded views can follow incrementally.

## Failure handling and acceptance

Controller unavailable -> read-only cached/observed UI, mutations disabled.
Invalid YAML -> keep last-known-good runtime; show validation and repair draft.
Failed apply -> compensation/recovery status with working management path, or
explicit local CLI recovery when web itself is down. Test auth/CSRF/session expiry,
parallel edit conflicts, batch switches, core-disable rejection, scoped credential
rotation/revocation and all blocked actions. No secret in routine config/log views.

The later optional UI includes login/bootstrap, feature/config plan/apply, status
and key/share operations. It does not wait for the full compatibility catalogue,
Telegram daemon or final analytics dashboard. Measure actual core resource cost.

## Deferred personal accounts and publication

The planned minimal UI is later optional owner administration. Per-user login/LK,
admin user overview and login -> protocols -> named tokens are post-release
(sdd-identity.md). Protocols default from configuration, initially Amnezia preset;
secret/config/QR is shown during creation only, never later even to admin. A lost
secret requires revoke/delete and new creation. Date-based names remain editable.
Local-site publication and scoped agent onboarding/dashboards are a separate
post-release optional module (sdd-web-publish.md), not a release requirement.

## Revised phase and configuration interface

No web UI ships in R1. The later UI is a separate Docker module and always calls
the optional full management API. Editing uses atomic parameter batches through
the standalone configurator, preserving independent edits and drafts on conflict;
file revision changes alone are not semantic conflicts. UI and API/configurator
startup cannot depend on valid target YAML. Backend unavailability is confined to
admin routes and cannot break ordinary site/VPN or ingress cold start/reload.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

## Current M1 boundary

Full administration remains TODO/deferred under OR-45. The optional passwordless
client-key page is a read-only static delivery feature in the existing Caddy,
owned by sdd-client-access-page; it does not reuse an admin route/session or
bootstrap unauthenticated management. Do not implement ADM/web/API as its dependency.
