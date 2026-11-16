# hton secrets

Управляет отдельным репозиторием полного профиля секретов.

## Использование

```sh
./scripts/hton/secrets init [--yes] [--json]
./scripts/hton/secrets import [--json]
./scripts/hton/secrets status [--json]
./scripts/hton/secrets set KEY [VALUE] [--json]
./scripts/hton/secrets --root DIR COMMAND ...
```

`set` запрашивает значение без эха, если VALUE не задан; `status` никогда не печатает значения секретов.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/secrets`](../../../../scripts/secrets)
