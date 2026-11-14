# hton uninstall

Удаляет hton с учётом владения ресурсами.

## Использование

```sh
./scripts/hton/uninstall [--keep-certs] [--keep-env] [--foreign] [--purge]
```

`--purge` необратим; `--keep-certs` и `--keep-env` сохраняют соответствующие данные.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/uninstall`](../../../../scripts/uninstall)
