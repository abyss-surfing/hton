# core/src/identities.rs

Stable ID: `core.identities` · Phase: `R1` · Kind: source

I2 coordinator implementation binding; native host drivers and final acceptance remain pending. Purpose: Release device identity/credential bindings

Implementation wave: `I2-C` (coordinator source present; final acceptance pending).

## Features and responsibility

- devices

## Methods, arguments and result

- `issue` — arguments: identityId, kind, actor; result: issuance result.

## Structures and injected dependencies

IdentityBinding/IssuanceResult; release credential binding rules

Dependencies: qualified backend capabilities, protected secret store, shared apply.

## Logic

- Bind keys to selected qualified profiles.
- Do not promise shared password is independent per-user credential.
- R2 migrates semantics through account/issuance layer.

## Calls / reads

- [core/src/secrets.rs](secrets.rs.md) — `issue` -> `generate`.
- [core/src/apply.rs](apply.rs.md) — `issue` -> `apply`.

## Called / read by

- [scripts/keys](../../scripts/keys.md) — `run` -> `issue`.
- [controller/src/api.rs](../../controller/src/api.rs.md) — `handle` -> `issue`.

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

IdentityCoordinator.issue(IdentityIssueRequest,ctx) supports qualified independent
UUID credentials for owned R1 VLESS profiles. It rejects shared-password claims,
unqualified or externally owned profiles; native Amnezia remains independent.
Identity journal intent and protected originals precede SecretStore.generate and
CAS pending inventory publication. IdentityInventory keeps previous accepted bindings
available during a pending replacement; it never overwrites a live engine config.
Its plan_issue/plan_revoke compose the shared planner with identity inventory revision
in readSet, then EffectApplyPort performs the same protected stage/apply path.

No generated material is disclosed before verified backend acceptance. Stopped saved
enrollment returns Pending/Partial rather than claiming an active credential. Explicit
R1 keys --show/share uses exports over accepted bindings; repeat disclosure is still
R1 policy and does not implement future R2 accounts semantics. Successful native issue
finalizes metadata only after accepted_verified; revocation finalizes only after
active AND admissible fallback credential/fence verification. Old secret versions are
retained for protected recovery, not made exportable by fallback. Interrupted retries
return durable metadata or require reconciliation, never generate another key blindly.


First-key bootstrap: an installed qualified profile may prepare a Pending UUID
while disabled/incomplete, without enabling a listener or applying a candidate.
IdentityInventory.run_state supplies the actual durable intent/accepted ID; saved
inactive enrollment reports Partial explicitly. After explicit enable/up,
confirm_accepted independently verifies that the current accepted bundle contains
this exact scoped credential before marking metadata Accepted. It generates no
new secret and never exposes an unaccepted pending key. R2 accounts are unchanged.
