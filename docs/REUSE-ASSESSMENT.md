# Reuse assessment

Reviewed 2026-10-31. Broad shortlist, not every repository on the Internet and not
an independent security audit. Read upstream documentation, license files and
selected source files; no installer or third-party executable was run and no
upstream source was incorporated. Default branches are research evidence, never
release pins. Pin and audit an exact release/commit before integration.

## Decision

Reuse network engines, DNS, ACME and gateway packages as isolated upstream processes.
Keep one hton configuration owner and a small controller/admin integration layer.
Do not install a second all-in-one panel beside it: competing config writers and
reusable subscriptions conflict with our declarative state and phase-2 one-time
credential disclosure. A replacement/fork strategy remains an alternative requiring
explicit architecture/license tradeoffs, not a hidden implementation shortcut.

## Panels and complete platforms

| Project | Useful existing work | Fit / recommended treatment |
|---|---|---|
| [3x-ui](https://github.com/MHSanaei/3x-ui) | Xray UI, config/link serializers, client quotas and API | Closest Go/SQLite panel candidate. Study pure generators/forms; do not adopt its secret-retrieval UX or host installer as core |
| [Marzban](https://github.com/Gozargah/Marzban) | Xray user model, API, converters and subscriptions | Python/API reference or isolated alternative; current subscription handlers intentionally regenerate configs, contrary to future single disclosure |
| [Marzneshin](https://github.com/marzneshin/marzneshin) | Panel/node separation and protocol grants | Reference for interfaces; extra node/control-plane roles need justification on one small VPS |
| [s-ui](https://github.com/alireza0/s-ui) | Sing-box management, parsers and exporters | Candidate interoperability reference; do not expose another admin API or config writer |
| [Hiddify Manager](https://github.com/hiddify/Hiddify-Manager) | Broad integrated protocol provisioning and onboarding | Closest product concept, but inspected Compose uses privileged/host networking plus database/Redis. Its ownership model differs from ours |
| [Remnawave](https://github.com/remnawave/backend) | Node management, API, profiles and subscription UI | Official panel requirements: minimum 2 GB RAM / 2 cores; Postgres and Redis. Fleet alternative, not default for weak VPS |
| [Pangolin](https://github.com/fosrl/pangolin) | App publication, users, policies and agent UX | Valuable product reference/standalone alternative. WireGuard-based architecture and dual licensing must be reconciled; do not silently replace approved edge |
| [zrok](https://github.com/openziti/zrok) | Managed public/private sharing | Strong standalone option; OpenZiti control plane is more infrastructure than a single reverse-tunnel module |

Sources for resource/packaging claims:
[Remnawave requirements](https://docs.rw/install/requirements/),
[database/Redis config](https://docs.rw/install/environment-variables/),
[Hiddify Compose](https://github.com/hiddify/Hiddify-Manager/blob/dev/docker-compose.yml).
Reported requirements are upstream recommendations, not our measured footprints.

## Engines, routers and clients

| Component | Treatment |
|---|---|
| [Xray-core](https://github.com/XTLS/Xray-core) | Existing chosen server engine; reuse native config validation, routing and supported per-user outbounds; no protocol rewrite |
| [sing-box](https://github.com/SagerNet/sing-box) | Preferred qualification for gateway/TUN/IPv6/client routing; run separately rather than importing its core into hton |
| [Mihomo](https://github.com/MetaCubeX/mihomo/tree/Meta) | Alternative gateway engine with existing selection/dashboard ecosystem; pin actual engine branch/tag |
| [dae](https://github.com/daeuniverse/dae) | Linux/eBPF gateway alternative; hardware/kernel qualification before adoption |
| [Passwall2](https://github.com/Openwrt-Passwall/openwrt-passwall2) | Existing OpenWrt integration, not a replacement UI we should rewrite |
| [Nikki](https://github.com/nikkinikki-org/OpenWrt-nikki) | Separate Mihomo-based router profile; do not pretend its config is Passwall/Xray JSON |
| [OpenClash](https://github.com/vernesong/OpenClash) | Router alternative; pin its engine and package separately |
| [HomeProxy](https://github.com/immortalwrt/homeproxy) | Sing-box-based router alternative |
| [Hiddify app](https://github.com/hiddify/hiddify-app) / [FlClash](https://github.com/chen08209/FlClash) | Client candidates; qualify import, transport and TV controls, never fork a full client just to brand it |
| [sing-box Apple client](https://sing-box.sagernet.org/clients/apple/) | tvOS candidate; availability and exact version are qualification inputs |
| [metacubexd](https://github.com/MetaCubeX/metacubexd) | Gateway dashboard reference/optional local operator tool, not a multi-user VPN credential portal |
| [Tailscale](https://github.com/tailscale/tailscale) / [Headscale](https://github.com/juanfont/headscale) | Optional overlay/control plane; prior policy gate remains. Headscale is not itself a VPN client or an IPv6 Internet provider |

Research discovered that repository default-branch metadata/license results can
identify different code than the intended engine branch (notably Mihomo main
versus Meta). Do not classify the engine as MIT from an unrelated branch file;
inspect the actual release source/LICENSE. This inconsistency is not evidence of
malware by itself. sing-box LICENSE includes GPL terms and a naming restriction;
GitHub's NOASSERTION label is not permission to ignore the license text.

## Reverse tunnels

| Project | Useful fit | Decision |
|---|---|---|
| [rathole](https://github.com/rathole-org/rathole) | Rust TCP/UDP tunnels, service-specific tokens, TLS/Noise and config reload | First small-module qualification candidate; benchmark locally, not upstream advertised numbers |
| [frp](https://github.com/fatedier/frp) | Broad proxy features, auth/plugin interfaces and management | Feature-rich alternative; global token mode alone is not per-user publication authorization |
| [Chisel](https://github.com/jpillora/chisel) | HTTP/WebSocket transport and reverse forwarding | Candidate when tested HTTP path transport is needed; restricted user/remote mappings required |
| [GOST](https://github.com/go-gost/gost) | Existing protocol adapters and forwarding chains | Optional adapter/toolbox; avoid creating a second general unbounded proxy control plane |
| [OpenSSH](https://man.openbsd.org/ssh.1) | Existing restricted reverse/L3 tunnels | Useful session-only baseline; no autossh or public unrestricted reverse listeners |

Caddy stays the browser HTTPS owner; tunnel data/management sockets remain scoped.
Select one server backend per needed mode, not all reverse-tunnel implementations.

## Core and UI reuse

| Component | Decision |
|---|---|
| HAProxy, Caddy | Keep current baseline; [caddy-l4](https://github.com/mholt/caddy-l4) is an alternative consolidation experiment, not an unreviewed plugin replacement |
| [Naive forwardproxy fork](https://github.com/klzgrad/forwardproxy/tree/naive) | Use the existing upstream fork as-is in pinned Caddy builds; not a new hton fork |
| acme.sh / Unbound / Hysteria / mtg | Use upstream CLI/config interfaces; keep issuer and config ownership singular |
| [SmartDNS](https://github.com/pymumu/smartdns) / [AdGuard Home](https://github.com/AdguardTeam/AdGuardHome) | Optional DNS features; do not install beside Unbound without an explicit dependency/resource need |
| [Uptime Kuma](https://github.com/louislam/uptime-kuma) | External monitoring option/reference, not a second mandatory always-on dashboard |
| [Authelia](https://www.authelia.com/) | Optional future SSO/MFA for protected sites; does not implement token ownership or network backend policy |
| [PocketBase](https://github.com/pocketbase/pocketbase) | Embedded-app backend alternative; would change current Rust/state ownership model, so not selected silently |
| [shadcn/ui](https://ui.shadcn.com/docs), [TanStack Query](https://tanstack.com/query/latest) | Selected UI starting point; source-level UI reuse plus cache/query library, with credential material excluded from query cache |
| [Refine](https://refine.dev/core/docs/), [react-admin](https://marmelab.com/react-admin/), [Mantine](https://mantine.dev/) | Credible admin/UI alternatives. Choose one foundation, not several overlapping frameworks |

## Concrete source review and possible extraction

Read these upstream files, without copying them into the product:

- 3x-ui docs/lib/xray/links.ts and frontend/src/pages/clients/SubLinksModal.tsx:
  small parsing/building helpers are useful references; the modal's repeatable
  download/copy workflow is not our future secret UX. GPL-3.0 licensing applies.
- s-ui util/genLink.go and util/linkToJson.go: protocol URI parsing/generation,
  but coupled to upstream models. Treat as reference or explicitly licensed
  extraction with tests and dependency review, not drop-in MIT helpers.
- Marzban app/subscription/share.py and app/routers/subscription.py: converter
  coverage and reusable subscription behavior. Code is coupled to application
  config/database/Xray state and AGPL-labeled; not a minimal pure library.
- Hiddify docker-compose.yml: integrated packaging reference, not a compatible
  least-privilege template for our mandatory core.

Useful candidates for direct reuse are narrow, reviewable serializers, schema
metadata and accessible UI primitives. Preserve provenance, license notices and
an upstream commit for each adopted file. Do not extract cryptography, remote
install/update logic, unrestricted shell execution or authentication shortcuts
from an untrusted project. Copying a fragment does not make it safe or licensed
under our MIT header. No hton fork or third-party source extraction exists yet.

## Selection and release gate

1. Prefer a pinned unmodified upstream process behind a narrow adapter.
2. Add a library only when it fits our runtime/license/security boundary.
3. Extract narrowly only after file-level provenance, dependency/side-effect review,
   behavior tests and ongoing security-update ownership. Maintain THIRD_PARTY_NOTICES.
4. Fork only with a documented need, license plan and upstream merge/security plan.
5. For GPL/AGPL/MPL and mixed-license projects, inspect actual file/commit terms;
   preserve applicable obligations. A process boundary is not an automatic legal
   exemption. Our own original glue remains MIT; do not relabel copied code.
6. Compare cold start, steady/peak RAM, CPU/goodput and update behavior on actual
   target hardware. No curl|sh, unpinned images, on-VPS builds or automatic install
   in this research phase. Signature/checksum and provenance checks precede use.

Architecture consequence: reuse mature data planes and client stacks; hton owns
consistent configuration, plans/recovery, user policy, one-time issuance and device
UX. No full panel is selected as a second runtime configuration authority.
