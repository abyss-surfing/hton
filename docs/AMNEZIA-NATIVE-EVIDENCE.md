# Native Amnezia: domain, certificate and shared TCP/443

Research date: **2026-10-31**. Status: primary-source research,
**not a runtime check, qualification or authorization to modify a VPS**.
Installers were not run; only documentation and source code were examined.

## R1 conclusion

After this research, the owner accepted the native REALITY passthrough option below
(D80); a genuine certificate for the owner's domain is required for the ordinary
HTON HTTPS site. The finding below that no native VPN flow issues such a certificate
is a research result, not a new requirement to rework Amnezia REALITY. Exact
installed identity/SNI/target/restart qualification is still required.

[OR-4](OWNER-REQUIREMENTS.md) remains in force in full. The sources reviewed do
**not confirm a standard Amnezia VPN installation with a publicly issued certificate
for its own domain**. Native Xray REALITY and a regular HTTPS server certificate
have different semantics. The ADP-0 blocker in [foreign adapters](specs/sdd-foreign-adapters.md)
has therefore been made more specific, not removed. The basis and limits of this
conclusion follow; failure to find a flow does not prove that such a solution is
impossible in every future version.

## Pinned sources and applicability

- AmneziaVPN release tag **5.0.3.0**, commit
  [`de93650a90739b87bb47a632872ea9d0adc9412f`](https://github.com/amnezia-vpn/amnezia-client/tree/de93650a90739b87bb47a632872ea9d0adc9412f).
  References to the `release` below point to this source tree, not the installed VPS.
- The current dev branch was also examined, commit
  [`7f9b7db482b8b1662b7466937e578008cbcb716e`](https://github.com/amnezia-vpn/amnezia-client/tree/7f9b7db482b8b1662b7466937e578008cbcb716e).
  Dev is not represented as released or suitable for reinstalling an existing
  service.
- Official Amnezia Docs, Caddy Docs and the HAProxy manual were reviewed on
  2026-10-31; these web sources can change. The REALITY protocol contract is pinned
  to [`83f621788ca88b83179810b12baa7ae518aacb83`](https://github.com/XTLS/REALITY/blob/83f621788ca88b83179810b12baa7ae518aacb83/README.en.md).

## Supported, unsupported and unknown behavior

Here, `unsupported` means “the verified standard path does not satisfy the stated
contract,” not “the engine cannot possibly be configured manually.”

| Native component / scenario | Status for own domain + issued HTTPS certificate | Basis |
|---|---|---|
| Self-hosted Xray VLESS REALITY | **unsupported** as a normal publicly trusted VPN handshake certificate; SNI is supported | The official flow changes SNI and port; it does not issue a certificate: [protocol settings](https://docs.amnezia.org/documentation/instructions/install-configure-protocols/). The server configurator generates a REALITY private key, `dest`, `serverNames`, and `shortIds`: [release configurator](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/core/configurators/xrayConfigurator.cpp). |
| Native Xray TLS in dev | **unsupported** as a standard ACME flow | `ensureTlsCertificate` runs `openssl req -x509`, uses CN `amnezia-xray` with a 3650-day lifetime, then computes a fingerprint. This is a self-signed pinned certificate, not a public ACME certificate for the domain: [dev configurator](https://github.com/amnezia-vpn/amnezia-client/blob/7f9b7db482b8b1662b7466937e578008cbcb716e/client/core/configurators/xrayConfigurator.cpp#L412). |
| AmneziaWG / WireGuard | **unsupported** for HTTPS/SNI passthrough; a separate owner-authorized UDP endpoint is implemented by OR-66/D174 | Native launch publishes UDP, not TLS: [AWG](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/awg/run_container.sh), [WireGuard](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/wireguard/run_container.sh). Using a domain as a server address does not turn the transport into HTTPS. |
| OpenVPN | **unsupported** as a native public-domain HTTPS certificate flow | The native script creates an EasyRSA CA and signs `AmneziaReq`: [run](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/openvpn/run_container.sh); the server uses those CA/cert/key files: [configure](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/openvpn/configure_container.sh). |
| Historical IKEv2 | **unsupported** as a native ACME flow | The script creates its own IKEv2 CA and a certificate identified by `$SERVER_IP_ADDRESS`: [IPsec configure](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/ipsec/configure_container.sh). |
| Shadowsocks / OpenVPN over Cloak | **not confirmed** as an own-domain ACME flow | The official description distinguishes Shadowsocks AEAD from Cloak with a camouflage website; it does not describe issuing a certificate for the owner's domain: [protocol descriptions](https://docs.amnezia.org/documentation/protocols-info/). Support for a particular already-installed Cloak service cannot be inferred from the current protocol list. |
| **TProxy (Telegram WEB)** | **supported in source** for hostname + native Caddy HTTPS automation; **not a VPN** | The installer requires a hostname and ACME email: [controller](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/core/controllers/selfhosted/installController.cpp). The script creates a domain Caddyfile: [configure](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/tproxy/configure_container.sh). The service is a Telegram WEB proxy: [container metadata](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/core/utils/containers/containerUtils.cpp). |
| Actually installed Amnezia | **unknown** | Installed installer/client versions, containers, config and restart owners were not provided; upstream source is not an inventory of the VPS. |

**Important clarification to the negative result:** it would be incorrect to say
“Amnezia has no native ACME/domain flow at all”; TProxy has one. But TProxy's
Telegram WEB proxy must not be presented as Amnezia VPN, nor can its Caddy be moved
to Xray as an already supported configuration. TProxy runs `mtproto-proxy`,
`tproxy-server` and Caddy, and stores Caddy data under `/data/caddy`; see the
[native start script](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/tproxy/start.sh).
For a public DNS name, Caddy automatically obtains and renews a public certificate
when DNS, challenge access and persistent storage conditions are met. This is a
Caddy property, not evidence that a certificate was issued on this VPS:
[Caddy automatic HTTPS](https://caddyserver.com/docs/automatic-https).

## REALITY: why SNI and a “genuine certificate” are different

After successful authentication, a REALITY client expects a temporary trusted
certificate signed with the temporary authentication key. Receiving the genuine
certificate from the destination site means a different path (including fallback);
the client distinguishes these cases. REALITY is specifically designed to use a
third-party website without an owned domain or TLS server. This is stated directly
in the [pinned REALITY README](https://github.com/XTLS/REALITY/blob/83f621788ca88b83179810b12baa7ae518aacb83/README.en.md).

The official Amnezia page describes receiving a genuine external-site certificate
when an unauthenticated connection reaches port 443. It does not claim to issue a
certificate for the VPN owner's domain: [Amnezia XRay](https://docs.amnezia.org/documentation/xray/).
Research consequence: setting an owned SNI alone does not satisfy OR-4; an HTTPS
probe receiving a genuine certificate also does not prove a successful VPN
connection. This is a semantic inference from the two sources, not a test result.

## Shared TCP/443 and HAProxy

HAProxy can select a backend from the ClientHello SNI in TCP mode:
[official manual, use-server/SNI](https://www.haproxy.com/documentation/haproxy-configuration-manual/new//2-1r1/).
Engineering inference: REALITY may work with **byte-preserving TCP passthrough** if
SNI uniquely selects the intended REALITY backend. This is not Amnezia-specific
qualification. TLS termination at HAProxy is not passthrough and does not turn the
native REALITY handshake into one authenticated by a publicly issued certificate;
see the [REALITY protocol contract](https://github.com/XTLS/REALITY/blob/83f621788ca88b83179810b12baa7ae518aacb83/README.en.md).

The plan has the following mandatory future compatibility conditions:

- Different backends with the same SNI cannot be distinguished by SNI **alone**.
  Using the owned website as the REALITY target through the same frontend without
  a separate route risks a loop. This is inferred from generated `dest=site:443`
  in the [release configurator](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/core/configurators/xrayConfigurator.cpp)
  and SNI routing; no ready-made target design is claimed here.
- Native Xray publishes the selected port for both TCP and UDP:
  [run script](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/xray/run_container.sh).
  The official instructions require a free port for both transports:
  [settings](https://docs.amnezia.org/documentation/instructions/install-configure-protocols/).
  This is a separate conflict from HAProxy's ownership of TCP/443.
- TProxy passthrough to Caddy is a possible design candidate, but HTTP-01 requires
  public port 80; TLS-ALPN-01 requires public port 443 with the correct ALPN.
  Changing an internal port does not move the public challenge endpoint:
  [Caddy challenges](https://caddyserver.com/docs/automatic-https#acme-challenges).
  Native TProxy publishes host ports to container ports 80/443 and stores `/data`
  in `amnezia-tproxy-data`: [run](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/tproxy/run_container.sh).
  No HTTP reverse proxy in front of REALITY or SSL termination is assumed.

## Containers and restart owners

Native names include `amnezia-awg`, `amnezia-awg2`, `amnezia-xray`, and
`amnezia-dns`; Cloak uses the special name `amnezia-openvpn-cloak`:
[name mapping](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/core/utils/containers/containerUtils.cpp).
These are possible names, not resources discovered on this VPS.

The reviewed native run scripts set Docker `--restart always` for
[Xray](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/xray/run_container.sh),
[AWG](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/awg/run_container.sh),
[legacy AWG](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/awg_legacy/run_container.sh),
[DNS](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/dns/run_container.sh)
and [TProxy](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/tproxy/run_container.sh).
The Amnezia client also explicitly runs `docker restart` after writing Xray config:
[configurator](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/core/configurators/xrayConfigurator.cpp).
Therefore whole-server down must account for both Docker policy and external client
management; whether this VPS has additional systemd/watchers remains unknown.
Discovery grants no authority to change these resources.

The version matters for data preservation: dev adds volume
`amnezia-xray-data:/opt/amnezia/xray`, absent from the reviewed release run:
[dev run](https://github.com/amnezia-vpn/amnezia-client/blob/7f9b7db482b8b1662b7466937e578008cbcb716e/client/server_scripts/xray/run_container.sh).
Do not design recreation/remap from a universal assumption about volumes.

## Facts needed before implementing the integration

This is a list of missing facts, **not an instruction to perform remote actions**:

1. Exact Amnezia app/installer version that created and last modified the server;
   selected self-hosted protocol; confirmation it is not a Premium/Free endpoint.
   The official FAQ distinguishes Premium and Self-hosted:
   [FAQ](https://docs.amnezia.org/faq/).
2. Read-only inventory of actual container IDs/image digests, engine version,
   published/bound ports, networks/mounts, restart policy, related systemd/watchers
   and current state. Exclude secrets from the report.
3. Only the required non-secret transport fields: listen/port, security,
   SNI/serverNames, dest/target, certificate issuer/SAN/validity/fingerprint,
   native renewal owner and challenge routing. Compare these with the pinned
   installer.
4. Later, after full implementation and the unit phase, functionally qualify the
   selected candidate as required by the [orchestrator](IMPLEMENTATION-ORCHESTRATOR.md):
   native install/renewal, unchanged clients before/after remap, cold boot/down/
   restart, rollback and drift.
   This document does not substitute for those checks.

## Limited local-lab observation (2026-11-02)

Read-only `docker inspect` in the local Docker environment confirmed that the
`amnezia-xray` container was running at the time; its local image ID was
`sha256:1ad75e45b341b11c53750910727a5d0666425872cd61e35f84ff9cb95b3e3b4e`, its
Docker restart policy was `always`, and it published TCP and UDP host ports
`0.0.0.0:443` and `[::]:443`. This is the local host, not a separate production VPS;
the container was not restarted, stopped or changed.

These facts confirm that an existing bind on 443 conflicts with placing HAProxy on
the same port and that Docker owns one restart policy. They do not identify the
Amnezia installer/client or Xray version, prove all restart owners, confirm SNI/
target or REALITY config, establish client compatibility, or prove that moving the
service to a private port is safe. The image ID is not a verified registry manifest
digest. The local HTTPS placeholder used a separate self-signed `hton.local`
certificate; it is not a public-domain certificate and does not confirm ACME/renewal.
In a limited trial, HAProxy on loopback `127.0.0.1:4443` forwarded a test connection
with SNI `www.googletagmanager.com` to the existing upstream and received its
fallback certificate. This checks only TCP/SNI forwarding; no VPN client handshake
or Amnezia remap was performed. Full lab notes are in [implementation status](IMPLEMENTATION-STATUS.md).

## Owner-accepted choice if installed native VPN matches the research

**The owner accepted the agent's option with a “+” (D80):** keep Amnezia independent
and allow its native REALITY through transparent TCP/443 passthrough with the
explicitly described temporary REALITY certificate semantics; a genuine certificate
for the owner's domain is required for HTON's ordinary HTTPS site. This is an
**explicitly accepted change to OR-4 scope for Amnezia VPN**, recorded in
OWNER-REQUIREMENTS.md and D80. It does not expand protocol support or substitute for
qualifying the installed stack. AmneziaWG remains a separate UDP constraint if it
is selected; OR-66/D174 implements it as its own public UDP endpoint, not as
HTTPS/SNI passthrough.

Alternative: retain the literal requirement for a public certificate on the VPN's
own domain and separately approve a new design: a natively supported ordinary-TLS
VPN backend, its single external certificate/renewal owner, verified Amnezia client
support and shared-443 routing. No native Xray ACME flow was found; do not promise
it as a ready reinstall or silently add an HTON certificate manager.

The certificate-scope choice was accepted by the owner under D80. Until exact
installed facts are available, the specific R1 Amnezia integration remains blocked.
This research does not replace other requirements or architecture.

## Pinned Xray config path (2026-11-02)

For the researched upstream commit `de93650a90739b87bb47a632872ea9d0adc9412f`,
the Xray server JSON path is confirmed in source: `protocolConstants.h` declares
`amnezia::protocols::xray::serverConfigPath = /opt/amnezia/xray/server.json`, and
`xrayConfigurator.cpp` uses this constant to read and write the container config
([constant](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/core/utils/constants/protocolConstants.h#L894-L905),
[reader](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/core/configurators/xrayConfigurator.cpp#L2162-L2177)).
This confirms the path for that upstream checkout, but not the version of the
installed client/container or whether the file is a regular non-symlink file with
no separate mount. A future observer must bind the read to the exact immutable ID,
require two matching structural fingerprints, verify file type/path and enforce a
strict size limit. The JSON contains secrets, including the REALITY private key and
client IDs: raw bytes may exist only in protected memory while parsing; returned
evidence, Debug output and logs contain only bounded non-secret fields, and secrets
must not be serialized.

## Own-SNI migration applied to the native listener (2026-11-05)

OR-35/D133 reverses the earlier "never edit native" engineering restriction for
one explicitly consented operation. The native REALITY JSON keeps its private
key, short IDs, flow, clients and listener; only `serverNames` and `dest` change.
`dest` must be an internal address reachable from the bridged native container
(the `mtg` chain listener or the site TLS backend on the Docker bridge gateway),
never public `example.com:443`. The pinned Xray accepts the rewritten JSON with
`run -test` in a read-only, networkless container; the rendered HAProxy
configuration routes the own SNI to native with the site backend as a
health-checked fallback. Existing clients remain valid while the legacy name is
retained; removing that name is a later explicit step after reissue. No live
native mutation was performed for this note.
