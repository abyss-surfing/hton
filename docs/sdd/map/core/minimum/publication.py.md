# core/minimum/publication.py

Stable ID: core.minimum.publication · M0 · source

Owns explicit setup publication preflight. get fetches bounded fixed HTTPS
endpoints without ambient proxies/config. external_ipv4 returns a validated public
IPv4 observation; records queries configured public resolver A/AAAA. check compares
observed records with declared entry addresses, shows universal instructions and
permits a bounded manual retry through the supplied local prompt. It never writes
DNS, requests provider secrets, executes setup, or runs during renew/recover.
Typed results distinguish matched, mismatch and observation failure. Configured
entry topology wins over outbound IP. No raw HTTP responses or credentials leak.
Authority: OR-19/20/21 and sdd-minimum-installation.md.
