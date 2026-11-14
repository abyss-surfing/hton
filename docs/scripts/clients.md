# hton clients

List, add, remove and refresh native Amnezia (VLESS REALITY) clients and their share files.

## Usage

```sh
./scripts/hton/clients [--container ID] list [--json]
./scripts/hton/clients [--container ID] add [NAME...] [--json]
./scripts/hton/clients [--container ID] rm NAME
./scripts/hton/clients [--container ID] refresh [NAME...] [--json]
```

`list` needs access to the Docker socket; `add` and `rm` need root and rewrite the native config and protected share files. The `guest` client cannot be removed.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/clients`](../../scripts/clients)
