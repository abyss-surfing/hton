# modules/index.yaml

Stable ID: `module.index` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Supported built-in descriptor index

Implementation wave: `I1-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- No user-provided plugin discovery or arbitrary hooks.
- Index descriptor versions, phase and settings-schema references; reject duplicate/missing descriptors. R2 entries never enter the R1 execution closure.
- Own fixed `profileSuggestions` sets for minimal/home/compat/lab. Every suggestion names a known implemented R1 descriptor and remains a first-run hint requiring explicit selection; profile metadata never changes saved desired state or grants foreign authority.

## Calls / reads

- [modules/vless.yaml](vless.yaml.md) — declarative dependency.
- [modules/naive.yaml](naive.yaml.md) — declarative dependency.
- [modules/mtproto.yaml](mtproto.yaml.md) — declarative dependency.
- [modules/hysteria.yaml](hysteria.yaml.md) — declarative dependency.
- [modules/dot.yaml](dot.yaml.md) — declarative dependency.
- [modules/foreign-amnezia.yaml](foreign-amnezia.yaml.md) — declarative dependency.
- [modules/foreign-outline.yaml](foreign-outline.yaml.md) — declarative dependency.
- [modules/router-gateway.yaml](router-gateway.yaml.md) — declarative dependency.
- [modules/linux-gateway.yaml](linux-gateway.yaml.md) — declarative dependency.
- [modules/ipv6-he.yaml](ipv6-he.yaml.md) — declarative dependency (far-future only; excluded from earlier execution closure).
- [modules/ssh-session.yaml](ssh-session.yaml.md) — declarative dependency.
- [modules/site-content.yaml](site-content.yaml.md) — declarative dependency.
- [modules/doctor-watch.yaml](doctor-watch.yaml.md) — declarative dependency.
- [modules/telegram.yaml](telegram.yaml.md) — declarative dependency.
- [modules/outline-wss.yaml](outline-wss.yaml.md) — declarative dependency.
- [modules/private-socks.yaml](private-socks.yaml.md) — declarative dependency.
- [modules/https-proxy.yaml](https-proxy.yaml.md) — declarative dependency.
- [modules/trojan-ws.yaml](trojan-ws.yaml.md) — declarative dependency.
- [modules/trojan-tls.yaml](trojan-tls.yaml.md) — declarative dependency.
- [modules/tuic.yaml](tuic.yaml.md) — declarative dependency.
- [modules/openvpn.yaml](openvpn.yaml.md) — declarative dependency.
- [modules/sstp.yaml](sstp.yaml.md) — declarative dependency.
- [modules/softether.yaml](softether.yaml.md) — declarative dependency.
- [modules/openconnect.yaml](openconnect.yaml.md) — declarative dependency.
- [modules/tailscale-exit.yaml](tailscale-exit.yaml.md) — declarative dependency.
- [modules/smart-connect.yaml](smart-connect.yaml.md) — declarative dependency (R2 only).
- [modules/client-ipv6.yaml](client-ipv6.yaml.md) — declarative dependency (far-future only; excluded from earlier execution closure).
- [modules/web-publish.yaml](web-publish.yaml.md) — declarative dependency (R2 only).
- [modules/management-api.yaml](management-api.yaml.md) — declarative dependency.
- [modules/configurator.yaml](configurator.yaml.md) — declarative dependency.
- [modules/web-admin.yaml](web-admin.yaml.md) — declarative dependency.

## Called / read by

- [core/src/catalog.rs](../core/src/catalog.rs.md) — `describe` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

I1 binding: trusted index fields schemaVersion/catalogueVersion/profileSuggestions/modules[]. Each
entry has id, phase, implemented and optional descriptor inventory ID. Only vless
and foreign-amnezia have R1 implementation bindings. An implemented descriptor is
not native compatibility or deployment qualification. Deferred entries remain
metadata and cannot enter active dependency closure. Current suggestion sets are
minimal=[], home=[vless], compat=[foreign-amnezia], lab=[vless,foreign-amnezia];
they are displayed as hints and are never auto-persisted on an initialized install.
