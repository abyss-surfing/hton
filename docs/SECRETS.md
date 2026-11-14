# Private full-profile secrets repository

`.secrets/` is a **separate private Git repository**, maintained by the operator.
It is ignored by the main project and is not a submodule. Keep it **private**.

It contains one editable file plus a generated lock and the published guest share
artifacts:

| Path | Purpose |
|---|---|
| `.secrets/profile.yml` | complete profile: root settings, module settings, secret values, optional site content |
| `.secrets/images.lock.json` | container image lock (generated; not a secret) |
| `.secrets/share/` | per-user guest artifacts and `client-page.md`, generated after a successful install |

`.secrets/share/` contains `amnezia-<name>.md`, `mtproto.md`, `hysteria.md`,
`outline-<name>.yaml` and `client-page.md` (the bearer URL). All are `0640`;
nothing there is served directly by the web server.

`profile.yml` sections:

```yaml
schemaVersion: 1
root:        # instance, site domains/SNI, edge port, certificate, DNS, client page
modules:     # each module: enabled: true|false plus its settings
             #   e.g. foreign-amnezia: {enabled: true, settings: {...}}
secrets:     # dns, amnezia admin key, hysteria password, outline paths
site:        # optional indexHtml web-root content
```

All values live in this one file, so editing and copying is a single operation.

## Flow

1. `scripts/secrets init` seeds `.secrets/profile.yml` from the tracked
   `.secrets.example/profile.yml`, describes each section, and waits until the
   operator confirms the profile is complete.
2. The operator edits `.secrets/profile.yml` and commits/pushes this private repo.
3. `scripts/secrets import` (or the first `install`) materializes the profile into
   the protected `config/` tree: `config/hton.yaml`, `config/modules/*.yaml`,
   `config/locks/images.lock.json`, `config/secrets/**`, and `config/site/index.html`.
4. `-q`/`--json` installation never prompts: a missing profile is a hard error
   with the exact `scripts/secrets init` command.
5. After a successful `install`, guest credentials and `client-page.md` are
   published into `.secrets/share/` in the runtime directory and copied into the
   source checkout `.secrets/share/` when that private repository exists. Commit
   the resulting files to the separate private repository; a failed install
   writes nothing. An absent `root.core.clientAccessPage.path` generates the
   bearer URL on first install, an explicit empty path opts out, and
   `--rotate-client-page` replaces it.

On every `install`/`renew`, each required image is re-pulled by its `sourceTag`
and containers are recreated when the image changes; REALITY/client keys, SNI
and configuration stay fixed.

## Deployment

`scripts/deploy` copies the source `.secrets/` tree into the target directory from
`installation.directory` in the installation configuration (default `/opt/hton`).
It mirrors regular files with `0600`/`0700`, refuses symlinks, and does not copy
the private repository's `.git`. Target-only files are preserved. `site.root` selects the existing website
document root (imported as `core.site.root`; empty means detect `/var/www/html`
or `/var/www` and ask once per install) and the tree is only ever mounted
read-only. The next
`scripts/install` normalizes the copied tree to `0640`/`0750` `root:hton` like the
rest of the installation, so `hton` members can read the profile. `--dry-run`
reports whether a `.secrets` profile is present.

## Security

- The repository must remain **private**. Do not add it to the main project.
- `share/client-page.md` contains a bearer URL; anyone with it can read the guest
  credentials. Never copy it into the public site root or a public mirror.
- Placeholder values start with `CHANGE-ME`; import reports them but never prints
  or substitutes values.
- Rotate any secret that has ever been exposed outside this private repository.

## No `.env`

There is no `.env` source or template. The single `.secrets/profile.yml` (plus the
generated `images.lock.json`) is the only installation profile; do not introduce a
parallel environment-file format.

## Enabling Outline

`outline-wss` is disabled by default because it needs the pinned
`outline-ss-server` binary at `/opt/hton/bin/outline-ss-server` (repo `tooling/`
has no downloader). Provide the binary and set `modules.outline-wss.enabled: true`.
