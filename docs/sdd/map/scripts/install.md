# scripts/install

Stable ID: `cmd.install` · Selected package: `M1-E` · Status: planned M1 contract

Thin rollout entry, no duplicated domain policy. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `run(common flags, source and saved settings) -> exit code`.

## Calls

- `run` -> `shell.project.hton_cli`.

## Called by

Entry/declarative/final-evidence artifact; producer/consumer binding is in the plan.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.

## Historical baseline contract (superseded where M1 differs)

# scripts/install

Stable ID: cmd.install · M0 · source

OR-17/D111 orchestrator. run accepts declared install options/common flags, resolves
configured target and compares physical CWD. Outside target, preflight source,
obtain sudo, invoke scripts/deploy for groups/modes/current-source replacement, cd into
target and invoke target scripts/setup. Inside target skip delivery and invoke setup.
Setup owns all parameter/secret collection and Core entry. Install never duplicates
configuration or Docker logic. Dry-run from source previews delivery read-only.

Shared shell seam: hton_install_target(source, target, literal options). Preserve
quoted arguments and all common flags; propagate deploy failure without setup.
Target/setup missing is actionable failure. No release archive or Rust build runs
here. Canonical input: docs/CLI.md, OWNER-REQUIREMENTS OR-14/16/17,
INTERFACES.md, specs/sdd-minimum-installation.md. New command supersedes older
derived direct-Core/source-location comparison wording.
