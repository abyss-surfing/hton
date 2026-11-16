# SDD Naive and MTProto

Settings shape: root features.<id>.enabled plus config/modules/<id>.yaml settings (sdd-config.md). Old .env names
are migration inputs only, not another configuration contract.

- NMP-1 HAProxy passes own TLS to Caddy with the existing Naive fork of forwardproxy.
  Caddy handles authenticated HTTP/2 CONNECT and default site. No HAProxy H2 parsing
  or extra h2c conversion in this baseline; verify exact pinned client/build.
- NMP-2 Export qualified Naive format from accepted native credential binding only.
  Public HTTP proxy semantics are not a full TCP/UDP device VPN.
- NMP-3 config/modules/mtproto.yaml: settings.backend is mtg or telemt; enabled:false disables
  the capability. mtg is an own private container; telemt is an existing consented
  adapter/relay target, not a promise of zero additional relay infrastructure.
- NMP-4 Exact settings.camouflageName routes raw before wildcard/default TLS.
  ALPN absence is not protocol identity. Reject conflicting exact raw assignments;
  nested own-domain MTProto is possible only with explicit precedence and safe
  internal TLS fronting target, avoiding public-name loops. OR-38/OR-39 supersede
  OR-31's single-SNI default: the configured MTProto name has its own exact route
  to mtg/qualified telemt, which fronts to the private real HTTPS site. REALITY's
  separate name falls back directly to that site. Existing accepted chains require
  a reviewed transition, never silent replacement.
- NMP-5 Generate fakeTLS secret through the selected backend; mtg hex format uses
  ee prefix. Do not reformat another backend's existing secret blindly. Share link
  advertises the selected public port, never the private backend listener.
- NMP-6 Backend/config changes pass independent final-bundle qualification and
  runtime guardian. Drain/native reload semantics are measured, not assumed.
- NMP-7 Per-user keys require backend support for independently revocable users;
  one shared backend secret cannot be mislabeled as separate R2 tokens.

Acceptance: real Naive/Telegram clients, unauthorized/ordinary site behavior,
raw SNI collision/absence, disabled reservations, fronting-loop rejection, reload
and management-down tests. DPI invisibility is not guaranteed by a website/port.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).
