# Area SDD index

## Product language policy

Per [OR-13/D102](../OWNER-REQUIREMENTS.md#or-13-english-source-language-and-deferred-product-localization-2026-11-02),
all product code and user-visible product text (help, prompts, errors, logs and
diagnostics) are English. Product localization is deferred; do not add translated
strings or localization infrastructure in this phase. Russian prose is allowed
only under `docs/_lang/ru/`; canonical specs, maps and other docs are English. This policy
constrains text content and does not select a programming language.

Owner requests are recorded in [DECISIONS](../DECISIONS.md). Engineering choices implement those requests; they are not additional owner quotations. Historical local task notes do not override tracked plans.

[Architecture](../ARCHITECTURE.md) defines boundaries; [runtime safety](sdd-runtime-safety.md) governs admission/recovery; [config](sdd-config.md) owns declarative shape; [CLI](../CLI.md) owns arguments. [Roadmap](../ROADMAP.md) separates release and deferred work. These are design contracts, not implemented functionality.

The [project map](../sdd/map/README.md) supplies per-file responsibilities, methods, arguments and bidirectional dependencies. File bindings and languages may change while logical contracts remain consistent.

| Area SDD | Responsibility |
|---|---|
| [sdd-admin.md](sdd-admin.md) | SDD admin — core web configuration, devices and optional Telegram |
| [sdd-capabilities.md](sdd-capabilities.md) | SDD capabilities — optional protocols and device onboarding |
| [sdd-certs.md](sdd-certs.md) | SDD certs — DNS-01 certificate lifecycle |
| [sdd-cli.md](sdd-cli.md) | SDD CLI — thin public interface and independent runtime bootstrap |
| [sdd-client-ipv6.md](sdd-client-ipv6.md) | SDD client IPv6 — IPv6 destinations over an IPv4-only access network |
| [sdd-config.md](sdd-config.md) | SDD config — one declarative source, protected core and optional features |
| [sdd-control.md](sdd-control.md) | SDD control — plans, transactions and bounded administration |
| [sdd-dns-dot.md](sdd-dns-dot.md) | SDD dns-dot — private recursion and optional public DoT |
| [sdd-foreign-adapters.md](sdd-foreign-adapters.md) | SDD foreign adapters — Amnezia/Outline coexistence (t08) |
| [sdd-identity.md](sdd-identity.md) | Deferred SDD identity — accounts, protocol profiles and one-time credentials |
| [sdd-installer.md](sdd-installer.md) | SDD installer — declarative bootstrap and reconciliation |
| [sdd-ipv6.md](sdd-ipv6.md) | SDD IPv6 — server-side egress and listener provisioning |
| [sdd-naive-mtproto.md](sdd-naive-mtproto.md) | SDD Naive and MTProto |
| [sdd-proxy.md](sdd-proxy.md) | SDD proxy — HAProxy edge |
| [sdd-router.md](sdd-router.md) | SDD gateway — devices without VPN apps |
| [sdd-runtime-safety.md](sdd-runtime-safety.md) | SDD runtime safety — VPN survives management failure |
| [sdd-smart-connect.md](sdd-smart-connect.md) | SDD smart-connect — device onboarding, local diagnosis and controlled failover |
| [sdd-testing.md](sdd-testing.md) | SDD testing — final bundle, failure containment and real clients |
| [sdd-web-publish.md](sdd-web-publish.md) | Deferred SDD web-publish — local-site reverse tunnels and HTTPS routes |
| [sdd-whoami.md](sdd-whoami.md) | SDD whoami — exhaustive host/DNS introspection (t02) |
| [sdd-xray.md](sdd-xray.md) | SDD xray — VLESS core (t04) |

## Shared constraints

- Accepted VPN boots and routes without management, its database or compiler. Management limits reserve data-plane resources.
- Native checks and isolated Docker real-client tests qualify the final pinned bundle; read-only dry-run cannot substitute for them.
- Independent guardian handles killed or hung management; compatible fallbacks respect revocation, strict-family and certificate-validity fences.
- hton.yaml owns desired settings; secretRef owns protected references; optional features use enabled and nested settings. Core has no toggle.
- Persistent service state uses project bind mounts, not named volumes or container writable layers.
- Foreign mutations, port takeover and new host privileges require local explicit approval. Planning is not deployment authorization.
- Build externally and measure weak-VPS budgets. Personal accounts and single-disclosure credentials are post-release.

## Current contract additions and development order

- [sdd-lifecycle.md](sdd-lifecycle.md): explicit installed registry, resumable add,
  enable/remove/purge/update distinctions, partial up/down and durable stopped intent.
- [sdd-configurator.md](sdd-configurator.md): deferred standalone parameter editing,
  semantic merge, preservation and format migration; invalid target diagnostics.
- [Detailed shared interfaces](../INTERFACES.md): records, owners, methods, arguments,
  return values, effects, errors and SOLID/GRASP/DRY seams.
- [Implementation orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md): future file owners,
  parallel waves and complete implementation before unit then functional testing.

Root features carry enabled only; module settings are separate versioned files.
Core R1 retains bounded lifecycle selection/seed writes, not a general editor.
Atomic parameter API is deferred; Docker candidate staging is optional. IPv6 and
strict family work are far future. Down suspends jobs; restore stays stopped.

## M1 current installation contract set

The [tracked implementation plan](../plans/INSTALLATION-IMPLEMENTATION.md) and
[Markdown file graph](../sdd/INSTALLATION-FILE-GRAPH.md) select exact current scope.
[Managed Amnezia](sdd-managed-amnezia.md) owns provisioning/app/host access;
[client access page](sdd-client-access-page.md) owns the optional passwordless
client-only page. Config/proxy/DNS/certificate/lifecycle/installer contracts retain
one owner per shared rule. Full administration remains TODO/deferred. The old
single-SNI/UDP8443/default-HTTP01 baseline is not the M1 wildcard profile.
