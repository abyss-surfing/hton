# SDD dns-dot — private recursion and optional public DoT

## Purpose

Independent Unbound recursion for VLESS, plus opt-in public DNS-over-TLS.
features.dot.enabled enables public DoT; private Unbound remains when VLESS requires it.

## Requirements

- DOT-1 Unbound binds its container network interface on 1053 TCP/UDP, without
  host publishing. HAProxy uses TCP; Xray uses explicitly configured DNS transport.
  No dependency on foreign Amnezia DNS.
- DOT-2 Enabled HAProxy DOT_PORT terminates TLS for DOT_SUBDOMAIN and forwards
  DNS/TCP to unbound:1053. Include that name in certificate planning.
- DOT-3 Enable DNSSEC and QNAME minimization; define bounded cache, request,
  concurrency and rate limits using the pinned version's supported options.
- DOT-4 Reject invalid/foreign SNI. Correct SNI is public information, not client
  authentication; the resolver is public when enabled. Apply explicit allow/rate
  policy and test resource exhaustion without claiming SNI prevents abuse.
- DOT-5 diag checks A/AAAA and cert coverage and distinguishes local listeners
  from external reachability. No public plaintext TCP/UDP 53 listener.

## Design and failure modes

Templates remain in etc/unbound/; candidate configs use native validation and
shared apply. Certificate reload belongs to HAProxy. Do not assume Unbound HUP
preserves cache; verify supported reload/cache semantics for the pinned version.
Unbound failure makes dependent VPN DNS unhealthy. An ISP blocking 853 affects
DoT clients; their fallback is explicit router/client policy, not a server action.

## Acceptance

- Real DoT query with CA and hostname verification resolves valid data and
  rejects invalid DNSSEC data; wrong SNI fails.
- Test both families and public absence of plain 53; prove Xray's private DNS
  continues when public DoT is disabled.
- Validate cache/reload behavior and limits; reject invalid candidate configs.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

## DNS privacy review and planned correction — OR-41

Current Unbound implementation uses iterator recursion; DNSSEC/QNAME minimization
and a private inbound listener do not encrypt upstream queries. Current Rust Xray
uses that private resolver, but minimum Amnezia/Outline/Hysteria do not share an
enforced resolver policy. See [source review](../SECURITY-MASKING-REVIEW.md).

Engineering privacy-profile design: one shared DNS policy owner under root
`core.dns` (planned typed extension) declares `upstreamMode: tls-forward` versus
explicit `recursive`, named upstream objects with address, port, TLS authentication
name, trust and bootstrap evidence, plus `fallback: fail-closed`. Unbound owns
serialization and upstream transport; adapters consume its private endpoint rather
than clone resolver defaults. Use authenticated DoT forwarding for the initial
privacy implementation, no plaintext fallback on timeout/cert failure. An upstream
outage blocks affected name resolution, not unrelated established VPN traffic.
Retain DNSSEC validation/cache limits; the upstream resolver still observes queries.
No guessed resolver vendor defaults or DNS-over-HTTPS code added to every adapter.
Bootstrap IP and TLS verification name are distinct typed values; plan trust/time
prerequisites without circular DNS lookup. Native format compatibility remains a
required implementation prerequisite.

Per-protocol native DNS owner must be explicit, including Docker/OS lookup paths.
Do not redirect all host DNS or alter unassociated AmneziaDNS under this plan.
Client DNS coverage is separate: full-device/TUN capture and fail-closed routing,
application remote resolution for SOCKS/CONNECT, and Telegram-only scope for
MTProto. Client/browser DoH, endpoint bootstrap, split routes and IPv6 exceptions
must be reported. Public DoT remains optional; its absence does not remove private
VPN DNS. Client-protection evidence is per OS/app/version, never inferred from a
server DNS field or one successful lookup.

Final unit then functional scenarios include configured encrypted upstream,
invalid certificate, timeout with no plaintext fallback, TCP/UDP DNS, actual client
physical-interface DNS capture, tunnel loss/reconnect, split/local zones and native
IPv6. Explicit bounded captures use synthetic query names and protected evidence;
no additional destination/query logging or background public diagnostics.

### M1 binding and example

The Onomat DNS runtime owns the Unbound policy/render implementation, including
the Rust Unbound adapter, under `hton/services/onomat/runtime/dns/`; HTON consumes
the renderer and applies the private Unbound service. There is no second DNS
controller in HTON. The exact root core.dns fields and initial upstream example
are frozen in config and examples/M1-CONFIGURATION.md. Per-backend native lookup
must use that private endpoint: when the container cannot reach host loopback,
bind only to the qualified bridge-private interface/ACL rather than publishing
DNS. Server bootstrap addresses and TLS name are separate; never bypass upstream
certificate verification.
