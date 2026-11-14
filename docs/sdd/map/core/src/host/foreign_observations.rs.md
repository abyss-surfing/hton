# core/src/host/foreign_observations.rs

Stable ID: `core.host-foreign-observations` · Phase: `R1` · Kind: source

Read-only local binding from `adapter.foreign::ForeignObservePort` to bounded
host Docker facts. Candidate discovery returns only exact immutable IDs, image
digests, runtime state, published TCP/UDP bindings and structural fingerprints.
Two stable full-inventory passes are required by host-support. Exact-ID native
Amnezia qualification and registered-resource evidence remain typed unsupported
until their protocol, listener, ownership and protected-snapshot facts have trusted
producers.

## Dependencies and authority

- `host-support::DockerObserver::foreign_candidate_facts` for stable inventory.
- `host-support::DockerObserver::read_amnezia_xray_server_config` is the fixed,
  fingerprint-bracketed source for native Xray JSON bytes; the pure Core parser
  must borrow secret-bearing values and emit only bounded nonsensitive facts.
- Injected `OperationControl` for cancellation, time and operation limits.
- `adapter.foreign::ForeignObservePort` DTOs; no config, registry, adoption,
  mutation, certificate or remap authority.

## Methods

- `LocalForeignObserver::discover_candidates` — explicit `Budget` and
  `OperationContext` → complete redacted `ForeignCandidateInventory` or typed
  error; bounded to 256 containers, 4 MiB output and at most 10 seconds.
- `inspect_candidate` — exact selected ID → stable bounded inventory lookup,
  exact fixed-path Amnezia Xray JSON read bracketed by the selected structural
  fingerprint, then strict redacted parse of one VLESS/VMess TCP REALITY inbound.
  Native REALITY may intentionally use its target hostname as a serverName;
  that equality is not itself an edge routing loop. It rejects a stopped container, loopback-only
  native binding, and a native TCP port without exactly one corresponding published
  host port. These facts
  only establish config/mapping consistency; it additionally requires a matching
  TCP LISTEN row from the exact container network namespace, but that row does not
  identify its process owner. It still returns typed `ModuleUnsupported` until
  independent resolved edge-loop exclusion, restart-owner and supported
  recreation evidence is available.
  Process attribution additionally uses the fixed Xray executable/config-argv
  owner observation; namespace-only rows never substitute for it.
- `observe_registered` — exact registry identity → typed `ModuleUnsupported`
  until complete qualified native evidence is implemented.

## Calls and callers

- Calls `host-support::DockerObserver::foreign_candidate_facts` only for candidate
  inventory.
- `inspect_candidate` consumes the fixed native config reader and parser, plus
  `DockerObserver::observe_container_network_listeners`, but the
  parsed subset is never emitted as qualification evidence. It verifies only that
  the selected running candidate's declared TCP REALITY port corresponds to one
  published Docker host port, that a matching TCP socket exists in its network
  namespace. It preserves native target/serverName equality; proving that the target
  does not route back to the HTON edge remains a separate qualification obligation. Namespace
  membership does not prove that Xray owns that socket or that the selected backend
  route avoids a resolved-address loop. Resolved edge-loop exclusion, non-Docker restart owners and complete
  recreation compatibility remain missing independent qualification evidence.
  The strict no-mount snapshot capability is observed read-only; actual image
  custody is a separately consented later action.
- Is injected into the foreign adapter; discovery never registers a candidate.
- See [adapter foreign map](../../../adapters/foreign.rs.md),
  [Docker map](../../../host-support/src/docker.rs.md),
  [interfaces](../../../../../INTERFACES.md), and
  [foreign adapter SDD](../../../../../specs/sdd-foreign-adapters.md).

## Completion limits

The source is not compiled or accepted. Candidate inventory is available as a
read-only summary; it does not qualify Amnezia or make allocation/remap safe.

## Observation budget

All native observations share the earliest of the caller Budget deadline, the
operation deadline and a ten-second local ceiling. Cancellation is checked before
starting Docker work. Empty, expired or over-limit inputs never extend an
observation window, including direct calls that do not pass through the adapter.

The native observer also calls `observe_amnezia_xray_listener` after namespace
matching to establish exact supported process/socket/config-argv attribution and
executable identity. This clears only process attribution; external restart owners, resolved loop evidence and supported recreation
remain independent blockers. Binary version and no-mount snapshot capability now
have separate concrete observers.

Fixed `observe_amnezia_xray_version` now follows successful process attribution:
no shell or arbitrary argv, only `/usr/bin/xray version` in the exact immutable
container as unprivileged 65534:65534, bounded strict native output, and executable
identity rechecked around it. This is binary-reported Xray build evidence only;
the exact Amnezia creator/client version remains unknown. Its narrow runtime
execution authority is specified in the foreign SDD and INTERFACES.

`observe_no_mount_snapshot_capability` verifies stable complete inspect bytes
and rejects mounts/volumes/binds/tmpfs without creating files, locks, images or
archives. The supported capability is separate from the protected snapshot store;
no snapshot mutation occurs during candidate qualification or registry adoption.
