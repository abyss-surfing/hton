# host-support/src/docker_snapshot.rs

Stable ID: `infra.host-foreign-snapshot` · R1 · I3-B

Privileged protected Docker rootfs snapshot custody. Fixed root
`/opt/hton/work/foreign-snapshots`, unique validated 64-hex child IDs, SecureDir
0700/0600 descriptor-relative writes, per-store bounded lock. Dependencies:
DockerObserver exact inspect, private fixed Runner, SecureDir/StreamWriter,
SHA-256, serde. No shared policy, shell, arbitrary path, deletion or native config
editing. Capture requires prior coordinator journal/consent.

`ForeignSnapshotStore::open()` opens existing private store;
`initialize()` explicitly creates it as root. `capture(exactId, fingerprint,
snapshotId, Budget, Cancellation)` preserves complete protected inspect metadata,
rejects mounts/declared volumes/binds/tmpfs, journals a private prepared record,
commits exact container with pause=true into deterministic local snapshot tag,
refuses an already-existing deterministic snapshot tag, streams immutable image save <=2 GiB to protected image.tar, hashes/fsyncs and
publishes a completion manifest only after stable source reinspection. Existing
completed capture is reusable only after exact source binding and archive/manifest
validation. Incomplete capture returns Busy preserving all recovery material.

`verify(snapshotId, Budget, Cancellation)` returns nonsensitive handle metadata
after private-file permissions and protected manifest/inspect/archive digest verification. No whole archive is
held in memory. Snapshot bytes include secrets and are never serialized in reports
or Debug. Docker commit may briefly pause the original; no original container or
image is removed, renamed or reconfigured. Incomplete daemon-side commit cannot be
inferred cancelled from CLI failure. Remap/restore execution and mount snapshot
ownership remain separate unimplemented boundaries.
