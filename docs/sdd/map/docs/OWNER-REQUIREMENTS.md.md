# docs/OWNER-REQUIREMENTS.md

Current M1 planning is governed by [the tracked plan](../../../plans/INSTALLATION-IMPLEMENTATION.md)
and the canonical document. Older wording below is baseline/history where it
conflicts; OR-37–OR-45 and D143–D148 prevail.

OR-11/OR-14 require `scripts/install` to configure group `hton` and start `scripts/deploy`;
deploy copies current working-tree product source into `/opt/hton`, including
uncommitted changes, while excluding `.git`, `docs/_lang/ru/tasks/*`, generated output,
secrets and mutable state. No prepared release or provenance file is required;
missing build helpers still block install without target compilation. See the
canonical owner record and D103 for this deployment contract. The earlier D99/D100
method was superseded; group permission, atomic activation, state preservation and
internal sudo requirements continue to apply.
OR-12 sets the end goal: working `example.com` with trusted SSL and exact-SNI
site/Amnezia separation, counted complete only after D81 checks and environment
evidence. `scripts/install` reports ready only after trusted served-certificate and
both route verification. OR-13 requires English product text and defers product
localization; Russian text is limited to `docs/_lang/ru/`. See the canonical owner
requirements and D102/D104.

Stable ID: `doc.docs-OWNER-REQUIREMENTS-md` · Phase: `design` · Kind: documentation

Planned file binding; no implemented functionality is claimed. Purpose: Canonical planning document docs/OWNER-REQUIREMENTS.md

Implementation wave: `P0/I3-C` (planned ownership, not completion).

## Features and responsibility

- doc

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Own the stated requirements/contract area; reference other owners instead of duplicating semantics.
- Product implementation phase was explicitly started by the owner on 2026-10-31; the earlier planning-only restriction is superseded. Architecture and contracts are reconciled before implementation, and acceptance remains separate.
- Planned interfaces and orchestration are not product implementation or acceptance evidence.
- D82 records the requested HTTPS/HAProxy/Amnezia trial. D84 is historical loopback prototype scope and does not override later OR-9/OR-10 production installer requirements for `example.com:443` and persistent random VPN ports; the foreign-resource qualification/remap boundaries remain.
- OR-10 records the owner's random 5000–6000 VPN service port requirement and durable per-service persistence/reuse across every repeated or resumed `scripts/install`, including process/host restart. Service settings are the source of truth; memory or journal alone is insufficient. D91 traces it and D92 records the allocator interpretation. It satisfies OR-9's request for an Amnezia port distinct from Outline 8443 without granting permission to move Outline.

## Calls / reads

- No internal dependency in this direction.

The decision trace is maintained by [docs/DECISIONS.md](DECISIONS.md.md); implementation evidence is maintained by [docs/IMPLEMENTATION-STATUS.md](IMPLEMENTATION-STATUS.md.md).

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
