# hton service

Совместимый делегат к командам modules, config и lifecycle.

## Использование

```sh
./scripts/hton/service ...
```

Отдельной бизнес-логики нет; используйте `modules`, `config` или `up`/`down` напрямую.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/service`](../../../../scripts/service)
