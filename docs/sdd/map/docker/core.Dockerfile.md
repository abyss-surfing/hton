# docker/core.Dockerfile

Stable ID: `build.core` · Phase: `R1` · Kind: configuration

I3-B build binding written; execution/acceptance evidence is separate. Purpose: Externally built management artifacts

Implementation wave: `I3-B` (planned ownership, not completion).

## Features and responsibility

- build

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- Pinned builders and production images; no VPS compilation.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


## I3-B concrete binding (2026-11-01)

I3-B uses a FROM scratch artifact-export target copying externally built static
libexec/hton-core and libexec/hton-runtime from the release package tree. It is not
a runtime image or a builder, has no ENTRYPOINT and executes no installation or
compiler commands. Scratch has no mutable upstream image tag. The release workflow
owns the exact compiler/toolchain and package checksums; native startup never builds.
