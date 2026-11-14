# M1 installation file and call graph

Authoritative Markdown planning overlay for OR-37–OR-55. Scope is the selected
minimum Python delivery, not full R1. `file-index.json` and module-index.json retain
the historical M0/R1 baseline until their future implementation-package update;
this overlay supplies exact new file owners/methods/edges now, without changing
live parsers. OR-49/OR-52 client-page and private-share publication are implemented
in source as of 2026-11-07 with unit evidence only. The next run must materialize
this overlay into those indexes with reciprocal edge validation.
No implementation agent is launched by writing this graph.

## Owned files

| Stable ID | Exact path | Package | Contract |
|---|---|---|---|
| `core.minimum.configuration` | `python/minimum/configuration.py` | M1-A | [mirror](map/core/minimum/configuration.py.md) |
| `core.minimum.managed-amnezia` | `python/minimum/managed_amnezia.py` | M1-B | [mirror](map/core/minimum/managed_amnezia.py.md) |
| `core.minimum.host-access` | `python/minimum/host_access.py` | M1-B | [mirror](map/core/minimum/host_access.py.md) |
| `core.minimum.access-links` | `python/minimum/access_links.py` | M1-B | [mirror](map/core/minimum/access_links.py.md) |
| `core.minimum.dns` | `services/onomat/runtime/dns/python/dns.py` (Onomat-owned) | M1-C | [mirror](map/services/onomat/runtime/dns/python/dns.py.md) |
| `core.minimum.client-page` | `python/minimum/client_page.py` | M1-D | [mirror](map/core/minimum/client_page.py.md) |
| `core.minimum.install` | `python/minimum/install.py` | M1-E | [mirror](map/core/minimum/install.py.md) |
| `core.minimum.updates` | `python/minimum/updates.py` | M1-E | [mirror](map/core/minimum/updates.py.md) |
| `core.minimum.protocols` | `python/minimum/protocols.py` | M1-D | [mirror](map/core/minimum/protocols.py.md) |
| `core.minimum.ingress` | `python/minimum/ingress.py` | M1-D | [mirror](map/core/minimum/ingress.py.md) |
| `core.minimum.docker` | `python/minimum/docker.py` | M1-B | [mirror](map/core/minimum/docker.py.md) |
| `core.minimum.storage` | `python/minimum/storage.py` | M1-A | [mirror](map/core/minimum/storage.py.md) |
| `core.minimum.migrations` | `python/minimum/migrations.py` | M1-A | [mirror](map/core/minimum/migrations.py.md) |
| `core.minimum.certificates` | `python/minimum/certificates.py` | M1-C | [mirror](map/core/minimum/certificates.py.md) |
| `cmd.deploy` | `scripts/deploy` | M1-E | [mirror](map/scripts/deploy.md) |
| `cmd.install` | `scripts/install` | M1-E | [mirror](map/scripts/install.md) |
| `cmd.update` | `scripts/update` | M1-E | [mirror](map/scripts/update.md) |
| `cmd.uninstall` | `scripts/uninstall` | M1-E | [mirror](map/scripts/uninstall.md) |
| `shell.project` | `lib/hton` | M1-E | [mirror](map/lib/hton.md) |
| `build.amnezia` | `platform/docker/amnezia-xray.Dockerfile` | M1-C | [mirror](map/docker/amnezia-xray.Dockerfile.md) |
| `build.amnezia-script` | `tooling/build-amnezia-image` | M1-C | [mirror](map/tooling/build-amnezia-image.md) |
| `image.lock` | `platform/images.lock.json` | M1-C | [mirror](map/images.lock.json.md) |
| `systemd.client-page-service` | `platform/systemd/hton-client-page.service` | M1-E | [mirror](map/systemd/hton-client-page.service.md) |
| `systemd.client-page-timer` | `platform/systemd/hton-client-page.timer` | M1-E | [mirror](map/systemd/hton-client-page.timer.md) |
| `schema.v1` | `platform/schemas/hton-v1.schema.json` | M1-A | [mirror](map/schemas/hton-v1.schema.json.md) |
| `schema.v2` | `platform/schemas/hton-v2.schema.json` | M1-A | [mirror](map/schemas/hton-v2.schema.json.md) |
| `schema.module` | `platform/schemas/module-v1.schema.json` | M1-A | [mirror](map/schemas/module-v1.schema.json.md) |
| `tests.minimum.unit` | `tests/minimum/test_minimum.py` | U1 | [mirror](map/tests/minimum/test_minimum.py.md) |
| `test.minimum-security` | `tests/minimum/test_security_contracts.py` | U1 | [mirror](map/tests/minimum/test_security_contracts.py.md) |
| `tests.minimum.functional` | `tests/minimum/functional.py` | F1 | [mirror](map/tests/minimum/functional.py.md) |
| `test.minimum-app` | `tests/minimum/AMNEZIA-ACCEPTANCE.md` | F1 | [mirror](map/tests/minimum/AMNEZIA-ACCEPTANCE.md.md) |

