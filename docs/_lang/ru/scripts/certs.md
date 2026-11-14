# hton certs

Выпускает и проверяет TLS-сертификаты для настроенных имён сайта.

## Использование

```sh
./scripts/hton/certs issue [--domain DOMAIN]... [--force] [--json]
./scripts/hton/certs status [--json]
./scripts/hton/certs list [--json]
```

`issue` — подкоманда по умолчанию и использует только настроенные имена. `status` показывает SAN, срок действия и фактически отдаваемый сертификат.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/certs`](../../../../scripts/certs)
