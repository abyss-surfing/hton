# host-support/src/systemd.rs

Stable ID: `adapter.host-systemd` · Phase: `R1` · Kind: source · Wave: I3-B

Literal system unit metadata observations. Source written; no compilation/product acceptance claimed (D81).

## Methods and responsibility

- `inspect` — UnitName, Budget, Cancellation → UnitFacts; no mutation authority.

Authority and limits: [INTERFACES](../../../../INTERFACES.md), I3 bounded native observation transport / D81.

Calls [host-support/src/process.rs](process.rs.md) — inspect → run.

No Core/runtime contracts, shell, mutation APIs, secret serialization or ownership discovery. Driver errors are closed/redacted; unavailable facts cannot imply success. Consumers must qualify ownership/admission/fingerprints independently.

Literal service/timer names only; fixed show property inventory, exact returned Id, reject missing/duplicate/unknown fields. Absent service-only PID/restart values on timers are None, not fabricated states. No pager/password prompt/actions. [Primary source](https://www.freedesktop.org/software/systemd/man/latest/systemctl.html).

R1 SystemdControl accepts literal known-authorized units with a closed lifecycle
verb enum, no shell/host arbitrary command. Stop/disable includes timers. Domain
owners must supply exact protected unit identity and their approved operation.

`link_platform_units(Budget,Cancellation)` links only the five fixed
`/opt/hton/work/systemd/hton-{data,guardian,acme}.service` and
`hton-acme.timer` and `hton-native-log@.service` paths, then runs daemon-reload. The installation owner must
validate protected template content, journal intent and exclude foreign destination
paths first. No caller path/content/enable/start is accepted by this method.

Maintenance additionally has fixed `stop_platform_units` plus `reload_manager`.
The uninstall owner first proves exact link/template ownership and whole-server
stopped state. `stop_if_loaded(UnitName,Budget,Cancellation)` distinguishes exact
manager not-found state from errors and stops only a known loaded unit. Runtime
uses it for a recorded container's native-log collector; no wildcard discovery.
