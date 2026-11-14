# hton validate

Run an explicit isolated Docker qualification of a bundle.

## Usage

```sh
./scripts/hton/validate BUNDLE_ID [--json]
```

This is not a dry run: it performs mutating Docker work.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/validate`](../../scripts/validate)
