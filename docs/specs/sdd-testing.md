# SDD testing — final bundle, failure containment and real clients

> Authority: [owner requirements](../OWNER-REQUIREMENTS.md). R1 is standalone CLI/Core with manual YAML validation and safe apply; API/configurator/web implementation is deferred. Native checks are required, Docker staging optional, IPv6 far future. See [final audit](../FINAL-REVIEW.md) for unresolved design details.

Area owner: sdd-runtime-safety.md. This is a plan, not completed tests.

- TEST-1 Every candidate's final native configs and Compose are independently
  checked with compatible pinned native validators. Optional isolated Docker staging exercises affected routes with real test clients; record candidate identity and deterministic
  test-only port/network mapping. No production/foreign mutations during staging.
- TEST-2 Syntax/listener checks alone cannot prove Naive, REALITY, MTProto, UDP or
  XHTTP. Selected module acceptance uses its actual protocol and credential checks.
- TEST-3 Public staging uses the consented non-443 endpoint and trusted domain/SNI;
  optional foreign paths remain synthetic until their separate remap approval.
  External ISP/DPI/device evidence is separate from isolated Docker results.
- TEST-4 Public 443 takeover requires explicit original-holder/remap/cutover consent.
  Port publishing can require recreation. Failures release the new port before
  restoring prior holder and report interruption honestly.
- TEST-5 Kill management web/controller/compiler, corrupt desired YAML and its
  database, and reboot. Accepted VPN/native credential routing must keep working
  or cold-start from verified bundle without those components.
- TEST-6 Inject crash at every intent/effect/record/promotion/lease transition.
  Independent guardian must restore a compatible accepted state or preserve
  unaffected routes and mark recovery-required. Never prune recovery evidence early.
- TEST-7 Stale attestation/image/secret hashes, changed port holder and revoked-key
  policy fences reject admission. Rollback cannot restore revoked credentials,
  downgrade strict-family exits or blindly use expired certs.
- TEST-8 Separate empty/minimal core and each selected optional module. Turning
  features off removes owned effects, preserves shared engines still needed and
  never toggles mandatory core or foreign resources by accident.
- TEST-9 ACME staging covers issue/renew/delivery/served checks and long-lived stream
  behavior. No forced production issuance as routine testing; invalid certs preserve
  current usable material. Down suspends automatic renewal; explicit manual preparation never starts the stack.
- TEST-10 Encrypted clean-VM restore includes config/secrets, policy fence, accepted
  bundles/images and journal. Recreated Docker IDs are not byte-identical evidence;
  compare stable configuration and real client behavior.
- TEST-11 Later accounts/publication: one-time reveal across lost/retried responses, browser/cache/CLI/TG, ownership and route/agent isolation. IPv6/TUN/strict family are far-future tests; autonomous failover awaits a separate scope decision.
- TEST-12 Generate load externally; measure aggregate memory, CPU/goodput, handshake
  latency/jitter, reload interruption and resource-starved management containment.
  No fake fixed resource estimates or all-protocol on-host stress by default.

Dry-run is not Docker testing. Full tests need isolated resources and a budget.
A failed optional/backend probe does not justify endless global restarts or turning
on a different protocol. Document unknown/unverified results rather than green UI.

## Revised phase-specific acceptance

R1 additionally tests interrupted add/resume, installed versus enabled membership,
remove versus purge, image update versus up, down with invalid YAML, partial
start/stop, reboot after down and down racing guardian rollback. Logs redact secrets
at every level and rotate under size/disk pressure without stopping working VPN.
Root/module/Docker snapshot inputs and retained customization are covered.

API atomic parameter merges, configurator migrations and web tests are deferred
with those implementations; cross-editor race guarantees need their own proof.
IPv6 and strict family tests are far future, not R1/R2 blockers. Config restore
checks versions/secrets/availability and does not start services. Candidate Docker
staging is optional; real-client evidence for a shipping module is required.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

D81, 2026-11-01: compilation is now deferred too. Before the selected implementation
plans are fully complete, perform source syntax/document consistency only; no
cargo check/build/release compilation. After completion, build → U0 → F0. Earlier
intermediate compile evidence remains historical, not evidence for new source.

## Explicit bundle validation runner binding (2026-11-02)

The R1 `bundle.validate` host coordinator owns protected exact candidate loading,
artifact/image verification and attestation publication; the scoped
`infra.validation-docker` driver owns fixed sandbox mechanics. The first fixed
semantic topology covers mandatory HAProxy/Caddy HTTPS site traffic with pinned
ACME-image curl clients, ordinary certificate trust and byte-exact bodies. Native
checks precede semantic scenarios. Unsupported selected optional protocol scenarios
are explicit blockers; native parser success cannot substitute for a client
handshake. This coverage limitation is reported and does not narrow R1 acceptance.

Deterministic operation names and exact labels are durably bound before create,
so effect-before-receipt crashes can be reconciled. Cleanup verifies scoped
identities and removes only sandbox resources; uncertain cleanup is recovery-required.

The concrete runner also supports the final shared Unbound and VLESS/Xray service
composition. Fixed DNS UDP/TCP clients query a sandbox-only `.invalid` local zone;
this proves native listener transport, not public recursion or DNSSEC authority.
The VLESS client uses the exact staged native UUID and WS/XHTTP path, ordinary TLS
trust and site SNI through HAProxy/Caddy to Xray. An HTTP fixture inside the private
sandbox provides deterministic egress bytes. Because production policy correctly
blocks private destinations, the isolated copy grants only that exact fixture IP
and port before its otherwise unchanged private-address block rule. This explicit
test-only exception is part of the scenario digest, never a production artifact.
The generated client uses fixed SOCKS inbound and pinned Xray, while a pinned
ACME-image curl performs the request. Curl is used for trusted certificate checks;
BusyBox wget cannot establish that evidence. Images and generated client/config
mapping digests are bound into the protected attestation. Native parser tests still
precede all server/client execution. External recursion and actual devices remain
unverified.

## M1 final acceptance scope and order

[The current plan](../plans/INSTALLATION-IMPLEMENTATION.md) selects M1-A–E,
then B1 external build, U1 unit, F1 functional and subsequently scoped A1. No
product tests execute during document preparation. Public non-443 staging in old
TEST-3 is not the M1 browser-port profile: isolated fixtures may use private mapped
ports, while explicit actual public acceptance uses approved TCP/UDP443 only.
Cover managed/associated app behavior, retained remove/purge, exact 403/308/421
host matrix, DNS evidence and client-page publication/no-prompt install: absent
path generates, explicit empty opts out, rotation invalidates the old URL, and
`.secrets/share` receives only active guest files plus the bearer URL. A generated
page is a timestamped snapshot; freshness failure is not an excuse to claim current
native client coverage. Do not conflate local fixture success with ISP/DPI evidence.

The Echo blocking test (OR-67) is a manual Tartarus scenario, separate
from production acceptance. It proves, in order: the local `abyss-hton` accepts
the test SNI; `core.edge.testTunnel` renders the loopback HAProxy backend; the
taenarum reverse forward binds only `127.0.0.1:<port>` on the server; and the
`abyss-echo` client completes TLS/REALITY to the real server address and
returns through the local HTON endpoint. A failed blocking observation is a
remote-network result, not proof that the reverse route is broken; the route
must be verified with the tunnel active before drawing ISP/DPI conclusions.
