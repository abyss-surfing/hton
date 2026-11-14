# hton validate

Выполняет явную изолированную проверку набора в Docker.

## Использование

```sh
./scripts/hton/validate BUNDLE_ID [--json]
```

Это не dry-run: выполняются изменяющие операции Docker.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/validate`](../../../../scripts/validate)
