# core/src/lock.rs

Stable ID: `core.lock` · Phase: `R1` · Kind: source

Source/binding is written; compilation evidence is tracked in IMPLEMENTATION-STATUS.md. Product acceptance is not claimed. Purpose: Single project mutation lock

Implementation wave: `I1-B` (ownership assignment; completion evidence is in the status ledger).

## Features and responsibility

- concurrency

## Methods, arguments and result

- `acquire` — arguments: projectId, timeout; result: LockGuard.

## Structures and injected dependencies

LockGuard; bounded RAII handle for short-held mutation lock

Dependencies: OS locking primitive and monotonic timeout clock; no nested management/runtime wait.

## Logic

- Serialize desired/planning edits with config lock.
- Do not hold the management lock while awaiting runtime trial; runtime owns short-held effect lock and durable lease fencing.
- No recursive child acquisition or indefinite guardian wait on failed management.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- [core/src/apply.rs](apply.rs.md) — `apply` -> `acquire`.
- [core/src/backup.rs](backup.rs.md) — `backup` -> `acquire`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## Concrete implementation binding (2026-10-31)

Concrete R1 binding: a mutation guard wraps the shared hton-storage file lock.
`acquire` receives a validated project ID, monotonic deadline and cancellation port;
readOnly mode cannot create a lock file. OS lock acquisition is bounded, RAII releases
on drop, and callers release management locks before any runtime-trial wait.

## Implemented ownership details

Implemented acquire uses OperationContext deadline/cancellation, caps one wait at
30 seconds, rejects readOnly and delegates all lock mechanics to SecureDir. The
project ID is trusted correlation metadata; Directory capability fixes lock scope.
