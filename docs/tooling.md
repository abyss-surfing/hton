# Internal tooling

Not user-facing and not delivered by `scripts/deploy`. Run from a development
checkout:

- `build-minimum` — package the standalone minimum source artifact.
- `package-release` — external release builder (Rust/musl; explicit invocation).
- `build-amnezia-image` — build the pinned managed Amnezia Xray image.

User commands live only in `scripts/`.
