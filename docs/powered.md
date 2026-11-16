# Powered by — provenance and reuse plan

Updated: 2026-10-31. This repository currently contains a Bash scaffold and design
artifacts; it does not yet ship a functioning integrated runtime stack. The table
records selected upstream components and planned treatment, not installed software.
No third-party source has been copied into the product and no hton fork exists.
Research alternatives are in [REUSE-ASSESSMENT.md](REUSE-ASSESSMENT.md).

| Component | Treatment | Status / role |
|---|---|---|
| Bash + host core tools/systemd/Docker Compose | Upstream as-is, thin own command wrappers | Bash scaffold exists; integrated lifecycle planned |
| [HAProxy](https://www.haproxy.org/) | Upstream as-is | Selected TCP/SNI edge, not yet integrated |
| [Caddy](https://caddyserver.com/) | Upstream + pinned plugin build | Selected HTTPS/site/HTTP routing |
| [Naive forwardproxy](https://github.com/klzgrad/forwardproxy/tree/naive) | Existing upstream fork as-is; not a hton fork | Selected Caddy plugin; validate exact build |
| [Xray-core](https://github.com/XTLS/Xray-core) | Upstream as-is through generated config/API | Selected VPN engine; MPL file-level terms if modifying source |
| [Unbound](https://nlnetlabs.nl/services/unbound/about/) | Upstream as-is | Selected private DNS/optional DoT backend |
| [acme.sh](https://github.com/acmesh-official/acme.sh) | Upstream as-is, dns_internetbs | Selected short-lived DNS-01 issuer; own validated delivery adapter |
| [mtg](https://github.com/9seconds/mtg) | Upstream as-is | Planned optional MTProto; foreign telemt remains adapter-managed |
| [Hysteria](https://github.com/apernet/hysteria) | Upstream as-is | Planned optional UDP/QUIC engine |
| [sing-box](https://github.com/SagerNet/sing-box) | External upstream process; no core copy | Preferred gateway/TUN/IPv6 qualification; honor license/naming terms |
| [Passwall2](https://github.com/Openwrt-Passwall/openwrt-passwall2) | Existing package as-is | Planned router adapter/export; alternative packages independently qualified |
| [rathole](https://github.com/rathole-org/rathole) | Upstream as-is if qualification passes | First reverse-tunnel candidate; frp/Chisel alternatives, not simultaneous mandatory services |
| Rust axum/reqwest + vetted auth/serialization libraries | Normal library dependencies, exact versions/license audit pending | Planned small own controller/admin; no handwritten crypto |
| React + Vite | Upstream build dependencies; static production assets | Selected frontend foundation, external build |
| [shadcn/ui](https://ui.shadcn.com/docs) | Planned source-level component extraction/customization with MIT notices | Selected UI starting point; no files imported yet |
| [TanStack Query](https://tanstack.com/query/latest) | Upstream dependency as-is | Planned reactive metadata state; exclude raw credential results |
| [3x-ui](https://github.com/MHSanaei/3x-ui), [s-ui](https://github.com/alireza0/s-ui), [Marzban](https://github.com/Gozargah/Marzban) | Reference only; possible narrow explicitly licensed extraction | Parsers/generators studied; no fork/code copy selected |
| OpenVPN / SoftEther / ocserv / TUIC implementation | Optional upstream adapters, implementation pins pending | Planned compatibility modules; not release-required |
| Tailscale / Headscale | Upstream candidate, no code extraction | Lab/policy-gated overlay; not currently enabled |
| Config/module compiler, transaction journal, bounded controller, identity policy and UX | Original hton integration | Planned own code; Bash-first, complex logic via structured helper |

## Fork/extraction rules

- An external upstream fork (Naive plugin) is distinct from a maintained hton fork.
- "Extract essence" means either an independently implemented behavior from a
  documented interface or a clearly attributed source extraction with its actual
  license. Do not translate/copypaste copyleft code and label it MIT.
- Before importing code: source URL, commit, file list, dependency/side-effect review,
  tests, notices, applicable source-distribution obligations and update owner.
- Populate shipped THIRD_PARTY_NOTICES and release lock/SBOM from actual artifacts,
  not this planning table. Container/API separation does not erase license duties.
- Copying code from a potentially risky project does not remove its bugs; review
  the extracted dependency closure and tests. No installer/auth/crypto shortcuts.

See [reuse analysis](REUSE-ASSESSMENT.md), [admin UX](ADMIN-UX.md) and
[implementation map](sdd/map/README.md) for the current design. Runtime claims
require actual integration and acceptance; this document makes none.
