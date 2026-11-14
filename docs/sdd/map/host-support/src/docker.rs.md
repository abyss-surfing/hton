# host-support/src/docker.rs

Stable ID: `adapter.host-docker` · Phase: `R1` · Kind: source · Wave: I3-B

Exact immutable local Docker observations. Source written; no compilation/product acceptance claimed (D81).

Typed `PublishedPort` and `TransportProtocol` are defined for TCP/UDP host
bindings. `PublishedPort::new` rejects port zero, unspecified IPv4 and multicast
addresses. This value type does not expose Docker argv.

## Methods and responsibility

- `inspect_container` — full ContainerId, Budget, Cancellation → protected ContainerFacts.
- `inspect_image` — repository digest reference, Budget, Cancellation → protected ImageFacts.
- `observe_container_network_listeners` — exact immutable container ID, expected structural fingerprint, Budget, Cancellation → bounded TCP LISTEN/UDP bind rows from that exact container network namespace, bracketed by exact inspect and stable init-process/namespace identity; read-only and does not attribute a socket to a process.
- `published_port_bindings` — bounded all-container ID inventory, host binding,
  container endpoint and structural fingerprint, read twice → stable exact records
  or typed incomplete/error.
- `published_port_reservations` — projection of the exact records to transport/host
  port/ID/fingerprint collision evidence; no identity association or mutation authority.
- `foreign_candidate_facts` — maximum count, Budget, Cancellation → complete
  display-only summaries for every full container ID, including image digest, runtime
  status, exact published TCP/UDP host bindings and structural fingerprint. It reads
  IDs and exact-inspects all containers twice; any ID/fact drift, invalid binding,
  truncation or inspect error fails the whole inventory.
  no identity association or mutation authority.
- `read_amnezia_xray_server_config` — exact immutable `ContainerId`, expected
  structural fingerprint, Budget, Cancellation → protected bounded file bytes plus
  exact ID/fingerprints/content digest. Uses only fixed Docker copy/archive argv for
  `/opt/amnezia/xray/server.json`; rejects non-regular/ambiguous archive members,
  path escape, mounts, malformed/truncated output and drift across inspect/copy/copy/
  inspect. Bytes are non-serializable with redacted Debug; no ownership authority.

Authority and limits: [INTERFACES](../../../../INTERFACES.md), I3 bounded native observation transport / D81.

Calls [host-support/src/process.rs](process.rs.md) — inspect_container → run.

Calls [host-support/src/process.rs](process.rs.md) — inspect_image → run.

`read_amnezia_xray_server_config` calls the private bounded Docker runner three
times for exact inspect and twice for `container cp`; all use the same fixed socket,
config, immutable ID and caller budget. The POSIX TAR subset parser is bounded and
accepts only the expected single regular member. It never follows symlinks or runs a
process inside the foreign container. The returned protected byte wrapper exists
only for the in-memory native config parser and redacts its `Debug` output.

`observe_container_network_listeners` takes the Docker init PID only from exact
inspect facts, rejects stopped/zero-PID resources, reads fixed `/proc/<pid>/net`
tables, and repeats exact inspect plus PID start-time/namespace-inode checks. Its
rows prove only a socket in that container network namespace, not that Xray owns
the socket or that the socket implements the observed config.

No Core/runtime contracts, shell, mutation APIs, secret serialization or ownership discovery. Driver errors are closed/redacted; unavailable facts cannot imply success. Consumers must qualify ownership/admission/fingerprints independently.

`published_port_reservations` is a collision-only observation exception to exact-object
inspection: it enumerates IDs and published bindings, not arbitrary container
configuration. It succeeds as complete only when two bounded snapshots match and
every exact-ID inspection succeeds. It neither returns launch secrets nor associates
resources with modules. Each binding carries its immutable container ID; Core may
map it to a service only when the ID and structural fingerprint from the same inspect
match exact registry identity. Concurrent changes after
observation still require immediate pre-effect refresh by the domain coordinator.

`foreign_candidate_facts` includes containers without published ports. It compares
two complete passes of full IDs and projected exact-inspect facts; it exposes no
launch args, env, labels, mount sources or inferred module identity. The projection
is display data, not qualification evidence or ownership authority.

Full immutable container IDs and repository digest references only. Returned identity/RepoDigests must match. Launch/env/mount/label facts have no Serialize and redacted Debug; no raw stderr. Local fixed socket/config; no pulls, discovery, starts or adopted authority.

Generated running Caddy Compose and deterministic parked Caddy have no `/var/www`
mount. The site body is captured into the immutable Caddyfile by Core's renderer;
Docker's service-scoped `/etc/hton` bind continues to expose only that admitted
configuration and its private certificate files. Arbitrary host source paths are
never selected by Docker composition or ownership validation.

Primary CLI sources: [container inspect](https://docs.docker.com/reference/cli/docker/container/inspect/), [image inspect](https://docs.docker.com/reference/cli/docker/image/inspect/), [container cp](https://docs.docker.com/reference/cli/docker/container/cp/).

R1 DockerControl adds fixed explicit setup/update image pull and parked-container
provisioning, plus exact-ID start/stop/restart/restart-policy/reload actions. A
planned `ForeignRemapPort` may add bounded `snapshot_container`,
`remap_published_ports` and `restore_snapshot`: each takes immutable container ID,
exact expected fingerprint, opaque protected snapshot handle and a validated TCP/UDP
binding set; it returns an independently inspected before/after receipt. Snapshot
preserves writable layer and every supported create/network/mount/restart fact.
Unsupported create semantics fail closed. These methods are not currently bound;
they cannot accept arbitrary Docker argv, image, shell or client locator.

The planned snapshot/recreate executor remains unimplemented. Safe implementation
requires protected durable writable-layer custody and exact support for every
Docker creation/network/mount/runtime field; the current inspect model does not
prove that complete coverage. Do not claim remap readiness until that boundary is
completed and its shared types are reconciled with `INTERFACES.md`.

`observe_amnezia_xray_listener` is the fixed native process-attribution operation.
It accepts exact ID/fingerprint and parsed native IPv4/port, brackets the procfs
process/socket attribution with stable Docker inspect/PID/start-state, and returns
a protected executable digest observation. It does not execute Xray or equate the
digest to an installer/build version. The pinned source start.sh uses
`xray -config /opt/amnezia/xray/server.json`, and Dockerfile installs `/usr/bin/xray`:
[Amnezia 5.0.3 source](https://github.com/amnezia-vpn/amnezia-client/tree/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/xray).

`observe_amnezia_xray_version` is the sole narrow foreign execution exception.
After process/socket/executable attribution it runs fixed `/usr/bin/xray version`
as 65534:65534 in the exact immutable container ID, with fixed workdir and locale,
and repeats exact process/executable attribution and Docker identity. Its strict
two-line parser emits bounded native version/build fields; raw output never
leaves the protected boundary. This does not infer the Amnezia installer version
or verify an upstream release signature. The driver exposes no generic exec API.

`remove_stopped_owned_exact` is Runtime's narrower owned-uninstall primitive:
exact immutable ID and expected structural fingerprint, stopped/nonpaused/nondead
state, restart policy `no`, then literal `container rm` without force or volume
deletion. Runtime independently supplies admitted owned scope/current epoch.
The host method cannot turn a discovered foreign candidate into owned scope.
