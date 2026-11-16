# SDD client IPv6 — IPv6 destinations over an IPv4-only access network

Status: far future under current owner clarification. No R1/R2 implementation or acceptance dependency; earlier phase labels below are historical planning.


Status: planned qualification, not implemented. Owner requests: IPv6 where the
client uplink has only IPv4; IPv4/IPv6 egress selector on credential creation.
The latter is far future together with IPv6, separate from initial accounts.

## Distinct capabilities

| Level | Promise | Requirements and limits |
|---|---|---|
| proxy-egress | Supported TCP/UDP flows reach IPv6 destinations | IPv4 connection to hton; working IPv6 from the actual server engine; compatible client/protocol. No public client address or raw ICMP claim |
| device-tun | Device applications can use IPv6 sockets through a virtual interface | Qualified client TUN with IPv6 capture/DNS; protocol still may proxy flows rather than carry arbitrary IP packets |
| lan-routed | IPv6 on a selected home LAN/SSID, including tested L3 traffic | Controlled gateway, real packet tunnel, routing/firewall/RA and prefix/egress policy. Separate from tun-to-proxy behavior |

```text
IPv4-only ISP / NAT / CGNAT
  -> authenticated tunnel to VPS IPv4 address
  -> qualified proxy engine or L3 gateway with actual IPv6 egress
  -> IPv6 destination
```

No direct client protocol-41/6in4 dependency. Use existing allowed encrypted
transport for proxy-egress first. Native IPv6 at the VPS is preferred; the optional
server HE tunnel is upstream provisioning, not the client feature itself. Host
IPv6 does not prove Docker engine IPv6: test inside the selected service/network.
If IPv6 egress is absent, report unavailable; do not advertise success or fake AAAA.

## Qualification sequence

1. Make an IPv4 connection to a dual-stack VPS, query real AAAA and fetch a
   controlled IPv6-only HTTPS endpoint through existing VLESS. Test UDP/XUDP too.
2. Configure an IPv6-capable client TUN; verify application IPv6 literals, AAAA-only
   names, DNS, MTU and route capture. Label unsupported ICMP explicitly.
3. Test a real packet tunnel in an isolated Linux/OpenWrt lab. Existing SSH -w
   may be a session-only proof of concept; OpenVPN TUN is another already-planned
   compatibility candidate. Do not claim SOCKS, VLESS or SSH -D alone is raw L3.
4. For LAN mode, allocate a collision-free local /64 per LAN and configure RA/
   SLAAC/DNS on the controlled gateway only. Routed GUA requires a genuinely
   delegated/routed prefix. An on-link VPS /64 is not automatically delegable.
   If only a server address is available, a qualified ULA + stateful NAT66 egress
   option is outbound-only, explicitly labeled, and does not grant public addresses.
5. Require forwarding/firewall permissions only for selected L3 mode. Preserve
   native routes and necessary ICMPv6/PMTU; choose MTU from encapsulation overhead
   (IPv6 link minimum 1280), not a copied Ethernet value. NAT state is bounded.

LAN mode may need host/controller changes and therefore concrete local approval.
No unsolicited inbound client reachability; public hosting is a separate publication
module. ND/RA remain local to each LAN; do not bridge untrusted users into one L2.
On failure withdraw unusable default routes promptly, preserve native IPv4/LAN,
and do not leave a half-dead IPv6 default blackholing dual-stack applications.

## Credential egress policy (far future)

Add egressFamily: ipv4 | ipv6 to each immutable credential's policy. The creation
wizard labels it "Internet exit: IPv4 / IPv6". Future config
accounts.defaults.egressFamily seeds new creation, default ipv4; protocol defaults
remain the configured Amnezia preset. Users may choose only granted, supported
families; missing verified IPv6 or adapter enforcement blocks creation.

The choice governs payload destination/source family on the VPS, not which family
the client uses to connect to the tunnel, and not automatic IP assignment to the
client. IPv4 clients can use ipv6 egress. It is a family choice, not selection of a
country, dedicated address or guaranteed stable public address.

Strict semantics: ipv4 uses IPv4 destinations/egress; ipv6 uses IPv6 destinations/
egress. Resolve names for the selected family and reject incompatible literals or
AAAA/A absence. No silent cross-family fallback. IPv4-only destinations do not
magically work under strict IPv6; NAT64/464XLAT solve different problems and are
not this feature. Infrastructure control/DNS traffic has a separate scoped policy.

Bind rules to authenticated credential IDs before dialing. DNS preference alone
is insufficient: destination literals, UDP and fallback paths must be enforced.
Prefer shared family-specific outbounds and per-identity routing when the backend
supports it; isolate only if necessary and within resource budget. Never globally
change all users' outbound for one token. Foreign Amnezia enforcement is blocked
until a qualified adapter and required consent exist. Unsupported protocol versions
show unsupported, not a decorative selector. Rename does not change policy;
changing family replaces the credential through revoke/create, with one-time reveal.

Smart failover can change transport only among candidates preserving this policy.
If IPv6 fails, selected IPv6 flows fail visibly; it cannot silently choose an IPv4
exit/direct route. Explicit other traffic policies are separately labeled.

## Acceptance

- IPv4-only/CGNAT lab reaches controlled IPv6-only TCP/UDP destinations; no fake
  success inferred solely from a dual-stack website or IPv4 fallback.
- Simultaneous IPv4 and IPv6 credentials keep independent egress. Test domain and
  numeric destinations, wrong-family literals, UDP, outage and revoke/restore.
- device-tun and lan-routed are separately certified. Test IPv6 literals, AAAA-only,
  PMTU/large transfers and ICMPv6 for the real L3 claim, plus LAN discovery/PCVR.
- ULA is never described as globally routable. GUA mode verifies real delegation
  and return route. No open public forwarding or accidental cross-user access.
- Restart/rollback/disable remove only owned routes, advertisements and NAT state.
- Measure idle memory and CPU/goodput for proxy and L3 modes before production.

Primary references: [sing-box TUN](https://sing-box.sagernet.org/configuration/inbound/tun/),
[Xray routing](https://xtls.github.io/en/config/routing.html),
[Xray outbound](https://xtls.github.io/en/config/outbounds/freedom.html),
[OpenVPN TUN](https://openvpn.net/community-docs/community-articles/openvpn-2-6-manual/),
[SSH tunnel](https://man.openbsd.org/ssh.1),
[ULA](https://www.rfc-editor.org/rfc/rfc4193.txt),
[ND/RA](https://www.rfc-editor.org/rfc/rfc4861.txt),
[IPv6](https://www.rfc-editor.org/rfc/rfc8200.txt),
[NAT64 direction](https://www.rfc-editor.org/rfc/rfc6146.txt).

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).
