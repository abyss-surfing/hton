# t07 — Собственные сертификаты и безопасная доставка

Статус: задача реализации I2-C/I3-B; выполнение и приёмка не заявлены.
Контракт: [certs SDD](../../../specs/sdd-certs.md),
[control SDD](../../../specs/sdd-control.md),
[runtime SDD](../../../specs/sdd-runtime-safety.md).

Для HTON-owned потребителей один издатель acme.sh выполняет DNS-01 с явно выбранной
CA Let's Encrypt и InternetBS. Systemd timer запускает короткоживущий закреплённый
контейнер; постоянный ACME daemon отсутствует. DNS credentials выдаются только ACME
из config/secrets по scoped secretRef. Caddy загружает готовые сертификаты и не
становится вторым издателем. Amnezia сохраняет собственный native certificate
workflow; дополнительный слой управления её сертификатами не входит в этот пакет.

ACME пишет только кандидаты и completion marker в фиксированных местах; у него
нет Docker socket и канала исполнения на хосте. Общий apply проверяет SAN, chain,
matching key и expiry, формирует immutable bundle, проходит native admission и
ограниченную доставку через runtime/guardian с проверкой обслуживаемого сертификата.
Не выполнять прямой HUP или запись в accepted mount tree из hook. Ошибка доставки
сохраняет рабочий материал и не вызывает повторную выдачу без необходимости.

Хранение под /opt/hton: work/acme — аккаунт/кандидаты, work/certs — установленные
оригиналы, work/bundles — принятые read-only копии. Защищать issuer state как секреты; отдельного архива R1 не требует — он regenerable, а долговременная копия профиля живёт в приватном `.secrets`. Старые var/acme, var/certs, var/config/generations и .tmp/acme
не являются актуальной привязкой.

Down приостанавливает автоматическое продление/доставку. Явная ручная подготовка
на выключенном сервере не запускает сервисы; up выполняет необходимое согласование.
Без домена/валидного сертификата допустима unpublished/degraded установка;
публичная готовность требует настоящего доверенного сертификата.

Сценарии заключительного F0 после полного I4 и юнит-этапа U0: issue → marker →
apply → served verification, неверный кандидат, пропущенный marker, interrupted
promotion, failed reload, сохранение stopped intent и совместимый откат без возврата
истёкшего сертификата. Для выпуска проверяются реальные выбранные клиенты и сайт.
Staging CA для проверки выдачи не означает обязательный Docker staging каждого
runtime-кандидата; не форсировать production issuance ради теста.
