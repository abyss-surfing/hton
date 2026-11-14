# host-support/src/lib.rs

Stable ID: `infra.host` · Phase: `R1` · Kind: source · Wave: I3-B

Typed host observer composition and shared limits. Source written; no compilation/product acceptance claimed (D81).

## Methods and responsibility

- `new` — none → shared HostObserver.
- `docker` — none → DockerObserver.
- `systemd` — none → SystemdObserver.
- `network` — none → fixed HTTPS public IPv4 / DNS-A observation adapter.

Authority and limits: [INTERFACES](../../../../INTERFACES.md), I3 bounded native observation transport / D81.

Calls [host-support/src/docker.rs](docker.rs.md) — docker → declarative dependency.

Calls Docker reservation snapshot for collision-only complete TCP/UDP host-binding
evidence; calls fixed Linux `/proc/net` reader for host socket reservations. These
observations do not grant foreign ownership or mutation authority and return
incomplete on churn, bounds, or parse/inspection failures.

`HostObserver.port_reservations(Budget, Cancellation)` composes the two bounded
read-only sources, sorts and deduplicates reservations, and returns no partial
result when either base source is incomplete. Optional `ss` attribution enriches a
procfs row only after exact PID/FD/inode/executable/argv/Docker-binding proof; any
uncertain row remains unknown and blocks reuse.

Calls [host-support/src/systemd.rs](systemd.rs.md) — systemd → declarative dependency.

No Core/runtime contracts, shell, mutation APIs, secret serialization or ownership discovery. Driver errors are closed/redacted; unavailable facts cannot imply success. Consumers must qualify ownership/admission/fingerprints independently. Public IPv4/DNS-A probes are called only by explicit `identity.whoami`; diagnostics do not request the network.

The explicit `validation` module exposes only a fixed-command, operation-scoped
Docker sandbox driver. It uses the same bounded private Runner and never accepts
caller Compose, shell commands, production port publications or foreign mounts.
