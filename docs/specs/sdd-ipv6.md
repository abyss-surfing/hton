# SDD IPv6 — server-side egress and listener provisioning

Status: far future under current owner clarification. No R1/R2 implementation or acceptance dependency; earlier phase labels below are historical planning.


Status: target design; native/provider/HE state has not been tested on the VPS.
Client IPv6-over-IPv4 and per-token family policy are separate in sdd-client-ipv6.md.

## Requirements

- IP6-1 Prefer native provider IPv6. Validate a real usable address, route, DNS,
  MTU and external return path; do not infer success from an interface address.
- IP6-2 Optional HE uses hton.yaml features.ipv6-he.settings and secretRef credentials.
  Keep server/client tunnel link addresses distinct from any routed prefix. Require
  account/tunnel parameters; select an appropriate PoP during actual provisioning.
- IP6-3 Protocol 41 and endpoint reachability are HE prerequisites. A private/CGNAT
  IPv4 or blocked protocol 41 may prevent it; report unavailable. Never prescribe
  direct HE/6in4 on each NATed client as the IPv6-over-IPv4 access solution.
- IP6-4 Journal host interface/routes/units before apply; preserve native defaults.
  Derive/test MTU, do not globally disable IPv4. Tunnel endpoint refresh uses the
  configured provider API with a scoped credential and bounded scheduler.
- IP6-5 Provision ingress and egress separately. HAProxy publishes IPv6 only under
  selected listener policy. Docker containers need a qualified IPv6 network or
  scoped egress adapter; host IPv6 alone is not sufficient for Xray outbound.
- IP6-6 Publish AAAA guidance only after external verification. DNS-01 ownership
  validation is independent of A/AAAA listener readiness. Broker outage may remove
  IPv6 even while the native IPv4 path remains healthy.
- IP6-7 HE rides the same IPv4 underlay; it is not a guaranteed independent bypass
  for IPv4 throttling/blocks. No assurance that a broker will always be reachable.

## Forwarding scope

Proxy-egress generally opens outbound sockets and does not need host L3 forwarding.
Client lan-routed mode may need specific forwarding/firewall rules and real routed
prefix ownership; enable only through its approved plan. Do not mandate globally
forwarding=off and simultaneously promise routed client IPv6. No RA on the VPS's
unrelated networks; client LAN RA is local to its controlled gateway.

## Acceptance

Test native and broker modes, restart, endpoint change, protocol-41 denial, failed
apply and route-preserving rollback. Verify ingress externally and egress from
inside the actual proxy container. Run IPv6-only HTTPS and UDP tests independently
from dual-stack endpoints. Client-family selector stays unavailable until that
engine's IPv6 egress and per-credential enforcement pass qualification.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).
