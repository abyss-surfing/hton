# hton usage

Print per-user Outline WSS traffic collected by the local metrics collector.

## Usage

```sh
./scripts/hton/usage [--json] [--collect]
```

`--collect` stores a new sample without printing. Run with sudo because protected state is root-only.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/usage`](../../scripts/usage)
