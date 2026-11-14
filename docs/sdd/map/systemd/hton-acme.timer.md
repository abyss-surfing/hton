# systemd/hton-acme.timer

Stable ID: `systemd.acme-timer` · Phase: `R1` · Kind: configuration

I3 source binding written; host integration and acceptance are tracked separately. Purpose: Persistent bounded renewal schedule

Implementation wave: `I3-B` (planned ownership, not completion).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Jitter/retry policy, no permanent ACME daemon.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


I3 concrete binding: helpers are fixed absolute /opt/hton/libexec paths. Data unit
is boot-only, reads durable intent and accepted bundle, and has no ExecStop that
would convert saved Running intent to Stopped during reboot. Public down goes
through Core/runtime whole-server stop and disables all registered restart owners.
Guardian has independent lifecycle (no PartOf/Requires management/data chain), and
its restart budget is bounded. ACME service calls fixed certs.renew --automatic,
whose Core owner checks stopped intent before automatic jobs; timer never grants
foreign/443 consent. Code is read-only to service processes, only config/work
writes explicitly permitted per unit; no compiler/download or optional web dependency.
Compose source is non-authoritative seed only: Core renderer owns all effective
services/images/mounts/network and independent admission qualifies the final bundle.
