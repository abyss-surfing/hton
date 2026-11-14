# core/src/host/uninstall.rs

Stable ID: `host.uninstall` · R1 · I4

`execute(app,args,ctx)` owns exact installation removal composition. It accepts
only --keep-certs/--keep-env/--foreign/--purge. Default removal retains canonical
config, secrets, certificates, code snapshots and recovery material; no recursive
source/state deletion occurs. --keep-certs/--keep-env explicitly retain already
protected categories. --purge needs a separately qualified full deletion inventory
and fails preflight when absent. --foreign can target only proved menu-created
foreign resources; associated/unqualified foreign ownership blocks before effects.

Preflight binds the protected registry and exact resource IDs. Owned Docker
resources are removed only through independent Runtime's UninstallOwned action,
after whole-server down durably disables autostart and verifies stopped state.
Runtime independently rechecks shutdown authority, Owned container identity,
stopped/disabled evidence and run epoch under its effect gate; each native absence
is confirmed through complete bounded inventory and retained as a typed removal
receipt. Recorded container IDs scope native-log collector stops; no wildcard.

The uninstall intent is durable before stop/removal. Resume reuses exact registry
and resource identities, refusing re-add/drift; native removal records distinguish
known completed deletion from unexplained absence. After native removal, the
fixed systemd owner stops/disables/unlinks only exact delivered platform units.
Config/secret ownership remains retained, and any incomplete stop/removal/unlink
produces partial/recovery-required with precise survivors; no false success.
