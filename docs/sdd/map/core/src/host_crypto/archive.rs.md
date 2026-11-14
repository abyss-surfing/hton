# core/src/host_crypto/archive.rs

R1 I3-B. AgeArchiveStore implements ArchiveCrypto over protected work/backups.
A self-contained authenticated envelope contains the canonical ArchiveManifest and
ordered per-object/per-chunk plaintext/ciphertext hashes. Four MiB age-encrypted
chunks stream independently; no tar paths, symbolic links, device entries or whole
32 GiB in-memory allocation. Files use generated bucket/index names, not archive
paths. Decryption identities/recipient public keys resolve outside the archive.

Encrypt records intent before chunk publication, commits envelope/metadata atomically,
and creates an exact transaction completion reference. Interrupted begin without a
completed record requires explicit reconciliation. inspect authenticates envelope,
checks canonical manifest/entry/chunk order, sizes/digests; read_chunk verifies the
age authentication and exact plaintext hash before bytes leave the port. Retention
prunes only completed non-protected archives; pending/uncertain artifacts remain.
Raw plaintext has no debug/serialization output. Domain restore guards are rechecked
by BackupCoordinator/BackupInventory, not bypassed by this storage adapter.
