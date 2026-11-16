//! Age-encrypted logical chunks. No archive-selected paths or plaintext disk staging.
use crate::{
    backup::*,
    config::digest,
    journal::{Intent, JournalStore, OperationControl, deadline, ensure_mutating, failure},
};
use hton_contracts::{config::ReadCoherence, operations::*};
use hton_host::{
    Budget as NativeBudget, Cancellation,
    crypto::{AgeRecipient, CryptoTools, PrivateFile},
};
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const CHUNK: usize = 4 * 1024 * 1024;
const ENVELOPE_LIMIT: usize = 8 * 1024 * 1024;
const CIPHER_LIMIT: usize = 16 * 1024 * 1024;
struct Cancel<'a> {
    control: &'a dyn OperationControl,
    context: &'a OperationContext,
}
impl Cancellation for Cancel<'_> {
    fn cancelled(&self) -> bool {
        self.control.is_cancelled(&self.context.cancellation)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Chunk {
    offset: u64,
    size: u64,
    ciphertext_size: u64,
    plaintext: Sha256Digest,
    ciphertext: Sha256Digest,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EncryptedEntry {
    object: ObjectId,
    chunks: Vec<Chunk>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    schema_version: u32,
    archive_id: ObjectId,
    transaction_id: TransactionId,
    manifest: ArchiveManifest,
    objects: Vec<EncryptedEntry>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Begin {
    schema_version: u32,
    archive_id: ObjectId,
    transaction_id: TransactionId,
    request: Sha256Digest,
    manifest: Sha256Digest,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Completion {
    schema_version: u32,
    archive_id: ObjectId,
    transaction_id: TransactionId,
    metadata: EncryptedArchiveMetadata,
}

pub struct AgeArchiveStore {
    root: Arc<SecureDir>,
    tools: CryptoTools,
    recipients: BTreeMap<ObjectId, AgeRecipient>,
    identity: PrivateFile,
    control: Arc<dyn OperationControl>,
    journal: Arc<JournalStore>,
}
impl AgeArchiveStore {
    pub fn new(
        root: Arc<SecureDir>,
        tools: CryptoTools,
        recipients: BTreeMap<ObjectId, AgeRecipient>,
        identity: PrivateFile,
        control: Arc<dyn OperationControl>,
        journal: Arc<JournalStore>,
    ) -> Self {
        Self {
            root,
            tools,
            recipients,
            identity,
            control,
            journal,
        }
    }
    fn budget(&self, ctx: &OperationContext) -> Result<NativeBudget> {
        {
            if self.control.is_cancelled(&ctx.cancellation) {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationCancelled,
                    "archive operation cancelled",
                ));
            }
            let remaining = ctx
                .deadline
                .0
                .saturating_sub(self.control.now_ms())
                .min(300_000);
            if remaining == 0 {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDeadline,
                    "archive deadline reached",
                ));
            }
            NativeBudget::crypto(
                std::time::Instant::now() + std::time::Duration::from_millis(remaining),
                CIPHER_LIMIT,
            )
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationDeadline,
                    "archive native budget invalid",
                )
            })
        }
    }
    fn archive_name(id: &ObjectId) -> String {
        format!("a-{}", &digest(id.as_str().as_bytes()).as_str()[7..])
    }
    fn tx_name(id: &TransactionId) -> String {
        format!("t-{}", &digest(id.as_str().as_bytes()).as_str()[7..])
    }
    fn archive(&self, id: &ObjectId, ctx: &OperationContext) -> Result<SecureDir> {
        self.root
            .child("archives")
            .and_then(|dir| dir.child(&Self::archive_name(id)))
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive unavailable or unsafe",
                )
            })
    }
    fn tx_bucket(
        &self,
        id: &TransactionId,
        create: bool,
        ctx: &OperationContext,
    ) -> Result<Option<SecureDir>> {
        let name = Self::tx_name(id);
        let bucket = &name[2..4];
        let transactions = if create {
            self.root.create_child("transactions")
        } else {
            self.root.child("transactions")
        };
        let transactions = match transactions {
            Ok(dir) => dir,
            Err(hton_storage::Error::NotFound) => return Ok(None),
            Err(_) => {
                return Err(failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive transaction directory unavailable",
                ));
            }
        };
        match if create {
            transactions.create_child(bucket)
        } else {
            transactions.child(bucket)
        } {
            Ok(dir) => Ok(Some(dir)),
            Err(hton_storage::Error::NotFound) => Ok(None),
            Err(_) => Err(failure(
                ctx,
                ErrorCode::IoUnavailable,
                "archive transaction bucket unavailable",
            )),
        }
    }
    fn encode<T: Serialize>(value: &T, ctx: &OperationContext) -> Result<Vec<u8>> {
        serde_json::to_vec(value).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationConflict,
                "archive record encoding failed",
            )
        })
    }
    fn publish(
        directory: &SecureDir,
        name: &str,
        bytes: &[u8],
        ctx: &OperationContext,
        control: &dyn OperationControl,
    ) -> Result<()> {
        let locked = directory.lock(deadline(ctx, control)?).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "archive publication lock unavailable",
            )
        })?;
        let existing = locked.read(name, CIPHER_LIMIT).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "archive publication read failed",
            )
        })?;
        if existing.as_deref().is_some_and(|old| old != bytes) {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "immutable archive object already exists",
            ));
        }
        locked
            .compare_exchange(name, existing.as_deref(), bytes)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "archive publication durability uncertain",
                )
            })
    }
    fn entry_directory(
        archive: &SecureDir,
        index: usize,
        create: bool,
        ctx: &OperationContext,
    ) -> Result<SecureDir> {
        let bucket = format!("b{:02x}", index / 256);
        let leaf = format!("e{:04x}", index);
        let bucket = if create {
            archive.create_child(&bucket)
        } else {
            archive.child(&bucket)
        }
        .map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "archive object bucket unavailable",
            )
        })?;
        if create {
            bucket.create_child(&leaf)
        } else {
            bucket.child(&leaf)
        }
        .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "archive object unavailable"))
    }
    fn decrypt_envelope(
        &self,
        id: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<(EncryptedArchiveMetadata, Envelope)> {
        let archive = self.archive(id, ctx)?;
        if archive
            .read("retired.json", 65536)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive retirement state unavailable",
                )
            })?
            .is_some()
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "archive is retired",
            ));
        }
        let bytes = archive
            .read("metadata.json", 65536)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive metadata unavailable",
                )
            })?
            .ok_or_else(|| failure(ctx, ErrorCode::RecoveryRequired, "archive is incomplete"))?;
        let metadata: EncryptedArchiveMetadata = serde_json::from_slice(&bytes)
            .map_err(|_| failure(ctx, ErrorCode::RecoveryRequired, "archive metadata corrupt"))?;
        let cipher = archive
            .read("envelope.age", CIPHER_LIMIT)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive envelope unavailable",
                )
            })?
            .ok_or_else(|| failure(ctx, ErrorCode::RecoveryRequired, "archive envelope missing"))?;
        if metadata.archive_id != *id
            || !metadata.encrypted
            || digest(&cipher) != metadata.ciphertext_digest
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "encrypted archive envelope fingerprint mismatch",
            ));
        }
        let cancel = Cancel {
            control: self.control.as_ref(),
            context: ctx,
        };
        let plain = self
            .tools
            .age_decrypt(&self.identity, &cipher, self.budget(ctx)?, &cancel)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "archive authentication/decryption failed",
                )
            })?;
        if plain.expose().len() > ENVELOPE_LIMIT {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "archive envelope exceeds bound",
            ));
        }
        let envelope: Envelope = serde_json::from_slice(plain.expose()).map_err(|_| {
            failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "authenticated archive envelope invalid",
            )
        })?;
        Self::validate(&envelope, id, ctx)?;
        let manifest = Self::encode(&envelope.manifest, ctx)?;
        if metadata.manifest_digest != digest(&manifest)
            || metadata.created_at != envelope.manifest.created_at
            || metadata.size
                != cipher.len() as u64
                    + envelope
                        .objects
                        .iter()
                        .flat_map(|entry| &entry.chunks)
                        .map(|chunk| chunk.ciphertext_size)
                        .sum::<u64>()
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "archive manifest metadata mismatch",
            ));
        }
        Ok((metadata, envelope))
    }
    fn validate(envelope: &Envelope, id: &ObjectId, ctx: &OperationContext) -> Result<()> {
        if envelope.schema_version != 1
            || envelope.archive_id != *id
            || envelope.manifest.schema_version != FormatVersion(1)
            || envelope.manifest.read_set.coherence != ReadCoherence::Coherent
            || envelope.manifest.entries.is_empty()
            || envelope.manifest.entries.len() > 4096
            || envelope.objects.len() != envelope.manifest.entries.len()
        {
            return Err(failure(
                ctx,
                ErrorCode::ConfigUnsupportedVersion,
                "archive envelope inventory invalid",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut total = 0u64;
        for (entry, encrypted) in envelope.manifest.entries.iter().zip(&envelope.objects) {
            total = total.checked_add(entry.size).ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "archive total size overflow",
                )
            })?;
            if entry.object_id != encrypted.object
                || !ids.insert(&entry.object_id)
                || entry.size > 8 * 1024 * 1024 * 1024
                || total > 32 * 1024 * 1024 * 1024
                || entry.format_version.0 == 0
                || (entry.category != ArchiveCategory::Configuration
                    && entry.protection != ArchiveProtection::Private0600)
                || (envelope.manifest.kind == ArchiveKind::Configuration
                    && !matches!(
                        entry.category,
                        ArchiveCategory::Configuration
                            | ArchiveCategory::SecretVersion
                            | ArchiveCategory::PolicyFence
                    ))
                || encrypted.chunks.len() > 2048
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "archive object scope/size invalid",
                ));
            }
            let mut next = 0u64;
            for chunk in &encrypted.chunks {
                if chunk.offset != next
                    || chunk.size == 0
                    || chunk.size > CHUNK as u64
                    || chunk.ciphertext_size == 0
                    || chunk.ciphertext_size > CIPHER_LIMIT as u64
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "archive chunk ordering/size invalid",
                    ));
                }
                next = next.checked_add(chunk.size).ok_or_else(|| {
                    failure(ctx, ErrorCode::OperationDenied, "archive chunk overflow")
                })?;
            }
            if next != entry.size {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "archive chunk coverage invalid",
                ));
            }
        }
        Ok(())
    }
    pub fn list(&self, ctx: &OperationContext) -> Result<Vec<EncryptedArchiveMetadata>> {
        let archives = match self.root.child("archives") {
            Ok(dir) => dir,
            Err(hton_storage::Error::NotFound) => return Ok(vec![]),
            Err(_) => {
                return Err(failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive directory unavailable",
                ));
            }
        };
        let mut results = Vec::new();
        for name in archives.list(4096).map_err(|_| {
            failure(
                ctx,
                ErrorCode::OperationDenied,
                "archive inventory limit exceeded",
            )
        })? {
            if !name.starts_with("a-") {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "unknown archive directory",
                ));
            }
            let directory = archives
                .child(&name)
                .map_err(|_| failure(ctx, ErrorCode::RecoveryRequired, "archive path unsafe"))?;
            if directory
                .read("retired.json", 65536)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "archive retirement state unavailable",
                    )
                })?
                .is_some()
            {
                continue;
            }
            if let Some(bytes) = directory.read("metadata.json", 65536).map_err(|_| {
                failure(ctx, ErrorCode::IoUnavailable, "archive metadata unreadable")
            })? {
                let metadata: EncryptedArchiveMetadata =
                    serde_json::from_slice(&bytes).map_err(|_| {
                        failure(ctx, ErrorCode::RecoveryRequired, "archive metadata corrupt")
                    })?;
                if Self::archive_name(&metadata.archive_id) != name {
                    return Err(failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "archive metadata identity mismatch",
                    ));
                }
                let (verified, _) = self.decrypt_envelope(&metadata.archive_id, ctx)?;
                results.push(verified);
            }
        }
        results.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| a.archive_id.cmp(&b.archive_id))
        });
        Ok(results)
    }
}
impl ArchiveCrypto for AgeArchiveStore {
    fn encrypt(
        &self,
        request: &BackupRequest,
        manifest: &ArchiveManifest,
        inventory: &dyn BackupInventory,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<EncryptedArchiveMetadata> {
        ensure_mutating(ctx, self.control.as_ref())?;
        let recipient_keys = request
            .recipients
            .iter()
            .map(|id| {
                self.recipients.get(id).cloned().ok_or_else(|| {
                    failure(ctx, ErrorCode::OperationDenied, "unknown archive recipient")
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if recipient_keys.is_empty() || recipient_keys.len() > 32 {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "archive recipient bounds invalid",
            ));
        }
        if self.completed(transaction, ctx)?.is_some() {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "archive transaction already completed",
            ));
        }
        let manifest_bytes = Self::encode(manifest, ctx)?;
        let request_bytes = Self::encode(request, ctx)?;
        let begin = Begin {
            schema_version: 1,
            archive_id: request.archive_id.clone(),
            transaction_id: transaction.clone(),
            request: digest(&request_bytes),
            manifest: digest(&manifest_bytes),
        };
        let archives = self.root.create_child("archives").map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "archive directory creation failed",
            )
        })?;
        let archive = archives
            .create_child(&Self::archive_name(&request.archive_id))
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive staging creation failed",
                )
            })?;
        // A persisted begin always means reconciliation; encryption is not replayed blindly.
        if archive
            .read("begin.json", 65536)
            .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "archive begin unavailable"))?
            .is_some()
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "existing archive requires reconciliation",
            ));
        }
        Self::publish(
            &archive,
            "begin.json",
            &Self::encode(&begin, ctx)?,
            ctx,
            self.control.as_ref(),
        )?;
        let cancel = Cancel {
            control: self.control.as_ref(),
            context: ctx,
        };
        let mut objects = Vec::new();
        let mut ciphertext_size = 0u64;
        for (entry_index, entry) in manifest.entries.iter().enumerate() {
            let directory = Self::entry_directory(&archive, entry_index, true, ctx)?;
            let mut chunks = Vec::new();
            let mut offset = 0u64;
            let mut hasher = Sha256::new();
            while offset < entry.size {
                ensure_mutating(ctx, self.control.as_ref())?;
                let limit = (entry.size - offset).min(CHUNK as u64) as usize;
                let bytes = inventory.read_chunk(manifest, entry, offset, limit, ctx)?;
                if bytes.len() != limit {
                    return Err(failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "backup source chunk changed",
                    ));
                }
                hasher.update(&bytes);
                let encrypted = self
                    .tools
                    .age_encrypt(&recipient_keys, &bytes, self.budget(ctx)?, &cancel)
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "archive encryption interrupted",
                        )
                    })?;
                let name = format!("c{:04x}.age", chunks.len());
                Self::publish(
                    &directory,
                    &name,
                    encrypted.expose(),
                    ctx,
                    self.control.as_ref(),
                )?;
                ciphertext_size = ciphertext_size
                    .checked_add(encrypted.expose().len() as u64)
                    .ok_or_else(|| {
                        failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "archive ciphertext size overflow",
                        )
                    })?;
                chunks.push(Chunk {
                    offset,
                    size: bytes.len() as u64,
                    ciphertext_size: encrypted.expose().len() as u64,
                    plaintext: digest(&bytes),
                    ciphertext: digest(encrypted.expose()),
                });
                offset += bytes.len() as u64;
            }
            let observed =
                Sha256Digest::new(format!("sha256:{:x}", hasher.finalize())).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "archive hash encoding failed",
                    )
                })?;
            if observed != entry.digest {
                return Err(failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "backup source content fingerprint changed",
                ));
            }
            objects.push(EncryptedEntry {
                object: entry.object_id.clone(),
                chunks,
            });
        }
        let envelope = Envelope {
            schema_version: 1,
            archive_id: request.archive_id.clone(),
            transaction_id: transaction.clone(),
            manifest: manifest.clone(),
            objects,
        };
        Self::validate(&envelope, &request.archive_id, ctx)?;
        let bytes = Self::encode(&envelope, ctx)?;
        if bytes.len() > ENVELOPE_LIMIT {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "archive envelope exceeds bound",
            ));
        }
        let encrypted = self
            .tools
            .age_encrypt(&recipient_keys, &bytes, self.budget(ctx)?, &cancel)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "archive envelope encryption interrupted",
                )
            })?;
        Self::publish(
            &archive,
            "envelope.age",
            encrypted.expose(),
            ctx,
            self.control.as_ref(),
        )?;
        let metadata = EncryptedArchiveMetadata {
            archive_id: request.archive_id.clone(),
            manifest_digest: digest(&manifest_bytes),
            ciphertext_digest: digest(encrypted.expose()),
            encrypted: true,
            size: ciphertext_size + encrypted.expose().len() as u64,
            created_at: manifest.created_at,
        };
        Self::publish(
            &archive,
            "metadata.json",
            &Self::encode(&metadata, ctx)?,
            ctx,
            self.control.as_ref(),
        )?;
        let complete = Completion {
            schema_version: 1,
            archive_id: request.archive_id.clone(),
            transaction_id: transaction.clone(),
            metadata: metadata.clone(),
        };
        let txdir = self.tx_bucket(transaction, true, ctx)?.ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "archive transaction directory absent",
            )
        })?;
        Self::publish(
            &txdir,
            &format!("{}.json", Self::tx_name(transaction)),
            &Self::encode(&complete, ctx)?,
            ctx,
            self.control.as_ref(),
        )?;
        Ok(metadata)
    }
    fn inspect(
        &self,
        id: &ObjectId,
        ctx: &OperationContext,
    ) -> Result<(EncryptedArchiveMetadata, ArchiveManifest)> {
        let (metadata, envelope) = self.decrypt_envelope(id, ctx)?;
        // Validate every authenticated object before restore can publish its first byte.
        for entry in &envelope.manifest.entries {
            let mut offset = 0;
            let mut hasher = Sha256::new();
            while offset < entry.size {
                let bytes = self.read_chunk(id, entry, offset, CHUNK, ctx)?;
                if bytes.is_empty() {
                    return Err(failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "archive object truncated",
                    ));
                }
                offset += bytes.len() as u64;
                hasher.update(&bytes);
            }
            if Sha256Digest::new(format!("sha256:{:x}", hasher.finalize())).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "archive hash encoding failed",
                )
            })? != entry.digest
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "authenticated archive object fingerprint differs",
                ));
            }
        }
        Ok((metadata, envelope.manifest))
    }
    fn read_chunk(
        &self,
        id: &ObjectId,
        entry: &ArchiveEntry,
        offset: u64,
        max_bytes: usize,
        ctx: &OperationContext,
    ) -> Result<Vec<u8>> {
        if max_bytes == 0 || max_bytes > CHUNK || offset > entry.size {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "archive range is outside bounds",
            ));
        }
        let (_, envelope) = self.decrypt_envelope(id, ctx)?;
        let (index, actual) = envelope
            .manifest
            .entries
            .iter()
            .enumerate()
            .find(|(_, value)| value.object_id == entry.object_id)
            .ok_or_else(|| failure(ctx, ErrorCode::OperationDenied, "archive object unknown"))?;
        if actual != entry {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "archive entry differs from authenticated manifest",
            ));
        }
        if offset == entry.size {
            return Ok(vec![]);
        }
        let directory = Self::entry_directory(&self.archive(id, ctx)?, index, false, ctx)?;
        let cancel = Cancel {
            control: self.control.as_ref(),
            context: ctx,
        };
        let mut output = Vec::new();
        let target = (entry.size - offset).min(max_bytes as u64) as usize;
        for (chunk_index, chunk) in envelope.objects[index].chunks.iter().enumerate() {
            if chunk.offset + chunk.size <= offset || output.len() == target {
                continue;
            }
            let bytes = directory
                .read(&format!("c{chunk_index:04x}.age"), CIPHER_LIMIT)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "archive ciphertext unavailable",
                    )
                })?
                .ok_or_else(|| {
                    failure(ctx, ErrorCode::RecoveryRequired, "archive chunk missing")
                })?;
            if bytes.len() as u64 != chunk.ciphertext_size || digest(&bytes) != chunk.ciphertext {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "archive ciphertext fingerprint mismatch",
                ));
            }
            let plaintext = self
                .tools
                .age_decrypt(&self.identity, &bytes, self.budget(ctx)?, &cancel)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "archive chunk authentication failed",
                    )
                })?;
            if plaintext.expose().len() as u64 != chunk.size
                || digest(plaintext.expose()) != chunk.plaintext
            {
                return Err(failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "archive plaintext fingerprint mismatch",
                ));
            }
            let start = offset.saturating_sub(chunk.offset) as usize;
            let count = (target - output.len()).min(plaintext.expose().len() - start);
            output.extend_from_slice(&plaintext.expose()[start..start + count]);
        }
        if output.len() != target {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "archive object range incomplete",
            ));
        }
        Ok(output)
    }
    fn completed(
        &self,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<Option<EncryptedArchiveMetadata>> {
        let Some(directory) = self.tx_bucket(transaction, false, ctx)? else {
            return Ok(None);
        };
        let Some(bytes) = directory
            .read(&format!("{}.json", Self::tx_name(transaction)), 65536)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive completion unavailable",
                )
            })?
        else {
            return Ok(None);
        };
        let completion: Completion = serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "archive completion corrupt",
            )
        })?;
        if completion.schema_version != 1
            || completion.transaction_id != *transaction
            || completion.archive_id != completion.metadata.archive_id
        {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "archive completion identity mismatch",
            ));
        }
        let (metadata, envelope) = self.decrypt_envelope(&completion.archive_id, ctx)?;
        if envelope.transaction_id != *transaction || metadata != completion.metadata {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "archive completion authentication mismatch",
            ));
        }
        Ok(Some(metadata))
    }
    fn retain(&self, keep: u16, protected: &ObjectId, ctx: &OperationContext) -> Result<()> {
        ensure_mutating(ctx, self.control.as_ref())?;
        if keep == 0 || keep > 100 {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "archive retention bound invalid",
            ));
        }
        // Pending or unauthenticated directories are never candidates for deletion.
        if self
            .journal
            .unfinished(ctx)?
            .iter()
            .any(|transaction| matches!(transaction.intent, Intent::Restore { .. }))
        {
            return Ok(());
        }
        let archives = self.list(ctx)?;
        let mut retained = usize::from(
            archives
                .iter()
                .any(|metadata| metadata.archive_id == *protected),
        );
        for metadata in archives {
            if metadata.archive_id == *protected {
                continue;
            }
            if retained < (keep as usize) {
                retained += 1;
                continue;
            }
            let (_, envelope) = self.decrypt_envelope(&metadata.archive_id, ctx)?;
            let archive = self.archive(&metadata.archive_id, ctx)?;
            // A durable retirement reservation prevents future readers treating partial cleanup as usable.
            Self::publish(
                &archive,
                "retired.json",
                &Self::encode(&metadata, ctx)?,
                ctx,
                self.control.as_ref(),
            )?;
            for (index, encrypted) in envelope.objects.iter().enumerate() {
                let directory = Self::entry_directory(&archive, index, false, ctx)?;
                let locked = directory
                    .lock(deadline(ctx, self.control.as_ref())?)
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "archive prune lock unavailable",
                        )
                    })?;
                for chunk in 0..encrypted.chunks.len() {
                    let name = format!("c{chunk:04x}.age");
                    if let Some(bytes) = locked.read(&name, CIPHER_LIMIT).map_err(|_| {
                        failure(ctx, ErrorCode::IoUnavailable, "archive prune read failed")
                    })? {
                        locked.remove(&name, &bytes).map_err(|_| {
                            failure(
                                ctx,
                                ErrorCode::RecoveryRequired,
                                "archive prune durability uncertain",
                            )
                        })?;
                    }
                }
                drop(locked);
                let bucket = archive
                    .child(&format!("b{:02x}", index / 256))
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "archive prune bucket unavailable",
                        )
                    })?;
                bucket
                    .lock(deadline(ctx, self.control.as_ref())?)
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "archive prune bucket lock unavailable",
                        )
                    })?
                    .remove_empty_child(&format!("e{index:04x}"))
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "archive prune directory uncertain",
                        )
                    })?;
            }
            for bucket_index in 0..((envelope.objects.len() + 255) / 256) {
                archive
                    .lock(deadline(ctx, self.control.as_ref())?)
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "archive prune lock unavailable",
                        )
                    })?
                    .remove_empty_child(&format!("b{bucket_index:02x}"))
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "archive prune bucket durability uncertain",
                        )
                    })?;
            }
            let locked = archive
                .lock(deadline(ctx, self.control.as_ref())?)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "archive prune terminal lock unavailable",
                    )
                })?;
            for name in [
                "begin.json",
                "envelope.age",
                "metadata.json",
                "retired.json",
            ] {
                if let Some(bytes) = locked.read(name, CIPHER_LIMIT).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "archive prune terminal read failed",
                    )
                })? {
                    locked.remove(name, &bytes).map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "archive prune terminal uncertain",
                        )
                    })?;
                }
            }
            drop(locked);
            let parent = self.root.child("archives").map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive prune parent unavailable",
                )
            })?;
            parent
                .lock(deadline(ctx, self.control.as_ref())?)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "archive prune parent lock unavailable",
                    )
                })?
                .remove_empty_child(&Self::archive_name(&metadata.archive_id))
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "archive prune parent durability uncertain",
                    )
                })?;
        }
        Ok(())
    }
}
