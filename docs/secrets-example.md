# Example full-profile secrets repository

This directory is a **tracked template** for the private `.secrets/` repository.
It contains no real secrets: placeholder secret values start with `CHANGE-ME`.

`profile.yml` is the complete editable profile (root settings, module settings,
secrets and optional site content); `images.lock.json` is the generated image lock.

A successful install also writes `share/` with the per-user guest artifacts and
`client-page.md` (the bearer URL). Keep this repository private and commit those
files deliberately; they are never served by the website directly.

On a fresh host, `scripts/secrets init` copies this template into `.secrets/`,
describes each section, and waits for confirmation. The operator edits
`.secrets/profile.yml`, commits it to the **private** repository, and runs
`scripts/secrets import`.

Do not store real credentials in this template; put them only in `.secrets/`.
