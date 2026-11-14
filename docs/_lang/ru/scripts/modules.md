# hton modules

Показывает каталог модулей и управляет составом и жизненным циклом модулей.

## Использование

```sh
./scripts/hton/modules list [--json]
./scripts/hton/modules add ID
./scripts/hton/modules enable ID... [--save-only]
./scripts/hton/modules disable ID... [--save-only]
./scripts/hton/modules remove ID
./scripts/hton/modules purge ID
./scripts/hton/modules update ID
```

`add` устанавливает модуль выключенным; `remove` сохраняет данные, а `purge` безвозвратно их удаляет.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/modules`](../../../../scripts/modules)
