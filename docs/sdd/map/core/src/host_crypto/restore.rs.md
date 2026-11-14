# core/src/host_crypto/restore.rs

R1 I3-B. RuntimeRestore implements RestoreRuntime through independent typed runtime
IPC only. StopAll must confirm no survivors, then ReserveMaintenance CAS; every
write verifies exact operation-owned reservation/epoch, stopped-disabled receipts and
nondecreasing credential floor. Authenticated staged runtime metadata publishes only
via RestoreStopped; archived running intent/epoch/resources are never replayed.
Release requires exact committed Restore terminal and reservation owner; an absent
reservation is idempotent completion only after that terminal proof. TTY proof scope
is archive+manifest+instance-bound, never --yes. All native/metadata mutations remain
under runtime state/admission owners, no Core raw state overwrite.
