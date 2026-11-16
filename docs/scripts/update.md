# hton update

Fast-forward origin/main and reinstall the HTON-owned composition.

## Usage

```sh
./scripts/hton/update [--dry-run]
```

Foreign containers are untouched; `--dry-run` reports the plan without network or Docker effects. A daily check updates only after a week without install or update.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/update`](../../scripts/update)
