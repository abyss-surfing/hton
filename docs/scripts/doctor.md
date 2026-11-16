# hton doctor

Report local read-only observations about services, runtime and Docker.

## Usage

```sh
./scripts/hton/doctor [--json]
```

`--fix` and `--watch` are declared but not implemented; they return a structured incomplete result.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/doctor`](../../scripts/doctor)
