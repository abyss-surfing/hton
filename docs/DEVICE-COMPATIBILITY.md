# TV and Quest/Oculus connectivity assessment

Date: 2026-10-31. Status: research incorporated into the owner-requested optional toolbox backlog
(ROADMAP.md T14–T22 and specs/sdd-capabilities.md). No protocol is implemented or
enabled by this document; deployment qualification and policy blocks still apply. TV OS/model, headset version and router hardware are pending.
Preserve the existing rejection of new public IPsec/WireGuard-class endpoints
except for the explicitly owner-authorized AmneziaWG2 module (OR-66/D174);
discussing alternatives does not cancel owner decisions.

## Main finding

The limitation is primarily client placement, not absence of server protocols.
A device that cannot install a tunnel client generally does not gain one when
the VPS adds another protocol. Prefer a local gateway using existing VLESS;
the VPS does not need another listener, daemon or certificate for that topology.
All added devices still consume aggregate bandwidth and server encryption CPU.

```text
TV / Quest -- ordinary Wi-Fi or Ethernet --> local gateway
              -> existing VLESS over HTTPS -> hton HAProxy -> Caddy -> Xray
```

The gateway can be a supported OpenWrt router or a separate Linux machine with
appropriate network interfaces. If the current router cannot run the software,
a dedicated gateway/AP avoids replacing it. An SSID alone does not implement a
tunnel: its forwarding and DNS/IPv6 policies must actually pass through the client.
Stock GL.iNet/OpenWrt-derived firmware does not automatically include a tested
VLESS client; check firmware, packages, CPU, RAM and Wi-Fi capabilities first.

## Device matrix

| Device | Practical first path | Qualification |
|---|---|---|
| Android TV / Google TV | Existing-protocol client or gateway | Amnezia FAQ lists Android 9+ including Android TV; test specific protocol, import, remote UI and background persistence |
| Android TV with awkward mobile UI | FlClash or Hiddify candidate | Official project releases only; TV improvements exist but exact firmware/remote/import behavior needs tests |
| Samsung Tizen / LG webOS | Gateway | Cannot assume Android APKs or VpnService are available; vendor-approved clients vary by model/OS. Rooting TV is not baseline |
| Apple TV | sing-box Apple client or gateway | Official sing-box documentation lists tvOS 17+; App Store/account region matters |
| Meta Quest / Oculus | Gateway first | Meta documents Android APK installation through developer mode/ADB, not compatibility of every Android VPN app |
| Quest direct-client experiment | Known Android client APK, only on the actual device | Must verify VPN permission, system-wide traffic, sleep/reboot and game behavior; sideload success alone is insufficient |

Meta's Horizon app distribution policy separately restricts VPN apps without
prior approval. That is not proof that personal sideloading cannot work, nor a
guarantee that it will. Avoid promising an officially supported Quest VPN app.
The v2rayNG maintainer states that the mobile app is not designed for TV; current
TV remote-control regressions make it a test candidate rather than a universal
recommendation. Commercial Proton apps illustrate platform support, but are not
generic hton configuration importers and do not connect to our VPS by default.

## Candidate server additions

| Candidate | Integration / resource effect | Decision for this setup |
|---|---|---|
| No new protocol; router VLESS | Reuse current ingress and Xray; client work moves to LAN gateway | First choice for TV and Quest |
| Trojan over WS/TLS | Optional inbound in existing Xray, Caddy WS path, same 443/certificate; explicit client transport match | Reasonable compatibility option only for a concrete client need |
| Native Trojan TLS | Dedicated exact SNI passthrough to Xray TLS; backend receives LE cert through existing deployment pipeline | Alternative when client lacks WS; no new daemon required, but another cert consumer |
| Outline/Shadowsocks over WSS | Caddy WS paths to compatible Outline server, same 443 and LE pipeline | Conditional; client/server migration required; retains inner SS encryption in addition to TLS |
| Ordinary authenticated HTTPS proxy | Existing Caddy capability only if explicitly supported/configured/tested | Apps that honor a proxy only; not full-device TCP/UDP VPN, no open proxy |
| Tailscale exit node | Additional daemon/overlay, account/control plane and device client; direct/relay paths need testing | Conditional Apple/Android TV convenience, not default censorship transport; requires revisiting owner WG-class policy |
| OpenVPN / SSTP / SoftEther / ocserv | Extra daemon, routes, privileges/certificate management; TLS wrapping does not make all of these HTTP-path routable | Only when a specific usable client requires it; no evidence it fixes unknown TV/Quest |
| TUIC | Additional QUIC implementation/listener competing with Hysteria role | Planned experimental module; qualify benefit against existing Hysteria/VLESS |
| New public IPsec/WireGuard, PPTP or plain SOCKS | New public protocol surface | Only the owner-authorized AmneziaWG2 UDP endpoint (OR-66); other new WireGuard-class endpoints remain rejected |