## Callable edges (both endpoint projections are mirrored)

| Caller | Callee |
|---|---|
| `cmd.install.run` | `shell.project.hton_cli` |
| `cmd.update.run` | `shell.project.hton_cli` |
| `shell.project.hton_cli` | `core.minimum.updates.fast_forward` |
| `shell.project.hton_cli` | `cmd.deploy.run` |
| `shell.project.hton_cli` | `core.minimum.install.main` |
| `core.minimum.install.main` | `core.minimum.configuration.load` |
| `core.minimum.install.main` | `core.minimum.configuration.validate` |
| `core.minimum.install.main` | `core.minimum.migrations.plan` |
| `core.minimum.install.main` | `core.minimum.migrations.apply` |
| `core.minimum.install.main` | `core.minimum.managed-amnezia.plan` |
| `core.minimum.install.main` | `core.minimum.managed-amnezia.prepare` |
| `core.minimum.install.main` | `core.minimum.host-access.plan` |
| `core.minimum.install.main` | `core.minimum.host-access.reconcile` |
| `core.minimum.install.main` | `core.minimum.host-access.deactivate` |
| `core.minimum.install.main` | `core.minimum.host-access.purge` |
| `core.minimum.install.main` | `core.minimum.dns.plan` |
| `core.minimum.install.main` | `core.minimum.dns.render` |
| `core.minimum.install.main` | `core.minimum.dns.prepare` |
| `core.minimum.install.main` | `core.minimum.certificates.ensure` |
| `core.minimum.install.main` | `core.minimum.protocols.plan` |
| `core.minimum.install.main` | `core.minimum.protocols.prepare` |
| `core.minimum.install.main` | `core.minimum.access-links.encode` |
| `core.minimum.install.main` | `core.minimum.client-page.plan` |
| `core.minimum.install.main` | `core.minimum.client-page.render` |
| `core.minimum.install.main` | `core.minimum.client-page.refresh` |
| `core.minimum.install.main` | `core.minimum.client-page.generate_path` |
| `core.minimum.install.main` | `core.minimum.install.publish_secret_share_artifacts` |
| `core.minimum.install.publish_secret_share_artifacts` | `core.minimum.install.secret_share_artifacts` |
| `core.minimum.install.publish_secret_share_artifacts` | `core.minimum.managed-amnezia.client_inventory` |
| `core.minimum.install.cutover` | `core.minimum.ingress.prepare` |
| `core.minimum.install.cutover` | `core.minimum.ingress.probe` |
| `core.minimum.install.cutover` | `core.minimum.protocols.probe` |
| `core.minimum.configuration.load` | `core.minimum.storage.Store` |
| `core.minimum.migrations.apply` | `core.minimum.storage.commit` |
| `core.minimum.managed-amnezia.prepare` | `core.minimum.docker.command` |
| `core.minimum.managed-amnezia.prepare` | `core.minimum.docker.qualify` |
| `core.minimum.managed-amnezia.reconcile` | `core.minimum.docker.snapshot` |
| `core.minimum.dns.prepare` | `core.minimum.docker.command` |
| `core.minimum.protocols.prepare` | `core.minimum.docker.command` |
| `core.minimum.ingress.prepare` | `core.minimum.ingress.configs` |
| `core.minimum.ingress.prepare` | `core.minimum.docker.command` |
| `core.minimum.client-page.refresh` | `core.minimum.client-page.plan` |
| `core.minimum.client-page.refresh` | `core.minimum.client-page.render` |

