# hton install

Install or reconcile the stack: deploy the current source when it differs, then run the installed setup as root.

## Usage

```sh
./scripts/hton/install [OPTIONS]
./scripts/hton/install --help
```

Run from the cloned project root. Common options include `--profile`, `--site-domain`, `--certificate-email` and `--with`/`--without`; `--help` lists the exact set. On Ubuntu/Debian, missing Python 3 is bootstrapped with root/sudo and ordinary package-install consent before target resolution. Minimum setup then bootstraps its remaining Python and Docker dependencies. A preview without Python stops with guidance and installs nothing. Existing foreign containers are remapped only with explicit local consent.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/install`](../../scripts/install)
