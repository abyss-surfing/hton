# External build and release candidates

Current 2026-11-07 implementation authority: [M1 implementation plan](plans/INSTALLATION-IMPLEMENTATION.md).
It freezes OR-37–OR-49 and OR-52–OR-55 and supersedes conflicting older
design/default descriptions below; historical source/build/test observations
remain historical evidence. The OR-49/OR-52/OR-53 slices are implemented in the
minimum path. The Rust workspace build is currently blocked by documented
full-R1 integration errors; release packaging and live acceptance are not claimed.
See IMPLEMENTATION-STATUS.md for the exact failure list.

The selected OR-15 minimum has its own completed source closure and M0 marker.
Logs rotate on size and by age; entries older than seven days are deleted (OR-53).
The private `.secrets` repository is the durable copy; there is no backup/restore
release artifact (OR-54). Build it with `tooling/build-minimum --output /absolute/output`; then run
`python3 -B -m unittest discover -s tests/minimum -p 'test_*.py' -v`, followed by
`python3 -B tests/minimum/functional.py`. The package contains the current
deployable scripts and inventory, with no credentials, runtime state or Git data.
This is a minimum source artifact, not the full R1 binary release below.


D81 postpones compilation until every selected implementation plan is complete.
The coordinator must first record the exact line
`<!-- hton-implementation-complete: R1 -->` in IMPLEMENTATION-STATUS.md based on
actual I4 completion. That marker is absent now. It is not a user bypass flag or
proof of tests/security acceptance. CI runs source/document checks only before it;
compile jobs and release packaging refuse to proceed until it exists.

After implementation, the manual external builder is:

```sh
tooling/package-release --output /absolute/build-output
```

It requires a clean committed source tree, Linux x86_64 GNU builder, exact Rust
1.99.1 with the x86_64-unknown-linux-musl standard library, rust-lld, Python, Git,
GNU tar/gzip/coreutils, readelf and ripgrep. The recipe never installs a toolchain
or compiles on the target VPS. It builds locked hton-core and hton-runtime static
helpers, checks absence of an ELF interpreter, packages protected distribution
inputs and places helpers in libexec. No config/work/log state is archived.

The candidate contains BUILD-PROVENANCE.json with commit, target, compiler, lock
hash and per-file SHA256. Tar metadata and gzip headers are deterministic for the
same compiled inputs; bit-for-bit reproducibility across builders is not established.
An adjacent archive checksum detects accidental corruption. These artifacts are
unsigned development candidates: a checksum from the same untrusted channel is
not publisher authentication. Production distribution must authenticate its release
source/signature before installation; the current recipe claims no signing evidence.

The workflow uploads candidate artifacts with read-only repository permissions;
it neither publishes a release/image nor deploys or starts product services.
docker/core.Dockerfile is a scratch-based artifact export target for already built,
verified helpers, with no executable image entrypoint or image/toolchain download.

After the completed-plan build comes U0, then F0. Final acceptance requires explicit
workflow dispatch and the readiness marker. Missing test scripts fail rather than
skip. Running the packaging recipe does not run tests or claim their results.

## M1 next-build boundary

Future M1-C implements the pinned native-image recipe/builder and metadata rules;
B1 runs it only after M1-A–E integration is actually complete. Record actual
per-architecture source checksums, maintained base digest, OCI image digest and
provenance; missing facts block build/release, never become placeholder digests.
No target build, automatic upstream latest, external image publication or deployment
is authorized by this plan. M1-E delivers only used minimum helpers/refresh units.
Full-R1 build markers above do not grant M1 readiness; U1 then F1 follow B1.
