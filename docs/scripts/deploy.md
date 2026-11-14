# hton deploy

Copy the minimum runtime closure into the configured target directory without building or downloading anything.

## Usage

```sh
./scripts/hton/deploy [--source-dir ABSOLUTE_DIR] [--dry-run] [--json] [--quiet] [--yes]
```

Copies the Python minimum installer, shell entry points, operator docs and the pinned image inventory. Mutable state and full-R1 sources are excluded. Files are replaced individually under the delivery lock. If delivery detects source changes, repeat delivery from a stable source before running setup. `--dry-run` only reports the plan.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/deploy`](../../scripts/deploy)
