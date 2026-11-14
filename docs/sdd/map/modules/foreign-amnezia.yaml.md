# modules/foreign-amnezia.yaml

Stable ID: `module.foreign-amnezia` · Phase: `R1` · Kind: configuration

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: foreign-amnezia declarative capability descriptor

Implementation wave: `I1-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- foreign-amnezia

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Phase: R1; execution location: server.
- Resolve shared services: relay.
- Own settings/dependencies/check identifiers only; no executable hooks or arbitrary image/mount strings.
- Reference the canonical feature settings schema; declare versioned defaults, required-on-enable fields, allowed listener scopes, feature dependencies and service consumers separately.
- Disabled configuration retains schema-valid settings without new runtime effects or credential issuance. Availability and phase gate activation, not retention.
- Root controls enabled only; versioned settings live in config/modules/foreign-amnezia.yaml.
- `settings.hostPorts.tcp` and `.udp` persist the same Core-allocated port (inclusive 5000–6000); the value is generated for missing allocation and reused across install. Native in-container ports are observed from the registered container and preserved unchanged. Legacy out-of-range values remain diagnosable for exact review.
- Trusted `portAllocation` declares `vpn-backend`, service `foreign-amnezia`, canonical settings object ID `module.foreign-amnezia`, one binding group, tagged Host namespace, public IPv4 wildcard address, and exact TCP/UDP `settings.hostPorts` paths; Core allocation uses this catalogue mapping rather than a module-specific path table.
- `listenerClaims: []` is intentional: catalogue `bindAllocation` materializes Host TCP/UDP ListenerClaims from the validated mapping, candidate port and declared public Host address template so no duplicate claim source exists.
- Site names/listener/DNS-01 secretRefs remain in core root config, not this module.
- Declare setup checkpoints, semantic check, lifecycle/update behavior and explicit resources/ownership; add stays disabled and resumable.
- Execution evidence and product tests are final waves after implementation; native runtime validation remains product behavior.
- External configuration/certificate ownership; explicitly associated lifecycle includes whole-server stop/autostart control.
- Native domain/TLS/restart qualification is unresolved; do not advertise support from a manifest.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/catalog.rs](../core/src/catalog.rs.md) — `describe` -> declarative dependency.
- [modules/index.yaml](index.yaml.md) — declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

I1 binding consumes the externally owned foreign-amnezia service, not a new relay
or certificate issuer. Capabilities are amnezia.check/observe/associate/lifecycle;
render/update are absent. Config must provide explicit native serverNames for exact
SNI passthrough and independent verified qualification before activation. Unknown
native behavior is a blocker; manifest presence alone never reports readiness.

Owner D80 accepts native REALITY through byte-preserving TCP passthrough; genuine
own-domain HTTPS applies to the ordinary site. Exact installed identity/SNI/target/
restart evidence remains mandatory. No custom certificate issuer or implicit
authority over discovered containers is introduced.

## OR-39/OR-40 planning amendment

Read canonical sdd-config, sdd-installer, sdd-foreign-adapters and INTERFACES:
all SNI are config-owned and resolved once; administrative username comes from
`foreign-amnezia.settings.admin.user`. Managed provisioning is separate from
foreign association, starts on private bindings and delegates account effects to
HostAccess. Exact future file ownership and schema changes remain implementation
prerequisites in TASK-AMNEZIA-PROVISION; this mirror does not change live parsers.
