# Deferred SDD web-publish — local-site reverse tunnels and HTTPS routes

## Purpose and phase

Owner request, 2026-10-31: publish a local website through the project's final
public domain, using reverse proxy or port forwarding. Plan as a separate optional
POST-RELEASE module alongside the deferred accounts/credential work. It is not a
release blocker or authorization to expose a local service now.

```text
Browser -> https://app.example.com -> HAProxy -> Caddy HTTPS route
        -> private tunnel-server binding -> encrypted outbound agent tunnel
        -> owner's selected local HTTP service, e.g. 127.0.0.1:3000
```

For a backend already reachable on the VPS/private managed network, ordinary
Caddy reverse_proxy is sufficient. For a home/dev machine behind NAT, require an
outbound authenticated agent connection; a public hostname cannot make the VPS
reach someone else's localhost. No inbound home-router port forwarding required.

## Publication model

A publication has immutable ID, ownerId, editable name, assigned hostname/path,
backend kind (private-upstream or agent-tunnel), local target/service ID, access
mode, credential binding, desired/actual state and resource policy. Server route
config is declarative with opaque secret references, never raw reusable secrets.

- Default hostname is an owner-authorized subdomain under project DNS. Use the
  existing DNS-01 inventory/wildcard when valid; DNS still needs its own A/AAAA
  record or explicit wildcard DNS. Wildcard certificates do not create DNS records.
- Permit publication on the final project root hostname as an explicit route
  replacement for the mask, retaining higher-priority reserved core/admin routes.
  Do not globally replace the core router or accidentally expose local admin paths.
- Optional path mount, such as /apps/demo/, requires backend base-URL compatibility.
  Test redirects, cookies, absolute asset paths, Origin/Host and WebSockets; a
  prefix strip alone cannot repair every application. Prefer a subdomain otherwise.
- Reserved admin/API/bootstrap/health paths and raw MTProto/REALITY SNI reservations
  cannot be claimed by an ordinary user. Validate names and path ownership through
  the same catalogue/plan conflict checker. No arbitrary external domain takeover.
- Support explicit private/authenticated or public site access. Token possession
  authenticates the agent, not the site's visitors. Anonymous publishing requires
  an explicit owner choice and authorized publication scope.

## Tunnel and endpoint boundaries

A scoped agent may register only assigned publications and approved local targets;
it cannot request arbitrary remote ports, claim another user's host/path, change
Caddy config or receive the Docker socket. Private reverse listeners bind only
loopback/project interfaces, reached by the approved relay/network topology.
Control and management interfaces are not publicly published.

Browser HTTPS terminates once in Caddy, with existing Let's Encrypt automatic
renewal. Agent transport is independently encrypted and authenticated; this is
necessary security for the second network leg, not needless HTTPS re-encryption.
A configured local HTTPS upstream verifies its chain/name rather than disabling
verification. Preserve streaming, WS and bounded upload/timeouts.

Qualify a pinned frp implementation versus restricted SSH reverse forwarding.
Do not commit to a global shared frp token as a multi-user authorization system:
its documented token mode shares a token between client/server, so ownership,
allowed publications and revocation need an actual tested isolation/auth adapter.
If it cannot supply this within resource budget, isolate appropriately or choose
a different implementation. Disable exec/plugin hooks and unused proxy types.

SSH fallback uses a dedicated restricted identity, no shell/general forwarding,
loopback-only reverse listen and explicit allowed targets/bindings. It is session-
scoped without autossh by default, preserving the owner's session-only decision;
a distinct reconnecting web agent needs its own explicit selected lifecycle, not
reuse of the router SSH recipe with changed semantics.

Shared public TCP/443 for agent control is an implementation qualification, not an
assumption: verify a routable TLS ClientHello/SNI or an actual supported WS/HTTP
transport. A protocol that only uses TLS internally is not automatically HTTP-path
routable. Otherwise plan an explicitly approved dedicated endpoint and show that
resource/port cost; no unplanned listener or conflict with Hysteria UDP/443.

## Accounts and credentials (phase 2)

Each user sees their own publications; admin sees all publication metadata/state.
Enforce server-side ownership for create/update/delete/connect, not only UI filters.
User grants limit publishable hosts, routes, private targets and allowed endpoints;
new host privileges/foreign changes still need local approval.

Generate a named scoped agent credential through the unified creation flow of
sdd-identity.md. Show token/full agent config once at creation. After close it
cannot be downloaded again; delete/revoke and create a replacement. Existing
sessions and new connections lose authority on verified revocation. The admin
cannot redisclose agent secrets. Never use a shared privileged tunnel-server key
as a user's publication token.

## Failure handling and resource acceptance

Disconnected agent -> publication offline, neutral bounded unavailable response
or the explicitly selected mask fallback. Do not route to another user's site.
No core admin exposure, stale reuse of a revoked listener or silently direct
connection to arbitrary internal resources. Unregister and release owned routes
on disable/delete through journaled apply; retain last-known-good core routes.

Limit per-user connections, publications, bandwidth and idle time. No unbounded
buffers/full-body caching. Idle optional module has no resident server/agent until
selected; reuse Caddy and existing ACME. Measure VPS+agent memory, CPU/goodput,
stream latency and total TLS load before declaring support on a weak VPS.

## Deferred work and acceptance

- Declarative publication model and owner grants, hostname/root/path route planning.
- Private-upstream adapter, outbound agent module, pinned client artifact and setup.
- Existing LE SAN/wildcard/DNS integration and reload/stream verification.
- Scoped one-time agent provisioning and per-user/admin publication dashboards.
- Test NAT/CGNAT, reconnect and agent death, long WS/upload streams, access modes,
  name collisions, redirect/cookie/path behavior and IPv4/IPv6.
- Cross-user host/route/credential claims, private-network target abuse and arbitrary
  remote binding attempts fail. No shell/management access is granted to an agent.
- Revocation, uninstall, failed apply and clean restore preserve core management
  and ownership without redisclosing credentials.

Primary references for implementation qualification:
[frp authentication](https://gofrp.org/en/docs/features/common/authentication/),
[OpenSSH restrictions](https://man.openbsd.org/sshd.8),
[Caddy reverse proxy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy).

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).
