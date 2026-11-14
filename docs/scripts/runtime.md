# hton runtime

Inspect and switch the accepted runtime bundle.

## Usage

```sh
./scripts/hton/runtime status [--json]
./scripts/hton/runtime start [BUNDLE_ID] [--json]
./scripts/hton/runtime rollback [BUNDLE_ID] [--json]
```

`status` does not depend on Core, the compiler or YAML and reads only durable state. `start` defaults to last-known-good; rollback is a one-step fallback.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/runtime`](../../scripts/runtime)
