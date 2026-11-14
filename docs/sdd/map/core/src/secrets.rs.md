# core/src/secrets.rs

Stable ID: `core.secrets` · Phase: `R1` · Kind: source

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Protected versioned secret storage

Implementation wave: `I1-B` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- credentials

## Methods, arguments and result

- `generate` — arguments: scope, kind, explicit replacement policy, OperationContext; result: SecretVersionRef and explicit disclosure handle.
- `resolve` — arguments: validated SecretRef, ConsumerGrant; result: scoped protected material.

## Structures and injected dependencies

SecretStore/SecretRef/SecretVersion/ConsumerGrant; protected version repository

Dependencies: cryptographic randomness, protected filesystem handles, consumer authorization, safe event sink.

## Logic

- Use vetted cryptographic randomness and hash libraries.
- No raw secrets in YAML/logs/metadata.
- Backend files may need material; root access is not prohibited by one-time UI policy.
- Resolve protected immutable versions under /opt/hton/config/secrets; reject traversal/symlink escapes; no entire-store API export.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [scripts/keys](../../scripts/keys.md) — `run` -> `generate`.
- Core config receives only supplied SecretAvailability metadata; it does not resolve material.
- [core/src/render.rs](render.rs.md) — `compile` -> `resolve`.
- [core/src/identities.rs](identities.rs.md) — `issue` -> `generate`.
- [core/src/exports.rs](exports.rs.md) — `exportProfile` -> `resolve`.
- [core/src/certificates.rs](certificates.rs.md) — `issue` -> `resolve`.
- [configurator/src/migration.rs](../../configurator/src/migration.rs.md) — `migrate` -> `generate`.
- [accounts/src/issuance.rs](../../accounts/src/issuance.rs.md) — `create` -> `generate`.
- [tests/unit/secrets-contracts.rs](../../tests/unit/secrets-contracts.rs.md) — `verify` -> `resolve`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Concrete implementation binding (2026-10-31)

Concrete R1 binding: the protected Directory is config/secrets, opened only by a
trusted composition entry. An immutable `SecretRecord` binds secret ID, scope,
kind, consumer allowlist and cryptographic version. `generate` uses getrandom and
create-only durable publication; replacement creates a new version and never
modifies prior material. `resolve` requires exact ID/version and consumer/scope grant.
Material has no Serialize/Display and redacted Debug; explicit disclosure is a
separate handle. Mutation rejects readOnly/deadline/cancellation. Shared storage owns
all anchored path/permission/atomic write mechanics; no parallel filesystem helper.

## Implemented ownership details

Implemented SecretKind is RandomToken, UuidV4 or imported Opaque; unsupported algorithm-specific
credentials remain their adapter/identity owner's responsibility. Create refuses an
existing logical secret, Reuse returns metadata only, Rotate requires explicit
confirmed prior exact reference. Per-request/per-secret intent is durable before
material publication; identical retries return metadata without replaying disclosure,
interrupted intents require explicit reconciliation. Version files are immutable.
Canonical slash references map injectively to dot-joined IDs because dots cannot
occur in source components. Filenames length-prefix logical IDs to avoid prefix
ambiguity. No raw material enters Debug/Serialize on public disclosure handles.

`SecretRef.from_yaml` requires at least two slash-separated components and a
128-byte maximum before splitting, matching the configured reference grammar.


I2 setup/certificate input: SecretStore.import accepts bounded ProtectedMaterial
for externally supplied opaque DNS credentials and certificate/key bytes, sharing
immutable publication/replacement/idempotency with generate. SecretKind.Opaque
cannot be randomly generated. Import intent binds a protected material digest;
ordinary logs/results contain no material. The store admits at most 256 KiB of
material and reads bounded 2 MiB records (JSON byte-array overhead included).
No installer/certificate component writes SecretRecord files behind this owner.

I2 current-version metadata: generate/import atomically publish a protected logical
current binding after durable immutable version bytes. current_version(id,ctx)
returns only the selected version metadata, never material. Rotation checks the
explicit previous version under the same lock. Interrupted binding publication
reconciles only the same retained intent/version and exact previous pointer; a
later independent version causes conflict, never rewind. Accepted runtime bundles
still resolve their explicit older granted versions until a separate safe apply.

validate_archive_record(name,bytes,ctx) is a pure owner validator for immutable
secret/version, current-selector and generation-request records before restore.
It checks supported schema, bounded material/consumers, exact canonical filename
and request identity hash; it never discloses material or creates storage. The
archive owner must also check cross-record references/coherent selection before
writes. Unknown record names fail, not silently restore arbitrary secret files.
