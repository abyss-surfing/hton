# Documentation index

The [M1 implementation plan](plans/INSTALLATION-IMPLEMENTATION.md) and later
owner requirements define the minimum profile. See [implementation status](IMPLEMENTATION-STATUS.md)
for shipped behavior and the scope of existing acceptance evidence.

R1 I0–I2 libraries, engine serializers and domain coordinators are written and
compile; concrete native ports and installed host/CLI composition remain. The integrated VPN release is not complete.

Current target product ownership: HTON is the server platform, D6ton the client
platform, Tartarus the deployment/development-environment platform, and Onomat
owns VPN and Unbound as server-side services. See [OR-69](OWNER-REQUIREMENTS.md#or-69-server-client-platforms-onomat-owns-vpn-2026-11-13)
and [D177](DECISIONS.md). Existing implementation remains monolithic pending
authorized migration.

- [Implementation progress and remaining packages](IMPLEMENTATION-STATUS.md)

- [Final consistency review](FINAL-REVIEW.md)
- [Binding owner clarifications](OWNER-REQUIREMENTS.md) — OR-68 records the
  target split and OR-69 records HTON as server platform, D6ton as client
  platform, Tartarus as development-environment platform, and VPN as a separate
  server-side service
- [Requirements and authority](DECISIONS.md) — D176 and D177 record these
  boundaries
- [Architecture](ARCHITECTURE.md) — the top note marks the edge sections as the
  pre-split M1 path
- [Phased implementation plan](ROADMAP.md)
- [CLI contract](CLI.md)
- [Private full-profile secrets repository](SECRETS.md)
- [Area SDD index](specs/README.md)
- [Project-shaped per-file map](sdd/map/README.md)
- [Logical file/call graph](sdd/file-index.json)
- [Module design index](sdd/module-index.json)
- [Release YAML example](../platform/hton.example.yaml), [future YAML example](examples/hton.future.yaml)
- [Release schema](../platform/schemas/hton-v1.schema.json), [future schema](../platform/schemas/hton-v2.schema.json)
- [Reuse analysis](REUSE-ASSESSMENT.md), [provenance](powered.md), [admin UX](ADMIN-UX.md)
- [Product opportunities: modules, ready-made technologies, improvements](PRODUCT-OPPORTUNITIES.md)
- [Systemic Core gaps](CORE-GAPS.md)
- [Device research](DEVICE-COMPATIBILITY.md), [edge rationale](EDGE-PROXY-ANALYSIS.md)
- [User-script pages](scripts/) and [localized Russian overview](_lang/ru/README.MD)
- [Chinese overview](_lang/cn/README.MD)

Schemas/examples and the map are reviewable design artifacts. They do not install
packages, validate a real VPS, constitute a security audit or enable any future feature.
docs/_lang/ru/ and docs/_lang/ru/tasks/ are historical/local material; use the tracked current
contracts above. SDD documents themselves are not recursively mapped as product
source files; generated runtime artifacts and third-party internals have producing
file/module contracts rather than infinite individual source-file specifications.

Current deep planning entry points:

- [Structures, dependencies, methods, arguments and API](INTERFACES.md)
- [Future parallel implementation orchestrator](IMPLEMENTATION-ORCHESTRATOR.md)
- [Module lifecycle](specs/sdd-lifecycle.md)
- [Deferred standalone configurator](specs/sdd-configurator.md)
- [Per-module settings schema](../platform/schemas/module-v1.schema.json)
- [Registry schema](../platform/schemas/registry-v1.schema.json)

Current owner development sequence: plan -> complete implementation -> unit tests
-> functional tests -> determine further acceptance. Older phased test gates are
superseded; document consistency checks do not constitute product acceptance.

Current tracked preparation: [M1 installation](plans/INSTALLATION-IMPLEMENTATION.md),
[security/DNS source review](SECURITY-MASKING-REVIEW.md),
[managed Amnezia SDD](specs/sdd-managed-amnezia.md),
[client page SDD](specs/sdd-client-access-page.md),
[exact M1 file graph](sdd/INSTALLATION-FILE-GRAPH.md).
