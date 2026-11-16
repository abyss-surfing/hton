# core/src/host/systemd_installation.rs

Stable ID: `core.host-systemd-installation` · Phase: R1 · Wave: I3-A

One owner of fixed platform systemd template delivery. `install(app,ctx)` validates
the five compiled unit templates and exact destination link identities, journals
their digest, seeds immutable protected work/systemd files, links only the five
literal platform units through host-support and reloads the system manager.
It never starts/enables services or timers and never overwrites foreign units or
changed staged templates. Repeated/interrupted delivery is idempotent and finishes
only exact matching pending installation intents after all links/files recheck.
Template drift remains a retained-state conflict requiring an explicit upgrade.

Dependencies: installation coordinator, JournalStore, SecureDir, fixed
SystemdControl platform-unit delivery. Effects are confined to protected fixed
template files, their exact /etc/systemd/system links and daemon-reload.
No shell, arbitrary unit path, user-supplied unit content or service action.

`uninstall(app,ctx)` is a separate maintenance capability after verified whole-server
shutdown: checks stopped intent/no trial, exact protected template bytes and link
destinations, disables/stops fixed data/ACME jobs and guardian, then removes only
those exact links and reloads systemd. It retains protected template originals.
The runtime uninstall owner stops exact native-log instances before container
removal; this owner never uses a template wildcard to stop arbitrary instances.