| `core.minimum.install.main` | `core.minimum.updates.record` |
| `core.minimum.install.main` | `core.minimum.updates.due` |
| `shell.project.hton_cli` | `core.minimum.updates.due` |
| `core.minimum.install.main` | `core.minimum.managed-amnezia.reconcile` |
| `core.minimum.install.main` | `core.minimum.managed-amnezia.client_inventory` |
| `core.minimum.install.main` | `core.minimum.protocols.client_inventory` |
| `core.minimum.managed-amnezia.client_inventory` | `core.minimum.access-links.encode` |

## Declarative dependencies and shared ownership

Configuration consumes root/module schemas and emits typed normalized records.
Managed/DNS/protocol plans consume `image.lock` without pulling images. Builder
produces OCI/provenance; package M1-C alone updates image.lock with actual digests.
Ingress consumes bounded website snapshot, client-page artifact, certificate
receipt and resolved host policies; it never reads the whole secret store. Page
consumes only typed client inventory from access-links and protocol owners. The
refresh systemd service invokes fixed install `client-page-refresh`; its timer is
owned by run intent and cannot mutate VPN or discover public IP/DNS.

M1-A also owns Markdown schema contracts/examples, future `hton.example.yaml` and
module-example changes. M1-E owns closure changes in scripts/deploy, lib/hton,
operational docs and future index materialization; M1-D owns ingress/protocols/page.
Existing native mechanisms in docker.py retain their own state custody; M1-B adds
methods rather than copying observers into managed_amnezia. Broader Rust adapters
consume the same recorded architecture in a later independent run, never as a
second controller imported by the minimum installer.

## Supplemental fixed-file ownership

| File(s) | Sole package | Responsibility |
|---|---|---|
| lib/install_config.py | M1-E | Existing install-root input only; preserve target restrictions, no page secret parsing in shell |
| hton.example.yaml; docs/examples/hton.future.yaml | M1-A | Root design examples; `clientAccessPage.path` is absent for automatic publication, explicit `""` is opt-out |
| docs/examples/config/modules/foreign-amnezia.yaml; mtproto.yaml; outline-wss.yaml; hysteria.yaml (same directory) | M1-A | Module examples synchronized to typed schemas |
| docs/sdd/file-index.json; docs/sdd/module-index.json | M1-E | Materialize this Markdown delta and reciprocal projections during implementation |
| docs/CLI.md; docs/MINIMUM-INSTALL.md; README.md; docs/README.md; docs/BUILD-RELEASE.md; docs/IMPLEMENTATION-STATUS.md | M1-E | Exact supported CLI/closure/operational status without future feature claims |

lib/install_config.py mirror remains its existing contract; no new independent
configuration normalizer is introduced there. All area-SDD/INTERFACES schema rule
changes are reconciled by the coordinator before changing a caller contract.

M1-B/E shared deployment contract: new mutable `<installation>/host-access/`
holds dedicated SSH homes outside 0700 config/work stores. Deploy's exact retained
top-level allowlist gains host-access; it is not shipped from source or pruned.
HostAccess owns per-receipt identity/modes; protected private keys stay in config/
secrets. M1-E alone changes deploy, after consuming M1-B's declared home receipt.
