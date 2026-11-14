# docs/CLI.md

Stable ID: `doc.docs-cli-md` · Phase: `design` · Kind: documentation

Planned file binding; no implemented functionality is claimed. Purpose: Tracked docs/CLI.md document contract

Implementation wave: `P0/I3-C` (planned ownership, not completion).

## Features and responsibility

- documentation

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Own only its documented area; implementation claims require evidence and current status links.
- Do not duplicate CLI/config/runtime safety authority; link canonical sources.
- First publication gathers site values through bounded non-secret prompts; VPN ports are allocated by Core in 5000–6000, persisted in service settings and reused. Public TCP/443 remains the edge. Protected DNS secretRefs remain in SecretStore.
- Install profiles are catalogue-owned first-run suggestions and never replace saved selections; `--menu` revisits saved choices through Core. Profile/module selection is explicit, revision-bound and part of the reviewed install transaction; it is not foreign association or remap consent.
- Public listener/foreign remap consent remains separate and is not bypassed by `--yes`; port conflict is checked before config write/effects.
- `scripts/install` source path stores typed site/email inputs and exact VPN allocation, then runs certificate issue/reuse, normal ingress apply and local HTTPS/body verification; completion remains partial when credential/import, native setup, apply or verification is incomplete.
- `scripts/install` reports ready only when HAProxy runs from the active `/opt/hton` deployment, the site SNI serves HTTPS with a valid publicly trusted domain certificate, and qualified Amnezia SNI reaches its native backend without REALITY termination; otherwise it reports the exact degraded/blocked state.
- `scripts/install` configures the `hton` code-read group, deploys current working-tree files into `/opt/hton`, excluding `.git`, `docs/_lang/ru/tasks/`, generated build output and mutable state, then re-enters the installed CLI. `scripts/deploy` hashes the source inventory and atomically activates the copied source snapshot; no release manifest or clean commit is required.
- Do not describe Core install wiring as absent. Current host has no trusted `example.com` certificate; Amnezia remap remains qualification/snapshot gated. Link to the dated evidence in IMPLEMENTATION-STATUS.md.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.
