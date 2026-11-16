# Edge proxy choice for a small VPS

Date: 2026-10-31. Status: rationale incorporated into the implementation plan and current architecture. Does not authorize changes to running foreign services.
Additional owner constraint: HTTPS, automatic Let's Encrypt validation/renewal,
and efficient operation on a weak server take priority.

## Recommendation

Retain HAProxy as the public TCP classifier, using explicit SNI routing and
passthrough. Prefer one Caddy + Naive forwardproxy process for own-domain TLS,
HTTP routing and site serving. Hysteria owns a separate UDP endpoint. This
removes HAProxy's HTTP/2 handling from the Naive path and avoids inspecting the
same HTTP traffic in two processes. It does not eliminate an existing second
TLS encryption layer: the old HAProxy -> plaintext Caddy design already had
only one external TLS termination.

This is a compatibility and simplicity recommendation, not a measured claim
that Caddy TLS outperforms HAProxy/OpenSSL. Compare CPU and memory on the actual
VPS before calling either topology optimal. No service implementation or server
benchmark exists in this repository yet; hardware capacity is unspecified.

## Actual protocol boundaries

Xray is an implementation and VLESS a protocol, not two independently detectable
services. Amnezia is a manager for multiple protocols. Detect and route actual
configured transports rather than container names or product brands.

| Transport | Entry and routing | TLS owner |
|---|---|---|
| Own VLESS WS/XHTTP | TCP/443 -> own HTTP frontend -> validated path -> Xray | Caddy in proposed topology |
| Naive HTTP/2 CONNECT | TCP/443 -> own HTTP frontend -> authenticated Naive handler | Caddy + naive fork |
| Amnezia Xray REALITY | TCP/443 -> exact configured camouflage SNI -> raw backend | REALITY backend; no edge termination |
| MTProto fakeTLS | TCP/443 -> explicit camouflage SNI -> raw mtg/telemt | Backend handles fakeTLS; no edge termination |
| Existing bare Outline/Shadowsocks | Its existing TCP/UDP port | No standard TLS/SNI to route |
| Optional Outline over WSS | TCP/443 -> own HTTP frontend -> configured WS paths | Caddy; requires compatible server/client setup |
| Hysteria2 | UDP/443 if available, otherwise configured UDP port | Hysteria's QUIC endpoint |
| AmneziaWG and other foreign UDP | Existing independent UDP endpoints | Respective protocol implementation |
| Site and admin | TCP/443 -> own HTTP frontend -> site/default or admin path | Caddy |
| DoT | Optional TCP/853 | Separately declared TLS terminator; measure if keeping HAProxy here |

TCP/443 and UDP/443 can coexist on one IP. Two independent services cannot
both own the same UDP/443 binding. Inspect existing Amnezia bindings and disable
Caddy's separate HTTP/3 listener when Hysteria owns UDP/443. Hysteria's HTTP/3
masquerade may serve/proxy the same site, but its QUIC protocol is not an ordinary
HTTP reverse-proxy upstream. Do not terminate it in Traefik/Nginx HTTP/3 and
assume the Hysteria authentication and datagrams will pass through.

Shadowsocks-over-WebSockets is supported by the Outline project, but that does
not make existing ss:// keys and an installed legacy Outline server WSS-capable
without migration. Retain old endpoints until supported clients are verified.

## Routing refinement

The current global rule "non-own SNI + no ALPN -> MTProto" is fragile. Ordinary
TLS and REALITY may also omit ALPN. Absence of ALPN is not a protocol identity.
Replacing HAProxy with Nginx or Traefik cannot resolve indistinguishable inputs.

Prefer this ordered classifier:

1. Wait for a complete bounded ClientHello; reject malformed/oversized/timeouts.
2. Exact configured REALITY/foreign SNI names -> their raw backend.
3. Exact configured MTProto camouflage SNI names -> their raw backend.
4. Other valid TLS -> the own Caddy frontend, with a default site where its
   certificate/host policy permits it. Unknown names cannot receive a trusted
   certificate matching arbitrary third-party domains. No-SNI uses an explicit
   default certificate policy or closes; do not promise browser-valid HTTPS by IP.