Trojan and Outline WSS do not provide built-in TV/Quest support on their own.
Adding a path on 443 is less infrastructure than another daemon, but still adds
configuration/authentication/upgrade surface. No claim of zero incremental cost.
Do not add every protocol simply because Xray/sing-box exposes it.

## VR, UDP and LAN correctness

- Verify game/voice UDP through the selected gateway client and server. VLESS
  can carry UDP using compatible XUDP; a working HTTP page does not prove this.
- HTTP CONNECT or SSH -D alone is not an arbitrary UDP tunnel. Existing SSH
  recipes cannot be assumed to cover Quest voice/games without another mechanism.
- Keep local PCVR/Air Link/Virtual Desktop, casting, discovery and NAS traffic
  on the LAN. Preserve multicast discovery and avoid guest/client isolation
  between the headset and local PC when these features are required.
- Test IPv4, IPv6, DNS and MTU together. A direct IPv6 default can bypass the
  intended VPN path even when IPv4 appears correct. Keep management and endpoint
  bootstrap traffic out of a routing loop.
- Start with a dedicated test SSID/device policy; leave unrelated household
  devices unchanged. Respect the owner's direct-fallback choice and visibly
  report when internet egress is direct. SSH remains session-only under D28.
- Compare existing VLESS versus planned Hysteria for actual ISP loss, latency,
  jitter and CPU. QUIC can help some UDP workloads but does not guarantee better
  latency, concealment or throughput; UDP blocking is an independent failure.
- The VPS's physical location constrains internet VR latency regardless of
  tunnel choice. Local PCVR streams should not make a round trip through the VPS.

## Acceptance before expanding server scope

Record TV OS/model/version, Quest model/Horizon version, router model/firmware,
available LAN gateway hardware, and whether use is video, downloads, multiplayer
or local PCVR. Test an existing VLESS client/gateway first. Verify remote UI,
profile import without public credentials, reboot/sleep, UDP voice/game session,
LAN discovery, DNS/IPv6, fallback and aggregate VPS/gateway resource usage.

Only then add a protocol profile with demonstrated interoperability benefit.
Reuse DNS-01 and the existing LE delivery path; never disable TLS verification
for compatibility. Use per-device credentials and support revocation. No public
plaintext proxy and no new device-specific control panel on the VPS.

## Primary sources

- [Amnezia platform FAQ](https://docs.amnezia.org/faq/)
- [FlClash releases and TV changes](https://github.com/chen08209/FlClash/releases)
- [Hiddify platform/install guide](https://github.com/hiddify/hiddify.com/blob/main/docs/app/How-to-install-Hiddify-app.md)
- [v2rayNG maintainer on TV support](https://github.com/2dust/v2rayNG/issues/5916)
- [sing-box Apple platforms](https://sing-box.sagernet.org/clients/apple/)
- [Meta ADB and APK installation](https://developers.meta.com/vr/documentation/native/android/ts-adb/)
- [Meta app policies](https://developers.meta.com/vr/policy/app-policies/)
- [sing-box TUN and OpenWrt integration](https://sing-box.sagernet.org/configuration/inbound/tun/)
- [VLESS UDP/XUDP](https://sing-box.sagernet.org/configuration/outbound/vless/)
- [Xray Trojan inbound](https://xtls.github.io/en/config/inbounds/trojan.html)
- [Xray transports](https://xtls.github.io/en/config/transport.html)
- [Outline WSS](https://developer.getoutline.org/vpn/advanced/websockets/)
- [Tailscale Android/TV](https://tailscale.com/docs/install/android)
- [Tailscale Apple TV](https://tailscale.com/docs/install/appletv)
- [Tailscale exit nodes](https://tailscale.com/docs/features/exit-nodes)
- [SoftEther specifications](https://www.softether.org/3-spec)
- [ocserv TLS/DTLS](https://ocserv.openconnect-vpn.net/features.html)
- [OpenVPN TCP/UDP tradeoffs](https://openvpn.net/as-docs/v3/tutorials/tutorial--vpn-performance-troubleshooting/)
- [TUIC specification](https://github.com/tuic-protocol/tuic/blob/master/SPEC.md)
