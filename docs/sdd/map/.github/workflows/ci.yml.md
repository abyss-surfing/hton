# .github/workflows/ci.yml

Stable ID: `ci.workflow` · Phase: `R1` · Kind: configuration

I3-B build binding written; execution/acceptance evidence is separate. Purpose: External schema/unit/Docker qualification pipeline

Implementation wave: `I3-B` (planned ownership, not completion).

## Features and responsibility

- build

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Untrusted PR jobs receive no deployment/write secrets.

## Calls / reads

- [validation/run-bundle](../../validation/run-bundle.md) — `testBundle`.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


## I3-B concrete binding (2026-11-01)

After D81, automatic CI runs only document/Bash source syntax checks until the
coordinator commits the exact R1 I4-completion marker in IMPLEMENTATION-STATUS.md.
Only then does the build job compile/format. Manual final_acceptance additionally
runs U0 and then F0; missing test scripts fail. Read-only token/pinned actions,
no deployment, no early build or product tests.
