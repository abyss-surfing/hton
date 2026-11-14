# core/src/host_crypto/application.rs

R1 I3-B. Concrete CLI composition for certificates, archive setup/create/list/restore,
identity key issuance and deliberate accepted export. handle returns metadata only;
output returns HostOutput::Protected for scoped reveal/export, never secret Value.

Certificate configured names derive current Core desired input; protected public CA
trust snapshot is seeded only in explicit journalled issue/setup, never read-only
verification. Forced issue/rotation/restore use exact bounded nonblocking TTY proofs;
ordinary --yes is not authority. Credential bytes are only resolved through SecretStore.

Backup initialize alone generates age identity, stores it0600 outside its own archive
and records recipient metadata; metadata contains only path/public fingerprint and
off-host-copy guidance. Create/list/restore require initialized credentials; authenticated
logical archive and stopped runtime reservation owners execute all operations.

R1 keys dns --import-key ABSOLUTE_FILE --import-password ABSOLUTE_FILE imports only
configured InternetBS SecretRefs, private euid-owned bounded regular files through
storage.read_private, and SecretStore.import with acme/dns.internetbs grants. At most
one terminal line ending is removed; embedded LF/CR/NUL and invalid UTF-8 reject env
injection. Existing exact material is reused; replacement requires --force and an
exact TTY challenge. --all never generates third-party credentials. Dry-run neither
reads files nor discloses values. DNS import metadata includes refs only.

Forced replacement of an accepted UUID preflights the same effects revocation owner
before allocating a new secret; absent policy-clean fallback refuses with current
binding intact. Pending disabled issuance remains partial metadata, not accepted
success/disclosure. Native Xray JSON export is qualified; unsupported client/format/
device recipes fail rather than substitute. Protected --out requires existing private
directory, no overwrite and stdout only on TTY.

Explicit backup/trust initialization retries resync retained exact bytes and resolve
only matching initialization intents after proven complete state. They never mark
unknown backup/issuance operations complete from unrelated file presence.
