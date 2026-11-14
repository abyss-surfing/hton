# hton certs

Issue and inspect TLS certificates for the configured site names.

## Usage

```sh
./scripts/hton/certs issue [--domain DOMAIN]... [--force] [--json]
./scripts/hton/certs status [--json]
./scripts/hton/certs list [--json]
```

`issue` is the default subcommand and uses configured names only. `status` reports SANs, expiry and the served certificate.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/certs`](../../scripts/certs)
