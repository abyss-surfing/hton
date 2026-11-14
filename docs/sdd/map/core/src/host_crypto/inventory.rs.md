# core/src/host_crypto/inventory.rs

R1 I3-B FsBackupInventory maps validated ArchiveLocation scopes to protected Config/
Work capabilities. Snapshot performs bounded recursive inventory/hash/reread checks;
required pinned image exports are explicit journalled backup work, not startup pulls.
Decryption identity/backups/cache/sockets/distribution code are excluded.

Restore validates full authenticated config overlay, secret/registry/identity records
through their owners, fixed categories/formats and explicit foreign association match.
Archived runtime state is staged only; journal history merges inertly without
replaying archived active intents. Config originals are journal-protected before
writes. Image blobs use streaming new-file publication, hash-qualified fixed Docker
load and exact cached pin checks. Every write rechecks current stopped reservation.

Restore planning treats archived runtime state as a new staged logical object, not
an overwrite of live runtime/state.json. Its expected current object is None, so
StopIntent/maintenance epoch changes cannot invalidate the archived-state staging
baseline. Large immutable image streaming uses the declared final filename from
begin_stream through finish_new, with no mismatched temporary target alias.

Historical owned registry locators/version drift is rejected during prevalidation,
before shutdown or writes, unless an explicit native restoration mapping has already
qualified matching current physical inventory. Another installation's runtime state
is rejected before publication. This prevents apparently successful restore from
leaving unobservable old Docker IDs; no arbitrary discovery/re-adoption is introduced.
