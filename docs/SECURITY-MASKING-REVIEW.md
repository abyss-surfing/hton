# Security and masking review — actual code, planned corrections

Recorded 2026-11-06. Sources: OR-39–OR-44, D137–D142. Static source review only;
no tests, live probing, server mutation or runtime protection claim. Current source
and new planned behavior deliberately have different status. Complete all product
implementation before unit, then functional acceptance. Detailed work list:
[tasks/obfuscation-hardening](plans/INSTALLATION-IMPLEMENTATION.md).

## Findings in the current source

| Source | Observed code | Planned correction / owner |
|---|---|---|
| `core/minimum/protocols.py:selected` | Fresh selection enables all three optional protocols when no selection exists | Preserve OR-32 argument-free selection, show/persist it; replace unsafe UDP defaults and block missing prerequisites explicitly |
| `core/minimum/protocols.py:DEFAULT_HYSTERIA_PORT`, `plan`, `hysteria_config` | Public UDP/8443 default; hostname tied to site; no explicit DNS resolver or obfs policy in generated config | OR-42 UDP/443, configured hostname, one port owner, backend-qualified DNS and QUIC profile; sdd-config/proxy |
| `core/minimum/protocols.py:DEFAULT_CAMOUFLAGE`, `plan` | Third-party fallback prompt default remains; single-SNI chain still overrides protocol name | Remove third-party fresh defaults, resolve distinct saved names once; preserve existing profiles through reviewed migration |
| `core/minimum/protocols.py:outline_key`, `plan` | WSS host derived from chain/site; exported transport has no explicit DNS policy | Configured Outline name plus separately qualified client DNS policy; transport export is not whole-device leak protection |
| `core/minimum/protocols.py:_outline_path` | Long random path is generated as a protected value | Implement the owner-requested ordinary configured paths without treating paths as credentials; path is inside TLS and is not a passive cleartext fingerprint |
| `core/minimum/protocols.py:_container` | Host networking and UID 0, with cap-drop/read-only/no-new-privileges | Narrow per-engine bind/identity and privileges; verify pinned binary supports non-root. Do not grant broad capabilities to force support |
| `adapters/engines/xray.rs:render` | Only `vless` is supported; injects private Unbound `tcp+local://` endpoint | Do not describe this as DNS coverage for minimum Amnezia/Outline/Hysteria |
| `services/onomat/runtime/dns/etc/unbound.conf.template`, `services/onomat/runtime/dns/rust/unbound.rs` | DNSSEC-validating iterator recursion; query logs off; no encrypted forward zone | DNSSEC verifies answers, not confidentiality. Plan explicit authenticated encrypted forwarding for the privacy profile; recursion stays a declared less-private mode |
| `core/minimum/protocols.py:hysteria_config`, `outline_config`, copied native REALITY config | No common enforced DNS policy for these paths; native config copied from installed state | Qualify each actual DNS owner; no accidental OS/Docker resolver fallback. Unknown coverage reports unverified, not protected |
| `core/minimum/ingress.py:configs` and `etc/haproxy/haproxy.cfg.template` | Already reject ClientHello with unknown SNI, plus bounded WAIT_END rejection | Preserve rejection; add only OR-43's covered own-wildcard redirect branch |
| `core/minimum/ingress.py:configs` | Existing Caddy serves website and WSS, admin disabled; no separate fallback Caddy necessary | Add host-scoped denial/redirect to this same instance; REALITY target is its private TLS endpoint |
| `core/minimum/ingress.py:authentication`, `protocols.py` client probe | curl uses `socks5h` | Good for remote lookup in that probe; does not prove system DNS capture or client leak prevention |

## DNS protection boundary and coverage

Client ISP, VPS provider and upstream resolver are different observers. Tunnel DNS
can hide queries from the client ISP; ordinary VPS recursive DNS can still expose
queries on the server network. Authenticated DoT/DoH upstream limits that network
exposure, but the selected resolver still sees queries. Direct DoH from a device
is encrypted DNS outside the VPN, not proof that DNS follows the VPN route.

| Mode | Required client behavior / limitation |
|---|---|
| Amnezia/VLESS, Outline or Hysteria in full-device VPN/TUN | Actual OS DNS capture and routing through tunnel; kill switch or explicit fail-closed policy; qualify exact app/OS/version |
| SOCKS5 including SSH `-D` | Application must send destination hostname (e.g. curl socks5h). SOCKS5 alone does not capture system DNS; SSH dynamic forwarding does not supply general UDP DNS |
| HTTP/HTTPS CONNECT, Naive, browser proxy, WS/XHTTP transport | Proxy remote names where supported; other apps/system/browser resolver rules remain separate. Bare HTTP/SOCKS transport needs a protected outer transport for network confidentiality |
| Telegram MTProto | Telegram-only proxy, not device-wide DNS/VPN. Record bootstrap/proxy-host resolution and app-specific bypass behavior separately |
| OpenWrt/Linux gateway or split tunnel | Explicit selected-device DNS routing/capture; preserve LAN discovery and configured local zones; classify direct exceptions, not universal protection |

Endpoint hostname bootstrap happens before a tunnel can exist. Use a qualified
bootstrap strategy (for example explicit endpoint IP with retained TLS SNI) or
label the DNS exception. Do not globally rewrite host DNS or block administrators'
SSH as a side effect. Tunnel loss, sleep/reconnect, browser DoH, DHCP changes and
native IPv6 are final client leak scenarios. R1 server IPv4-only is not proof of
client IPv6 leak prevention. Client recipe design stays within existing onboarding
scope; do not implement the external uplink client from this repository (OR-5).

## Masking decisions and tradeoffs

