# hton update

Обновляет origin/main fast-forward и переустанавливает состав, принадлежащий HTON.

## Использование

```sh
./scripts/hton/update [--dry-run]
```

Чужие контейнеры не затрагиваются; `--dry-run` показывает план без сети и Docker. Ежедневная проверка обновляет только после недели без install или update.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/update`](../../../../scripts/update)
