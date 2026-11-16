# hton secrets

Manage the standalone full-profile secrets repository.

## Usage

```sh
./scripts/hton/secrets init [--yes] [--json]
./scripts/hton/secrets import [--json]
./scripts/hton/secrets status [--json]
./scripts/hton/secrets set KEY [VALUE] [--json]
./scripts/hton/secrets --root DIR COMMAND ...
```

`set` prompts without echo when VALUE is omitted; `status` never prints secret values.

## Platform

Linux and macOS only. The entry point is a Bash launcher and is not available on Windows.

## Project script

[`scripts/secrets`](../../scripts/secrets)
