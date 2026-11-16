# hton uninstall

Remove hton in an ownership-aware way.

## Usage

```sh
./scripts/hton/uninstall [--keep-certs] [--keep-env] [--foreign] [--purge]
```

`--purge` is destructive; `--keep-certs` and `--keep-env` retain the corresponding data.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/uninstall`](../../scripts/uninstall)
