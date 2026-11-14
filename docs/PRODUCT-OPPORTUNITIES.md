# Product opportunities for HTON

Research snapshot: 2026-10-31. Question: which user-facing capabilities and
ready-made technologies are worth considering for HTON, beyond naming more VPN
protocols? This is a product assessment, not an owner decision or a claim that
the features already exist. Existing owner requirements in
[OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md) remain authoritative.

## Recommendation

HTON's strongest opportunity is to make a small number of engines feel like a
coherent service: reliable setup, clear device onboarding, safe module changes,
useful routing, and recovery when the network or server changes. Adding every
possible protocol would make a weak VPS harder to operate without necessarily
helping users. Prefer optional adapters around upstream engines and let users
choose a profile by device and use case.

## Candidates worth considering

| Priority | Capability | User value | Suggested treatment |
|---|---|---|---|
| P1 | Ready-made device profiles and share/export | A user chooses Android TV, phone, router, Quest, or Apple TV and gets a tested client, compatible protocol/transport, import instructions, and a device-scoped credential. | Make this the main product surface. Qualify exact device, OS, client and version combinations; distinguish direct-client support from gateway support. |
| P1 | Safe routing profiles | Let a device or credential choose direct Internet, the HTON server's normal egress, or an already-configured external VPN uplink. | Use native Xray routing/outbounds where suitable; do not manage the external VPN client's configuration. Present route and failure behavior before applying. |
| P1 | Connectivity diagnosis and recovery guidance | Explain whether failure is DNS, certificate, listener, server health, client import, or upstream route, and give a bounded repair action. | Start with local and server-side checks and evidence; avoid automatic changes to routes or foreign services. Xray exposes metrics and connection-observatory data that can inform status. |
| P1 | Safe update and rollback | Upgrade an engine or module with a known last-good configuration and a recovery path. | Pin image/binary versions and checksums, offer rollback, and make updates explicit. `up` must retain the owner's no-implicit-update semantics; the private `.secrets` repository is the durable copy, not a separate backup archive. |
| P2 | Optional private overlay for server/operator access | Reach admin/SSH and private services without exposing their management ports publicly; connect subnets containing devices that cannot run a client. | Evaluate Headscale/Tailscale-class overlay as a separately consented optional module, after resolving the existing policy restriction. Separate private subnet routing from public Internet exit-node behavior. |
| P2 | Optional DNS policy and encrypted DNS | Give users a predictable resolver, local names, and optional domain-based routing/filtering. | Reuse Unbound for the current private-DNS role; qualify one additional DNS product only for a concrete need. sing-box already supports DNS rules, caching and strategies if it is selected for gateway/client duties. |
| P2 | Site publication and private app access | Publish a user's site or reach a private service through an authenticated reverse tunnel without opening its origin directly. | Existing roadmap already plans publication. Qualify a narrow tunnel such as rathole or an existing reverse SSH mode; keep public website and admin destinations separate. |
| P3 | Compatibility protocol pack | Support a real client/device constraint that the baseline cannot serve (for example OpenVPN/OpenConnect/SSTP). | Add only after a named user scenario and client test. Treat TCP/443 compatibility as a separate endpoint/path problem; do not promise every protocol can share HTTPS ingress. |
| P3 | Multi-server / regional profiles | Let users select a nearby or alternate server and route through a chain. | First support a manually configured external uplink. Consider orchestration/failover across HTON servers only after a separate decision on coordination, health policy and failure semantics. |

## Ready-made functions HTON should expose before inventing new ones

