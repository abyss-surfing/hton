# contracts/src/config.rs

Stable ID: `contract.config` · Phase: `R1` · Kind: contract

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Snapshot/read-set/check/descriptor DTOs

Implementation wave: `I0-A` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Data-only records from INTERFACES section 3; no module activation policy or parser side effects.
- Desired enabled selection differs from registry installation facts and observed runtime.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/lib.rs](../../core/src/lib.rs.md) — declarative dependency.
- [adapters/engines/xray.rs](../../adapters/engines/xray.rs.md) — `check` -> declarative dependency.
- [adapters/engines/caddy.rs](../../adapters/engines/caddy.rs.md) — `check` -> declarative dependency.
- [adapters/engines/haproxy.rs](../../adapters/engines/haproxy.rs.md) — `check` -> declarative dependency.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I0 concrete Rust binding (2026-10-31)

`config` owns documents/read sets, desired/normalized snapshots, descriptors,
registry facts, dependency claims and observations. Canonical module settings use
`serde_json::Value` until their owning module validates a versioned schema; arbitrary
settings are never interpreted as actions. Config document source bytes and desired
settings are protected inputs: their Debug output is suppressed. Known relative
locations use validated path-component records rather than raw external paths.
Object revisions bind an object ID and SHA-256 of exact bytes. Parse failures remain
inside DesiredReadResult so callers can diagnose invalid configuration and stop
from durable registry/runtime state. No runtime state is inferred from desired YAML.
`ReadSet.absent_objects` records only known descriptor object IDs proven absent by
the same coherent inventory; present and absent ID sets are disjoint. The field has
an empty serde default for earlier records.

Exact Docker identity is `DockerContainerId` (64 lowercase hexadecimal characters),
not a logical module/container name; every deserialization revalidates it. Registry
membership explicitly retains removed/cleanup-required records, separately from
setup status, enabled selection and observed process state. `CheckIssue` prose has
the same trusted-redaction obligation as `operations::Error`; raw parser snippets
must never embed protected configuration source bytes.

I1 contract correction: `ListenerClaim.namespace` is mandatory and independent of
`public` exposure. `ListenerNamespace` is Host, Project{networkId}, or
Service{serviceId}. Only equal namespaces have listener collisions; host wildcard
addresses still overlap host-loopback listeners. No public/private namespace guess.

I3 allocation records: `ModuleDescriptor.portAllocation` optionally binds one
service, settings document, binding group, typed host listener template and exact
field/transport mappings. `AllocationRequest`, fresh `ReservationEvidence`, and
`PortAllocationCandidate` carry selected/retained assignments, coherent config
ReadSet, exact separate registry revision, reservation digest and expiry without raw entropy. `DependencyGraph` retains the
trusted bindings so catalogue materialization uses the same source as persistence.
`PortAllocationReceipt.transactionId` is absent only for an idempotent no-op where
all selected values already match saved module settings; changed values carry the
journal transaction and per-document revisions.
Each allocated service binding also carries the catalogue-injected descriptor
revision; it is bound into the allocation candidate so recovery cannot recreate a
missing settings file from changed defaults after a software/catalogue update.
For first-install module settings, the candidate binds the settings document in
`ReadSet.absent_objects`; its writer creates trusted descriptor defaults and the
chosen port together after complete preflight, and recovery distinguishes absent
from an existing empty document.

I3 installation input records are non-secret DTOs. `InstallInput` contains only the
site domain, public `HostEndpoint`, optional non-secret certificate account email,
and per-field provenance; backend VPN ports and credential bytes are not input.
`InstallConfigCandidate` contains a fixed `InstallConfigField` enum (`SiteDomain`,
`PublicEdge`, or `CertificateEmail`), typed before/after values, a coherent
`ReadSet`, and a digest. It has no caller-supplied path or arbitrary patch. A
first-publication writer may seed only the fixed Let's Encrypt DNS-01/InternetBS
metadata and fixed protected-store references `dns/internetbs/api-key` and
`dns/internetbs/api-password`; it never accepts credential values. Existing
certificate provider/reference settings are preserved. `InstallInputReview`
references a protected local approval by ID and binds digest, instance, and
operation. `InstallConfigReceipt` contains transaction and document revision
metadata only, never source bytes.
