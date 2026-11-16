# hton diag

Collect read-only diagnostic evidence for the stack.

## Usage

```sh
./scripts/hton/diag [--json] [--verbose] [--section SECTION]...
```

Diagnostics do not change configuration or runtime state.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/diag`](../../scripts/diag)
