# hton keys

Создаёт scoped-учётные данные и секреты релиза.

## Использование

```sh
./scripts/hton/keys [KINDS...] [--device ID] [--show] [--force] [--json]
```

`--show` раскрывает учётные данные только при отдельном scoped-разрешении.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/keys`](../../../../scripts/keys)
