# SDD capabilities — optional protocols and device onboarding

## Purpose and authority

Owner request, 2026-10-31: add the assessed options to the plans and make hton a
polished integrated toolbox. This accepts planning the catalogue below; it does
not claim implementation, certify third-party clients, authorize deployment or
silently reverse previously rejected public protocols. Weak-VPS efficiency,
Bash-first, bind-mounted state, DNS-01 and consent boundaries remain in force.

## Requirements

- CAP-1 One versioned declarative capability catalogue owns each feature's ID,
  implementation service, dependencies, conflicts, listeners, routes, certificate
  consumers, state, credentials, required privileges, export adapters and checks.
  No user-supplied executable plugin hooks. Registry validation is shared by CLI,
  installer, Compose rendering, diagnostics and admin; do not duplicate lists.
- CAP-2 A protocol is not necessarily a service: VLESS/Trojan share Xray; site,
  Naive and WSS routing share Caddy. Prefer existing engines when compatible.
  New daemons are optional, with measured aggregate resource cost.
- CAP-3 Optional modules default off; mandatory core is not a feature toggle (sdd-config.md). Planning/install profiles suggest selections;
  neither choosing a profile nor installing packages enables every module. Plan
  shows public listeners, privileged host changes, dependencies and resource
  evidence before apply. An unmeasured feature is labeled unmeasured, not cheap.
- CAP-4 Every enabled listener is explicit and unique by address/protocol/port.
  Separate TCP/UDP and exact SNI ownership; reserve raw routes before wildcards.
  Reject collisions and unsupported routing instead of silently picking winners.
- CAP-5 TLS consumers use the same Let's Encrypt DNS-01 certificate inventory
  and atomic delivery pipeline. No second issuer or skip-verification recipes.
  Protocols without ordinary TLS do not acquire HTTPS properties from their port.
- CAP-6 Device onboarding separates device/OS, client, protocol and transport.
  Offer only versioned tested combinations as supported; the rest are labeled
  experimental or blocked with a concrete reason. Model/version may remain
  unknown during planning, but acceptance requires actual device evidence.
- CAP-7 Per-device credential identity, explicit share/QR/config export and
  revocation are part of each module. Protect exports; redact logs and diffs.
  Release-owner subscriptions may be explicitly selected. R2 single-disclosure tokens cannot have a reusable endpoint that reconstructs delivered secrets; separately scoped metadata/update channels must not bypass sdd-identity.md.
- CAP-8 No remote third-party subscription/bot is required to provision our clients.
  Document official project download sources, version and update verification;
  open source is not proof of a security audit or firmware compatibility.
- CAP-9 Disable/uninstall removes only owned routes/listeners/privileges and
  schedules; preserve unrelated modules and foreign resources. Journal changes
  and test rollback and upgrades with module dependencies intact.

## Planned capability catalogue

All entries are planned, not implemented or enabled.

## Module development standard

Every module implementation follows the owner's required SDD, DRY, SOLID and
GRASP canons. Its SDD records responsibility and interface, dependencies and
shared-service ownership, configuration schema and semantic checker, privileges
and resources, logs/health signals, lifecycle effects, failure behavior, and
acceptance evidence. Keep orchestration and domain decisions in their proper
owners; reuse shared mechanisms instead of copying config parsing, planning,
secret handling, apply/recovery, or logging logic into each module. A module is
not complete when only its manifest exists: implementation and SDD must agree.

| Capability ID | Implementation and ingress | Delivery / qualification |
|---|---|---|
| vless | Existing Xray; Caddy HTTPS WS/XHTTP | Baseline, transport-specific exports |
| naive | Caddy with Naive forwardproxy; own HTTPS | Existing plan, real H2 client tests |
| mtproto | mtg or existing telemt; exact SNI raw passthrough | Existing plan, protect existing users |
| hysteria | Hysteria2, explicit UDP endpoint | Existing plan; certify game/voice UDP and resource cost |
| amneziawg | AmneziaWG 3.1 userspace, explicit public UDP endpoint | OR-66/D174; per-user `.conf`, D6ton interoperability check |
| trojan-ws | Existing Xray inbound; Caddy HTTPS WS route | New optional compatibility module |
| trojan-tls | Existing Xray TLS inbound; exact SNI passthrough | Native-Trojan clients; additional cert consumer |
| outline-wss | Compatible Outline WS backend; Caddy HTTPS paths | Separate stream/packet profiles; retain legacy endpoints during migration |
| https-proxy | Authenticated explicitly tested Caddy handler/profile | Proxy-aware applications only; never advertised as full-device UDP VPN |
| tuic | Compatible pinned QUIC implementation, own UDP endpoint | Experimental alternative; compare to Hysteria before enabling |
| openvpn | Dedicated optional service and explicit TCP/UDP endpoint | Compatibility module; do not assume HTTPS SNI/path multiplexing |
| sstp | Qualified SSTP implementation, TLS endpoint | Compatibility module; verify client SNI and otherwise require separate endpoint |
| softether | Optional pinned SoftEther service | Share service with sstp only if tested; disable unused listeners/management access |
| openconnect | Optional ocserv, TLS control and optional DTLS endpoint | Explicit routes/privileges and client interoperability tests |
| tailscale-exit | Optional overlay exit-node integration | Lab qualification; policy-blocked for deployment until prior WG-class restriction is explicitly reconciled |
| router-gateway | Local OpenWrt client; existing VLESS primary | No new server listener; support per-device/SSID selection |
| linux-gateway | Local Linux forwarding/TUN/AP recipe | Reuse server profiles; test interfaces, DNS/IPv6, routing and shutdown cleanup |
| ssh-session | Existing D1/D2 recipes | Session-only, no autossh; do not imply D1 carries arbitrary UDP |
| dot | Existing Unbound plus TLS listener | Public DNS optional; private DNS retained for dependencies |

