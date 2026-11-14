# hton switch

Cut publication over to a different public TLS port.

## Usage

```sh
./scripts/hton/switch [PORT] [--validate-only] [--json]
```

Public 443 and foreign resources require exact approval; a cutover may disconnect clients.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/switch`](../../scripts/switch)
