# hton install-unknown

Print manual dependency guidance for unsupported operating systems and check for the required tools (internal helper).

## Usage

```sh
./scripts/hton/install-unknown
```

Takes no positional arguments and is hidden from public help. It exits 3 when required dependencies are missing.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/install-unknown`](../../scripts/install-unknown)
