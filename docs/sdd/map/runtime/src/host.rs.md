# runtime/src/host.rs

Independent trusted host composition, R1 I3. Reads root-private immutable
work/runtime/authorities/<bundle>.json produced through journalled Core preparation;
current shutdown projection is work/runtime/authority.json. Wire RuntimeAuthority
is metadata only: no caller validity boolean. Auth binds instance/bundle digest,
resources/locators, native validator evidence, approvals/fallbacks and secret grants.
Independent admission still verifies exact files/hashes, local images, launch metadata,
claims, fences and OS caller. Native facts never grant discovered ownership.

Accepted-service restart uses only `NativeAction::Restart` for the exact admitted
Owned service resource. The host verifies post-restart process status and admitted
launch identity; unsupported semantic `ProbeKind`s remain unknown, so the executor
returns Partial with recorded action receipts/survivors rather than claiming health.
Structural drift fails closed: Restart cannot Recreate or rewrite service artifacts.

HostRuntime implements StateRepository wiring, EffectGate, NativeAction,
AdmissionPorts, ShutdownAuthority, GuardianSupervisor and ProbePort. Fixed host
operations are through hton-host typed drivers; compiler/web imports forbidden.
Service artifact installation belongs only to the explicitly declared native action;
no hidden post-review publish step. Unknown semantic probe evidence is blocking.
Native action outputs never include raw env/config/native stderr. No code execution,
compilation or tests before full plan completion (D81).

I3 retained launch verification: canonical service revisions exclude global
haproxy/compose.json as a loaded native artifact and bind per-service retained
Compose launch instead. Immutable accepted evidence has no arbitrary five-minute
expiry; current certificates/fences/actual launch remain checked. Scoped validation
selects the proposed bundle authority, while shutdown locators use the independent
installed projection. Healthy unchanged generations retain honest effective-bundle
metadata and qualify through exact native revision and current-file equality.

Native owned removal resolves only the protected shutdown/binding identity. It
verifies stopped Docker state, disabled restart policy and exact launch identity,
writes a root-private removal intent before rm, stops only that container's fixed
native-log collector unit, and confirms exact ID absence via bounded stable Docker
inventory. An unexplained missing container is drift; an exact retained removal
intent permits interrupted completion. No force, volume/image or filesystem data
deletion occurs, and retained bindings/artifacts remain recovery material.
