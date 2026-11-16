# hton modules

Inspect the module catalogue and manage module membership and lifecycle.

## Usage

```sh
./scripts/hton/modules list [--json]
./scripts/hton/modules add ID
./scripts/hton/modules enable ID... [--save-only]
./scripts/hton/modules disable ID... [--save-only]
./scripts/hton/modules remove ID
./scripts/hton/modules purge ID
./scripts/hton/modules update ID
```

`add` installs a module disabled; `remove` preserves its data while `purge` deletes it irreversibly.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/modules`](../../scripts/modules)
