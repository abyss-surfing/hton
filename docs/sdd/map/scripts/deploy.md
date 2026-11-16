# scripts/deploy

Stable ID: `cmd.deploy` · Selected package: `M1-E` · Status: planned M1 contract

Literal minimum runtime closure delivery and pruning. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `run(source-dir, trusted installation root, common flags) -> DeployReceipt/exit`.

## Calls

No callable dependency in the selected overlay; injected observations/typed data only.

## Called by

- `shell.project.hton_cli` -> `run`.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.

## Historical baseline contract (superseded where M1 differs)

# scripts/deploy

Stable ID: `cmd.deploy` · Phase: `R1` · Kind: source

Privileged local source-tree delivery into `/opt/hton`; one owner of group setup,
source snapshot installation and active-source switching. Contract source:
OR-11/OR-14, D99–D100/D103, installer SDD, shared interfaces.

## Features and responsibility

- CLI
- safety
- current-source deployment

## Methods, arguments and result

- `run` — arguments: optional `--source-dir ABSOLUTE_DIR`, `--dry-run`, `--json`, `--quiet`, `--yes`; result: exit code and redacted source snapshot ID/current state.
- `inventorySource` — arguments: canonical working-tree root; result: bounded inventory of current product files, excluding `.git`, `docs/_lang/ru/tasks/`, generated build output, credentials and mutable state.
- `stageSource` — arguments: verified source inventory and fixed `/opt/hton` root; result: root-owned `deployments/<tree-digest>` candidate copied from current files, or exact staging cleanup.
- `activateSource` — arguments: staged source snapshot ID; result: atomic `current` symlink replacement; previous snapshots remain available.
- `configureGroup` — arguments: euid 0 and optional `SUDO_USER`; result: system group `hton`, invoker added when present, code group/modes configured; no sudoers change.

## Structures and injected dependencies

Standalone Python 3 entry for bounded source inventory, path/copy validation and atomic activation. Fixed target root; no shell fragments, network fetch, compiler, Docker or systemd action.

Dependencies: Python 3, `groupadd`, `usermod`; trusted local sudo invocation; current working source tree. Optional `libexec` files are copied when present and are never built here.

## Logic

- Infer the current working tree from the command location unless `--source-dir` is supplied; require a canonical local directory.
- Inventory and validate the source before requesting sudo; when invoked without root, re-exec through `sudo` with bounded original argv. Dry-run stays unprivileged.
- Copy current working files, including uncommitted changes, from the product source allowlist (`adapters/`, Rust crates, `bin/`, `scripts/`, `lib/`, `modules/`, `schemas/`, `etc/`, `systemd/`, `compose/`, `docker/`, `web/`, `scripts/`, and build manifests); `.git`, `docs/_lang/ru/tasks/`, `target/`, `var/`, credentials and mutable state are never copied. Exclude generated `etc/unbound/root.key` explicitly.
- Initialize only an absent/empty unmanaged `/opt/hton`; refuse non-empty roots without the HTON deployment marker.
- Preserve `/opt/hton/config`, `work`, `logs` and deployment snapshots; do not recursively change persistent-state permissions.
- Hash source files, copy exact inventory into root-owned private staging, then verify staged content hashes.
- Open source paths relative to directory descriptors without following symlinks;
  bound streaming copies to the inventoried size. Serialize promotions with a
  protected deployment lock and re-inventory the source before switching.
- Reused snapshots must retain exact root/group ownership and file/directory
  modes; reject changed permissions as well as changed bytes. Flush staged files,
  directories and link parent directories before reporting durable activation.
- Reuse a content-addressed snapshot only after its complete path inventory matches; reject extra files, including excluded task/build content.
- Store the copied tree at `/opt/hton/deployments/<tree-digest>` with root:`hton` ownership; directories 0750, ordinary files 0640, executable files 0750.
- Promote through one atomic `/opt/hton/current` symlink rename. Stable `/opt/hton/bin/hton` and top-level code links resolve the current source tree; config/work remain at the stable root.
- Create group `hton`; add the invoking sudo user if one is present. Group membership grants code read/execute only and no sudoers entry.
- No network, build, service start/restart, config mutation or foreign-resource effect. Copy hashes establish transfer integrity, not source publisher authenticity.
- Repeated same-tree deploy is a no-op; retain prior source snapshots for recovery. Remove only exact incomplete staging paths created by this invocation.

## Calls / reads

- Current product files in the working directory — direct deployment source; no clean commit or provenance manifest required.
- `scripts/install` — invokes this owner before the first Core operation when running outside the active `/opt/hton` source snapshot.

## Called / read by

- `bin/hton` — public `deploy` command dispatch.
- `scripts/install` — initial install/update deployment handoff.

## Errors and completion

Unsafe path, missing source files, existing unmanaged target, missing sudo authority or copy/hash mismatch fail before active-source switch. Missing prebuilt Core/runtime helpers are reported without compilation. A switch failure leaves the previous `current` intact. The command never reports service readiness or certificate status.

Minimum binding: all five core/minimum Python sources are required. Nested Git,
bytecode, secrets directories, .env and certificate/private-key outputs are
excluded. Report minimum installer availability independently of Rust helpers.

Resolve installation.directory through lib/install_config.py before target
preflight. The existing bounded /opt protections apply to that selected target.
Copy bootstrap etc/install.json as a non-secret product input, never as a live
config overwrite. Installed settings/work stay at the configured stable root.

Human delivery success says OK: Code copied to TARGET. Errors begin ERROR.
Snapshot/helper availability stays in JSON metadata, not default operator output.

Current M1 delta: preserve host-access/ as a declared mutable top-level tree for
receipt-owned SSH homes; never copy it from source or prune it as stale product
code. This does not authorize changing config/work permissions to allow traversal.
