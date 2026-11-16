# Planned per-file contracts

Generated from [file-index.json](../file-index.json); this is documentation, not
implemented source. Every row links a mirrored file contract with methods, arguments,
Logic and reciprocal calls. Stable IDs survive binding/language changes.

Read [architecture](../../ARCHITECTURE.md), [shared structures/interfaces](../../INTERFACES.md),
[owner requirements](../../OWNER-REQUIREMENTS.md) and the relevant [area SDD](../../specs/README.md)
before using a row. [Orchestrator](../../IMPLEMENTATION-ORCHESTRATOR.md) assigns waves
and file ownership. Implementation completes before unit then functional testing.

`later-management` is deferred API/configurator/web work; `far-future` includes IPv6;
`compatibility` contains thin legacy-name delegates. R1 code may read descriptor
data for future modules, but cannot execute/import their implementations.
Generated work/ artifacts are produced at installation/runtime, not shipped state.

| File | Stable ID | Phase | Wave |
|---|---|---|---|
| [.github/workflows/ci.yml](.github/workflows/ci.yml.md) | ci.workflow | R1 | I3-B |
| [.github/workflows/release.yml](.github/workflows/release.yml.md) | ci.release | R1 | I3-B |
| [Cargo.lock](Cargo.lock.md) | build.rust-lock | R1 | I0-B |
| [Cargo.toml](Cargo.toml.md) | build.workspace | R1 | I0-B |
| [LICENSE](LICENSE.md) | doc.license | design | P0/I3-C |
| [README.md](README.md.md) | doc.readme-md | design | P0/I3-C |
| [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md.md) | notices | R1 | P0/I3-C |
| [accounts/Cargo.toml](accounts/Cargo.toml.md) | build.accounts-manifest | R2 | deferred-selected-release |
| [accounts/src/egress.rs](accounts/src/egress.rs.md) | accounts.egress | far-future | deferred-far-future |
| [accounts/src/issuance.rs](accounts/src/issuance.rs.md) | accounts.issuance | R2 | deferred-selected-release |
| [accounts/src/model.rs](accounts/src/model.rs.md) | accounts.model | R2 | deferred-selected-release |
| [adapters/engines/caddy.rs](adapters/engines/caddy.rs.md) | adapter.engine-caddy | R1 | I2-A |
| [adapters/engines/haproxy.rs](adapters/engines/haproxy.rs.md) | adapter.engine-haproxy | R1 | I2-A |
| [services/onomat/runtime/dns/rust/unbound.rs](services/onomat/runtime/dns/rust/unbound.rs.md) | adapter.engine-unbound | R1 | I2-A |
| [adapters/engines/xray.rs](adapters/engines/xray.rs.md) | adapter.engine-xray | R1 | I2-A |
| [adapters/foreign.rs](adapters/foreign.rs.md) | adapter.foreign | R1 | I2-B |
| [adapters/gateway](adapters/gateway.md) | adapter.gateway | optional | unselected-optional |
| [adapters/ipv6](adapters/ipv6.md) | adapter.ipv6 | far-future | deferred-far-future |
| [adapters/resources.rs](adapters/resources.rs.md) | adapter.resources | R1 | I2-B |
| [admin/Cargo.toml](admin/Cargo.toml.md) | build.admin-manifest | later-management | deferred-selected-release |
| [admin/src/api.rs](admin/src/api.rs.md) | admin.api | later-management | deferred-selected-release |
| [admin/src/auth.rs](admin/src/auth.rs.md) | admin.auth | later-management | deferred-selected-release |
| [admin/src/events.rs](admin/src/events.rs.md) | admin.events | later-management | deferred-selected-release |
| [admin/src/main.rs](admin/src/main.rs.md) | admin.entry | later-management | deferred-selected-release |
| [admin/src/telegram.rs](admin/src/telegram.rs.md) | admin.telegram | later-management | deferred-selected-release |
| [agent/Cargo.toml](agent/Cargo.toml.md) | build.agent-manifest | R2 | deferred-selected-release |
| [agent/src/main.rs](agent/src/main.rs.md) | agent.entry | R2 | deferred-selected-release |
| [agent/src/policy.rs](agent/src/policy.rs.md) | agent.policy | R2 | deferred-selected-release |
| [agent/src/probes.rs](agent/src/probes.rs.md) | agent.probes | R2 | deferred-selected-release |
| [bin/hton](bin/hton.md) | cli.entry | R1 | I3-A |
| [scripts/certs](scripts/certs.md) | cmd.certs | R1 | I3-A |
| [scripts/config](scripts/config.md) | cmd.config | R1 | I3-A |
| [scripts/diag](scripts/diag.md) | cmd.diag | R1 | I3-A |
| [scripts/deploy](scripts/deploy.md) | cmd.deploy | R1 | I3-A |
| [scripts/doctor](scripts/doctor.md) | cmd.doctor | R1 | I3-A |
| [scripts/down](scripts/down.md) | cmd.down | R1 | I3-A |
| [scripts/feature](scripts/feature.md) | cmd.feature | compatibility | I3-A |
| [scripts/help](scripts/help.md) | cmd.help | R1 | I3-A |
| [scripts/install](scripts/install.md) | cmd.install | R1 | I3-A |
| [scripts/install-ubuntu](scripts/install-ubuntu.md) | cmd.install-ubuntu | R1 | I3-A |
| [scripts/install-unknown](scripts/install-unknown.md) | cmd.install-unknown | R1 | I3-A |
| [scripts/keys](scripts/keys.md) | cmd.keys | R1 | I3-A |
| [scripts/modules](scripts/modules.md) | cmd.modules | R1 | I3-A |
| [scripts/remap-undo](scripts/remap-undo.md) | cmd.remap-undo | R1 | I3-A |
| [scripts/restart](scripts/restart.md) | cmd.restart | R1 | I3-A |
| [scripts/runtime](scripts/runtime.md) | cmd.runtime | R1 | I3-A |
| [scripts/service](scripts/service.md) | cmd.service | compatibility | I3-A |
| [scripts/share](scripts/share.md) | cmd.share | R1 | I3-A |
| [scripts/switch](scripts/switch.md) | cmd.switch | R1 | I3-A |
| [scripts/tunnel](scripts/tunnel.md) | cmd.tunnel | far-future | deferred-far-future |
| [scripts/uninstall](scripts/uninstall.md) | cmd.uninstall | R1 | I3-A |
| [scripts/up](scripts/up.md) | cmd.up | R1 | I3-A |
| [scripts/update](scripts/update.md) | cmd.update | M1 | I3-A |
| [scripts/upgrade](scripts/upgrade.md) | cmd.upgrade | later-management | deferred-selected-release |
| [scripts/validate](scripts/validate.md) | cmd.validate | R1 | I3-A |
| [scripts/whoami](scripts/whoami.md) | cmd.whoami | R1 | I3-A |
| [compose/controlplane.yaml](compose/controlplane.yaml.md) | compose.control | later-management | deferred-selected-release |
| [compose/dataplane.yaml](compose/dataplane.yaml.md) | compose.data | R1 | I3-B |
| [configurator/Cargo.toml](configurator/Cargo.toml.md) | build.configurator-manifest | later-management | deferred-selected-release |
| [configurator/src/changes.rs](configurator/src/changes.rs.md) | configurator.changes | later-management | deferred-selected-release |
| [configurator/src/main.rs](configurator/src/main.rs.md) | configurator.entry | later-management | deferred-selected-release |
| [configurator/src/migration.rs](configurator/src/migration.rs.md) | configurator.migration | later-management | deferred-selected-release |
| [configurator/tests/changes.rs](configurator/tests/changes.rs.md) | test.configurator | later-management | deferred-selected-release |
| [contracts/Cargo.toml](contracts/Cargo.toml.md) | build.contract-manifest | R1 | I0-B |
| [contracts/src/config.rs](contracts/src/config.rs.md) | contract.config | R1 | I0-A |
| [contracts/src/lib.rs](contracts/src/lib.rs.md) | contract.library | R1 | I0-A |
| [contracts/src/operations.rs](contracts/src/operations.rs.md) | contract.operations | R1 | I0-A |
| [controller/Cargo.toml](controller/Cargo.toml.md) | build.controller-manifest | later-management | deferred-selected-release |
| [controller/src/api.rs](controller/src/api.rs.md) | control.api | later-management | deferred-selected-release |
| [controller/src/events.rs](controller/src/events.rs.md) | control.events | later-management | deferred-selected-release |
| [controller/src/main.rs](controller/src/main.rs.md) | control.entry | later-management | deferred-selected-release |
| [core/Cargo.toml](core/Cargo.toml.md) | build.core-manifest | R1 | I0-B |
| [core/src/apply.rs](core/src/apply.rs.md) | core.apply | R1 | I2-C |
| [core/src/backup.rs](core/src/backup.rs.md) | core.backup | deferred | I2-C |
| [core/src/catalog.rs](core/src/catalog.rs.md) | core.catalog | R1 | I1-A |
| [core/src/certificates.rs](core/src/certificates.rs.md) | core.certificates | R1 | I2-C |
| [core/src/checks.rs](core/src/checks.rs.md) | core.checks | R1 | I2-C |
| [core/src/host/mod.rs](core/src/host/mod.rs.md) | core.host | R1 | I3-A |
| [core/src/host/observations.rs](core/src/host/observations.rs.md) | host.observations | R1 | I3-A |
| [core/src/config.rs](core/src/config.rs.md) | core.config | R1 | I1-A |
| [core/src/exports.rs](core/src/exports.rs.md) | core.exports | R1 | I2-C |
| [core/src/identities.rs](core/src/identities.rs.md) | core.identities | R1 | I2-C |
| [core/src/journal.rs](core/src/journal.rs.md) | core.journal | R1 | I1-B |
| [core/src/lib.rs](core/src/lib.rs.md) | core.library | R1 | I3-A |
| [core/src/lifecycle.rs](core/src/lifecycle.rs.md) | core.lifecycle | R1 | I2-B |
| [core/src/host/modules.rs](core/src/host/modules.rs.md) | host.modules | R1 | I3-A |
| [core/src/lock.rs](core/src/lock.rs.md) | core.lock | R1 | I1-B |
| [core/src/main.rs](core/src/main.rs.md) | core.entry | R1 | I3-A |
| [core/src/plan.rs](core/src/plan.rs.md) | core.plan | R1 | I1-A |
| [core/src/registry.rs](core/src/registry.rs.md) | core.registry | R1 | I1-B |
| [core/src/render.rs](core/src/render.rs.md) | core.render | R1 | I2-A |
| [core/src/secrets.rs](core/src/secrets.rs.md) | core.secrets | R1 | I1-B |
| [core/src/selection.rs](core/src/selection.rs.md) | core.selection | R1 | I2-B |
| [core/src/setup.rs](core/src/setup.rs.md) | core.setup | R1 | I2-B |
| [docker/caddy.Dockerfile](docker/caddy.Dockerfile.md) | build.caddy | R1 | I3-B |
| [docker/core.Dockerfile](docker/core.Dockerfile.md) | build.core | R1 | I3-B |
| [docs/ADMIN-UX.md](docs/ADMIN-UX.md.md) | doc.docs-admin-ux-md | design | P0/I3-C |
| [docs/AMNEZIA-NATIVE-EVIDENCE.md](docs/AMNEZIA-NATIVE-EVIDENCE.md.md) | doc.docs-AMNEZIA-NATIVE-EVIDENCE-md | design | P0/I2-B |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md.md) | doc.docs-architecture-md | design | P0/I3-C |
| [docs/CLI.md](docs/CLI.md.md) | doc.docs-cli-md | design | P0/I3-C |
| [docs/DECISIONS.md](docs/DECISIONS.md.md) | doc.docs-decisions-md | design | P0/I3-C |
| [docs/FINAL-REVIEW.md](docs/FINAL-REVIEW.md.md) | doc.docs-FINAL-REVIEW-md | design | P0/I3-C |
| [docs/IMPLEMENTATION-ORCHESTRATOR.md](docs/IMPLEMENTATION-ORCHESTRATOR.md.md) | doc.docs-IMPLEMENTATION-ORCHESTRATOR-md | design | P0/I3-C |
| [docs/IMPLEMENTATION-STATUS.md](docs/IMPLEMENTATION-STATUS.md.md) | doc.docs-IMPLEMENTATION-STATUS-md | design | P0/I3-C |
| [docs/INTERFACES.md](docs/INTERFACES.md.md) | doc.docs-INTERFACES-md | design | P0/I3-C |
| [docs/OWNER-REQUIREMENTS.md](docs/OWNER-REQUIREMENTS.md.md) | doc.docs-OWNER-REQUIREMENTS-md | design | P0/I3-C |
| [docs/README.md](docs/README.md.md) | doc.docs-readme-md | design | P0/I3-C |
| [docs/REUSE-ASSESSMENT.md](docs/REUSE-ASSESSMENT.md.md) | doc.docs-reuse-assessment-md | design | P0/I3-C |
| [docs/ROADMAP.md](docs/ROADMAP.md.md) | doc.docs-roadmap-md | design | P0/I3-C |
| [docs/examples/config/modules/dot.yaml](docs/examples/config/modules/dot.yaml.md) | example.module.dot | design | I3-C |
| [docs/examples/config/modules/foreign-amnezia.yaml](docs/examples/config/modules/foreign-amnezia.yaml.md) | example.module.foreign-amnezia | design | I3-C |
| [docs/examples/config/modules/hysteria.yaml](docs/examples/config/modules/hysteria.yaml.md) | example.module.hysteria | design | I3-C |
| [docs/examples/config/modules/mtproto.yaml](docs/examples/config/modules/mtproto.yaml.md) | example.module.mtproto | design | I3-C |
| [docs/examples/config/modules/trojan-tls.yaml](docs/examples/config/modules/trojan-tls.yaml.md) | example.module.trojan-tls | design | I3-C |
| [docs/examples/config/modules/tuic.yaml](docs/examples/config/modules/tuic.yaml.md) | example.module.tuic | design | I3-C |
| [docs/examples/config/modules/vless.yaml](docs/examples/config/modules/vless.yaml.md) | example.module.vless | design | I3-C |
| [docs/examples/hton.future.yaml](docs/examples/hton.future.yaml.md) | config.future-example | R2 | deferred-selected-release |
| [docs/powered.md](docs/powered.md.md) | doc.docs-powered-md | design | P0/I3-C |
| [docs/specs/sdd-configurator.md](docs/specs/sdd-configurator.md.md) | doc.docs-specs-sdd-configurator-md | design | P0/I3-C |
| [docs/specs/sdd-config.md](docs/specs/sdd-config.md.md) | doc.docs-specs-sdd-config-md | design | P0/I3-C |
| [docs/specs/sdd-lifecycle.md](docs/specs/sdd-lifecycle.md.md) | doc.docs-specs-sdd-lifecycle-md | design | P0/I3-C |
| [docs/specs/sdd-installer.md](docs/specs/sdd-installer.md.md) | doc.docs-specs-sdd-installer-md | design | P0/I3-C |
| [docs/specs/sdd-certs.md](docs/specs/sdd-certs.md.md) | doc.docs-specs-sdd-certs-md | design | P0/I3-C |
| [docs/specs/sdd-foreign-adapters.md](docs/specs/sdd-foreign-adapters.md.md) | doc.docs-specs-sdd-foreign-adapters-md | design | P0/I3-C |
| [core/src/host/installation.rs](core/src/host/installation.rs.md) | core.host-installation | R1 | I3-A |
| [etc/caddy/Caddyfile.template](etc/caddy/Caddyfile.template.md) | template.caddy | R1 | I2-A |
| [etc/haproxy/haproxy.cfg.template](etc/haproxy/haproxy.cfg.template.md) | template.haproxy | R1 | I2-A |
| [etc/hysteria/config.yaml.template](etc/hysteria/config.yaml.template.md) | template.hysteria | R1 | I2-A |
| [etc/mtg/config.toml.template](etc/mtg/config.toml.template.md) | template.mtg | R1 | I2-A |
| [services/onomat/runtime/dns/etc/unbound.conf.template](services/onomat/runtime/dns/etc/unbound.conf.template.md) | template.unbound | R1 | I2-A |
| [etc/xray/config.json.template](etc/xray/config.json.template.md) | template.xray | R1 | I2-A |
| [fs-support/Cargo.toml](fs-support/Cargo.toml.md) | build.storage-manifest | R1 | I1-B |
| [fs-support/src/lib.rs](fs-support/src/lib.rs.md) | infra.storage | R1 | I1-B |
| [hton.example.yaml](hton.example.yaml.md) | config.example | R1 | I3-C |
| [images.lock.json](images.lock.json.md) | image.lock | R1 | I2-A |
| [lib/bootstrap](lib/bootstrap.md) | shell.bootstrap | R1 | I3-A |
| [lib/hton](lib/hton.md) | shell.project | R1 | I3-A |
| [logging/Cargo.toml](logging/Cargo.toml.md) | build.logging-manifest | R1 | I0-B |
| [logging/src/lib.rs](logging/src/lib.rs.md) | infra.logging | R1 | I1-B |
| [modules/client-ipv6.yaml](modules/client-ipv6.yaml.md) | module.client-ipv6 | far-future | deferred-far-future |
| [modules/configurator.yaml](modules/configurator.yaml.md) | module.configurator | later-management | deferred-selected-release |
| [modules/doctor-watch.yaml](modules/doctor-watch.yaml.md) | module.doctor-watch | optional | unselected-optional |
| [modules/dot.yaml](modules/dot.yaml.md) | module.dot | optional | unselected-optional |
| [modules/foreign-amnezia.yaml](modules/foreign-amnezia.yaml.md) | module.foreign-amnezia | R1 | I1-A |
| [modules/foreign-outline.yaml](modules/foreign-outline.yaml.md) | module.foreign-outline | optional | unselected-optional |
| [modules/https-proxy.yaml](modules/https-proxy.yaml.md) | module.https-proxy | optional | unselected-optional |
| [modules/hysteria.yaml](modules/hysteria.yaml.md) | module.hysteria | optional | unselected-optional |
| [modules/index.yaml](modules/index.yaml.md) | module.index | R1 | I1-A |
| [modules/ipv6-he.yaml](modules/ipv6-he.yaml.md) | module.ipv6-he | far-future | deferred-far-future |
| [modules/linux-gateway.yaml](modules/linux-gateway.yaml.md) | module.linux-gateway | optional | unselected-optional |
| [modules/management-api.yaml](modules/management-api.yaml.md) | module.management-api | later-management | deferred-selected-release |
| [modules/mtproto.yaml](modules/mtproto.yaml.md) | module.mtproto | optional | unselected-optional |
| [modules/naive.yaml](modules/naive.yaml.md) | module.naive | optional | unselected-optional |
| [modules/openconnect.yaml](modules/openconnect.yaml.md) | module.openconnect | optional | unselected-optional |
| [modules/openvpn.yaml](modules/openvpn.yaml.md) | module.openvpn | optional | unselected-optional |
| [modules/outline-wss.yaml](modules/outline-wss.yaml.md) | module.outline-wss | optional | unselected-optional |
| [modules/private-socks.yaml](modules/private-socks.yaml.md) | module.private-socks | optional | unselected-optional |
| [modules/router-gateway.yaml](modules/router-gateway.yaml.md) | module.router-gateway | optional | unselected-optional |
| [modules/services.yaml](modules/services.yaml.md) | service.index | R1 | I1-A |
| [modules/site-content.yaml](modules/site-content.yaml.md) | module.site-content | optional | unselected-optional |
| [modules/smart-connect.yaml](modules/smart-connect.yaml.md) | module.smart-connect | R2 | deferred-selected-release |
| [modules/softether.yaml](modules/softether.yaml.md) | module.softether | optional | unselected-optional |
| [modules/ssh-session.yaml](modules/ssh-session.yaml.md) | module.ssh-session | optional | unselected-optional |
| [modules/sstp.yaml](modules/sstp.yaml.md) | module.sstp | optional | unselected-optional |
| [modules/tailscale-exit.yaml](modules/tailscale-exit.yaml.md) | module.tailscale-exit | lab-policy-blocked | unselected-optional |
| [modules/telegram.yaml](modules/telegram.yaml.md) | module.telegram | later-management | deferred-selected-release |
| [modules/trojan-tls.yaml](modules/trojan-tls.yaml.md) | module.trojan-tls | optional | unselected-optional |
| [modules/trojan-ws.yaml](modules/trojan-ws.yaml.md) | module.trojan-ws | optional | unselected-optional |
| [modules/tuic.yaml](modules/tuic.yaml.md) | module.tuic | optional | unselected-optional |
| [modules/vless.yaml](modules/vless.yaml.md) | module.vless | R1 | I1-A |
| [modules/web-admin.yaml](modules/web-admin.yaml.md) | module.web-admin | later-management | deferred-selected-release |
| [modules/web-publish.yaml](modules/web-publish.yaml.md) | module.web-publish | R2 | deferred-selected-release |
| [publication/Cargo.toml](publication/Cargo.toml.md) | build.publication-manifest | R2 | deferred-selected-release |
| [publication/src/adapter.rs](publication/src/adapter.rs.md) | publication.adapter | R2 | deferred-selected-release |
| [publication/src/model.rs](publication/src/model.rs.md) | publication.model | R2 | deferred-selected-release |
| [runtime-contract/Cargo.toml](runtime-contract/Cargo.toml.md) | build.runtime-contract-manifest | R1 | I0-B |
| [runtime-contract/src/lib.rs](runtime-contract/src/lib.rs.md) | runtime-contract.wire | R1 | I0-A |
| [runtime/Cargo.toml](runtime/Cargo.toml.md) | build.runtime-manifest | R1 | I0-B |
| [runtime/src/actions.rs](runtime/src/actions.rs.md) | runtime.actions | R1 | I1-C |
| [runtime/src/guardian.rs](runtime/src/guardian.rs.md) | runtime.guardian | R1 | I1-C |
| [runtime/src/lib.rs](runtime/src/lib.rs.md) | runtime.library | R1 | I1-C |
| [runtime/src/main.rs](runtime/src/main.rs.md) | runtime.executor | R1 | I1-C |
| [runtime/src/manifest.rs](runtime/src/manifest.rs.md) | runtime.manifest | R1 | I1-C |
| [runtime/src/probes.rs](runtime/src/probes.rs.md) | runtime.probes | R1 | I1-C |
| [runtime/src/state.rs](runtime/src/state.rs.md) | runtime.state | R1 | I1-C |
| [schemas/hton-v1.schema.json](schemas/hton-v1.schema.json.md) | schema.v1 | R1 | I1-A |
| [schemas/hton-v2.schema.json](schemas/hton-v2.schema.json.md) | schema.v2 | R2 | deferred-selected-release |
| [schemas/module-v1.schema.json](schemas/module-v1.schema.json.md) | schema.module | R1 | I1-A |
| [schemas/registry-v1.schema.json](schemas/registry-v1.schema.json.md) | schema.registry | R1 | I1-B |
| [systemd/hton-acme.service](systemd/hton-acme.service.md) | systemd.acme | R1 | I3-B |
| [systemd/hton-acme.timer](systemd/hton-acme.timer.md) | systemd.acme-timer | R1 | I3-B |
| [systemd/hton-control.service](systemd/hton-control.service.md) | systemd.control | later-management | deferred-selected-release |
| [systemd/hton-data.service](systemd/hton-data.service.md) | systemd.data | R1 | I3-B |
| [systemd/hton-guardian.service](systemd/hton-guardian.service.md) | systemd.guard | R1 | I3-B |
| [tests/integration/client-failover.sh](tests/integration/client-failover.sh.md) | test.failover | R2 | deferred-selected-release |
| [tests/integration/client-ipv6.sh](tests/integration/client-ipv6.sh.md) | test.client-ipv6 | far-future | deferred-far-future |
| [tests/integration/identity-policy.rs](tests/integration/identity-policy.rs.md) | test.identity | R2 | deferred-selected-release |
| [tests/integration/legacy-restore.sh](tests/integration/legacy-restore.sh.md) | test.migration | R1 | F0 |
| [tests/integration/lifecycle.sh](tests/integration/lifecycle.sh.md) | test.lifecycle | R1 | F0 |
| [tests/integration/protocol-matrix.sh](tests/integration/protocol-matrix.sh.md) | test.protocols | R1 | F0 |
| [tests/integration/publication-isolation.sh](tests/integration/publication-isolation.sh.md) | test.publication | R2 | deferred-selected-release |
| [tests/integration/runtime-recovery.sh](tests/integration/runtime-recovery.sh.md) | test.recovery | R1 | F0 |
| [tests/unit/adapters-contracts.rs](tests/unit/adapters-contracts.rs.md) | test.unit-adapters | R1 | U0 |
| [tests/unit/backup-contracts.rs](tests/unit/backup-contracts.rs.md) | test.unit-backup | deferred | U0 |
| [tests/unit/config-contracts.rs](tests/unit/config-contracts.rs.md) | test.config | R1 | U0 |
| [tests/unit/journal-contracts.rs](tests/unit/journal-contracts.rs.md) | test.unit-journal | R1 | U0 |
| [tests/unit/lifecycle-contracts.rs](tests/unit/lifecycle-contracts.rs.md) | test.unit-lifecycle | R1 | U0 |
| [tests/unit/logging-contracts.rs](tests/unit/logging-contracts.rs.md) | test.unit-logging | R1 | U0 |
| [tests/unit/registry-contracts.rs](tests/unit/registry-contracts.rs.md) | test.unit-registry | R1 | U0 |
| [tests/unit/runtime-contracts.rs](tests/unit/runtime-contracts.rs.md) | test.unit-runtime | R1 | U0 |
| [tests/unit/secrets-contracts.rs](tests/unit/secrets-contracts.rs.md) | test.unit-secrets | R1 | U0 |
| [validation/compose.yaml](validation/compose.yaml.md) | validation.compose | R1 | I3-B |
| [validation/run-bundle](validation/run-bundle.md) | validation.bundle | R1 | I3-B |
| [web/admin/index.html](web/admin/index.html.md) | web.html | later-management | deferred-selected-release |
| [web/admin/package.json](web/admin/package.json.md) | build.ui | later-management | deferred-selected-release |
| [web/admin/pnpm-lock.yaml](web/admin/pnpm-lock.yaml.md) | build.ui-lock | later-management | deferred-selected-release |
| [web/admin/src/app/App.tsx](web/admin/src/app/App.tsx.md) | web.shell | later-management | deferred-selected-release |
| [web/admin/src/components/CredentialCreate.tsx](web/admin/src/components/CredentialCreate.tsx.md) | web.credentials | later-management | deferred-selected-release |
| [web/admin/src/components/PlanReview.tsx](web/admin/src/components/PlanReview.tsx.md) | web.plan | later-management | deferred-selected-release |
| [web/admin/src/lib/api.ts](web/admin/src/lib/api.ts.md) | web.api | later-management | deferred-selected-release |
| [web/admin/src/lib/draft.ts](web/admin/src/lib/draft.ts.md) | web.draft | later-management | deferred-selected-release |
| [web/admin/src/lib/events.ts](web/admin/src/lib/events.ts.md) | web.events | later-management | deferred-selected-release |
| [web/admin/src/main.tsx](web/admin/src/main.tsx.md) | web.entry | later-management | deferred-selected-release |
| [web/admin/src/pages/Configuration.tsx](web/admin/src/pages/Configuration.tsx.md) | web.config | later-management | deferred-selected-release |
| [web/admin/src/pages/Connectivity.tsx](web/admin/src/pages/Connectivity.tsx.md) | web.connectivity | later-management | deferred-selected-release |
| [web/admin/src/pages/Devices.tsx](web/admin/src/pages/Devices.tsx.md) | web.devices | later-management | deferred-selected-release |
| [web/admin/src/pages/Features.tsx](web/admin/src/pages/Features.tsx.md) | web.features | later-management | deferred-selected-release |
| [web/admin/src/pages/Login.tsx](web/admin/src/pages/Login.tsx.md) | web.login | later-management | deferred-selected-release |
| [web/admin/src/pages/Operations.tsx](web/admin/src/pages/Operations.tsx.md) | web.operations | later-management | deferred-selected-release |
| [web/admin/src/pages/Overview.tsx](web/admin/src/pages/Overview.tsx.md) | web.overview | later-management | deferred-selected-release |
| [web/admin/src/pages/Publications.tsx](web/admin/src/pages/Publications.tsx.md) | web.publications | later-management | deferred-selected-release |
| [web/admin/src/pages/Users.tsx](web/admin/src/pages/Users.tsx.md) | web.users | later-management | deferred-selected-release |
| [web/admin/src/styles/theme.css](web/admin/src/styles/theme.css.md) | web.styles | later-management | deferred-selected-release |
| [web/admin/tests/workflows.spec.ts](web/admin/tests/workflows.spec.ts.md) | test.ui | later-management | deferred-selected-release |
| [web/admin/tsconfig.json](web/admin/tsconfig.json.md) | build.ui-types | later-management | deferred-selected-release |
| [web/admin/vite.config.ts](web/admin/vite.config.ts.md) | build.ui-config | later-management | deferred-selected-release |
| [web/mask/index.html](web/mask/index.html.md) | site.mask | R1 | I2-A |
| [work/registry.json](work/registry.json.md) | state.registry | R1 | I1-B |

