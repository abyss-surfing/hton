# fs-support/src/lib.rs

Stable ID: `infra.storage` · Phase: `R1` · Kind: source · Wave: `I1-B`

Single owner of shared filesystem mechanics in INTERFACES §12. `SecureDir`
owns an open directory descriptor; dependencies are rustix safe Linux filesystem
and process APIs, std clocks and bounded I/O. It imports no Core/runtime/DTOs.

Methods: `open/open_private/create(&Path) -> Result<SecureDir>`;
`child/create_child(&str)`, `try_clone()`, `read(name,usize) -> Option<Vec<u8>>`,
`list(usize) -> Vec<String>`; `lock(Instant) -> LockedDir`;
`LockedDir.compare_exchange(name,Option<&[u8]>,&[u8])`,
`remove(name,&[u8])`, `read/list`, `directory() -> &SecureDir`.

Read opens every path component with NOFOLLOW and only reads regular single-link
files with an explicit byte cap, detecting descriptor changes during reads.
No raw paths/contents appear in errors. Directory creation and mutation require
euid-owned mode0700 directories; files are private mode0600. Locks are nonblocking
flock retries bounded by monotonic deadline. Temp names are generated internally;
durable publication fsyncs file and directory. CAS assumes cooperating writers;
arbitrary external editors remain outside the guarantee. No business CAS/recovery
or logger logic. Readers do not create locks/directories/files. Unsupported file
types, traversal, symlinks, hardlinks, bounds, changed bytes and deadlines fail.

Consumers: Core config reader/stores, independent runtime state/admission storage,
logging. Domain callers retain originals and reconcile uncertain publication.
Tests deferred until U0 after I4; compilation is permitted during integration.

I3 archives: file_size(name,max_file_bytes) and read_range(name,offset,max_bytes,
max_file_bytes) share read's single anchored regular/single-link/no-follow reader
and before/after/path identity checks. File ceiling is explicit and at most 8GiB;
per-read allocation remains <=64MiB. Out-of-range offset conflicts. Whole-file read
retains its existing <=64MiB limit and refuses oversized whole records. Metadata
and ranges are snapshots, not cross-chunk consistency: archive owner binds initial/
final content revisions and rejects changes during collection.

LockedDir.remove_empty_child(name) removes only an empty private exact opened
child after current path identity check, through descriptor-relative unlinkat
REMOVEDIR and parent fsync under cooperating writer lock. No recursive removal,
symlink following or arbitrary directory deletion; nonempty/races fail retaining
remaining files. Host root remains outside cooperating-writer guarantees.

Private large-blob verification uses `file_size_private` and
`read_private_range`, the same bounded anchored reader with required euid owner
and mode0600-equivalent privacy on every segment. StreamWriter output remains
private; verification does not chmod/fsync/create any file. Callers still bind the
whole content digest and before/after manifest identity around chunked reads.
