# core/src/host/install_input.rs

Stable ID: `core.host-install-input` · Phase: `R1` · Kind: source

Pure construction of a bounded first-publication input candidate. It validates the
typed domain, public IPv4 TCP/443 endpoint and optional non-secret certificate
account email; reads only one coherent successful `DesiredReadResult`; computes
exact typed changes for `core.edge.domains`, `core.edge.listen`, and
`core.certificates.email`; and binds the full source `ReadSet` in the candidate
digest. When certificate metadata is absent, a supplied email is required so the
writer can seed the fixed Let's Encrypt DNS-01/InternetBS metadata. Existing
certificate metadata must already be valid, including a valid email and schema-
valid credential SecretRefs; absent or malformed existing values block rather
than being invented. This function performs no I/O, YAML serialization, approval,
journal or write.

## Methods

- `prepare_input_candidate(input, desired, ctx)` — `InstallInput`, coherent
  `DesiredReadResult`, `OperationContext` → `InstallConfigCandidate` or a typed
  config/input error. It rejects invalid saved tree shape and a no-op candidate.

## Ownership and invariants

- Site-domain syntax is delegated to the shared `installation::valid_site_domain`
  policy; no duplicate domain rule is maintained here.
- Public target must be IPv4 wildcard TCP/443. The only editable targets are
  typed site domain, public edge and certificate account email fields; caller
  paths and patches do not exist.
- Email is a bounded ASCII mailbox with one valid lowercase DNS domain. Email
  provenance must accompany an explicitly supplied value; otherwise only the
  valid saved account email can be reused.
- Invalid saved domains, existing multi-domain values, and unsupported listener scopes/families fail closed
  rather than being truncated by the single-domain/single-endpoint candidate model.
- The digest is SHA-256 over `("install-config-candidate-v1", changes, readSet)`.
  The owner-specific writer independently validates candidate integrity before CAS.

## Dependencies

- `core/src/host/installation.rs` — shared domain syntax validator and caller.
- `core/src/config.rs` — stable digest of canonical serialized typed values.
- `core/src/journal.rs` — safe typed error envelope only; no journal effects.
- `contracts/src/config.rs` — `InstallInput`, `DesiredReadResult`, candidate DTOs.

## Calls / callers

- [core/src/host/installation.rs](installation.rs.md) — preflight composes transient
  site input before host/foreign checks and before any write.
- [core/src/host/install_input_writer.rs](install_input_writer.rs.md) — persists the
  exact reviewed candidate after full preflight; no direct caller bypasses the
  installation coordinator.
- [docs/INTERFACES.md](../../../../../INTERFACES.md) — shared install input contracts.

## Completion

Source is written but not compiled or accepted. It is not wired into normal
`scripts/install` reconciliation until the InstallationCoordinator is implemented.
