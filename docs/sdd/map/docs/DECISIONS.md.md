# docs/DECISIONS.md

Current M1 planning is governed by [the tracked plan](../../../plans/INSTALLATION-IMPLEMENTATION.md)
and the canonical document. Older wording below is baseline/history where it
conflicts; OR-37–OR-45 and D143–D148 prevail.

D99/D100 record the superseded prepared-release deployment method. D103 replaces it
with current working-tree source delivery and excludes `.git`, `docs/_lang/ru/tasks/*`,
generated output, credentials and mutable state. Group setup, atomic
`/opt/hton/current` switching, state preservation and self-sudo remain. D102 traces
the English-only product language rule with localization deferred. D104 requires
`scripts/install` to verify a trusted served certificate and both routes before it
reports ready; successful source rendering alone is not deployment evidence.

Stable ID: `doc.docs-decisions-md` · Phase: `design` · Kind: documentation

Planned file binding; no implemented functionality is claimed. Purpose: Tracked docs/DECISIONS.md document contract

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
- D91 records the owner-selected inclusive 5000–6000 range and durable per-service persistence/reuse across repeated/resumed installs and process/host restart; service settings, not memory or journal alone, are authoritative. D92 assigns allocation to `core.port-allocation` with fresh reservation evidence before the pure planner. D93 binds the exact candidate to immutable lifecycle add/enable plans, separate local allocation approval, journal-before-CAS, and exact operation/digest recovery. D89's old 8443 collision is historical evidence, not the current target.
- D94 reconciles the derived `dns.ipv4` graph gate with owner OR-8/D85: public DNS-A stays unverified until explicit WHOAMI, while local HTTPS activation still requires fresh native certificate evidence.
- D95 lets structural install preflight prepare port candidates and safe earlier phases before certificate issuance; certificate requirements remain explicit, but genuine fresh local evidence is required through ordinary `resolve` immediately before ApplyIngress.
- D96 records the non-secret certificate account email in first-publication input, fixed InternetBS DNS-01 metadata/SecretStore refs for a new cert config, preservation of existing refs, and credential-file import through the existing protected SecretStore owner; missing credentials leave install resumable and unpublished.
- D84 is retained as historical evidence of the initial loopback sandbox prototype; it is superseded for production installation by OR-9/OR-10 and D88/D91.
- D97 binds install's `/var/www/index.html` absence/content and filesystem identity into preflight, permits the placeholder only after that basis is rechecked, and allows ready only after bounded local TLS/SNI/200/exact-body verification with unchanged source identity, digest and bytes. The probe supports the webroot's 1 MiB file bound; public DNS remains unverified.
- D98 records the bundle artifact choice: publish exact UTF-8 source bytes as immutable `site-index.html` with the generated Caddy route, serve only that fixed route without a `/var/www` mount, and verify byte-exact local response plus stable source identity/digest. Bundle rollback restores content and route together; the choice also avoids parser transformations and symlink exposure. This supersedes only D84's mount detail; D82 trial intent remains.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

OR-39/D137: all SNI and administrative usernames are configurable; scripts create
the dedicated managed account. OR-40/D138: review masking after the reported ban;
separate observed evidence from hypotheses and do not claim guaranteed anonymity.

## OR-41–OR-44 / D139–D142 planning amendment

Canonical authority: docs/SECURITY-MASKING-REVIEW.md, sdd-proxy, sdd-config and
sdd-dns-dot. Public TCP/443 and selected QUIC UDP/443; no public 8443 or competing
UDP owner. Exact protocols precede owned covered wildcard redirects; other unknown
SNI is rejected. The existing Caddy serves `/var/www/html`, REALITY-host 403 and
unassigned wildcard 308 to the apex; no extra Caddy or nested MTProto fallback.
Current code gaps (including plaintext recursive DNS and absent cross-protocol
client DNS evidence) are planned corrections, not implemented protection. Update
typed schemas and consumers together; no product/parser readiness is implied.
