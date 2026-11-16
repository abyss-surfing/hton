# core/src/host/bundle_validation.rs

Stable ID: `core.host-bundle-validation` · R1 · I3-B

Explicit mutating `bundle.validate BUNDLE_ID` coordinator. Protected BundleId
lookup reads canonical candidate.json; verifies instance, canonical digest, complete
artifact/service ownership and bounded exact artifact bytes and cached image pins.
Dry-run performs only those reads and reports scenarios/effects without creating
files, journals or containers. Unknown or unsupported selected semantic scenarios
are rejected explicitly, never accepted by hash/native checks alone.

A dedicated journal intent binds BundleId, candidate digest and deterministic
sandbox scope before effects. Interrupted validations are cleaned by exact scoped
labels before new work. Record immutable created IDs before start. Native parser
checks precede isolated actual server/client site scenarios. The fixed runner supports HAProxy/Caddy, shared Unbound UDP/TCP and VLESS WS/XHTTP
through a pinned Xray client. Unknown selected protocol scenarios and foreign
Amnezia remain explicit blockers. Exact final bytes are copied into protected
per-operation staging, then private IP overrides are bound into scenario evidence.
Public ports, production containers, foreign resources and runtime accepted state
are never mutated.

Only a complete positive native/semantic result and independently confirmed cleanup
permit a protected BundleValidationAttestation. Every outcome includes external
DNS/device reachability as unverified. Failed cleanup retains unfinished journal
intent and staging for explicit recovery. Attestations are stored under the bundle
validation directory with an operation-derived name and 5-minute validity. Protected copied artifacts are deleted only after confirmed
sandbox cleanup; unfinished intents retain exact operation scopes for recovery.

Dependencies: core.journal, SecureDir, runtime-contract hash/decoding,
infra.validation-docker. Caller: core.host fixed dispatch. No testing/build is run
while implementing this package (D81).

Repeated terminal operation identities cannot rerun sandbox effects or overwrite
attestations. Recovery removes a pre-terminal attestation only after checking its
exact bundle, operation and candidate identities, then cleans the original scope
and records the interrupted attempt as failed. A new attempt requires a fresh
operation identity. Successful attestations bind the pinned curl/dig image as well
as all server image digests. Native parser and semantic/client evidence have distinct
vantages. The isolated test subnet is deterministic from the operation scope; an
existing subnet conflict is an explicit failure, never permission to edit another
network.
