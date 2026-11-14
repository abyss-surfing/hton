# core/minimum/install.py

Stable ID: `core.minimum.install` · Selected package: `M1-E` · Status: M1 contract; OR-49/OR-52 methods implemented in source (unit evidence only)

Sole effect sequencing, consent, quiet install and cutover. Canonical authorities: `docs/plans/INSTALLATION-IMPLEMENTATION.md`,
`docs/sdd/INSTALLATION-FILE-GRAPH.md`, owning area SDD and `docs/INTERFACES.md`.
Implemented methods do not imply product/live acceptance.

## Methods and typed results

- `main(install/recover/renew/update/client-page-refresh, typed args) -> redacted OperationResult + exit`. The `update` action is the non-interactive install path: it requires committed state, refuses native client-list drift, skips public DNS observation and re-pulls/changes only owned images. `--auto` applies the seven-day schedule guard.

- `cutover(admitted plan, exact receipts, lease) -> accepted generation or recovered receipt`.

- `rollback(receipt-bound prior accepted state and policy fence) -> RecoveryResult`.

- `native_share_links(exact native container, optional connection label) -> guest REALITY links + protected share paths`. The fragment uses `<instance.label> XRay`/`XRay` (or a per-credential name override); the share filename uses the credential slug.

- `emit_ready(typed result, exact native container, config?) -> friendly summary with guest credentials; publishes private share artifacts after success and records the successful install/update stamp`.

- `install_units(work, client_page_enabled, refresh_seconds) -> linked systemd units`, including the daily `hton-minimum-update.timer` that runs `scripts/update --auto --quiet`; the seven-day policy lives in `core.minimum.updates`.

- `ensure_client_page_path(root document, typed args, config Store) -> bool` — absent path generates one CSPRNG segment; explicit empty opt-out; `--rotate-client-page` replaces.

- `secret_share_artifacts(protocol plan, domain, native entries, page path, names) -> {safe file name: bytes}` — one `<user>.md` bundle per credential identity with its Amnezia/Outline entries and shared MTProto/Hysteria sections, plus `client-page.md`; colliding slugs get a digest suffix.

- `publish_secret_share_artifacts(config, work, protocol plan, domain, container, page path) -> None` — runtime and source-checkout `.secrets/share`, `0640`, stale and legacy-layout cleanup, safe receipts; native fragments honor `clientAccessPage.names`/`instance.label`.

## Calls

- `main` -> `core.minimum.updates.due` (automatic update guard).

- `emit_ready` -> `core.minimum.updates.record` (successful install/update stamp).

`emit` prints only the site, the client-access page URL and the share directory
on the human path (OR-64/D171); credential links are not printed.

- `main` -> `core.minimum.configuration.load`.

- `main` -> `core.minimum.configuration.validate`.

- `main` -> `core.minimum.migrations.plan`.

- `main` -> `core.minimum.migrations.apply`.

- `main` -> `core.minimum.managed-amnezia.plan`.

- `main` -> `core.minimum.managed-amnezia.prepare`.

- `main` -> `core.minimum.host-access.plan`.

- `main` -> `core.minimum.host-access.reconcile`.

- `main` -> `core.minimum.host-access.deactivate`.

- `main` -> `core.minimum.host-access.purge`.

- `main` -> `core.minimum.dns.plan`.

- `main` -> `core.minimum.dns.render`.

- `main` -> `core.minimum.dns.prepare`.

- `main` -> `core.minimum.certificates.ensure`.

- `main` -> `core.minimum.protocols.plan`.

- `main` -> `core.minimum.protocols.prepare`.

- `main` -> `core.minimum.access-links.encode`.

- `main` -> `core.minimum.client-page.plan`.

- `main` -> `core.minimum.client-page.render`.

- `main` -> `core.minimum.client-page.refresh`.

- `main` -> `core.minimum.client-page.generate_path`.

- `main` -> `core.minimum.install.ensure_client_page_path` (same module owner).

- `emit_ready` -> `core.minimum.install.publish_secret_share_artifacts` (same module owner).

- `publish_secret_share_artifacts` -> `core.minimum.install.secret_share_artifacts` (same module owner).

- `publish_secret_share_artifacts` -> `core.minimum.managed-amnezia.client_inventory`.

- `cutover` -> `core.minimum.ingress.prepare`.

- `cutover` -> `core.minimum.ingress.probe`.

- `cutover` -> `core.minimum.protocols.probe`.

- `main` -> `core.minimum.managed-amnezia.reconcile`.

- `main` -> `core.minimum.managed-amnezia.client_inventory`.

- `main` -> `core.minimum.protocols.client_inventory`.

- `native_share_links` -> `core.minimum.managed-amnezia.client_inventory`.

- `emit_ready` -> `core.minimum.protocols.share_links`.

## Called by

- `shell.project.hton_cli` -> `main`.

## OR-61/OR-62 implementation amendment

