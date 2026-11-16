# hton diag

Собирает диагностические данные о стеке только для чтения.

## Использование

```sh
./scripts/hton/diag [--json] [--verbose] [--section SECTION]...
```

Диагностика не меняет конфигурацию и состояние runtime.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/diag`](../../../../scripts/diag)
