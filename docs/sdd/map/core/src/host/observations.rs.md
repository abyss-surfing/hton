# core/src/host/observations.rs

Stable ID host.observations · R1 · I3-A

collect(app,ctx) reads only exact registry locators and bounded native driver facts,
returning fresh management observations plus independent wire identity/state hashes.
Shared hton-host canonical hashing is mandatory. Active artifact metadata is a
protected declared runtime input; it is not independently accepted evidence. Unknown
native failures produce Unknown plus redacted diagnostics and no wire grant. Actual
structural identity mismatches remain mismatches; no discovery/adoption or mutation.

`reservation_evidence(app,ctx)` reads the complete collision-only host TCP/UDP
snapshot from HostObserver. It maps a Docker binding to `holder_service` only when
both immutable container ID and observed structural fingerprint match one exact
registered `ResourceIdentity`; duplicate registry identities are a blocker. Procfs
listeners and unregistered/drifted Docker containers remain unknown holders. The
evidence digest binds sorted transport/port/container identity/service tuples; raw
IDs are not serialized into the evidence DTO or logs. Snapshot failure returns a
typed blocked result; no partial evidence is marked complete. This observation
grants no ownership or remap authority and must be refreshed before effects.

`reservation_evidence_for(registry,control,ctx)` accepts the exact registry snapshot
and operation-control capability from its caller, then performs the same bounded
read-only observation without opening `HostApplication` or initializing stores.
First-install virtual preflight can pass its empty expected registry through this
method. Registry revision is bound separately by the allocation candidate and must
be rechecked before effects. This seam does not grant foreign qualification or
association.

`listener_conflicts(graph,evidence)` owns the shared conservative host TCP/UDP
collision rule used by install and ordinary effect preparation. A reservation on
the claimed protocol/port blocks unless its exact registered holder service matches
the claim. Unknown and unregistered resources never inherit ownership from a port
or display name. Callers independently require complete fresh evidence before use.

`diagnose(app,sections,verbose,ctx)` composes at most 128 registered Docker/systemd
observations at 32 KiB per native response (4 MiB total) and runtime intent through
CheckCoordinator. It never makes network
requests and returns only counts, closed issue codes and explicit unverified
sections. `whoami(app,ctx)` is the only external-observation entry here: a deliberate
CLI invocation may perform one fixed public-IPv4 query plus DNS-A for at most eight
configured domains. The output carries DNS addresses as owner-facing public
metadata, never secret config or process output; unsupported identity fields remain
unprobed. Both paths are read-only and bounded by operation deadline and byte caps.
The fixed `doctor.observe` route reuses `diagnose` with its default local sections;
it does not create a second diagnostics implementation or broaden the probe policy.

Called by: Core fixed host dispatch for `diag.observe`, `doctor.observe`,
`identity.whoami` and installation/allocator preflight; direct CLI/API callers do
not own observation logic.