## I3 host/build additions

| Contract | Stable ID | Phase | Wave |
|---|---|---|---|
| [host-support/Cargo.toml](host-support/Cargo.toml.md) | build.host-manifest | R1 | I3-B |
| [host-support/src/lib.rs](host-support/src/lib.rs.md) | infra.host | R1 | I3-B |
| [host-support/src/process.rs](host-support/src/process.rs.md) | infra.host-process | R1 | I3-B |
| [host-support/src/docker.rs](host-support/src/docker.rs.md) | adapter.host-docker | R1 | I3-B |
| [host-support/src/systemd.rs](host-support/src/systemd.rs.md) | adapter.host-systemd | R1 | I3-B |
| [host-support/src/network.rs](host-support/src/network.rs.md) | adapter.host-network | R1 | I3-A |
| [tooling/package-release](tooling/package-release.md) | build.package | R1 | I3-B |
| [docs/BUILD-RELEASE.md](docs/BUILD-RELEASE.md.md) | docs.build-release | R1 | I3-B |

Minimum configuration/orchestration additions: [lib/install_config.py](lib/install_config.py.md),
[etc/install.json](etc/install.json.md), [scripts/setup](scripts/setup.md).
scripts/update implements OR-55 (fast-forward origin/main then update the
HTON-owned composition) and is no longer a blocked compatibility alias.

## Current M1 planning overlay

Before using a historical row for current installation work, read
[INSTALLATION-FILE-GRAPH](../INSTALLATION-FILE-GRAPH.md). It supplies exact new M1
paths, stable IDs, methods/arguments, reciprocal call projections and package
ownership. Existing JSON indexes remain historical baseline until M1-E materializes
the frozen delta during implementation. New helper/schema/entrypoint map pages
begin with the current M1 contract and retain any old section as labeled history.
