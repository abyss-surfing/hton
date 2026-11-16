# hton install-ubuntu

Устанавливает недостающие системные зависимости на Ubuntu или Debian (внутренний помощник).

## Использование

```sh
./scripts/hton/install-ubuntu [--dry-run] [--yes]
```

Скрыт из публичной справки и вызывается установщиком автоматически. Использует apt и отказывается заменять конфликтующие пакеты Docker или containerd.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/install-ubuntu`](../../../../scripts/install-ubuntu)
