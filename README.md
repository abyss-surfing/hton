# hton

**Product role:** HTON is the server platform. **Onomat owns the VPN service and
Unbound** under `hton/services/`; HTON provides installation, configuration,
validation, lifecycle and maintenance mechanisms for those services. Tartarus is
the development-environment platform for deployment and connectivity
maintenance; D6ton is the client platform. This is the target ownership recorded
in [OR-69](docs/OWNER-REQUIREMENTS.md#or-69-server-client-platforms-onomat-owns-vpn-2026-11-13)
and [D177](docs/DECISIONS.md). Existing monolithic code remains until a separately
authorized migration.

> **Chthonic role.** Hton is the thing that lives below the public threshold:
> a quiet core in the rock, holding REALITY, MTProto and TLS gates. In the
> target split Triodos wears the surface; Hton answers only those who know the
> shape of the silence behind it.

One public HTTPS site, selected VPN/proxy services underneath, and a small
self-hosted control plane. Target: an inexpensive VPS, devices including TV/Quest,
automatic ACME certificate renewal with HTTP or DNS validation and coexistence with existing VPN stacks.

**Status: the minimum installer is implemented; full R1 management remains incomplete.**
Target architecture (owner decision OR-68, 2026-11-12): the public edge (HAProxy,
Caddy, ACME, site, public ports) moves to the independent **Triodos** project, and
Onomat's VPN service is addressed by SNI behind Triodos. HTON remains the
platform hosting and managing it. The current monolith is
still the working data plane; the split is design/scaffold only until a separate
migration phase. See [OR-68](docs/OWNER-REQUIREMENTS.md#or-68-split-the-public-ingress-into-triodos-2026-11-12),
[D176](docs/DECISIONS.md) and the Triodos project's migration plan.
The default installer deploys the current working source tree to `/opt/hton`,
prepares the trusted site and safely remaps a qualified existing Amnezia Xray.
See [minimum installation](docs/MINIMUM-INSTALL.md) for prerequisites, usage and
verification limits. It uses Python 3 and pinned Docker engines; missing Rust
release helpers do not block this selected profile. Full R1 commands are separate.

The invariant: broken custom admin, compiler or automation must not break accepted
VPN routing. HAProxy/Caddy/native backends boot from an independently validated
last-known-good bundle, without a live admin/database/compiler dependency. Changes
are planned, natively validated and guarded; optional Docker staging is available; unsafe candidates never overwrite active files.

The target configuration is [hton.example.yaml](platform/hton.example.yaml): protected core,
platform enabled selection, separate versioned module settings under config/modules/,
and secretRef values. Editable Docker inputs and protected secrets share config/;
durable registry/runtime state lives in work/. CLI calls Core directly; later API/web
reuses the same domain operations.
Only selected engines run; builds happen outside the weak VPS.

- [Documentation index](docs/README.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Phased roadmap](docs/ROADMAP.md)
- [Public CLI contract](docs/CLI.md)
- [Project-shaped per-file SDD map](docs/sdd/map/README.md)
- [Reusable components and provenance](docs/powered.md)
- [Admin UX proposal](docs/ADMIN-UX.md)

R1 is the safe single-owner baseline. R2 adds personal accounts, named one-time
credentials and local-site publication through the project domain. IPv6/family
selection is far future; external client route ownership requires a separate decision. These future capabilities do
not silently expand release dependencies or override prior protocol/consent policy.
Historical Russian copies/local tasks are archival; current tracked docs own design.

Current work follows the [implementation status](docs/IMPLEMENTATION-STATUS.md).
[Detailed interfaces](docs/INTERFACES.md) and the
[multi-agent orchestrator](docs/IMPLEMENTATION-ORCHESTRATOR.md) define file
ownership and writing order. Complete implementation precedes unit then functional
testing; further acceptance is determined afterward. No product tests or deployment
are implied by documentation/schema consistency checks.
