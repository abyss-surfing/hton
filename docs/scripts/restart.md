# hton restart

Restart the named services, or all enabled optional and data services when none are named.

## Usage

```sh
./scripts/hton/restart [SERVICE...] [--wait SECONDS]
```

The default wait is 10 seconds.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/restart`](../../scripts/restart)