Use native qualified REALITY+Vision, WSS TLS and fakeTLS defenses rather than custom
crypto or random encapsulation. Pin builds; assess updates and actual TLS/ALPN
behavior. Secret generation uses OS randomness and credential revocation stays
separate from cosmetic names. No blanket padding, timed response tricks, fake
banners, disabling certificate validation, TLS key logging or unauthenticated
public proxy. Existing logging requirements remain; retain redacted operational
facts and avoid new destination/query logging merely for leak diagnosis.

Hysteria's standard QUIC/HTTP3 profile and Salamander are alternative wire modes.
Salamander removes ordinary HTTP3 compatibility, so it is not automatically layered
on the browser-like UDP/443 profile. No blind port hopping, experimental obfs or
kernel fake-TCP modules. Matching a browser port does not match browser traffic.
One UDP/443 owner per address; if Hysteria owns it, Caddy cannot also bind public
HTTP3 there. Do not advertise HTTP3 Alt-Svc on site names whose UDP endpoint is not
qualified to serve their HTTP3 requests. HAProxy's TCP SNI rules do not filter QUIC;
the UDP owner must qualify name/certificate/authentication handling separately.

REALITY custom fallback is an ordinary site response, not a disguise guarantee.
Disable special per-fallback throughput patterns by default; preserve bounded
resource protection without inventing per-protocol detectable throttling. Replay
and invalid-client handling must be supported by the pinned backend. WSS unauth
upgrade may produce protocol-specific errors after HTTP 101; do not promise that
Caddy can turn those encrypted native failures back into a normal page.

## Route and process contract — latest owner choices

One existing Caddy handles the admitted site content from `/var/www/html`, exact
fallback responses and redirects. Keep its current protected content/admission
mechanism; do not expose arbitrary host files, create another Caddy or serve HTON
secrets. Existing saved alternative webroots require an explicit reviewed change.

| Public name / input | HAProxy target | Ordinary HTTPS result |
|---|---|---|
| Configured apex `example.com` | Existing Caddy | Main website |
| Configured Amnezia `static.example.com` | REALITY; failed REALITY verification forwards original TLS bytes to existing Caddy private TLS endpoint | Trusted certificate, HTTP 403, `Access denied`, no server/protocol details |
| Other exact configured protocol names | Their declared backend, with qualified same-Caddy site fallback | Accepted per-protocol site behavior; never generic wildcard redirect |
| Unassigned one-label name under configured owned wildcard | Existing Caddy | Trusted wildcard TLS, HTTP 308 to fixed `https://example.com/` |
| Other unknown SNI, uncovered nested name, no SNI | Edge rejection | Connection closed; no VPN/Google backend reached |

Canonical names and apex URL come from the resolved snapshot, not literals in code.
Caddy's denial is an exact host policy selected by the configured REALITY name;
wildcard redirect is lower priority than every configured exact route. Host/SNI
mismatch cannot select a privileged host route: reject with a generic HTTP 421.
Redirect does not reflect arbitrary Host/path/query or forward credentials. ECH
inner SNI is unavailable to this TCP passthrough classifier: unsupported ECH must
not bypass allowlisting or be advertised as supported. Caddy can omit optional
software headers; do not forge another vendor's banner.

## Final acceptance and blockers

After complete implementation: unit contract cases, then functional real clients,
packet capture on the client physical interface and managed server DNS egress,
plus explicit external TCP/UDP exposure and TLS/HTTP behavior observations. Use
controlled synthetic query names, not real browsing histories. Capture is bounded,
protected, cleaned up and cannot silently enter normal diagnostics. Missing client
or upstream evidence reports `unverified`; no blanket anonymity verdict.

The M1 plan settles file contracts and HTON/app management ownership. Future
build/acceptance must establish actual image/client compatibility, per-backend DNS
support, client OS leak enforcement, UDP443 HTTP3 behavior and host admission.
Those facts are not yet observed. Current code has not been corrected by this
planning pass.

Primary sources: [REALITY transport](https://xtls.github.io/en/config/transports/reality.html)
warns fallback rate limits can fingerprint a service;
[Hysteria server configuration](https://v2.hysteria.network/docs/advanced/Full-Server-Config/)
distinguishes HTTP3 from obfuscated UDP;
[mtg](https://github.com/9seconds/mtg) documents fronting and replay defenses;
[curl proxy documentation](https://everything.curl.dev/transfers/conn/proxies.html)
distinguishes remote SOCKS name resolution;
[AmneziaDNS](https://docs.amnezia.org/documentation/instructions/change-dns/)
describes explicitly routing DNS inside the tunnel, not a guarantee for our builds.

## Current implementation-preparation result — D143–D147

The tracked [M1 plan](plans/INSTALLATION-IMPLEMENTATION.md) replaces the local task
drafts as implementation authority. OR-32 composition is preserved; replacing its
defaults with silent opt-out was only an unaccepted recommendation. Exact new file
owners, method graph, migration contract, app supporting key files and retained
remove/purge are settled. Optional client-only key page and quiet install follow
OR-45 and their SDD. Current product code remains unchanged. Final app/DNS/wire
behavior, built image digests and actual host admission remain future evidence,
not missing architectural decisions or facts invented during preparation.

### Quiet install and client-page additions

Actual common parsing in lib/hton already recognizes -q/--quiet, but current
minimum install still calls prompt for missing site/email and several consent/
protocol changes independently of quiet. Thus no-prompt behavior is a planned
correction, not a currently passing property. OR-45's page does not exist in the
current runtime. Its target root example is reviewed in examples/M1-CONFIGURATION.md;
M1-A/E must implement input policy across both shell elevation and Core callbacks.
The full-access app source additionally expects matching native key files;
sdd-managed-amnezia freezes those paths and native client-edit ownership.
