# SDD smart-connect — device onboarding, local diagnosis and controlled failover

Status: planned, opt-in qualification after the release baseline. User requested
planning the whole device-first connectivity proposal; no field implementation yet.

## User-facing contract

Choose TV, Quest, phone or gateway and a use case, not an unfamiliar protocol.
Suggest a tested client or an existing local gateway. Generate configuration and
run checks from the user's network. On a qualified gateway, offer per-device or
separate-SSID routing; do not promise installing software on arbitrary stock routers.
The gateway client, not the VPS, handles unsupported TV/Quest VPN interfaces.

Use existing sing-box/Mihomo/OpenWrt package capabilities through qualified adapters;
do not implement another tunnel engine, rule language or DNS resolver. Reuse client
URL tests/routing where qualified. hton owns the policy, compatibility metadata,
setup UX and explainable check results. See ../REUSE-ASSESSMENT.md for candidates.

## Agent and failover

An optional local agent runs bounded DNS/TCP/TLS/HTTP checks; UDP/voice use a real
UDP test. Probe only configured/approved endpoints with bounded frequency, bytes,
timeouts and concurrency, never scan or record browsing payload. Compare direct
and allowed tunnel paths from home, not only service health inside the VPS.
Show timestamps, vantage point and unavailable evidence. Avoid busy polling and
unbounded logs; disable agent timers when the feature is off.

Candidates are explicit allowed credential/profile references. Rank by health and
use-case requirements, then measured latency/loss/jitter/goodput where available.
Use cooldown, hysteresis, backoff and a manual pin. Move new flows; do not promise
live migration of arbitrary existing TCP sessions. UDP mappings and changed public
addresses can also interrupt game sessions; surface this honestly.

Never enable an unselected protocol, issue a new secret, change a port or bypass
IPv4/IPv6 credential egress just because a probe failed. Direct fallback follows
explicit policy for that traffic and is visibly identified. A strict-family token
cannot be downgraded by transport fallback. No auto-revive of session-only SSH;
managed client reconnection applies only to explicitly selected non-SSH lifecycles.

Preserve LAN discovery/casting/local PCVR. DNS/IPv6 routes must match the policy;
a changed transport does not justify stale RA/default routes or leaks. Endpoint
bootstrap traffic and management access must stay outside routing loops.

## "Why doesn't it work?"

Return structured failed checks with plain-language explanation and a bounded
proposed repair plan: DNS problem, certificate/time mismatch, missing IPv6 egress,
UDP unavailable, unhealthy backend, route leak, endpoint loop or unknown cause.
Do not assert censorship from one timeout. Repairs use shared apply and approved
own-resource scope; privileged/foreign changes remain local-consent operations.
An unknown diagnosis is better than an automatic destructive guess.

## Acceptance

One qualified gateway connects a TV and Quest without device apps. Test controlled
primary-path failure, new-flow fallback, no flapping, local PCVR preservation,
strict credential family, IPv4-only client IPv6 egress, DNS/MTU and agent removal.
User sees which path is active and why, including direct fallback or uncertainty.
Use synthetic controlled tests, not hard-coded third-party service probes.

Single-disclosure credential provisioning and account portals remain R2; agent
operation uses already-provisioned protected keys and cannot re-reveal them.

## Owner scope refinement

Start with a simple locally run PowerShell/Bash tool probing already provisioned
profiles from the user's network with bounded cost and measured caveats. Device
installation stays incremental/optional. Owning external-client routes, up/down
or autonomous path switching is outside this server repository pending a separate
decision. Earlier agent/failover text is future exploration, not approved server
implementation scope. IPv6-dependent acceptance is far future.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

## OR-41 DNS and masking evidence boundary

Follow [security review](../SECURITY-MASKING-REVIEW.md) and the DNS owner in
sdd-dns-dot. Server-side remote resolution does not establish device-wide DNS
privacy. Declare client mode, DNS routing, bootstrap/direct exceptions, tunnel-loss
behavior and OS/app/version coverage. Unsupported enforcement reports unverified.
Full-device/TUN and application-only SOCKS/CONNECT/MTProto have different scopes;
no hidden plaintext fallback. Preserve LAN exceptions and OR-5's external-client
ownership. Final checks follow complete implementation, unit then functional order.
