# core/minimum

Stable ID: core.minimum · minimum installation · source family

Ownership: sdd-minimum-installation.md. storage.py owns protected state/settings;
docker.py owns bounded exact-ID snapshot/remap; certificates.py owns HTTP/DNS ACME and trust; publication.py owns explicit setup
DNS/IPv4 preflight;
ingress.py owns immutable artifacts/probes; protocols.py owns optional MTProto,
Hysteria2 and Outline-WSS settings, artifacts, the native own-SNI rewrite and
lifecycle helpers; install.py owns Core sequencing, consent and the reversible
native migration transaction; updates.py owns the seven-day schedule stamp and the
bounded `main` fast-forward used by install and update.
The source helper needs Python 3, no compiler. Broader Rust R1 is not an active
controller in minimum mode. Product text is English; no secret native output.
