> ARCHIVE: historical document; current design is [../ARCHITECTURE.md](../../ARCHITECTURE.md). Statements below may be superseded.


> NOTE: это архивная копия. Актуальная архитектура: docs/ARCHITECTURE.md (EN). Здесь имена путей могли устареть (webroot→web/mask, admin-web→web/admin, build→docker).
# hton — «Хтонь»: архитектура

> Один публичный TLS-порт. Наверху — настоящий сайт.
> Внизу, под TLS, — хтонь: vless, naive, mtproto, DoT, passthrough чужих стеков.

## 1. Принципы

- Минимальный обвес для дешёвого хостинга; максимум логики — в Rust-бинарник.
- Источник истины — `.env`; конфиги сервисов рендерятся из шаблонов кодом cli.
- Секреты (`.env`, `.tmp/`, `certs/`) — в `.gitignore`.
- Изменения чужих стеков журналируются, любое откатывается.
- Публичная поверхность: только TLS/QUIC-порты с настоящими сертификатами.

## 2. Где что живёт

| Компонент | Где | Язык/образ | Примечание |
|---|---|---|---|
| Docker engine | host | apt (официальная инструкция) | ставит `hton install-ubuntu` |
| `hton` CLI | host, бинарь | Rust (clap-free, std::env) | все команды: install/diag/share/… |
| systemd unit `hton.service` | host | systemd + rust-бинарь | `sudo service hton {start,stop,restart}` |
| haproxy | контейнер (alpine) | HAProxy 3.2 | единственный владелец публичного TLS |
| xray | контейнер (alpine) | upstream xray-core | VLESS WS/XHTTP + socks-internal |
| caddy+naïve | контейнер (alpine) | Caddy+forwardproxy | CONNECT-ветка + сайт-маска |
| unbound | контейнер (alpine) | unbound | рекурсивный DNS, DoT-бэкенд |
| mtg | контейнер (alpine) | 9seconds/mtg v2 | запасной MTProto (.env-тумблер) |
| acme.sh | контейнер (service) | acme.sh | dns_internetbs, wildcard |
| admin-сервер | контейнер (scratch) | Rust: web (axum) + tg-демон (reqwest) | веб-админка и ТГ-бот — один процесс, два фасада |
| hysteria2 | контейнер (alpine) | hysteria | UDP-порт напрямую (опция) |
| HE sit-туннель | host (kernel) | ip tunnel | вне docker, `hton tunnel` |
| webroot | host dir → volume | HTML (vanilla) | сайт-маска для caddy |
| Apache | host | уже стоит | порт 80, не тронут |

## 3. Сеть

- Bridge-сеть `vpn-net`: внутренний трафик haproxy ↔ xray/caddy/admin/unbound.
- Passthrough чужих стеков: через `host-gateway` → 127.0.0.1:<порт>.
- Docker — ставит cli (install-ubuntu: идемпотентно, официально).

## 4. Роутинг 443 (haproxy: tcp + ClientHello sniffing)

```
SNI ∈ DOMAINS → TLS-терминация (crt-list: LE/unique + wildcard):
   CONNECT (метод)        → caddy+naïve
   путь = VPN_PATH        → xray (WS/XHTTP, TLS нет внутри)
   /.well-known/acme      → acme.sh (webroot)
   /ADMIN_PATH            → admin (веб-админка)
   прочее                 → caddy (сайт-маска)
SNI ∉ DOMAINS (отсутствует) → passthrough: amnezia-xray 127.0.0.1:10001
без ALPN (fakeTLS)        → mtproto: telemt:4431 | mtg:4432 (.env)
порт 853 (TLS)            → unbound:1053
```

## 5. Сертификаты: автоматизация Let's Encrypt

Публичная поверхность при выпуске/обновлении сертификатов — **ноль**:
dns-01 не требует ни 80, ни 443.

