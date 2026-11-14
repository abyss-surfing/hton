# SDD CLI — thin public interface and independent runtime bootstrap

Arguments and exits are owned exclusively by [CLI.md](../CLI.md). Current Bash
scaffold does not implement the target. File-level bindings are in the project map.

## Product language

Per OR-13/D102, source code and all user-visible product strings are English.
Product localization is deferred. Do not add non-English product strings or
localization machinery now; Russian text is permitted only in `docs/_lang/ru/`.

- CLI-1 bin/hton resolves root and dispatches fixed scripts/* scripts; unknown command
  is usage failure, not successful help. Bash-first remains the default binding.
- CLI-2 Commands use one structured helper for config/catalogue/plans/transactions;
  no ad-hoc YAML sourcing or separate web/CLI renderer. Complex logic may change
  language if stable interfaces remain consistent.
- CLI-3 Helpers/native services are built externally, pinned and deployed without
  host Rust/Node toolchains or dynamic downloads on accepted VPN startup.
- CLI-4 Read-only operations and dry-run never create files/images/containers or
  install packages. Explicit validate runs isolated Docker tests and is mutating.
- CLI-5 Schema/candidate failures leave accepted data-plane state intact. CLI works
  without web; runtime accepted boot/rollback additionally works without compiler.
- CLI-6 Service validation/dependencies/change actions come from the module/service
  catalogue, not a second service dictionary in every command.
- CLI-7 Separate systemd data/control/guardian/ACME units. The data unit invokes
  trusted runtime startAccepted, never hton update/install; public service wrapper
  cannot recursively invoke itself. No management health dependency on VPN boot.
- CLI-8 Mutation uses independent admission and durable compensation; foreign
  consent, ownership and policy fences apply equally to interactive/noninteractive use.
- CLI-9 Scope release credential reveal separately from R2 single-disclosure policy;
  after migration there is no hidden CLI escape to redisclose managed-user keys.

Acceptance: root discovery/usage/exit matrix, schema+semantic failures, no-write
planning, current-vs-pending evidence, management/compiler-down local recovery,
reboot from accepted bundle, explicit Docker validation and ownership-aware uninstall.

For installation publication, `scripts/install` reports ready only after it has
verified the domain certificate as trusted and served without client warnings,
the website SNI route to its HTTPS backend, and the qualified Amnezia SNI route to
the native Amnezia backend. See sdd-installer and sdd-foreign-adapters.

## Current command ownership

Core-backed scripts/modules, scripts/up and scripts/down implement sdd-lifecycle directly
without API daemon calls. Full upgrade/configurator migration is deferred; R1
manual YAML check/plan/apply remains self-sufficient. Invalid up requires a separate
explicit accepted start; down uses durable registry even with malformed desired
YAML. Systemd/Docker restart policy coordination preserves stopped intent on reboot.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).


I3-A transition: the public dispatcher now implements fixed command selection and
side-effect-free help. Legacy install/uninstall/whoami scaffolds are not a target
binding and remain unavailable through that dispatcher until replaced. Missing host
composition returns 3 rather than running .env-era operations behind R1 syntax.

I3 common argv parsing is shared and bounded; repeated boolean common flags are
idempotent, positional values remain literal and -- terminates common flag parsing.
Command-specific help resolves before helper/deployment availability. Fixed binding
availability cannot be bypassed by dropping an arbitrary executable into scripts/.
D81 now defers builds until completion of selected plans; syntax/docs checks only
remain permitted during implementation.
