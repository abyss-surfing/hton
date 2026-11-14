# web/mask/index.html

Stable ID: `site.mask` · Phase: `R1` · Kind: configuration

Planned file binding; no implemented functionality is claimed. Purpose: Always-available minimal default site

Implementation wave: `I2-A` (planned ownership, not completion).

## Features and responsibility

- contract

## Methods, arguments and result

- No executable methods; declarative contract is owned by its consumer.

## Structures and injected dependencies

Declarative record or document; no executable class

Dependencies: consumers listed below; authority and validation are owned by the corresponding SDD/INTERFACES.

## Logic

- No secret admin/key hints or dependency on admin process.

## Calls / reads

- No internal dependency in this direction.

## Called / read by

- No internal dependency in this direction.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

I2 implementation is a static independent notebook-style placeholder without
credentials, analytics, external assets, admin links or JavaScript dependencies.
Caddy embeds the constant HTML as the ordinary fallback response; content is
public and never depends on management availability or a secret-bearing directory.