5. Non-TLS does not silently become HTTP or Outline. Keep unsupported protocols
   on independent endpoints.

Compile exact raw-routing names before own-domain wildcard matches. Reject a
name assigned to multiple incompatible backends; otherwise an own tg.example.com
or *.example.com rule would steal MTProto traffic. If existing clients collide,
preserve their endpoint and plan a consented migration or use another port/IP.
Do not solve collisions by switching on unstable client fingerprints.

SNI sniffing observes visible outer ClientHello only; ECH needs an explicit
compatible endpoint policy, not an assumption that the inner name is readable.
MTProto without fakeTLS has no such route and needs a separate endpoint.

For mtg with a locally hosted camouflage name, configure its supported fronting
destination directly to an internal TLS frontend. Resolving the public name back
through the classifier can loop. Pin and test the backend's fronting settings;
keep that fallback behavior separate from valid fakeTLS authentication.

## Comparing the candidates

| Criterion | HAProxy | Nginx | Traefik |
|---|---|---|---|
| Visible SNI/ALPN passthrough | Direct TCP ACLs/maps | stream + ssl_preread module | TCP HostSNI/ALPN, explicit priorities |
| Generic HTTP sites/reverse proxy | Needs site backend | Native strength | HTTP routing, needs site backend |
| Naive | L4 passthrough to Caddy avoids H2 parsing here | L4 passthrough to Caddy is viable | L4 passthrough to Caddy is viable |
| Dynamic container discovery | Not needed for this static stack | Not needed for this static stack | Strong feature, little benefit for few fixed services |
| LE DNS-01 | External ACME client | External ACME client is viable | Integrated Lego ACME resolver available |
| Hysteria multiplexing | Use independent UDP endpoint | UDP relay is not protocol recognition | UDP routers do not classify SNI/URL |
| Fit for existing Bash-generated configs | Strong | Strong | Viable file provider; Docker socket unnecessary |

Nginx is a credible alternative, not technically incapable. Its stream module
can extract SNI/ALPN without terminating TLS; ensure it is in the pinned build.
Do not repeat the outdated absolute claim that Nginx cannot handle CONNECT:
its documented tunnel module appeared in 1.31.0, but specifies HTTP/1.1 CONNECT.
That alone does not establish Naive HTTP/2 padding compatibility.

Traefik is also technically viable, including TLS passthrough and automatic
certificates. However, TCP routes precede HTTP routes on a shared entrypoint;
broad TCP catchalls can consume the intended site traffic. Its operational
advantage is service discovery, not inherently lower resource consumption.
It can use a file provider without Docker socket access. No memory/CPU ranking
is asserted without matched measurements.

Naive's own repository documents Caddy with the naive fork of forwardproxy,
not an arbitrary CONNECT plugin. Its HAProxy wiki exists, but the maintainer's
May 2026 comment says HAProxy is not officially tested/supported by them after
switching to Caddy. A linked issue describes historical H2 PADDED-DATA failures.
For HAProxy TLS termination, pin an appropriately patched version and test real
clients under sustained load. The L4 passthrough design bypasses that H2 parser.

## HTTPS and automatic validation

Keep one issuer: acme.sh with dns_internetbs and an explicit Let's Encrypt CA.
Do not assume the ACME client's default CA is Let's Encrypt. DNS-01 validates
control via TXT records, does not need inbound 80/443, and supports wildcard
certificates. This works during staging on 4443 and leaves existing Apache alone.

For a small VPS, prefer a persistent systemd timer invoking a short-lived pinned
ACME container over a permanently resident ACME cron container. Persist account,
renewal metadata and keys under var/acme/. Run checks on a jittered schedule,
honor CA/client renewal scheduling, and serialize issuance/delivery with project
changes. A failed delivery retries installation without forcing reissuance.

After renewal: validate SANs, chain, expiry and key match; install atomically;
reload affected Caddy/HAProxy/Hysteria consumers using their tested mechanisms;
verify the served certificate. Keep last-known-good material. An inactive stack
may renew material without being started; reconcile service delivery on startup.
Explicitly loaded certificates mean Caddy is not a second issuer. Avoid using
force reload unnecessarily; test stream behavior when a reload is necessary.
DNS API secrets remain in ACME state only, not the edge or admin processes.

