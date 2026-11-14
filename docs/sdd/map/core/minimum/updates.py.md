# core/minimum/updates.py

Stable ID: `core.minimum.updates` · Selected package: `M1-E` · Status: OR-55 implemented (unit evidence only)

Owns the seven-day unattended-update policy and the fast-forward of the deployed
source checkout. Canonical requirement: OR-55 / D160. This contract does not imply
live acceptance.

## Methods and typed results

- `record(store, kind, now?) -> None` — write the protected `update.json` stamp
  (`schemaVersion`, `lastSuccessAt`, `kind`, `intervalSeconds`).

- `last_success(store) -> int | None` — validated timestamp or None when the
  stamp is absent, invalid or unreadable as a schedule record.

- `due(store, now?, interval=604800) -> bool` — True when the stamp is absent, at
  least one week old, or the clock moved backwards.

- `remaining(store, now?, interval=604800) -> int` — bounded seconds until due.

- `fast_forward(root) -> {status, reason, before?, after?}` — bounded status;
  `updated`, `current`, `skipped` or `failed`. Only a `main` branch is updated,
  with `git fetch origin main` and `git merge --ff-only origin/main`. Git output
  is not propagated.

- `main(due|fast-forward, args) -> exit code` — CLI for `lib/hton`: `due`
  returns 0 (due), 1 (not due), 2 (unsafe/unreadable schedule); `fast-forward`
  returns 1 only when `--required` and the fast-forward failed.

## Calls

- `due`/`remaining`/`record` -> `core.minimum.storage.Store`.

- `fast_forward` -> `git` as a bounded subprocess (no shell).

## Called by

- [scripts/update](../../../map/scripts/update.md) — `run` -> `due`,
  `fast_forward`.

- [lib/hton](../../../map/lib/hton.md) — `hton_source_update` ->
  `fast_forward`; `update` dispatch -> `due`.

- [core/minimum/install.py](install.py.md) — `main(update --auto)` -> `due`;
  `emit_ready` -> `record`.

## State, effects and errors

Only the protected work store and the source checkout are touched. The stamp is
advisory: failing to write it never fails a working installation. No network
operation runs in `--dry-run`; no prompt, public DNS/identity lookup, Docker
mutation or foreign-container access belongs to this module.
