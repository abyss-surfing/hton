# core/minimum/storage.py

Stable ID: `core.minimum.storage` · Selected package: `M1-A` · Status: planned M1 contract

Protected files, journal/receipt custody and atomic commits. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `Store(validated installation root, scope) -> protected store handle`.

- `commit(read-set, allowlisted mutation, journal intent) -> new revision or conflict`.

- `modes() -> (directory mode, file mode)`; `0750/0640` with group `hton` when it exists, else `0700/0600`.

- `adopt_tree(runtime root) -> normalized operator-readable tree` (OR-46).

- `safe_file(lstat record, owner) -> bool` for owner-held regular files in either accepted mode.

- `parse_document(bytes) -> mapping` checks configuration syntax from a single
  read and reports bounded errors without embedding source text.

- `write_share_file(directory descriptor, filename, bytes, uid, gid)` replaces
  a private artifact atomically and flushes the file and directory.

## Calls

No callable dependency in the selected overlay; injected observations/typed data only.

## Called by

- `core.minimum.configuration.load` -> `Store`.

- `core.minimum.migrations.apply` -> `commit`.

- `core.minimum.migrations.apply` -> `modes`.

- `core.minimum.install.main` -> `adopt_tree`.

- `core.minimum.docker.save_image` -> `modes`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Runtime trees are `root:hton` `0750/0640` so the
operator group can read them (OR-46); `Store` also admits legacy `0700/0600`
entries and falls back to legacy modes when the group cannot be applied. Product tests run only after all M1 packages integrate.

## Historical baseline contract (superseded where M1 differs)

# core/minimum/storage.py

Stable ID: `core.minimum.storage` · M0 · source

Protected atomic settings/state and finite parsing.

Contract: [minimum installation](../../../../specs/sdd-minimum-installation.md).
Shared interfaces: docs/INTERFACES.md. Inputs are bounded local values or protected
Store records; results are verified records or InstallError without raw native
output. Dependencies follow the family contract. State/effects are confined to
/opt/hton and exact admitted Docker IDs; install alone coordinates phases.
