# tooling/build-minimum

Stable ID: `scripts.build-minimum` · M0

Build M0 protected source artifact without compiling Rust.
Contract: docs/specs/sdd-minimum-installation.md and docs/BUILD-RELEASE.md.
Build has no deploy/service effects. Unit checks use private fixtures. Functional
checks create only labelled exact-ID temporary containers on loopback high ports;
cleanup verifies their per-run label and never changes production resources.
