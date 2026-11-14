# core/minimum/certificates.py

Stable ID: `core.minimum.certificates` · Selected package: `M1-C` · Status: planned M1 contract

Single pinned ACME owner, wildcard scope and renewal. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
This contract does not assert implemented methods or authorize execution.

## Methods and typed results

- `ensure(validated requested scope/challenge, secret refs, journal) -> CertificateReceipt or blocked`.

## Calls

No callable dependency in the selected overlay; injected observations/typed data only.

## Called by

- `core.minimum.install.main` -> `ensure`.

## OR-63 implementation amendment

`ensure(..., names=(), wildcard=False)` requests the site apex, each concrete own
name and, when `wildcard` is set, `*.<domain>`; a wildcard response must carry the
exact `DNS:*.<domain>` SAN (`validate_wildcard`). Concrete names remain the trust and
reuse check, so a wildcard certificate satisfies them. Wildcard issuance is
permitted only with the `dns-01` challenge selected by the caller.

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.

## Historical baseline contract (superseded where M1 differs)

# core/minimum/certificates.py

Stable ID: `core.minimum.certificates` · M0 · source

Private DNS credentials, cached issuance and system-trust verification.

Contract: [minimum installation](../../../../specs/sdd-minimum-installation.md).
Shared interfaces: docs/INTERFACES.md. Inputs are bounded local values or protected
Store records; results are verified records or InstallError without raw native
output. Dependencies follow the family contract. State/effects are confined to
/opt/hton and exact admitted Docker IDs; install alone coordinates phases.

Secret input calls stdlib getpass directly; it opens the controlling TTY safely.
Fallback warnings are errors. No text r+ stream or visible stdin fallback is used.
Pseudoterminal acceptance verifies both saved values and absence of terminal echo.

select_policy owns HTTP default, explicit DNS mode and narrowly bounded legacy
unissued migration; persist challenge in protected state. issuer_arguments owns
standalone host/80 versus DNS adapter/environment arguments. require_http_port
reserves no permanent listener and refuses unrelated holders. ensure never calls
credentials for HTTP mode. Renewal uses the selected mode and retained account;
external identity checks stay outside this module.
