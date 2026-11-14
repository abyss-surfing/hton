# Amnezia and installation acceptance matrix (F1)

Scenario contract for the selected minimum delivery. Status: **not executed**.
These scenarios require the built managed image (B1), real pinned clients and a
live isolated target; they are not implied by unit or schema consistency checks.
The separate disposable component harness `tests/minimum/functional.py` passed
on 2026-11-07 for issuer, exact-SNI routing, REALITY authentication, native
validator custody and snapshot/recreate; that local evidence does not execute
this matrix.
Evidence uses bounded local fixtures, synthetic query names and owned test
identities. No uncontrolled scans, real browsing histories or public IP claims are
made here. Client DNS evidence is per app/OS/mode and physical-interface capture.

## Managed Amnezia

- Fresh host with no Amnezia: provision creates the pinned bridge container with
  private `127.0.0.1:<5000–6000>` TCP/UDP bindings, REALITY keys and `server.json`;
  native TCP/443 is REALITY; no public native 443 is opened to satisfy association.
- Existing module without `mode`: associate; originals, credentials and resources
  are preserved; legacy Google/native SNI stays on its distinct accepted listener
  until the explicit `DROP-LEGACY-SNI` consent.
- App import of the exported profile succeeds; guest connects through the public
  endpoint and shared DNS/site policies.
- App add/revoke/restart: new client appears on the next accepted page refresh;
  revoked keys stop working; reinstall preserves the current native client array
  and never restores a revoked client.
- Reinstall, update, renew, reboot and removal: remove deactivates owned services
  and managed SSH access while retaining data; purge is explicit and deletes only
  the receipt-bound managed subtree.

## Ingress and names

- Apex site returns HTTP 200 with the admitted body and a trusted certificate.
- Primary own REALITY name returns 403 `Access denied` from the same Caddy.
- Covered unassigned one-label wildcard host returns 308 to the configured apex.
- Host/SNI mismatch returns generic 421; unknown/uncovered names and absent SNI
  close before any backend.
- No public 8443 in the owned composition; exactly one public UDP/443 owner;
  native HTTP3 behavior is verified for the selected owner.
- Certificate renewal failure keeps the previous accepted site; no hidden
  HTTP-01 substitution publishes individual protocol SANs for the wildcard profile.

## Client page

- `core.clientAccessPage.path` empty: no route and no artifact; quiet install never
  asks and never synthesizes a URL.
- Nonempty path: exact `GET`/`HEAD` on the base site host returns the escaped page
  with `Cache-Control: no-store`, `Referrer-Policy: no-referrer`,
  `X-Robots-Tag: noindex, nofollow, noarchive`, `X-Content-Type-Options: nosniff`
  and the restrictive CSP; other methods return 405.
- REALITY-denied, wildcard-redirect and other protocol hosts, and encoded path
  lookalikes do not retrieve the page.
- No admin/SSH/REALITY-private/API/ACME credential is retrievable; guest payloads
  only. A missing export marks coverage incomplete rather than fabricating keys.
- Refresh reconciles the accepted native snapshot; revocation withdraws the page
  before native key removal; disable/down stops serving under run intent.

## DNS

- Configured authenticated `tls-forward` upstream resolves with certificate and
  hostname verification; an invalid upstream certificate fails closed with no
  plaintext or OS fallback.
- Private Unbound answers on its bridge-private endpoint; no public plaintext 53.
- Tunnel loss, reconnect, split/local zones and native IPv6 are captured per
  client; unsupported devices/modes are reported as unverified, not universal.

## Evidence boundary

A failed proof blocks that feature/release claim and is fixed in its owning module;
it is never silently dropped from the accepted composition. Actual OCI digests,
per-architecture archive authentication, native package availability and real
app/OS DNS behavior are established only by the real build and this matrix.
