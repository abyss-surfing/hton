# runtime/src/manifest.rs

Stable ID: `runtime.manifest` · Phase: `R1` · Kind: source

I1 implementation binding; host integration and product acceptance remain pending. Purpose: Independent bundle admission

Implementation wave: `I1-C` (library implementation present; final acceptance pending).

## Features and responsibility

- safety

## Methods, arguments and result

- `admit` — arguments: bundleId, proof, policyFence; result: AdmittedBundle.

## Structures and injected dependencies

AdmissionProof/AdmittedBundle; bounded artifact/resource admission

Dependencies: immutable artifact handles, native validators, actual resource observations, approval/fence verifier.

## Logic

- Check artifact/image/secret hashes, native validators and final-bundle test attestation.
- Reject arbitrary mounts/commands, stale approvals and incompatible revoked-key fences.
- Validate versioned bundle contract independently; reject unknown action/adapter IDs and mismatched tested artifact hashes. Never import compiler/controller/admin packages.

## Calls / reads

- Caller supplies durable RuntimeState; admission never independently reads or mutates state.
- [runtime-contract/src/lib.rs](../../runtime-contract/src/lib.rs.md) — `admit` -> declarative dependency.

## Called / read by

- [runtime/src/main.rs](main.rs.md) — `execute` -> `admit`.
- [runtime/src/guardian.rs](guardian.rs.md) — `monitor` -> `admit`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


## I1-C concrete binding

Implemented as an independent `hton-runtime` library; binary composition follows I3.
All mutable host authority enters through injected trusted ports. Wire proposals
never manufacture AdmittedBundle, which has private fields and no Deserialize.
Native operations and observation ports enforce supplied absolute deadlines; the
coordinator narrows per-action deadlines and rechecks durable epoch under the
short effect lock. Port failures remain typed failures, never synthetic success.
No Core, management contract, YAML, network download or shell expression dependency.

Manifest digest recipe: SHA-256 of canonical serde_json compact object serialization
with manifestDigest, nativeEvidence and staging removed. The latter evidence binds
the resulting digest, avoiding a recursive self-hash. Admission also hashes the
exact protected source bytes against proof.protectedArtifactDigest. Artifacts remain
immutable bytes owned by the admitted handle and have a cumulative 16 MiB cap.
Trusted ports read protected bundle inventory, authenticate locally persisted
approvals/fallback qualification and observe image/secret/resource identities.
Accepted startup re-admits immutable material against durable accepted authority;
it does not require a compiler, download or new native validation process.

R1 wire Listener claims describe published host IPv4 endpoints only. Private
project/service namespaces live in native artifacts; claims_valid must parse those
artifacts using engine adapters, check native listener/mount/capability allowlists,
and reject claims/artifacts disagreement or cross-namespace ambiguity. It must not
infer listener namespaces from public/private reachability. Runtime wire stays
independent from management ListenerNamespace and accepts no additional enum yet.
Restart-owner actions require their separately observed exact resource identities;
never substitute the traffic-service container identity for a systemd/timer owner.

Validation levels have one owner: normal admission, accepted startup and runnable
fallback require full image/secret/native/certificate/claims readiness. Candidate
cleanup authenticates protected manifest structure/digest and durable lease authority,
but requires neither candidate image availability nor valid candidate certs/secrets.
Cleanup handles retain no candidate artifact bytes and cannot be used for apply/start.
Compensation may only launch fully admitted fallback material. Failure to authenticate
protected candidate authority still produces recovery-required. Updated actual native
identity is taken from protected receipts bound to the bundle or active lease, rather
than being overwritten by an older candidate resource identity after recreation.

Explicit rollback resolves the previous target only through the committed lease's
FallbackCompatibility record. Admission verifies protected manifest digest, cached
images, scoped secrets, native evidence, current credential fences and certificate
expiry, then resolves exact current resource bindings. It is repeated after native
compensation and before state acceptance; user-provided IDs cannot supply proof.


I2 retirement claims contain no active startup/image/native checks. All retiring
resources and restart owners are exact observed/approved owned resources. Only
fixed Stop/DisableAutostart forwards and declared Start/EnableAutostart inverses
are allowed; no arbitrary native config replacement is admitted for retired
services. Maintenance reservation rejects promotion and accepted startup.
