# hton remap-undo

Restore foreign containers remapped by hton, using the remap journal.

## Usage

```sh
./scripts/hton/remap-undo [last|all|ID]
```

Restore checks for drift and preserves original data; a missing selector requires a TTY.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/remap-undo`](../../scripts/remap-undo)
