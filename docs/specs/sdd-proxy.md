# SDD proxy — HAProxy edge

> **Status: frozen pre-split specification (OR-68, 2026-11-12).** Target
> ownership of this edge moves to the independent Triodos project (HAProxy,
> Caddy, certificates, site, public ports, pools and health checks). This SDD
> remains the source contract for the current M1 path and for the extraction; it
> does not describe the target Triodos architecture. See the Triodos project's
> `CONTRACT.md` and `MIGRATION.md`. No runtime change is authorized here.

## Purpose

Single public TLS entry with TCP classification, own-domain TLS passthrough to Caddy,
foreign passthrough and optional DoT. Architecture section 4 owns the ordered
ClientHello table, including absent SNI/ALPN and timeout behavior.

## Requirements

- PRX-1 Publish PUBLIC_PORT/tcp and, when enabled, DOT_PORT/tcp on explicit IPv4
  bindings (IPv6 deferred). Backends have no public publishing.
- PRX-2 After higher-priority exact raw assignments, configured exact site names
  go to Caddy's private HTTPS frontend. Covered unassigned owned wildcard names
  use the OR-43 apex redirect policy, not an implicit VPN or normal-site route.
  No ALPN remains valid for own TLS.
- PRX-3 Caddy terminates own HTTPS; after termination: enabled CONNECT -> Naive; VPN_PATH -> Xray;
  ADMIN_PATH plus slash descendants -> admin; else Caddy mask. Reject lookalike
  prefixes. Transport-specific subpaths are explicitly tested. No HTTP ACME route.
- PRX-4 Exact MTProto camouflage SNI routes raw to the selected backend before own wildcard matching. Reject conflicting assignments. ALPN absence is not protocol identity.
- PRX-5 Exact foreign/REALITY SNI routes raw regardless of ALPN; remaining valid TLS goes to Caddy under explicit certificate/host policy. Non-TLS closes. No-SNI/ECH require explicit policies. Unknown third-party names cannot receive matching trusted certs.
- PRX-6 DoT accepts DOT_SUBDOMAIN only, terminates TLS and forwards DNS/TCP to
  unbound:1053. Bound connections/resources; SNI does not authorize DNS users.
- PRX-7 Validate configs first, then SIGUSR2 to the HAProxy master in master-worker
  mode. Pin/test entrypoint and PID model. Docker port changes recreate the edge;
  do not promise uninterrupted switch. All changes use sdd-control.md.
- PRX-8 Templates live in etc/haproxy/; outputs and crt-list in candidate generations.
  No hand edits to accepted output; validate HAProxy and Caddy independently. HAProxy crt-list is needed only for its optional DoT termination.

## Failure modes

Missing first certificate permits NO-CERT degraded bootstrap only. Invalid candidate
keeps previous generation. Backend failure closes its branch without routing secret
traffic into another service. Conflicting published ports fail consented preflight.

## Acceptance

- Domain/SNI HTTPS returns mask and valid expected certificate. IP-only/no-SNI
  HTTPS is not a supported mask probe.
- Matrix covers own/foreign/absent SNI, ALPN presence, malformed/fragmented and
  timed-out ClientHello, disabled services and camouflage collisions.
- Real Naive, MTProto and foreign clients pass independently and multiplexed.
  openssl without ALPN alone does not prove MTProto interoperability.
- Config/certificate reload preserves an established stream; port switch tests
  measure interruption and prove rollback. Validate IPv4 in R1; both families only with future IPv6 support.

Reference: https://docs.haproxy.org/3.2/management.html

Accepted routing must boot/operate without admin/controller/compiler. Admission, guardian, Docker final-bundle tests and current-policy rollback follow sdd-runtime-safety.md.

## Current editable inputs and release scope

PRX-8 means accepted generated outputs cannot be edited in place; it does not
prohibit owner-required editing of config/docker inputs. Hash and validate those
inputs, then create immutable accepted copies. R1 listener/acceptance matrix is
IPv4; dual-stack checks apply only to future IPv6. Optional admin's absent DNS,
stopped backend or removal must not prevent edge startup/reload or ordinary-site
and VPN service. Qualify that behavior with admin never installed.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

## OR-38–OR-40 masking contract

All names and exact assignments come from Core's resolved configuration inventory;
no literal SNI in templates or probes. Separate protocol routes precede wildcard
site matching. Ordinary HTTPS on each known protocol SNI must serve the real site;
failed native authentication remains backend-specific and requires evidence.
This differs from backend outage: never forward authenticated application bytes
to a different protocol as failover. New managed backends are private from creation.
Follow [hardening task](../plans/INSTALLATION-IMPLEMENTATION.md) for final probe matrix,
TLS/ALPN observations, explicit unknown/no-SNI policy and exposure checks.
Wildcard certificates/DNS do not hide ordinary ClientHello SNI or shared IP.

## Latest routing and port policy — OR-42–OR-44

This section overrides older wildcard-to-site and unknown-SNI wording above.
Owned ingress uses TCP/443; selected QUIC uses UDP/443, never public 8443.
TCP and UDP reservations are independent. Exactly one UDP/443 owner per address;
reject competing Hysteria/Caddy HTTP3/TUIC claims. TCP SNI rules do not govern UDP.
Unassociated existing 8443 services require separate approval, not automatic remap.

Route order: configured exact native/REALITY/MTProto assignments; configured exact
site/WSS/management hosts; covered unassigned one-label own-wildcard names to the
existing Caddy redirect policy; reject other unknown names, no SNI and uncovered
nested names. Wildcard certificate presence alone grants no native VPN route.
No unknown branch reaches Google's/native fallback. Valid explicit legacy names
remain subject to OR-36. Bounded fragmented ClientHello inspection remains required.

One existing Caddy serves the website `/var/www/html` and host-specific policies:
configured REALITY host ordinary TLS gets 403 `Access denied`; unassigned covered
wildcard hosts get 308 to the configured apex root; apex serves the ordinary site.
REALITY forwards unauthenticated TLS to this same Caddy private endpoint; HAProxy
does not intercept HTTP errors. No extra Caddy/relay/container. A Docker bridge
fallback may use the existing private site binding with explicit access scope;
loopback in the Amnezia namespace is not the host's loopback. Bind the intended
certificate/SNI policies for every internal fallback. Reject HTTP Host/SNI mismatch
with generic 421 rather than exposing another host route. No public Caddy admin.
Full policy and final evidence: [security review](../SECURITY-MASKING-REVIEW.md).

## Optional client-page route — OR-45

On the canonical base site host only, an enabled exact core.clientAccessPage.path
routes to the dedicated generated client-only HTML artifact before the site file
server. No route on REALITY-denied, other protocol or wildcard redirect hosts;
Host/SNI mismatch cannot expose it. Apply the page SDD's methods/privacy headers
and no access logging; no public /share directory or second Caddy. Configuration
collision with an existing WSS/admin/site publication route is a hard error.
The same generation binds route, artifact digest and source credential revisions;
removal cannot roll back into an old revoked-credential page.
