# docs/ARCHITECTURE.md

Current M1 planning is governed by [the tracked plan](../../../plans/INSTALLATION-IMPLEMENTATION.md)
and the canonical document. Older wording below is baseline/history where it
conflicts; OR-37–OR-45 and D143–D148 prevail.

Stable ID: `doc.docs-architecture-md` · Phase: `design` · Kind: documentation

Planned file binding; no implemented functionality is claimed. Purpose: Tracked docs/ARCHITECTURE.md document contract

Implementation wave: `P0/I3-C` (planned ownership, not completion).

## Features and responsibility

- documentation

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Own only its documented area; implementation claims require evidence.
- Do not duplicate CLI/config/runtime safety authority; link canonical sources.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