`refresh_client_page(config, work, state, protocol_plan=None)` keeps the page
published when full validation has no `desired` root: it parses
`core.clientAccessPage` through `configuration.client_page`, and takes the site
domain and `instance.label` from the root document. Only the page's own
path/refresh validation can disable it; unrelated blocking diagnostics do not.
`cutover` passes an optional site root to `ingress.prepare` so Caddy emits the
plain-HTTP challenge/redirect server; without a root the listener is omitted.

`reality_certificate_names(domain, names) -> list` returns the own-domain REALITY
subdomains that must be covered by the site certificate. `run_managed` passes
them to `certificates.ensure` and to `ingress.prepare`; the site apex is excluded
because it is already the primary name, and third-party legacy names cannot be
issued. No self-signed REALITY certificate is generated (D169). The request
also includes `www.<domain>` and, for `dns-01` with a matching `wildcardDomain`,
`*.<domain>`. A REALITY name equal to the site domain selects native self-steal
in `ingress.prepare` (D170).

## State, effects and errors

Use the owning SDD's exact receipt/read-set and explicit effect scope. Pure planning/
rendering/encoding does not invoke filesystem mutations, Docker or network. Effect
methods require journal intent, exact owned resources and approved operation context;
errors are typed and redact keys, URL paths, native output and source snippets.
One writer per owned path. Product tests run only after all M1 packages integrate.

## Historical baseline contract (superseded where M1 differs)

# core/minimum/install.py

Stable ID: `core.minimum.install` · M0 · source

Resumable minimum Core, cutover lease and exact-original recovery.

Contract: [minimum installation](../../../../specs/sdd-minimum-installation.md).
Shared interfaces: docs/INTERFACES.md. Inputs are bounded local values or protected
Store records; results are verified records or InstallError without raw native
output. Dependencies follow the family contract. State/effects are confined to
/opt/hton and exact admitted Docker IDs; install alone coordinates phases.

check_foreign compares native declared network attachments independently from
Config/HostConfig, allowing only Docker's deferred empty NetworkID on stopped
resources after verifying the saved ID/name dependency. Running replacements must
match every declared network name and each custom dependency ID; the daemon-owned
default `bridge` is compared by name because Docker recreates its ID. Original
network objects remain untouched.

upgrade_state_fingerprints upgrades a legacy state once: it re-seals
`originalFingerprint` from the protected original record, re-seals
`replacementFingerprint` only after check_foreign proves the live replacement
equivalent, and re-derives the remap approval digest under the new encoding. The
versioned fields prevent a false drift report (for example after Docker recreates
its default bridge) from blocking every later renewal.

prompt uses separate non-seeking TTY read/write streams and converts unavailable
TTY into InstallError. failure_detail captures only exception kind, owned source
module/function/line and numeric errno; failure_message redacts unexpected text.
log/_log accept this optional diagnostic metadata. No traceback or locals persist.
_log rotates `minimum.log` on size or when its first entry is a week old, then
_prune_logs removes archived `minimum.log.*` files whose first entry (or mtime)
is older than seven days. Pruning skips symlinks and never blocks the VPN.

load_site_inputs merges only absent site/email from source bootstrap, prompts for
remaining missing inputs and preserves saved installed values. save_site_inputs
persists validated root/domain/email before native consent. Installed target drift
blocks; units and recovery bind the selected protected deployment root.

consent shows the exact resource and change in a readable multi-line summary,
then asks Continue [y/N]. Explicit y/yes returns the internally bound scope digest;
empty/other answers decline. No global --yes bypass and no hash-copy requirement.

progress reports STEP/OK human status outside structured output. emit produces
clear final human success or existing JSON. report_failure emits highlighted ERROR
and safe cause. Installation.phase optionally reports major user-facing stages.
Snapshot validator has isolated read-only DAC_READ_SEARCH access, preserving
native mode-0001 configuration. Recovery and original file permissions do not change.

Protocol integration: main validates `--with`/`--without` through
core.minimum.protocols, builds/reuses protected credentials and artifacts before
effects, saves module settings, and passes the plan to ingress preparation and
cutover. Cutover stops prior protocol containers, validates/starts candidates,
probes them, promotes restart policy and only then applies the accepted state.
Rollback stops candidates and restores the prior active protocol set.

OR-35/D133: main derives the own native SNI (`--chain-sni` or the site domain) and
asks a separate local y/yes consent before rewriting the foreign REALITY
`serverNames`/`dest` in the HTON-owned replacement. `validate_native_bytes` checks
the rewritten JSON with pinned `xray -test` through a read-only bind mount;
`migrate_native` stores the previous bytes, copies the new file, verifies its
digest, restarts a committed replacement and restores the previous file if it
does not stay running. `originalNativeDigest` and `replacementNativeDigest` keep
the parked original and replacement verifiable independently. Unattended renew
never migrates.