Компонент: acme.sh-контейнер (`neilpang/acme.sh`, alpine, digest-пин) в compose,
режим cron-демона (проверка ежедневно). API-доступ: `INTERNETBS_API_KEY/PASSWORD`
из .env мапятся в окружение контейнера.

### Выпуск (первичная выдача)

```
hton certs issue
  для каждого D ∈ DOMAINS (и WILDCARD_DOMAIN):
    acme.sh --issue --dns dns_internetbs -d <D> -d *.<D>
      (контейнер сам ставит TXT _acme-challenge через API InternetBS и ждёт propagation)
    acme.sh --install-cert -d <D> \
      --key-file    certs/<D>/key.pem      \
      --fullchain   certs/<D>/fullchain.pem
      --reloadhook  scripts/reload-haproxy.sh
```

### Каждое обновление (авто, cron внутри контейнера)

```
acme.sh cron (каждые 12ч, продление за 60 дней до конца)
  → renewed pem
  --reloadhook: скрипт внутри контейнера:
      1. собрать pem для haproxy: cat fullchain.pem key.pem > certs/haproxy/<D>.pem
      2. перегенерировать certs/haproxy/crt.list (cli-модуль render)
      3. graceful reload haproxy: docker kill -s HUP haproxy (без разрыва соединений)
```

### Структура certs/ (gitignored)

```
certs/<domain>/             ; native acme.sh: key.pem, fullchain.pem
certs/haproxy/<name>.pem    ; склеенный fullchain+key для haproxy
certs/haproxy/crt.list      ; рендерится из DOMAINS: "<domain.pem> [SNI]"
```

SNI-планы: в crt-list каждая строка — cert + его serverName, поэтому haproxy
выбирает серт по предъявленному SNI; wildcard-строка покрывает все поддомены.

### Отказоустойчивость

- Первый выпуск при падении: haproxy всё равно стартует с dummy/self-signed маской,
  cli ретраит и предупреждает (`state: NO-CERT`), retry-цикл 1).
- `hton certs status`: openssl x509 -enddate каждого pem; <14 дней → « Bouws трревога»
  в TG (admin-daemon) + запись в diag.
- Лог acme.sh: volume (acme-контейнер) → `docker compose logs acme` и веб-админка.
- Пропадание ключей InternetBS/смена доменов → re-render + issue повторно (idempotent).

## 6. Чужие стеки (адаптеры)

- Amnezia: xray-контейнер rebind → 127.0.0.1:10001 + SNI-passthrough (клиенты не замечают).
- `amnezia-awg*`, `amnezia-dns` — не тронуты.
- telemt живёт до тестов Мегафона → затем `MTPROTO_BACKEND=mtg|off`.
- Outline: stop + restart=no (не удаление). Клиентские конфиги operator-two не изменены.
- Reinstall Amnezia снова захватит 443 → `hton update` перенаппит.

## 7. Журнал ремапов / бэкап оригиналов (`.tmp/remap.journal`)

- Каждая запись: ts, контейнер, метод (docker-run-rebind | compose-edit),
  ПОЛНАЯ оригинальная конфигурация (docker inspect + оригинальная команда запуска),
  целевое состояние, шаг отката.
- `hton remap-undo` воспроизводит оригинал.
- Compose-стеки: `docker-compose.orig.<proj>.yml` — копия оригинала; рабочий файл редактируется только после бэкапа.

## 8. IPv6 (t09)

- sit-туннель HE (tunnelbroker), PoP region; регистрация при исполнении.
- `hton tunnel up|update|status`; при смене IPv4 — update endpoint.
- Домен: AAAA → IPv6 туннеля; haproxy слушает `[::]` на PUBLIC_PORT и 853.

## 9. Админка: веб + тг-демон (t13, оба в этом контейнере)

Один Rust-процесс `admin`, два независимых фасада:

- **Веб-админка** (axum): секретный путь `/ADMIN_PATH`; ванильный HTML+JS фронт (без npm);
  кнопки: share (ссылки+QR), status, restart, log views.
