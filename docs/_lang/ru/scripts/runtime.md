# hton runtime

Показывает и переключает принятый runtime-набор.

## Использование

```sh
./scripts/hton/runtime status [--json]
./scripts/hton/runtime start [BUNDLE_ID] [--json]
./scripts/hton/runtime rollback [BUNDLE_ID] [--json]
```

`status` не зависит от Core, компилятора и YAML и читает только устойчивое состояние. `start` по умолчанию берёт last-known-good; rollback — одношаговый откат.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/runtime`](../../../../scripts/runtime)
