# core/src/host/webroot.rs

Stable ID: `core.host-webroot` · Phase: `R1` · Kind: source

Bounded owner of `/var/www` preparation and the ordinary public site index.
It is called by install only after fresh listener/foreign preflight and before
Caddy setup or activation. It does not manage arbitrary host paths or edit user
content.

## Methods and responsibility

- `inspect_index(ctx) -> Result<SiteIndexObservation>` — read-only validation of
  the fixed `/var/www/index.html`; `Absent` includes a digest of the exact absent
  path basis, while `Present` carries bounded bytes, content digest and
  directory/file identity; present content is validated UTF-8.
- `ensure_index(ctx, expected) -> Result<SiteIndex>` — mutating local-root install
  effect. It rechecks the exact absent/present basis, creates `/var/www` with mode
  0755 only when absent and atomically publishes the shipped placeholder only when
  `index.html` is absent. Existing source content is never replaced.
- `SiteIndex::bytes() -> &[u8]`, `content_digest()`, and `identity_digest()` —
  bounded exact bytes for immutable bundle artifact generation and probe comparison,
  plus stable source evidence. `Debug` omits page content. Callers reread source
  identity/digest around the HTTPS probe.

Existing webroot symlinks, non-directories, and index symlinks, non-regular files,
hardlinks, empty files or files larger than 1 MiB are rejected. Existing files are
never replaced. An empty existing `index.html` is preserved and leaves install
unready until the owner supplies content. Other files under `/var/www` remain
user-owned and are neither read nor mounted into Caddy. The exact bounded index
bytes are emitted as a separate immutable `site-index.html` artifact in the Caddy
bundle and served over the site domain.

Source traversal is fixed to `/var/www` and uses descriptor-relative Linux
`openat` with `NOFOLLOW`, exact file/parent identity rechecks and bounded reads.
The Caddy bundle contains the exact page bytes as a normal immutable artifact; its
fixed route rewrites every non-VLESS request to that one file. This preserves the
source bytes without mounting the source tree or allowing a symlink into private
configuration. Errors are typed and redacted. Source filesystem mechanics do not
enter the generic private `SecureDir` writer because `/var/www` is intentionally a
public web directory.

Dependencies: `infra.storage` for established filesystem bounds/conventions,
Linux `rustix` safe filesystem APIs, `core.config` digest, and the bundled
`web/mask/index.html` placeholder. No Docker, network, certificate, registry or
foreign-resource authority.

## Calls / reads

- `web/mask/index.html` — placeholder bytes, used only when `index.html` is absent.
- `core/src/config.rs` — canonical SHA-256 digest.

## Called / read by

- `core/src/host/installation.rs` — install preflight binds the source observation;
  install ensures it before rendering; local served-site verification compares source
  identity, digest and exact bytes around its bounded HTTPS probe.

## Errors and completion

Unsafe path objects and empty/oversized pages fail without replacing originals.
Filesystem races return `config.readConflict`; missing webroot/index is safe only
in read-only inspection and is prepared by the mutating install owner. This module
does not prove public DNS or off-host reachability.
