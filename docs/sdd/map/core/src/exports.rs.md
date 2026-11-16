# core/src/exports.rs

Stable ID: `core.exports` · Phase: `R1` · Kind: source

I2 coordinator implementation binding; native host drivers and final acceptance remain pending. Purpose: Protocol/client export adapters

Implementation wave: `I2-C` (coordinator source present; final acceptance pending).

## Features and responsibility

- onboarding

## Methods, arguments and result

- `exportProfile` — arguments: accepted identity, kind, target, client, format; result: protected output.

## Structures and injected dependencies

AcceptedCredentialBinding/ClientProfile/ProtectedExport; transport serializers

Dependencies: accepted binding reader, qualified client catalogue, disclosure grant, protected output writer.

## Logic

- Derive serializers from supported native protocol specifications.
- No unsupported transport substitution.
- R2 prevents re-export of delivered credentials.

## Calls / reads

- [core/src/secrets.rs](secrets.rs.md) — `exportProfile` -> `resolve`.
- [core/src/catalog.rs](catalog.rs.md) — `exportProfile` -> `describe`.

## Called / read by

- [scripts/share](../../scripts/share.md) — `run` -> `exportProfile`.
- [adapters/gateway](../../adapters/gateway.md) — `configure` -> `exportProfile`.
- [controller/src/api.rs](../../controller/src/api.rs.md) — `handle` -> `exportProfile`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


## I2 concrete binding

Typed coordinators own sequencing and validate invariants; trusted injected ports
own bounded native processes/transport and protected logical inventories. Ports
must fail unavailable/unsupported explicitly and never return synthetic success.
Mutation requires OperationContext Mutating, live deadline/cancellation and durable
journal intent before any secret, certificate, archive or runtime effect. Secret
payload wrappers redact Debug and are never cached with ordinary operation status.
Product tests remain deferred until I4; compilation is integration, not acceptance.

ExportCoordinator.export_profile(identityId,profileId,clientId,format,ctx) resolves
only current accepted, enabled, non-revoked credential bindings at the current fence.
Disclosure authorization and client/version qualification are checked before scoped
SecretStore.resolve; accepted/fence is rechecked before returning protected bytes.
ProtectedExport deliberately has no Serialize and its Debug is redacted. Normal
metadata/idempotency responses therefore have no credential-bearing URI/config slot.

Formats are exact VLESS URI and current native Xray JSON outbound; WS and XHTTP
serialize separate wsSettings/xhttpSettings, exact host/path/mode and TLS serverName.
WS uses HTTP/1.1 ALPN; XHTTP offers H2/HTTP1.1 and explicit mode. No unsupported
client/transport substitution, insecure TLS flag, optional REALITY claim, arbitrary
raw JSON or QR implementation is invented. I3 can feed protected output into a
qualified QR tool after explicit disclosure authorization.

Primary serialization references consulted 2026-10-31:
[Project X VLESS outbound](https://xtls.github.io/en/config/outbounds/vless.html),
[WebSocket](https://xtls.github.io/en/config/transports/websocket),
[transport configuration](https://xtls.github.io/en/config/transport.html),
[upstream XHTTP protocol description](https://github.com/XTLS/Xray-core/discussions/4113).
Concrete client/version interoperability remains final functional qualification.
