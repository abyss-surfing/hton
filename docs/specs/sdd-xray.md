# SDD Xray — shared native proxy engine

## Responsibility and configuration

Native Xray provides VLESS and selected optional capabilities without querying
admin/controller/accounts database for authentication or routing. Caddy terminates
VLESS WS/XHTTP TLS behind HAProxy; optional Trojan-TLS is a separate exact-SNI
native certificate consumer.

- Read root features.vless.enabled and config/modules/vless.yaml settings.transport/path. Catalogue derives
  shared Xray and private Unbound dependencies; public DoT disable does not remove
  DNS required by proxies.
- render(normalizedConfig, credentialSnapshot, policyFence) creates protected
  native candidate config with explicit listeners, identities and outbound rules.
- validate(bundle) runs pinned native checks and real clients through final
  Caddy/HAProxy routes in isolated Docker qualification.
- activate(admittedBundle) is a runtime-executor action, never a direct UI write.
  Restart-required identity changes may interrupt sessions; test allowed and
  revoked identities, not merely an open socket.
- export(identity, transport) uses credential delivery policy and transport-specific
  serializers; WS and XHTTP require distinct interoperability evidence.

## Isolation and policy

Private SOCKS is separately disabled by default, authenticated and published on
host loopback only. Its descriptor owns the bind/listener. Shared engines remain
until their last consumer is disabled. Do not restart unrelated engines. Access
logs default off; diagnostics redact credentials. Recursive DNS makes external
queries; do not promise invisible DNS. Apply explicit access/resource policy.

Protected backend snapshots have one writer through the shared plan/admission
path. Release owner exports follow CLI policy; R2 identities follow
sdd-identity.md with no repeat disclosure. Per-token IPv4/IPv6 policy must use native
authenticated identity; unsupported adapters cannot advertise the selector.
Rollback must not resurrect revoked identities or relax strict-family policy.

## Failure, acceptance and callers

Catalogue/config renderer calls the adapter; independent runtime executor alone
activates admitted owned artifacts. Cold boot reads accepted native snapshots
without compiler/admin. Invalid desired YAML leaves accepted VPN intact. Follow
sdd-runtime-safety.md for reload, recreation, guardian and policy-clean fallback.
HAProxy preserves foreign exact-SNI precedence; Caddy routes declared HTTPS paths.
Real pinned clients test every offered transport, auth denial, private SOCKS
exposure and egress policy. Kill management and reboot with broken desired config:
accepted routing must survive. Observe retiring workers before pruning old files.

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