IPsec/L2TP-IPsec, new public bare WireGuard, PPTP and public plaintext SOCKS are
not added to the deployment catalogue. Prior owner exclusions remain. The
owner-authorized AmneziaWG2 UDP endpoint (OR-66/D174) is the one explicit
WireGuard-class exception. Tailscale planning does not constitute an exception:
document its policy block visibly.
Do not replace an existing AmneziaWG deployment as part of capability selection.

## Device and client deliverables

| Profile | Deliverable | Acceptance evidence |
|---|---|---|
| android-tv | Direct-client onboarding; FlClash/Hiddify/Amnezia evaluation and gateway alternative | Remote navigation, import, permissions, boot/sleep and transport tests |
| quest | Gateway-first onboarding; optional direct APK experiment | Exact headset/OS; permission and background operation; game/voice UDP; local PCVR unaffected |
| apple-tv | Qualified sing-box tvOS or optional Tailscale evaluation | OS/store availability; exact protocol and exit behavior |
| router | Versioned OpenWrt package/client recipe | Hardware/firmware package compatibility and client/server transport match |
| gateway | Linux/router gateway and selected SSID/device policy | Real forwarded TCP/UDP, DNS/IPv6, LAN discovery, fallback and loop avoidance |
| other-tv | Tizen/webOS or unknown-device gateway route | No assumption of Android APK support or rooted device |

Keep PCVR/Air Link/Virtual Desktop, casting and LAN discovery local. Do not send
local video streams through the VPS or put the headset and PC behind isolation
that breaks discovery. Test headset internet traffic separately from local PCVR.
Respect direct-fallback policy, identify active direct egress, and preserve the
special session-only SSH decision. A working browser is not proof of UDP support.

## Installer and admin experience

1. Ask which devices and uses matter: streaming, downloads, games, voice, local PCVR.
2. Suggest the least-cost existing protocol/client or gateway profile first.
3. Offer optional compatibility modules with dependencies, policy blocks and
   measured/unmeasured costs. Surface port/SNI collisions before installation.
4. Display the concrete plan and use shared apply and existing consent boundaries.
5. Export private per-device configuration plus short installation instructions.
6. Run checks and show ready/degraded/unverified state, then expose update/revoke.

The same catalogue supplies installer choices and admin feature/status views.
Web supports own-module config plans and toggles through the bounded controller within locally approved listener/privilege envelopes. New host privileges, foreign mutations, public-port switching and policy exceptions remain local consent operations. Core and hton.yaml semantics are owned by sdd-config.md; the controller never becomes a general shell/Docker API. CLI arguments are owned by ../CLI.md.

## Acceptance and resource gate

For each module, verify disabled state creates no feature-owned listener or new
credentials; retained credentials stay protected and shared daemons remain only
for core or other enabled consumers;
repeat apply is idempotent; enable/disable/revoke/upgrade/restore work; invalid
configuration leaves working services intact. Test certificate renewal for each
TLS consumer and reject conflicting listener/SNI ownership. Snapshot required
privileges and verify administrative APIs are private.

Publish compatibility evidence with device, OS, application/core versions and
transport. Test IPv4/IPv6/DNS, TCP, UDP when claimed, suspend/resume and LAN access.
Measure VPS and gateway idle/peak memory, CPU/goodput, latency/jitter, and
concurrent web/Naive/VR use. Establish budgets on actual hardware. Add no automatic
all-protocol stress load or compilation to the weak production VPS.

## Interfaces and sources

- [Current implementation plan](../ROADMAP.md)
- [CLI contract](../CLI.md)
- [Device research and primary references](../DEVICE-COMPATIBILITY.md)
- [Shared apply and controller](sdd-control.md)
- [Routing design](../ARCHITECTURE.md)

## Post-release catalogue extension

web-publish is a planned optional capability for private upstream/reverse tunnel
publication on the project domain (sdd-web-publish.md), not enabled in the release
schema/example. Future user tokens and scoped agent keys follow sdd-identity.md.
The Amnezia default is a future user-issuance preset, not a global module default.

## Consistent feature shape and isolation

Every manifest is indexed in ../sdd/module-index.json for the design map; the
implementation binding is modules/<id>.yaml plus modules/index.yaml and services.yaml.
Root config uses enabled; module files own nested settings. Release phase and implementation availability are checked separately from retained configuration; future modules cannot activate in R1. Shared engines, host-only private SOCKS, optional
management tasks and local-gateway execution are distinct descriptor capabilities.
The map lists the planned descriptor files, not implemented executable plugins.

sdd-runtime-safety.md governs admission: management/core inclusion is not VPN
health dependency; accepted native configs run without control plane. Disable of
an optional feature removes its own effects only; runtime guarded changes never
apply an untested arbitrary descriptor. No second panel config writer is admitted.

## Current configuration and phase binding

Root features.<id> contains enabled only; settings live in the versioned module
file config/modules/<id>.yaml. add installs without enabling; incomplete setup
resumes and cannot start. Full API/web/configurator comes later, IPv6 far future.
CAP-5's shared issuer applies to HTON-owned TLS consumers; associated Amnezia
retains its own qualified native certificate workflow. All protocols retain real
client release qualification; candidate Docker staging is optional.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).
