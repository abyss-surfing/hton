# host-support/src/docker_remap.rs

Stable ID: `infra.host-docker-remap` · minimum installation · source

Fixed Docker loopback publication mechanics for OR-15. Depends on protected
ForeignSnapshotStore, immutable Docker identity observation and bounded private
Runner. No policy/association/consent is inferred here.

`prepare(snapshot,port,budget,cancel)` verifies protected snapshot custody and
exact original identity, admits only no-mount default bridge/native TCP+UDP 443,
persists complete private create intent before Engine creation, and returns a
stopped snapshot-backed replacement ID with exact `127.0.0.1:port` bindings.
The 5000–6000 port comes from its durable settings owner. Preserve Config and
HostConfig; initial restart is disabled until the coordinator verifies cutover.
Docker API requests use a fixed local Unix socket, validated server API version
and finite JSON stdin; no shell, remote URL or caller Docker arguments.
Interrupted create resolves only its exact protected intent/name/image/config;
unknown collisions or launch drift block without adopting/deleting anything.

`resume(snapshot,budget,cancel)` reconstructs only the private recorded candidate
identity, never a discovered replacement. `cutover(handle,budget,cancel)` records
each intended transition before disabling original autostart, stopping/renaming
the original, naming/starting the replacement and confirming its loopback TCP bind.
Native config bytes must match the snapshot-backed candidate before and after
stopping the original. `accept` restores the original restart policy on the new
container only after caller-owned route/handshake verification.
`restore` disables/stops only that replacement, restores the exact original name,
restart policy and prior running state, and retains both containers/snapshot.
All steps compare exact captured launch/config metadata, allow only declared name,
image, host-binding and restart changes, and preserve durable intended-phase state
for idempotent resume. Unknown drift or missing original refuses mutation.
