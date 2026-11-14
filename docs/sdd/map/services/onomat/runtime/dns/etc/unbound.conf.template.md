# services/onomat/runtime/dns/etc/unbound.conf.template

Stable ID: `template.unbound` · Phase: `R1` · Kind: configuration

Owned by Onomat; HTON consumes the template through its renderer. Planned file
binding; no implemented functionality is claimed. Purpose: Bounded private recursion config

Implementation wave: `I2-A` (planned ownership, not completion).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Public DoT is not public plain DNS; DNSSEC and limits qualified.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/render.rs](../../../../../core/src/render.rs.md) — `compile` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

I2 serializer-owned ADDRESS/PORT/SUBNET slots; private recursion with DNSSEC validator,
prepared root.key, one thread, bounded caches/concurrency and no IPv6/query logging.
No listener is host-published. Native root-key qualification and unbound-checkconf
remain required independent evidence, not inferred from successful serialization.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
