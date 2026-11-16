# core/src/catalog.rs

Stable ID: `core.catalog` · Phase: `R1` · Kind: source

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Resolve built-in module graph

Implementation wave: `I1-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- modularity

## Methods, arguments and result

- `describe` — arguments: ModuleId; result: ModuleDescriptor or unknownModule.
- `profileSuggestions` — arguments: fixed ProfileId; result: trusted ordered first-run hints only; unknown/inconsistent profile inventory fails catalogue loading; no selection or effect.
- `resolve` — arguments: normalized selection, catalogueVersion, releasePhase, platformEvidence; result: DependencyGraph or structured conflicts.
- `resolve_for_install_preflight` — arguments: normalized selection, releasePhase, platform evidence; result: structural DependencyGraph that defers only local certificate readiness; never an activation-ready graph.
- `bindAllocation` — arguments: DependencyGraph with trusted allocation templates, exact PortAllocationCandidate, validated settings snapshot; result: DependencyGraph with Host ListenerClaims materialized from descriptor path/transport/address declarations.
- `allocationBindingsFor` — arguments: selected DependencyGraph and validated NormalizedSnapshot; result: descriptor-owned service bindings enriched with exact saved port values, or trusted descriptor defaults for an absent first-install document; malformed/inconsistent present documents remain structured issues.
- `retainedAllocationsFor` — arguments: selected service IDs, installed/retained registry service IDs, validated NormalizedSnapshot; result: saved allocations for nonselected managed backends, or issues for missing/inconsistent transport fields.

## Structures and injected dependencies

ModuleDescriptor/DependencyGraph; pure dependency and claim resolver

Dependencies: versioned descriptor/service inventory, platform/release evidence.

## Logic

- Compute shared engines and private dependencies.
- Reject unavailable/policy-blocked modules and SNI/address/protocol/port collisions.
- Do not load executable user plugins.
- Validate unique IDs, references, phase constraints and acyclic feature/service dependencies before rendering.
- Resolve profile suggestion sets from the trusted index. A suggestion must name a known implemented R1 descriptor; it is never treated as persisted selection or consent.
- Compute shared service closure including mandatory core consumers; never automatically enable another optional feature. Reject disabled required features with a proposed batch.
- Resolve scope defaults and allowed scopes; reject wildcard/dual-stack endpoint overlap and conflicting SNI/path ownership.
- Describe availability, settings schema, versioned defaults and blocked reasons separately from observed health.
- Catalogue availability differs from installed registry and desired enablement.
- `resolve_for_install_preflight` shares the structural resolver but omits only the local HTTPS certificate readiness conflict so the install owner can prepare port candidates and safe earlier phases before issuance. Certificate requirements stay explicit in InstallPlan; ordinary `resolve` with genuine fresh evidence remains mandatory immediately before ApplyIngress.
- Each trusted module declares semantic check, setup/update/lifecycle adapter, settings format, resources and ownership; no executable hooks in user config.
- Optional `portAllocation` is parsed and validated as a trusted service-to-settings binding: stable allocation/service IDs, owning module settings document, binding group and exact path/transport fields. Null is valid for shared-edge multiplexed services. Unknown services/paths/transports, duplicate fields, cross-document paths or nonnumeric targets fail descriptor validation.
- `settingsDocument` is the canonical config object ID `module.<moduleId>`, matching `ConfigDocument` and `ReadSet`; the file component still comes only from the descriptor's known `modules/` location.
- Dynamic host ListenerClaims are materialized only by `bindAllocation` from the same mapping and candidate; do not duplicate dynamic ports in `listenerClaims` or hardcode module paths in allocator.
- The planner receives saved values only through `allocationBindingsFor`; config traversal stays with the catalogue mapping owner and is never repeated in install/lifecycle callers.
- `retainedAllocationsFor` projects saved settings of installed disabled services into the same uniqueness set; it does not inspect host state or grant lifecycle authority.

## Calls / reads

- [modules/index.yaml](../../modules/index.yaml.md) — `describe` -> declarative dependency.
- [modules/services.yaml](../../modules/services.yaml.md) — `describe` -> declarative dependency.
- [modules/vless.yaml](../../modules/vless.yaml.md) — `describe` -> declarative dependency.
- [modules/naive.yaml](../../modules/naive.yaml.md) — `describe` -> declarative dependency.
- [modules/mtproto.yaml](../../modules/mtproto.yaml.md) — `describe` -> declarative dependency.
- [modules/hysteria.yaml](../../modules/hysteria.yaml.md) — `describe` -> declarative dependency.
- [modules/dot.yaml](../../modules/dot.yaml.md) — `describe` -> declarative dependency.
- [modules/foreign-amnezia.yaml](../../modules/foreign-amnezia.yaml.md) — `describe` -> declarative dependency.
- [modules/vless.yaml](../../modules/vless.yaml.md) — `describe` -> shared-edge route and no standalone allocation.
- [modules/foreign-outline.yaml](../../modules/foreign-outline.yaml.md) — `describe` -> declarative dependency.
- [modules/router-gateway.yaml](../../modules/router-gateway.yaml.md) — `describe` -> declarative dependency.
- [modules/linux-gateway.yaml](../../modules/linux-gateway.yaml.md) — `describe` -> declarative dependency.
- [modules/ipv6-he.yaml](../../modules/ipv6-he.yaml.md) — `describe` -> declarative dependency (far-future only; excluded from earlier execution closure).
- [modules/ssh-session.yaml](../../modules/ssh-session.yaml.md) — `describe` -> declarative dependency.
- [modules/site-content.yaml](../../modules/site-content.yaml.md) — `describe` -> declarative dependency.
- [modules/doctor-watch.yaml](../../modules/doctor-watch.yaml.md) — `describe` -> declarative dependency.
- [modules/telegram.yaml](../../modules/telegram.yaml.md) — `describe` -> declarative dependency (later-management only; descriptor data, not execution dependency).
- [modules/outline-wss.yaml](../../modules/outline-wss.yaml.md) — `describe` -> declarative dependency.
- [modules/private-socks.yaml](../../modules/private-socks.yaml.md) — `describe` -> declarative dependency.
- [modules/https-proxy.yaml](../../modules/https-proxy.yaml.md) — `describe` -> declarative dependency.
- [modules/trojan-ws.yaml](../../modules/trojan-ws.yaml.md) — `describe` -> declarative dependency.
- [modules/trojan-tls.yaml](../../modules/trojan-tls.yaml.md) — `describe` -> declarative dependency.
- [modules/tuic.yaml](../../modules/tuic.yaml.md) — `describe` -> declarative dependency.
- [modules/openvpn.yaml](../../modules/openvpn.yaml.md) — `describe` -> declarative dependency.
- [modules/sstp.yaml](../../modules/sstp.yaml.md) — `describe` -> declarative dependency.
- [modules/softether.yaml](../../modules/softether.yaml.md) — `describe` -> declarative dependency.
- [modules/openconnect.yaml](../../modules/openconnect.yaml.md) — `describe` -> declarative dependency.
- [modules/tailscale-exit.yaml](../../modules/tailscale-exit.yaml.md) — `describe` -> declarative dependency.
- [modules/smart-connect.yaml](../../modules/smart-connect.yaml.md) — `describe` -> declarative dependency (R2 only; descriptor data, not execution dependency).
- [modules/client-ipv6.yaml](../../modules/client-ipv6.yaml.md) — `describe` -> declarative dependency (far-future only; excluded from earlier execution closure).
- [modules/web-publish.yaml](../../modules/web-publish.yaml.md) — `describe` -> declarative dependency (R2 only; descriptor data, not execution dependency).
- [modules/management-api.yaml](../../modules/management-api.yaml.md) — `describe` -> declarative dependency (later-management only; descriptor data, not execution dependency).
- [modules/configurator.yaml](../../modules/configurator.yaml.md) — `describe` -> declarative dependency (later-management only; descriptor data, not execution dependency).
- [modules/web-admin.yaml](../../modules/web-admin.yaml.md) — `describe` -> declarative dependency (later-management only; descriptor data, not execution dependency).

## Called / read by

- [scripts/install](../../scripts/install.md) — `run` -> `resolve`.
- [scripts/restart](../../scripts/restart.md) — `run` -> `resolve`.
- [core/src/config.rs](config.rs.md) — `validate` -> `resolve`.
- [core/src/render.rs](render.rs.md) — `compile` -> `resolve`.
- [core/src/exports.rs](exports.rs.md) — `exportProfile` -> `describe`.
- [accounts/src/egress.rs](../../accounts/src/egress.rs.md) — `enforce` -> `describe`.
- [core/src/lifecycle.rs](lifecycle.rs.md) — `plan` -> `resolve`.
- [core/src/setup.rs](setup.rs.md) — `prepare` -> `describe`.
- [core/src/port_allocation.rs](port_allocation.rs.md) — `plan` reads validated typed descriptor binding.
- [core/src/host/installation.rs](host/installation.rs.md) — `preflight` -> `bindAllocation` after candidate planning.
- [core/src/lifecycle.rs](lifecycle.rs.md) — add/enable -> `bindAllocation` for selected managed VPN backends.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I1 concrete implementation binding (2026-10-31)

`ModuleCatalogue::builtin` loads embedded trusted index/services and R1 descriptors,
including fixed profile suggestion sets;
`describe` returns descriptor plus implementation/phase availability. Future IDs
remain metadata-only unavailable entries. Mandatory core is an implicit non-toggle
consumer of runtime, HAProxy and Caddy. VLESS adds shared Xray/private Unbound;
optional feature dependencies must already be enabled. Descriptor and service
cycles/references are checked before use. `resolve` receives normalized snapshot,
release phase and `ResolutionEvidence` (qualified capabilities, resolved protected
route claims); it returns a graph with explicit conflicts, never enables a feature.
Listener wildcard/IPv4-mapped IPv6 overlap, TCP/UDP distinction, exact/wildcard SNI,
HTTP path overlap and certificate/state ownership are checked centrally. Supplied
resolved secret route paths are protected inputs; their Debug is suppressed.

Listener namespace is explicit in shared DTOs: Host, Project{networkId}, or
Service{serviceId}. Collision checks compare equal namespaces and never infer
namespace from `public`. Host-loopback and public wildcard on the same host may
therefore conflict; separate container-private binds do not.

Published HTTPS requires a concrete own domain and fresh `certificates.native`
passing evidence for core, bound to the exact root ContentRevision. Certificate
evidence expires after five minutes and cannot be from the future. Public DNS-A
identity is not a graph/readiness prerequisite and is never queried automatically;
it remains unverified until explicit `identity.whoami` (OR-8). Private domain TLS
also requires certificate evidence. Domainless public activation remains blocked
while unpublished/incomplete setup remains savable via installer lifecycle; plain
document load/normalization never fabricates readiness or allocates secrets.

I3 allocation binding: descriptor validation rejects invalid ownership, duplicate
field/transport mappings and malformed host listener templates. `bind_allocation`
checks candidate settings/document/group/path bindings against the normalized
snapshot, then emits claims for each declared transport/address using the candidate
port. It is pure and adds conflicts on missing, extra or mismatched assignments.
