# core/src/host/install_input_writer.rs

Stable ID: `core.host-install-input-writer` · Phase: `R1` · Kind: source

Narrow journalled writer for reviewed `InstallConfigCandidate` values. It edits
only allowlisted site domain, public edge and certificate email values in the
known root `hton.yaml`, validates the opaque local input capability, preserves
unrelated YAML values, and uses descriptor-relative lock/CAS. A candidate may
seed fixed Let's Encrypt DNS-01/InternetBS metadata plus the two fixed credential
SecretRefs only when the certificates object was absent in the reviewed root.
For existing metadata it changes only `email` and preserves provider and refs;
malformed/missing existing metadata blocks. Recovery accepts only the exact
protected original or exact replacement derived from the journaled candidate.

## Methods

- `commit(candidate, expectedReadSet, approval, ctx)` — candidate + coherent read
  set + `InstallInputApproval` → revision-only `InstallConfigReceipt`; journal
  `Intent::InstallConfig` and protected original precede CAS.
- `commit_for_install(candidate, expectedReadSet, ctx)` — crate-private install
  path for exact typed CLI values, which are direct owner consent; independently
  validates the candidate/read set and CAS-checks root bytes without minting or
  accepting a generic `InstallInputApproval`.
- `recover(transactionId, ctx)` — exact install-config journal record → reconciled
  receipt or recovery-required. It never accepts a third document state.

## Ownership and limits

- Only root object `root` / file `hton.yaml` is writable here; VPN settings remain
  exclusively owned by `core.port-allocation` and each descriptor's module settings.
- It is a primitive for InstallationCoordinator, not a directly dispatched command.
- The coordinator must re-read and compare the complete candidate `ReadSet` just
  before `commit`; this writer independently CAS-checks the root document it owns.
- `install.reconcile` calls the direct typed-input path only after fresh plan and
  read-set rechecks. First-root values are folded into the canonical root seed.
  Other operations continue to use `commit` with the opaque local capability.

## Dependencies

- `core.host.approvals` — validates candidate fields; generic writer path also
  validates opaque local approval.
- `core.config` — bounded root decoding, content revision and digest.
- `core.journal` / `infra.storage` — exact intent, protected original, lock/CAS.
- `contracts/src/config.rs` — typed fields, candidate and receipt.

## Completion

Source binding exists but has not been compiled or accepted.
