# Admin UX plan

Status: planned; no frontend has been built. Web is deferred optional administration; R1 has no UI;
R2 adds personal accounts, one-time credentials, connectivity agent and publication.
Design contracts, not a claim of audited dependencies or finished UI.

## Selected starting point and alternatives

Use React + Vite static assets, shadcn/ui accessible components and TanStack Query.
Start with official sidebar/table/form/dialog primitives and a small design-token
layer, not a full bespoke component library or copied VPN panel. Build externally;
production serves static files from the Rust admin, with no Node runtime.

Refine is a viable resource/admin scaffold with data/auth/live providers; react-admin
is a viable CRUD framework; Mantine supplies a broad component suite. They do not
supply our revision-aware plans or one-time credential semantics. Pilot one module
screen with shadcn first; choose a single foundation before committing dependencies.
Do not combine all these toolkits or copy GPL panel UI into MIT without a license plan.

References: [shadcn](https://ui.shadcn.com/docs),
[license](https://github.com/shadcn-ui/ui/blob/main/LICENSE.md),
[Refine](https://refine.dev/core/docs/), [react-admin](https://marmelab.com/react-admin/),
[Mantine](https://mantine.dev/), [TanStack](https://tanstack.com/query/latest).

## Navigation and presentation

- Overview: overall readiness, active connections, pending plan, cert expiry and
  a small bounded CPU/RAM summary. Put failures/actionable next steps above charts.
- Connections: search/filterable optional module table with scope, availability,
  desired state, observed state and resource evidence. Core is a pinned group
  without off switches. Details open in a drawer; avoid 30 permanently open cards.
- Devices: choose TV/Quest/phone/router and a compatible client/profile; expose
  protocol/transport details progressively. No fictional one-click stock-router support.
- Configuration: schema-driven forms and protected diagnostic editing over parameter drafts,
  redacted diff, validation errors, dependencies and pending Apply bar.
- Operations: current/previous plans, progress, audit, bounded redacted diagnostics,
  certs and protected-profile recovery from the private `.secrets` repository. No terminal or raw Docker button.
- R2 user space: own protocol profiles and named tokens only; admin user browser
  sees all metadata. Publication and smart-connect pages appear only when qualified.

Use semantic colors, compact spacing, readable typography, dark/light/system theme,
keyboard navigation/focus, labeled switches, non-color status cues and reduced-motion
support. Persist harmless preferences only. Mobile layout and long/localized names
must work; avoid gratuitous charts/animations and CDN-fetched runtime assets.

## Mutations and reactive state

A toggle edits the draft, never immediately restarts a container. Sticky Apply bar
shows batch count; preview reports dependencies, newly exposed listeners, estimated
interruption, resource evidence and local-approval blocks. Validate -> plan -> apply
exact revision -> verify. Only verified observed state is green/ready; optimistic
updates are allowed for drafts/labels, not actual network readiness or revocation.

Use one bounded redacted SSE stream with event IDs and reconnect/resync for operation
progress/status; fall back to bounded backoff polling if unavailable. No per-card
polling loops, browsing payload telemetry or secret events. Backend events invalidate
relevant metadata queries; stale plans return conflict with rebase/preview guidance.
Never overwrite manual YAML edits or allow a submit from a stale browser tab.

Forms derive field metadata/types/validation from the canonical config schema and
module descriptors; UI layout hints are presentation only. Server validation remains
authoritative. Do not maintain a second incompatible hand-written settings schema.

## Credentials and IPv6 (R2)

Create drawer: user -> allowed protocols (configured default Amnezia preset) ->
editable name (date default) -> Internet exit IPv4/IPv6 -> preview/create. Disable
unsupported family/profile combinations with reasons. Explain IPv4 tunnel transport
versus IPv6 server egress; never silently downgrade the selected exit family.

Creation response alone contains raw keys, QR and client downloads. Keep it outside
query caches, browser storage, SSR state, URLs, logs and SSE. On close/refresh/back,
only metadata remains. Admin has no reveal exception. Lost response -> revoke and
recreate. Rename changes a label; replacing key/family creates a new credential.

## Acceptance

Test keyboard/mobile/theme/accessibility, empty/disabled/blocked/unavailable states,
slow/disconnected streams, revision conflicts, concurrent tabs, invalid YAML,
failed apply/recovery, core protection and local-consent boundaries. R2 tests
cross-user access, one-time reveal and loss/retry, strict family and publications.
Measure bundle size and browser interaction as well as server resource cost.

Current binding: UI always uses the optional full management API; parameter edits
use the standalone configurator. No whole-file replacement upload. Invalid target
YAML remains diagnosable without preventing tool/API startup. R1 has no UI; family
selector and IPv6 views are far future. Unit then functional tests execute after
completion of the selected implementation, not between feature-writing waves.
