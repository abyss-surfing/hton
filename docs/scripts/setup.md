# hton setup

Run setup from the configured target directory, collecting missing parameters and secrets.

## Usage

```sh
./scripts/hton/setup [OPTIONS]
./scripts/hton/setup --help
```

The physical working directory must equal the configured target. Setup can enable MTProto, Hysteria2 and the project's own Outline WSS; `--help` lists the exact options.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/setup`](../../scripts/setup)

## Tartarus workspace product setup

From the Tartarus repository root, `./hton/setup` configures the HTON server
product and offers to download missing server services into `hton/services/`.
It delegates to `scripts/setup --section hton`; it is separate from this HTON
server installation command.

## Standalone Eidolon

To check a separately installed Eidolon checkout before server setup, pass its
local path as the first option:

```sh
./scripts/setup --with-eidolon /path/to/eidolon
```

HTON still manages the lifecycle of its installed services. The option checks
the companion CLI; it does not change the service controller.
