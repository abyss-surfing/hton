# SDD gateway — devices without VPN apps

Owner D26/D28; protocol/provider constraints remain. Qualify existing OpenWrt or
Linux client packages through an adapter, not a new proxy engine.

- RTR-1 Baseline router recipe reuses existing VLESS WS/XHTTP profile, optional DoT
  and a qualified client. No new server listener merely for TV/Quest onboarding.
- RTR-2 Device OS, router model/firmware/package and transport version are evidence,
  not inferred from a product brand. Passwall2/Xray, Nikki/Mihomo, HomeProxy/sing-box
  and OpenClash are distinct adapters/serializers, never interchangeable JSON.
- RTR-3 Selected device/SSID/global/split route policies are explicit. An SSID alone
  does not proxy traffic; configure actual forwarding/DNS/IPv6 and endpoint exclusions.
  Unknown stock firmware can use a separate gateway, not arbitrary package install.
- RTR-4 Preserve PCVR/casting/discovery/NAS LAN routes. Avoid isolation between Quest
  and local PC when these are required. Scope DNS capture and distinguish explicit
  plain-DNS fallback from privacy-preserving tunnel DNS. Test UDP, not just web.
- RTR-5 share target/router options are declared only in ../CLI.md. Exports use
  accepted config and an actually qualified client/version/transport profile.
- RTR-6 Optional Hysteria is a separately measured UDP profile; no unconditional
  claim that QUIC always improves speed or latency.
- RTR-7 SSH D1 (-D) is session SOCKS, not arbitrary UDP. D2 (-w) is a real packet
  tunnel with selected host privileges. config/modules/ssh-session.yaml: settings.mode selects
  socks or tun. No autossh/watchdog revive; ServerAlive* and cleanup are client-side.
- RTR-8 Route metrics alone do not detect a stuck tunnel. Remove/withdraw stale
  routes after bounded detection and preserve selected direct fallback visibly.
  Far-future strict-family flow policy cannot be silently downgraded by this fallback.
- RTR-9 Client IPv6-over-IPv4 is a separate qualified capability: proxy-egress,
  device TUN and real LAN L3 have distinct addressing/RA/ICMP promises. Native
  provider IPv6 at home is not required; gateway/backend support is required.
- RTR-10 Smart-connect may manage allowed non-SSH profiles only; preserve the
  owner's SSH-session lifecycle. See sdd-smart-connect.md and sdd-client-ipv6.md.

Acceptance: actual TV/Quest through a qualified gateway, native LAN unchanged,
boot/sleep/connection interruption, DNS/IPv6/MTU/UDP, strict exit-family and cleanup
on disable/rollback. Hardware purchase/model choice waits for compatibility evidence;
no guaranteed router price or stock-firmware support is a project requirement.

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
