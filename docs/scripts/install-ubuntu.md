# hton install-ubuntu

Install missing system dependencies on Ubuntu or Debian (internal helper).

## Usage

```sh
./scripts/hton/install-ubuntu [--dry-run] [--yes]
```

Hidden from public help and called automatically by the installer. It uses apt and refuses to replace conflicting Docker or containerd packages.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/install-ubuntu`](../../scripts/install-ubuntu)
