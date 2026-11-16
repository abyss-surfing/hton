# hton config

Проверяет желаемую конфигурацию, показывает план без секретов и подготавливает или применяет изменения.

## Использование

```sh
./scripts/hton/config check [--json]
./scripts/hton/config plan [--file FILE] [--json]
./scripts/hton/config stage [--file FILE] [--json]
./scripts/hton/config apply [--plan ID] [--json]
```

`check` и `plan` работают только на чтение; `stage` и `apply` — явные изменяющие операции.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/config`](../../../../scripts/config)
