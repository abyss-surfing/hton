# hton config

Validate the desired configuration, preview a redacted plan and stage or apply changes.

## Usage

```sh
./scripts/hton/config check [--json]
./scripts/hton/config plan [--file FILE] [--json]
./scripts/hton/config stage [--file FILE] [--json]
./scripts/hton/config apply [--plan ID] [--json]
```

`check` and `plan` are read-only; `stage` and `apply` are explicit mutating operations.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/config`](../../scripts/config)