Xray already includes routing rules, outbounds, balancing, background outbound
observability, metrics and an API. These can support route profiles and status
without building a new proxy engine. Keep its API private to the host/controller;
do not expose it as the public product API. [Xray configuration reference](https://xtls.github.io/en/config/)
and [metrics reference](https://xtls.github.io/en/config/metrics.html).

sing-box provides configurable DNS servers and rules, cache behavior, and DNS
strategies such as IPv4 preference or IPv4-only. This is most useful if HTON
chooses it for a gateway/client role; adding sing-box as another always-on server
merely for DNS would duplicate responsibilities. [sing-box DNS configuration](https://sing-box.sagernet.org/configuration/dns/).

Docker Compose already supports profiles, per-service health checks, startup
ordering based on health, and file-mounted secrets granted only to selected
services. HTON can use these primitives for optional modules and dependency
readiness, while retaining its own persistent desired state and transaction
logic. [Compose profiles](https://docs.docker.com/compose/how-tos/profiles/),
[startup order and health checks](https://docs.docker.com/compose/how-tos/startup-order/),
[Compose secrets](https://docs.docker.com/compose/how-tos/use-secrets/).

Tailscale documents two distinct capabilities often conflated as “VPN”: exit
nodes route public Internet traffic through a selected device, while subnet
routers make private subnets reachable to overlay clients. It also separates
route approval from access policy. This is a useful product model if an overlay
module is reconsidered, but it is not an automatic recommendation to deploy
Tailscale in HTON. [Subnet routers](https://tailscale.com/docs/features/subnet-routers),
[exit nodes](https://tailscale.com/docs/features/exit-nodes),
[access control](https://tailscale.com/docs/features/access-control).

## Strong product ideas found in adjacent projects

Owner clarification: OpenWrt client installation and health checks are already
planned/existing, and device setup should be incremental. Autonomous self-healing
is not a missing feature; see [OWNER-REQUIREMENTS.md](OWNER-REQUIREMENTS.md).
The earlier general device-profile and “AI repair” suggestions should therefore
not be treated as newly discovered HTON gaps.

The owner has separately requested a local PowerShell/Bash tester that compares
available profiles and helps choose the best speed/reliability tradeoff, with
possible later Android/iOS support. This is now an owner requirement, not merely
a suggestion. The design below is an agent recommendation for how to make its
result credible.

### Local protocol/profile selector (owner-requested)

Build a small client-side command which takes HTON-issued test profiles, checks
each from the current network, and reports “reachable,” “working through the
tunnel,” latency, short-transfer throughput, loss/timeout rate, and variance.
Rank candidates using configurable priorities; call the result “best observed
here now,” not universally fastest or reliable. Run a few short rounds, randomize
candidate order, cap bytes/time, and report when results are too close to call.

Separate the test in stages so a cheap failure does not trigger a full benchmark:

1. Check prerequisites and local clock/DNS, then perform a protocol handshake.
   A TCP port-open or TLS handshake alone only proves reachability, not a working
   VPN tunnel.
2. Send a small authenticated request through the candidate tunnel to a
   controlled HTON endpoint and verify the observed egress/response. This proves
   the data path works, unlike a server-side health check.
3. Measure latency and a bounded download/upload through the tunnel; repeat to
   estimate stability. Use an HTON-controlled endpoint so unrelated public
   speed-test servers do not dominate or receive user traffic.
4. Print the winning profile plus the exact compatible client/import action.
   Keep fallback profiles available; do not silently discard credentials for
   other candidates.

The result depends on the actual client engine and mode: testing VLESS with the
Xray core does not necessarily predict performance of a different GUI wrapper or
device gateway. First implement a portable CLI for Windows PowerShell and common
Linux/macOS shells. Reuse the selected Xray client/core where practical; its
upstream CLI supports configuration validation with `-test`, which can catch a
bad profile before any network probe. Android/iOS should later get a native
runner or an app integration using the same test contract, rather than trying to
run shell scripts on mobile.
[Xray CLI validation](https://xtls.github.io/en/document/command.html).

For throughput, iperf3 can emit structured JSON and bound test duration, but it
measures the path to the iperf server; it does not by itself prove that a VPN
profile works. Use it only as a controlled optional benchmark after tunnel
verification, or provide an equivalent small HTTPS test endpoint. [iperf3
JSON and duration options](https://software.es.net/iperf/invoking.html).

Avoid two tempting shortcuts: picking a client from a raw TCP connect or ping,
and doing one large speed test per profile. The first can rank a protocol that
never passes application traffic; the second is slow, noisy, consumes user data,
and can distort later candidates. Store no long-lived user traffic history by
default; return local results that the user can share explicitly for diagnosis.

### A real single-port fallback design may be more valuable than another protocol

Xray's VLESS/Trojan fallback feature can route a connection to another service
based on TLS SNI, negotiated ALPN, and (in supported cases) the first HTTP path;
it can also forward invalid or unauthenticated TLS clients to a fallback target.
This is a concrete upstream mechanism relevant to HTON's TCP/443 goal: one edge
can serve ordinary HTTPS and selected VPN paths while keeping protocol behavior
explicit. It is not a universal demultiplexer: the documented fallback is limited
to TCP+TLS, HTTP/2 path peeking is not supported, and the SDD already warns that
missing ALPN is not reliable protocol identity. Consider whether the baseline
can use Xray fallback directly to simplify HAProxy/Caddy routing, but compare
against the currently designed proxy chain with real clients before changing it.
[Xray fallback documentation](https://github.com/XTLS/Xray-docs-next/blob/main/docs/en/config/features/fallback.md).

### Make connection strategies visible and switchable

Amnezia's first-party AWG client ecosystem and Xray/REALITY point to a practical
resilience pattern: offer one fast UDP profile and one TCP/TLS-like fallback
profile where the user's network blocks UDP. This is more useful than claiming
one protocol works everywhere. HTON already plans multiple compatible engines;
the missing product behavior to evaluate is a clearly labeled per-device
“primary / backup” pair, with independent revocation and a dead-simple export
of both. Do not silently auto-switch on the server or promise censorship
resistance. REALITY's upstream docs explain its fallback behavior and protocol
configuration, while Xray docs separately specify transport limitations.
[Xray REALITY configuration](https://github.com/XTLS/REALITY/blob/main/README.en.md),
[Xray fallbacks](https://github.com/XTLS/Xray-docs-next/blob/main/docs/en/config/features/fallback.md).

### Client-generated configuration is a strategic constraint

The installable OWRT client means HTON should treat router capability as a first
class client target, but preserve the existing incremental plan. The high-value
question is not “install on every device at once”; it is whether one router
install can serve devices that cannot run VPN software and selectively route an
SSID/device while keeping LAN traffic local. This is already reflected in
router/gateway specs, so it is a validation and packaging priority, not a new
module proposal. The project should avoid building its own firmware UI if an
existing OpenWrt package can consume HTON-issued profiles.

### Treat transport options as a fallback matrix, not a protocol checklist

For each supported client profile, document what it does when UDP is blocked,
TCP/443 is occupied, SNI is filtered, DNS fails, or the server IP is blocked.
Possible responses are alternate transport, alternate endpoint, or manual
recovery instructions. They are not interchangeable: UDP protocols cannot be
magically carried through a TCP HTTPS listener, and a second endpoint requires
DNS/port/firewall planning. This makes the actual service promise legible and
turns the existing module catalogue into a usable connectivity plan.

### Manage access to resources, not just VPN accounts

Pangolin is the most interesting adjacent product to study. Its model is
“site -> resource -> grant”: users are granted access to a named web app, host,
network range, SSH target, or desktop, and a connector establishes an outbound
tunnel from a private network. Private resources are not exposed publicly, and
the system can select among sites that reach the same resource. This gives HTON
a clearer long-term product direction than a page of protocol toggles: “give
this device access to my home network / server / app.”
[Pangolin architecture](https://docs.pangolin.net/about/how-pangolin-works),
[resource types](https://docs.pangolin.net/manage/resources/understanding-resources).

This is a **product reference, not a recommendation to install Pangolin inside
HTON**. Pangolin brings its own identity, policy, proxy and tunnel control plane;
its published self-host networking requirements include additional UDP listeners
and its own networking layout. Those overlap with HTON's single-owner config,
TCP/443 edge, and existing policy limits. The useful idea is the resource/grant
experience. A small HTON implementation could expose a bounded catalog of
resources through the planned API and publication module while retaining HTON's
own lifecycle and approvals. [Pangolin networking requirements](https://docs.pangolin.net/self-host/dns-and-networking).

### Make an “AI repair” a safe plan, not a shell with root access

The compelling “hack” here is a guided operator loop: HTON collects local,
redacted evidence; identifies likely causes; proposes a typed plan; shows exactly
which owned module/listener/config will change; and runs it through the existing
validation, consent, journal, apply and rollback path. The LLM can explain and
rank evidence, but it should not receive arbitrary shell, Docker socket, or secret
access. The owner already requires module checkers and shared bounded operations,
so this could later add a conversational surface without inventing a second
control plane. Keep it local/optional and require confirmation for host privileges,
foreign resources and route changes.

### Private service access can be more useful than another VPN protocol

Pangolin's site connector approach illustrates a common gap: reaching a NAS,
printer, home automation server or private web app behind NAT, without installing
a VPN client on every target. A narrowly scoped outbound connector or reverse
tunnel could address that. HTON already has future publication and reverse-agent
work; the product opportunity is to turn it into a simple “publish privately to
these users/devices” flow, including SSH/RDP if an existing safe browser gateway
is selected. Do not promise raw layer-3 subnet access from a TCP-only tunnel.

### Start optional modules on demand only with a separate wake-up path

Compose profiles provide a clean way to model optional services, but the user's
request to idle-stop web administration has a special bootstrapping issue: a
stopped web server cannot wake itself. A small host-owned wake endpoint or socket
can start the admin module, wait for its health check, then proxy the request;
timeouts and failed starts must leave site and VPN ingress untouched. This
matches OR-7, which permits exploration but leaves trigger, activity and pending
operation semantics open. Docker Compose supports profiles and health-based
readiness, which are useful primitives, not the wake mechanism itself.
[Compose profiles](https://docs.docker.com/compose/how-tos/profiles/),
[health-based startup](https://docs.docker.com/compose/how-tos/startup-order/).

### Do not confuse a control plane with an exit node

Headscale is an attractive self-hosted coordination/control plane for clients,
and its embedded DERP can relay traffic when direct peer connections fail. But
DERP needs extra UDP/STUN reachability, and disabling public DERP alternatives
can make the single self-hosted relay a single point of failure. Its policy docs
also warn that an absent ACL/grants policy defaults to allow-all. If the owner
ever lifts the existing overlay deployment block, first ship an isolated lab
profile with explicit deny-by-default policy and relay failure tests; do not
silently equate installing Headscale with providing a complete Internet VPN.
[Headscale DERP](https://headscale.net/stable/ref/derp/),
[Headscale policy](https://headscale.net/stable/ref/policy/).

## Suggested product shape

The user should not need to understand protocol names first. Ask what devices
they have and what they need to do; offer a tested profile, then explain any
limitations such as UDP, local-network access, or a separate port. Under the
profile, HTON composes optional modules and dependencies, shows resource and
listener effects, previews changes, and applies them through one shared path.
Status should say what was actually checked and distinguish “working,”
“degraded,” and “not verified.”

The first useful product release should favor a reliable IPv4 TCP/443 baseline,
ordinary website, certificates, Core-driven CLI, device credential export,
health diagnosis, explicit updates and restore. These recommendations follow
the owner's R1/R2 boundaries: no web admin in R1, no traffic accounting through
R2, and IPv6 remains long-term work. They do not change those requirements.

## Keep out of scope unless a concrete need appears

- More protocol names without a tested device/use-case gap.
- A second all-in-one panel or config writer alongside HTON Core.
- Mandatory telemetry, traffic accounting in R1/R2, or a cloud account required
  for basic server operation.
- Automatic cluster creation or automatic configuration of the separately
  managed VPN uplink.
- Public bare WireGuard/IPsec/PPTP/plain SOCKS or deployment of Tailscale while
  the owner's existing policy restriction remains unresolved.

## Sources and evidence limits

Sources above are first-party project documentation. They establish that these
features exist in upstream software; they do not establish security, compatibility
with HTON's chosen versions, resource cost, or fit on a particular device. Every
candidate still needs pinned-version review, license review, interoperability
tests and RAM/CPU measurements before becoming a supported module. The current
capability catalogue and reuse shortlist are in
[sdd-capabilities.md](specs/sdd-capabilities.md) and
[REUSE-ASSESSMENT.md](REUSE-ASSESSMENT.md).