- **ТГ-демон** (tokio-таска + reqwest): long polling (не webhook, публиковано ничего лишнего не нужно);
  whitelist ADMIN_TG_IDS; команды /status /share /revoke /restart /logs;
  сюда же падают уведомления doctor/метрики (это и «канал киллер-фич»).
- Общее ядро: users.json → рендер → restart xray (холодно, ~1с — ок);
  docker.sock: только whitelist-действия (restart xray|haproxy).
- Аутентификация: `ADMIN_PASS` (веб) + `ADMIN_TG_IDS` (tg).

Сборка: multi-stage docker (rust:alpine → scratch), бинарник тонкий.

## 10. Установка

```
hton install          → OS-диспетчер: ubuntu|debian → install-ubuntu; иначе install-unknown
hton install-ubuntu   → идемпотентно: docker по официальной инструкции (repo docker-ce),
                        curl, gnupg, ca-certificates, qrencode, age
hton install-unknown  → инструкция + ссылки docs.docker.com, подтверждение, re-check
```

Триады меню установки (все читаются из .env, каждая ЯВНО спрашивается):

- «Ключа нет в .env» → ответ по умолчанию **нет**; «да» = да.
- Изменённый ответ **пишется обратно в .env** (повторный запуск не переспрашивает).
- Список: INSTALL_VLESS / INSTALL_NAIVE / INSTALL_HYSTERIA / INSTALL_DNS / INSTALL_SITE /
  INSTALL_ADMIN / INSTALL_SYSTEMD (=1, автозагрузка service) / INSTALL_PATH (=нет по умолчанию).

## 11. Тестирование и переключение (t11)

Двухфазный протокол:

**Фаза А (безопасная: ничего чужого не трогаем):**
1. новая стоп-стек на PUBLIC_PORT=4443 — все бэкенды живут; ничего чужого не трогаем;
2. read-тесты: `curl -k https://ip:4443` (маска), SNI-домены (site/vless/naive/admin/mtproto-no-ALPN,
   DoT 853, unbound сам резолвит), `openssl s_client`-пробы no-ALPN ветки;
3. живой клиент: vless://, naive://, тг-бот — проверка из Мегафона.

**Фаза Б (переключение на 443, требует СТОП-точку 2):**
1. предварительные проверки: кто держит 443 (ss, docker inspect);
2. если там слушает апач/nginx/любой HTTPS-демон — **явный вопрос**:
   stop+disable systemd юнита, или перепрописать порт, или отменить (ответ → .env: CONFLICT_*); и оно журналируется.
3. backup исходного состояния (docker inspect + файлы) в журнал;
4. remap amnezia-xray → 10001 (СТОП-точка 1 уже пройдена ранее);
5. `hton switch 443`: haproxy перегрузка, acme-сертификат уже живой;
6. post-проверки (список фазы А) + rollback-ключ: `hton remap-undo` + `hton switch 4443`.

## 12. CLI: сборка и команды

Один крейт с тремя бинарями (workspace):

```
crates/
├─ core/   lib: env(.env-парсер), render (шаблоны→конфиги), journal, docker, ask-menu
├─ cli/    bin: hton   — все команды ниже
└─ admin/  bin: admin — веб+тг-демон
```

Команды `hton`:

| Команда | Что делает | Задача |
|---|---|---|
| `hton install` | OS-диспетчер → install-ubuntu/unknown → меню → ремапы → compose up | t02 |
| `hton update` | перегенерация конфигов из .env + graceful reload | t02 |
| `hton diag` | диагностика: docker, порты, контейнеры, .env, серты, туннель | t02 |
| `hton keys` | секреты: UUID, VPN_PATH, MTG_SECRET, NAIVE_*, ADMIN_* | t02 |
| `hton restart [svc]` | сервисные рестарты compose | t02 |
| `hton switch [443\|PORT]` | перестановка PUBLIC_PORT с валидацией и журнал-бэкапом | t10 |
| `hton remap-undo` | откат всех ремапов из журнала | t10 |
| `hton share` | печать всех ссылок + QR | t12 |
| `hton tunnel up\|update\|status` | HE IPv6-туннель | t09 |
| `hton service start\|stop\|status` | systemd-обёртка (`sudo service hton …`) | t02 |

