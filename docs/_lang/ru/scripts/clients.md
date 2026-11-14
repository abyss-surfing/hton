# hton clients

Показывает, добавляет, удаляет и обновляет клиентов нативного Amnezia (VLESS REALITY) и их share-файлы.

## Использование

```sh
./scripts/hton/clients [--container ID] list [--json]
./scripts/hton/clients [--container ID] add [NAME...] [--json]
./scripts/hton/clients [--container ID] rm NAME
./scripts/hton/clients [--container ID] refresh [NAME...] [--json]
```

`list` требует доступа к Docker-сокету; `add` и `rm` требуют root и меняют нативный конфиг и защищённые share-файлы. Клиента `guest` удалить нельзя.

## Платформа

Только Linux и macOS. Точка входа — Bash-скрипт; в Windows она недоступна.

## Скрипт проекта

[`scripts/clients`](../../../../scripts/clients)
