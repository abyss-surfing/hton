# host-support/src/ports.rs

Stable ID: `infra.host-ports` · Phase: `R1` · Kind: source · Wave: `I3-B`.

Reads only fixed Linux `/proc/net/{tcp,tcp6,udp,udp6}` tables with strict byte and
row caps. TCP rows count only LISTEN; all UDP rows with a nonzero local port reserve
that transport/port. Missing, truncated or malformed tables return an error, never
partial evidence.

The fixed parser is also used for exact Docker-init network namespace tables at
`/proc/<inspect-pid>/net/{tcp,tcp6,udp,udp6}`. Namespace reads require stable PID
start time and namespace inode around the read, plus a matching exact Docker inspect
before and after. This exposes only transport/address/port socket rows; it does not
identify their process owner.

`HostPortReservation` contains transport, port, optional immutable container ID and
the structural fingerprint from the same exact Docker inspect. Linux socket rows set
both identity fields to `None`. Core maps a Docker ID only when the fingerprint also
matches the exact registry resource identity.
Debug output reports only whether a Docker identity is present, never the ID.
`HostObserver.port_reservations(Budget, Cancellation)` composes both sources,
sorts and deduplicates the result. It has no caller filesystem path, no subprocess
input, and no mutation/association authority. The caller owns freshness metadata and
evidence digest. This is a point-in-time read, not a port lease; install refreshes
before effects. If a Docker binding and procfs socket share a protocol/port, both
records remain unless the optional strict attribution proof succeeds: one bounded
`ss --no-header --listening --numeric --processes --tcp --udp --extended` row matches protocol/address/inode; one process fd resolves
to that socket; PID starttime is stable; the executable is trusted root-owned,
non-writable `/usr/bin/docker-proxy`; exact Moby args match the same unique Docker
binding and unique container address. Any ambiguity leaves the procfs holder unknown
and blocks reuse. Missing/unsupported `ss` only disables attribution; it does not
invalidate base Docker+procfs reservation evidence.

Primary-source flag/field contract: [Moby docker-proxy Linux entrypoint](https://github.com/moby/moby/blob/master/scripts/docker-proxy/main_linux.go) and [iproute2 `ss` reference](https://github.com/iproute2/iproute2/blob/main/man/man8/ss.8).

Called by: [`host-support/src/lib.rs`](lib.rs.md) — `port_reservations`.

Contract: [`docs/INTERFACES.md`](../../../../INTERFACES.md), bounded host observations;
policy: [`docs/specs/sdd-port-allocation.md`](../../../../specs/sdd-port-allocation.md).

No product acceptance or compilation claim.

## Native Xray socket attribution

`container_xray_listener(initPid, IPv4, port, Budget, Cancellation)` attributes one
exact TCP LISTEN inode to exactly one process in the Docker init process tree.
The bounded tree reads every task's children; all processes share the init network,
PID and mount namespaces. It requires the pinned `/usr/bin/xray` executable and
exact native `-config /opt/amnezia/xray/server.json` argv, a root-owned regular
executable without group/other write permissions, and a matching socket descriptor.
Process start times, task tree, namespace identities, argv, descriptor and executable
digest are rechecked around the observation. Limit: 256 processes, 1024 tasks,
4096 descriptors per process, 128 MiB executable bytes. No foreign command runs.
Only an executable SHA-256 digest is returned; it is not a software-version proof,
nor native authentication/client compatibility or restart/data-owner evidence.