Changing to Caddy/Traefik solely for built-in ACME is unnecessary: certificate
issuance is infrequent control-plane work and independent of data-plane routing.

## Small-server operating budget

- One HAProxy classifier and one Caddy process already required by Naive/site;
  avoid an additional Nginx/Traefik HTTP layer and per-protocol ingress proxies.
- One external TLS termination per own HTTP connection. Passthrough forwards
  ciphertext without decrypting/re-encrypting at the edge. L4 still has socket,
  buffer and kernel costs; it is not free and crypto still happens at the backend.
- Exact maps, bounded inspect buffers/deadlines, bounded connections and sensible
  idle timeouts. Do not sleep for the full inspection deadline after hello arrives.
- Preserve streaming for WS/XHTTP and HTTP/2; avoid full request/response buffering,
  payload logging and dynamic compression of already encrypted/compressed traffic.
- Precompress static site assets. Keep access logging minimal and rotate logs.
- Verify PROXY protocol on both ends before enabling it. A raw header inserted
  before REALITY/fakeTLS breaks a backend that does not expect it. Without it,
  backend per-client rate limiting may see only the proxy address.
- Size DNS cache and optional admin/watch tasks to actual RAM. Do not run both
  doctor watch and timer schedulers for the same checks. Disable unused services
  through explicit selected profiles, preserving owner feature choices.
- Build Rust/React/custom Caddy images elsewhere and deploy pinned images; a small
  VPS should not pay peak compiler RAM during installation. This deployment
  refinement needs publishing/build infrastructure; Docker build isolation alone
  does not reduce compilation resource requirements.
- Hysteria and tunneled encryption may dominate CPU before TCP classification
  does. Do not invent RSS/throughput numbers or assume maximum throughput.

## Measurement gate

Compare A: HAProxy L4 -> Caddy TLS and B: HAProxy TLS/HTTP -> plaintext backend,
with identical image versions, payloads, clients, network and feature set. Add
Nginx-stream -> Caddy only if there is an operational reason to migrate.
Run the load generator outside the VPS. Measure aggregate idle/peak RSS/PSS,
CPU per useful throughput, long-stream goodput, handshake/connect latency,
disconnects, and behavior during reload/renewal. Include Naive padding and
concurrent REALITY, MTProto and web connections. Test new handshakes separately
from established connections. Verify both families and unknown/default routes.

Choose the measured winner if resource pressure is material; the initial choice
is HAProxy L4 + Caddy because it simplifies the protocol path and uses existing
components. This does not assert that it must beat OpenSSL-backed HAProxy TLS.

## Primary references

- https://docs.haproxy.org/3.2/configuration.html
- https://nginx.org/en/docs/stream/ngx_stream_ssl_preread_module.html
- https://nginx.org/en/docs/http/ngx_http_tunnel_module.html
- https://doc.traefik.io/traefik/reference/routing-configuration/tcp/routing/rules-and-priority/
- https://doc.traefik.io/traefik/v3.0/routing/routers/
- https://doc.traefik.io/traefik/reference/routing-configuration/udp/routing/rules-priority/
- https://github.com/klzgrad/naiveproxy
- https://github.com/klzgrad/naiveproxy/wiki/HAProxy-Setup
- https://github.com/klzgrad/naiveproxy/issues/801
- https://github.com/9seconds/mtg/blob/master/example.config.toml
- https://xtls.github.io/en/config/transports/reality.html
- https://docs.amnezia.org/documentation/instructions/install-configure-protocols/
- https://v2.hysteria.network/docs/developers/Protocol/
- https://developer.getoutline.org/vpn/advanced/websockets/
- https://letsencrypt.org/docs/challenge-types/
- https://github.com/acmesh-official/acme.sh/blob/master/dnsapi/dns_internetbs.sh
- https://caddyserver.com/docs/automatic-https
- https://caddyserver.com/docs/caddyfile/directives/reverse_proxy
