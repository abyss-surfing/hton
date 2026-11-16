# SDD certs — certificate lifecycle and publication policies

## Purpose

For the public product (OR-19/20/21), issuer/challenge policy is per configured
domain/publication, with independent node state. HTTP-01 and DNS-01 are distinct
strategies; DNS providers are adapters, not a mandatory registrar. Shared apply
distributes validated material to the declared TLS consumers. D115 adds default HTTP-01 in the minimum acme.sh lifecycle; explicit DNS-01 uses
the existing InternetBS adapter. Broader fleet/provider strategies below remain
product contracts, not completed implementation. See CERTIFICATE-CHOICE.md for researched policy and gaps.

## Requirements

- CERT-1 HTTP-01 requires no DNS credentials. For DNS-01, DNS credentials resolve from hton.yaml secretRef values in config/secrets/ and are given only to ACME. acme.sh
  may persist them in work/acme/: protect that state; it is regenerable and is not part of the durable `.secrets` profile.
- CERT-2 Deduplicate explicit configured domain names, wildcard base plus wildcard SAN, and enabled
  endpoint names. Do not automatically request a wildcard under every host.
- CERT-3 Reuse valid matching material unless force is explicitly requested.
- CERT-4 A persistent systemd timer runs a short-lived pinned ACME container, explicitly selecting the Let's Encrypt CA and respecting the client/CA renewal policy, targeting renewal
  within the validity-derived window (one third, bounded to 1–30 days); diagnose failed or delayed renewal.
- CERT-5 The hook writes candidate material and a completion marker only. A host
  timer consumes fixed paths and runs shared apply: validate SANs/chain/key/expiry,
  build Caddy certificate files plus optional HAProxy DoT PEM/crt-list, native config checks, reload and served
  certificate verification. ACME has no Docker socket or host execution channel.
- CERT-6 Report per-name expiry; below 14 days is unhealthy and notifies admin
  if enabled. Rate-limit repeated warnings.
- CERT-7 work/acme/ owns account/candidates; work/certs/ holds installed originals;
  generation directories hold service-specific read-only active copies. Include
  all certificate consumers, including enabled Hysteria, in the plan.
- CERT-8 Failed first issue allows dummy NO-CERT bootstrap only; production
  cutover requires trusted valid certificates. Failed renew retains known-good.

## Interfaces and failure modes

CLI syntax is in ../CLI.md. Marker contents never supply arbitrary executable
commands or file paths. Reconciliation is idempotent and lock-protected. Down suspends automatic HTON renewal. Explicit manual preparation while stopped never starts services; delivery reconciles on up. Caddy uses explicitly loaded certificates and is not a second issuer. Retry failed delivery independently from issuance; do not
issue again merely because a reload failed. DNS API logs redact credentials.

## Acceptance

- Expected SANs, trusted chain and matching key; SNI serves installed material.
- Invalid candidate retains working certificates and desired state.
- Staging lifecycle test covers issue -> marker -> apply -> served verification,
  verifying Caddy long-lived stream behavior and, if enabled, HAProxy DoT reload. No resident ACME daemon is required.
- Missed marker/reconciliation and interrupted promotion recover on the next tick.
- Production certificates have normal remaining validity; routine tests do not
  require a fresh >=60-day certificate or force production issuance.

## Runtime admission boundary

Delivery uses the independent executor/guardian in
[sdd-runtime-safety.md](sdd-runtime-safety.md). Hooks cannot mutate accepted mount
trees directly. Admit a validated bundle, then perform bounded native reload and
served verification. Scheduling/delivery work without the web admin/database.
Failed delivery retains valid material; rollback never blindly restores an expired
certificate.

## Current storage and external certificate scope

Resolve own secrets from config/secrets, retain issuer/cert state in work/acme
and work/certs, and accepted copies in work/bundles. Paths are under /opt/hton.
Owner D80 accepts native Amnezia REALITY authentication semantics; the genuine
own-domain certificate is required for the ordinary HTTPS site. Any native
Amnezia certificate workflow stays externally owned; the shared issuer applies to
HTON-owned consumers and must not introduce an Amnezia certificate-management
layer. Validate native compatibility before integration. No-domain installation
may finish unpublished; trusted public readiness is a separate result.

## Install integration for an owned HTTPS site

`scripts/install` calls `install.reconcile`; the `InstallationCoordinator` reads the
configured site names and certificate `secretRef`, then delegates issue/reuse to
`CertificateCoordinator.issue` and distribution to the normal candidate/apply
path. It never embeds a domain in the installer or copies a private key directly
into HAProxy/Caddy. For the current instance `example.com` is an input value, not a
global default. A valid matching certificate is reused; a missing/renewal-due
certificate is obtained with the existing pinned acme.sh + Let's Encrypt DNS-01
InternetBS workflow. DNS API credentials are resolved from protected SecretStore
grants and are never CLI argv, environment sourced by Bash, logs, plans or errors.
The existing `hton-acme.timer` continues the same bounded renewal/apply lifecycle.

The certificate candidate must pass SAN, chain, key-match and validity checks and
be bound to the exact desired config read set before public edge activation. A
missing credential, issuer failure or invalid candidate leaves the current listener
untouched and installation unpublished/degraded; no HTTP-01/certbot alternative is
implicitly selected. In this host, public port 80 currently belongs to Apache and
is not taken over as an issuance fallback. Public DNS-A readiness remains
unverified during install under OR-8; successful DNS-01 proves control for issuance,
not that public traffic resolves to the host. A served certificate check is done
against the local admitted edge; public identity/reachability probes require an
explicit `identity.whoami` call.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

## Public-product and fleet constraints (OR-21/D114)

NodeConfig, domain/publication policies, named issuer/DNS account references and
private certificate state have separate ownership. Issuance groups declared names
without silently dropping later domains. HTTP-01 requires a consistent challenge
route across all public front ends for that name; DNS-01 requires concurrent TXT
value preservation and token-specific cleanup. Validation checks expected public
entry points, not unconditional equality with one node's outbound address.

Certificates are reused across deploy/restarts. Renewal batches are staggered,
limits/backoff apply across affected domain/account scopes, and expiry health is
visible per node/name. Shared issuance/key distribution is a separately configured
service with one writer and authenticated consumers, not an assumed fleet default.
Product code ships no mandatory owner domain, email or DNS vendor.

The current minimum runtime remains single-site/DNS-01/InternetBS and its broader
claim must remain blocked until these contracts are implemented and accepted.

## M1 wildcard profile override — OR-38–OR-45

The current selected profile needs exactly the configured apex and wildcard SAN,
not separate per-protocol SANs. It therefore selects DNS-01 and preserves the single
existing ACME owner/timer. HTTP-01 cannot issue the wildcard and cannot silently
replace the configured privacy profile. Missing DNS API/ownership/prerequisite
returns blocked before changing accepted ingress. Provider credentials remain
SecretRefs; InternetBS is the current qualified adapter, others require their own
implementation. This does not make a registrar mandatory for all future products.

Renew one validated artifact for the existing Caddy, selected native Hysteria and
other declared certificate consumers; REALITY's own native temporary auth certificate
remains distinct. DNS challenge updates preserve unrelated TXT records. Stage and
validate key/chain/expiry/wildcard+apex scope, distribute to all affected consumers,
then verify appropriate site/denial/redirect responses, not universal HTTP200.
Optional client-page content/path/keys are neither certificate SANs nor renewal
outputs. Quiet missing secret input fails without a prompt; no credential bytes in
argv/log/JSON. Cached accepted material stays active on renewal failure.