Сборка: docker-builder (rust:alpine) → бинарь на host в `bin/hton` + shim на PATH (по запросу).
Debian-пакеты, которых нет в репо (xray, mtg, acme.sh core) — только **внутри контейнеров**,
на хост ничего, кроме бинаря hton, не ставится.

## 13. Структура проекта

```
~/code/hton/
├── .gitignore              ; .env, .tmp/, certs/, *.crt|key|pem, target/
├── .env.example            ; все переменные с комментариями
├── Cargo.toml              ; workspace: crates/{core,cli,admin}
├── crates/
│   ├── core/               ; lib: env/render/journal/docker/menu
│   ├── cli/                ; bin hton (команды §12)
│   │   └── src/main.rs (+cmd модули)
│   └── admin/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs     ; Токио-рантайм: запускает web + tg демона
│           ├── web.rs      ; axum-админка (секретный путь)
│           ├── tg.rs       ; tg-демон long-polling
│           └── core.rs     ; общее mgr ядро (users/state → render → docker restart)
├── build/                  ; Dockerfile.multi-stage rust:alpine → scratch
├── compose/                ; docker-compose.yml (все сервисы, digest-пины)
│   └── docker-compose.orig.<proj>.yml  ; бэкапы compose чужих стеков
├── etc/
│   ├── haproxy/            ; шаблоны haproxy.cfg + crt.list
│   ├── xray/               ; config.json + users.json
│   ├── mtg/                ; конфиг mtg
│   ├── unbound/            ; unbound.conf
│   └── caddy/              ; Caddyfile (naïve + сайт-маска)
├── webroot/                ; landing/сайт-маска (HTML)
├── systemd/
│   └── hton.service        ; шаблон юнита
├── docs/
│   ├── ARCHITECTURE.md
│   └── tasks/              ; 00-decisions.md + t01…t13.md
└── runtime (не в гите: .tmp/, certs/, data/, .env)
```

Долговременная копия конфигурации и секретов — отдельный приватный Git-репозиторий `.secrets`; отдельного backup/restore в R1 нет.

## 14. Языки

- **Rust** — cli (hton), core (env/render/journal), admin (web + tg-демон). Единственный полноценный язык проекта.
- **HTML+JS (vanilla)** — фронт админки и сайт-маска webroot.
- **YAML/конфиги** — compose, haproxy.cfg, xray.json, unbound.conf, Caddyfile (шаблоны, рендерятся cli).
- **Deb-скрипты** — только distro-специфика install-ubuntu (константная официальная инструкция docker).

## 15. Стоп-точки и безопасность

- СТОП-точка 1: ремап amnezia-xray (t08) — явное согласие.
- СТОП-точка 2: переключение публичного 443 (t11) — явное согласие + подтверждение «что сейчас висит на 443».
- Конфликт с чужим HTTPS-демоном (apache/nginx/…): спро́сит — stop+disable | перепрописать | отменить (ответ → .env).
- SOCKS5 никогда не публичен. IPsec отклонен (сигнатура ISAKMP палит IP).
- telemt — до ваших тестов; потом `MTPROTO_BACKEND=mtg|off`.

## 16. Карта задач

t01 каркас+доки → t02 CLI (rust)+compose → t03 haproxy → t04 xray → t05 naive/mtg
→ t06 unbound/DoT → t07 acme.sh → t08 адаптеры → t09 IPv6 → t10 switch/undo
→ t11 тесты → t12 share/QR → t13 admin (web+tg, Rust).
