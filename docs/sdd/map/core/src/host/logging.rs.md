# core/src/host/logging.rs

Stable ID core.host-logging · R1 · I3-A

project(app,transaction,ctx) validates desired logging policy through Core load,
requires exact journal actor/operation, publishes bounded runtime/logging.json and
opens private configured log directory for closed LogSink events. Dry-run does not
open/create/rotate logs. Host runtime/collectors consume JSON projection without
YAML/compiler dependency. File budget is a ceiling, not promised used capacity;
actual fixed ring uses at most16x1MiB and never exceeds requested maxBytes. The
projection defaults retentionDays to 7 and rejects values outside 1..=7 (OR-53).
