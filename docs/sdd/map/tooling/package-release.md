# tooling/package-release

Stable ID: `build.package` · Phase: `R1` · Kind: source · Wave: I3-B

External gated release candidate packaging. Source written; no compilation/product acceptance claimed (D81).

## Methods and responsibility

- `package` — --output DIRECTORY; completed-plan marker and clean committed source → checksum/provenance candidate archive; no publish/deploy.

Authority and limits: [INTERFACES](../../../INTERFACES.md), I3 bounded native observation transport / D81.

Calls [core/Cargo.toml](../core/Cargo.toml.md) — package → declarative dependency.

Calls [runtime/Cargo.toml](../runtime/Cargo.toml.md) — package → declarative dependency.

Readiness marker and clean committed source precede compilation. External fixed Rust/musl builder; no tests, release publication or VPS installation. Metadata-normalized archive with per-file provenance and adjacent checksum; unsigned candidate only. No bit-for-bit/signature/acceptance claim.
