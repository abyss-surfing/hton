//! Coherent logical backup inventory and fenced, non-executable restoration.
use super::archive::AgeArchiveStore;
use crate::{
    backup::*,
    config::{self, ConfigSource, SourceError},
    host::HostApplication,
    journal::{ProtectedOriginal, deadline, ensure_mutating, failure},
};
use hton_contracts::{
    config::{ReadCoherence, ReadDependency, ReadSet},
    operations::*,
};
use hton_host::{
    Budget as NativeBudget, Cancellation,
    crypto::{CryptoTools, PrivateDirectory},
};
use hton_storage::SecureDir;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
const CHUNK: usize = 4 * 1024 * 1024;
struct Cancel<'a> {
    app: &'a HostApplication,
    ctx: &'a OperationContext,
}
impl Cancellation for Cancel<'_> {
    fn cancelled(&self) -> bool {
        self.app.control.is_cancelled(&self.ctx.cancellation)
    }
}
pub struct FsBackupInventory {
    app: HostApplication,
    tools: CryptoTools,
    crypto: Arc<AgeArchiveStore>,
    restore_archive: Option<ObjectId>,
    locations: Mutex<BTreeMap<ObjectId, ArchiveLocation>>,
}
impl FsBackupInventory {
    pub fn new(
        app: &HostApplication,
        tools: CryptoTools,
        crypto: Arc<AgeArchiveStore>,
        restore_archive: Option<ObjectId>,
    ) -> Self {
        Self {
            app: app.clone(),
            tools,
            crypto,
            restore_archive,
            locations: Mutex::new(BTreeMap::new()),
        }
    }
    fn parent(
        &self,
        location: &ArchiveLocation,
        create: bool,
        ctx: &OperationContext,
    ) -> Result<Option<SecureDir>> {
        let mut directory = match location.scope {
            ArchiveScope::Config => self.app.config.try_clone(),
            ArchiveScope::Work => self.app.work.try_clone(),
        }
        .map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "archive inventory capability unavailable",
            )
        })?;
        for component in &location.components[..location.components.len() - 1] {
            directory = match if create {
                directory.create_child(component)
            } else {
                directory.child(component)
            } {
                Ok(directory) => directory,
                Err(hton_storage::Error::NotFound) => return Ok(None),
                Err(_) => {
                    return Err(failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "archive inventory path unsafe/unavailable",
                    ));
                }
            };
        }
        Ok(Some(directory))
    }
    fn size(&self, location: &ArchiveLocation, ctx: &OperationContext) -> Result<Option<u64>> {
        let Some(directory) = self.parent(location, false, ctx)? else {
            return Ok(None);
        };
        directory
            .file_size(
                location.components.last().expect("validated location"),
                8 * 1024 * 1024 * 1024,
            )
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive source is unsafe or too large",
                )
            })
    }
    fn hash(
        &self,
        location: &ArchiveLocation,
        ctx: &OperationContext,
    ) -> Result<Option<(Sha256Digest, u64)>> {
        let Some(directory) = self.parent(location, false, ctx)? else {
            return Ok(None);
        };
        let name = location.components.last().expect("validated location");
        let Some(size) = directory
            .file_size(name, 8 * 1024 * 1024 * 1024)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "archive source size unavailable",
                )
            })?
        else {
            return Ok(None);
        };
        let mut offset = 0;
        let mut hasher = Sha256::new();
        while offset < size {
            self.check(ctx)?;
            let count = (size - offset).min(CHUNK as u64) as usize;
            let bytes = directory
                .read_range(name, offset, count, size)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "archive source changed during hashing",
                    )
                })?
                .ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "archive source disappeared",
                    )
                })?;
            if bytes.len() != count {
                return Err(failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "archive source truncated",
                ));
            }
            hasher.update(&bytes);
            offset += bytes.len() as u64;
        }
        if directory
            .file_size(name, size)
            .map_err(|_| failure(ctx, ErrorCode::ConfigReadConflict, "archive source changed"))?
            != Some(size)
        {
            return Err(failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "archive source size changed",
            ));
        }
        Ok(Some((
            Sha256Digest::new(format!("sha256:{:x}", hasher.finalize())).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "archive hash encoding failed",
                )
            })?,
            size,
        )))
    }
    fn check(&self, ctx: &OperationContext) -> Result<()> {
        if self.app.control.is_cancelled(&ctx.cancellation) {
            return Err(failure(
                ctx,
                ErrorCode::OperationCancelled,
                "archive inventory cancelled",
            ));
        }
        if self.app.control.now_ms() >= ctx.deadline.0 {
            return Err(failure(
                ctx,
                ErrorCode::OperationDeadline,
                "archive inventory deadline reached",
            ));
        }
        Ok(())
    }
    fn walk(
        &self,
        scope: ArchiveScope,
        directory: &SecureDir,
        prefix: Vec<String>,
        category: ArchiveCategory,
        entries: &mut Vec<(ArchiveLocation, ArchiveCategory)>,
        ctx: &OperationContext,
    ) -> Result<()> {
        if prefix.len() > 15 {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "archive directory depth exceeded",
            ));
        }
        for name in directory.list(4096).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "archive inventory listing unavailable",
            )
        })? {
            self.check(ctx)?;
            let mut components = prefix.clone();
            components.push(name.clone());
            let location = ArchiveLocation {
                scope,
                components: components.clone(),
            };
            match directory.file_size(&name, 8 * 1024 * 1024 * 1024) {
                Ok(Some(_)) => {
                    if location.valid_for(category) {
                        entries.push((location, category));
                        if entries.len() > 4096 {
                            return Err(failure(
                                ctx,
                                ErrorCode::OperationDenied,
                                "archive object count exceeded",
                            ));
                        }
                    }
                }
                Ok(None) => {
                    return Err(failure(
                        ctx,
                        ErrorCode::ConfigReadConflict,
                        "archive inventory changed",
                    ));
                }
                Err(hton_storage::Error::NotRegular) => {
                    let child = directory.child(&name).map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "archive symlink/special file refused",
                        )
                    })?;
                    self.walk(scope, &child, components, category, entries, ctx)?;
                }
                Err(_) => {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "archive special/oversized object refused",
                    ));
                }
            }
        }
        Ok(())
    }
    fn discovered(
        &self,
        kind: ArchiveKind,
        ctx: &OperationContext,
    ) -> Result<Vec<(ArchiveLocation, ArchiveCategory)>> {
        let mut entries = Vec::new();
        for name in ["hton.yaml", "backup.yaml", "hton.deprecated.yml"] {
            let location = ArchiveLocation {
                scope: ArchiveScope::Config,
                components: vec![name.into()],
            };
            if self.size(&location, ctx)?.is_some() {
                entries.push((location, ArchiveCategory::Configuration));
            }
        }
        for (name, category) in [
            ("modules", ArchiveCategory::Configuration),
            ("docker", ArchiveCategory::Configuration),
            ("secrets", ArchiveCategory::SecretVersion),
        ] {
            match self.app.config.child(name) {
                Ok(directory) => self.walk(
                    ArchiveScope::Config,
                    &directory,
                    vec![name.into()],
                    category,
                    &mut entries,
                    ctx,
                )?,
                Err(hton_storage::Error::NotFound) => (),
                Err(_) => {
                    return Err(failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "archive configuration directory unsafe",
                    ));
                }
            }
        }
        let location = ArchiveLocation {
            scope: ArchiveScope::Work,
            components: vec!["runtime".into(), "state.json".into()],
        };
        if self.size(&location, ctx)?.is_some() {
            entries.push((location, ArchiveCategory::PolicyFence));
        }
        if kind == ArchiveKind::FullDurable {
            let location = ArchiveLocation {
                scope: ArchiveScope::Work,
                components: vec!["registry.json".into()],
            };
            if self.size(&location, ctx)?.is_some() {
                entries.push((location, ArchiveCategory::Registry));
            }
            let location = ArchiveLocation {
                scope: ArchiveScope::Work,
                components: vec!["identities".into(), "bindings.json".into()],
            };
            if self.size(&location, ctx)?.is_some() {
                entries.push((location, ArchiveCategory::PolicyFence));
            }
            for (name, category) in [
                ("journal", ArchiveCategory::Journal),
                ("bundles", ArchiveCategory::AcceptedBundle),
                ("certificates", ArchiveCategory::Certificate),
                ("acme", ArchiveCategory::AcmeState),
                ("images", ArchiveCategory::RequiredImage),
            ] {
                match self.app.work.child(name) {
                    Ok(directory) => self.walk(
                        ArchiveScope::Work,
                        &directory,
                        vec![name.into()],
                        category,
                        &mut entries,
                        ctx,
                    )?,
                    Err(hton_storage::Error::NotFound) => (),
                    Err(_) => {
                        return Err(failure(
                            ctx,
                            ErrorCode::IoUnavailable,
                            "archive working directory unsafe",
                        ));
                    }
                }
            }
        }
        entries.sort_by(|a, b| a.0.object_id().ok().cmp(&b.0.object_id().ok()));
        Ok(entries)
    }
    fn export_images(&self, ctx: &OperationContext) -> Result<()> {
        ensure_mutating(ctx, self.app.control.as_ref())?;
        let registry = self.app.registry(ctx)?;
        let pins = crate::host::setup::locked_image_references(ctx)?;
        let required: BTreeSet<_> = registry
            .entries
            .values()
            .flat_map(|entry| entry.installed_artifact_versions.values().cloned())
            .collect();
        if required.is_empty() {
            return Ok(());
        }
        let directory = self.app.work.create_child("images").map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "image export inventory unavailable",
            )
        })?;
        let private =
            PrivateDirectory::open(&self.app.paths.work.join("images")).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "image export capability invalid",
                )
            })?;
        let cancel = Cancel {
            app: &self.app,
            ctx,
        };
        for digest in required {
            let pin = pins.get(&digest).ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ModuleUnsupported,
                    "installed image has no trusted pinned export source",
                )
            })?;
            let name = format!(
                "image-{}.tar",
                &crate::config::digest(pin.as_str().as_bytes()).as_str()[7..]
            );
            let metadata = format!(
                "image-{}.json",
                &crate::config::digest(pin.as_str().as_bytes()).as_str()[7..]
            );
            if let Some(bytes) = directory.read(&metadata, 65536).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "image export metadata unavailable",
                )
            })? {
                let (reference, hash, size): (String, Sha256Digest, u64) =
                    serde_json::from_slice(&bytes).map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "image export metadata corrupt",
                        )
                    })?;
                let location = ArchiveLocation {
                    scope: ArchiveScope::Work,
                    components: vec!["images".into(), name],
                };
                if reference != pin.as_str() || self.hash(&location, ctx)? != Some((hash, size)) {
                    return Err(failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "cached image export fingerprint mismatch",
                    ));
                }
                continue;
            }
            let remaining = ctx
                .deadline
                .0
                .saturating_sub(self.app.control.now_ms())
                .min(300_000);
            let budget =
                NativeBudget::crypto(Instant::now() + Duration::from_millis(remaining), 65536)
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::OperationDeadline,
                            "image export budget invalid",
                        )
                    })?;
            self.tools
                .export_image(pin, &private, &name, budget, &cancel)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "pinned image export failed or uncertain",
                    )
                })?;
            let location = ArchiveLocation {
                scope: ArchiveScope::Work,
                components: vec!["images".into(), name],
            };
            let (hash, size) = self
                .hash(&location, ctx)?
                .ok_or_else(|| failure(ctx, ErrorCode::RecoveryRequired, "image export missing"))?;
            let bytes = serde_json::to_vec(&(pin.as_str(), hash, size)).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "image export metadata encoding failed",
                )
            })?;
            let locked = directory
                .lock(deadline(ctx, self.app.control.as_ref())?)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "image export metadata lock unavailable",
                    )
                })?;
            locked
                .compare_exchange(&metadata, None, &bytes)
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "image export metadata durability uncertain",
                    )
                })?;
        }
        Ok(())
    }
    fn bytes(&self, entry: &ArchiveEntry, ctx: &OperationContext) -> Result<Vec<u8>> {
        if entry.size > 4 * 1024 * 1024 {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "structured restore object exceeds bound",
            ));
        }
        let archive = self.restore_archive.as_ref().ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::OperationDenied,
                "restore archive context absent",
            )
        })?;
        self.crypto
            .read_chunk(archive, entry, 0, (entry.size as usize).max(1), ctx)
    }
    fn validate_bytes(
        &self,
        entry: &ArchiveEntry,
        bytes: &[u8],
        ctx: &OperationContext,
    ) -> Result<()> {
        match entry.category {
            ArchiveCategory::Configuration => {
                if config::decode_yaml(bytes, config::LoadLimits::default()).is_err() {
                    return Err(failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "archived configuration syntax invalid",
                    ));
                }
            }
            ArchiveCategory::SecretVersion => crate::secrets::SecretStore::validate_archive_record(
                entry
                    .location
                    .components
                    .last()
                    .expect("validated location"),
                bytes,
                ctx,
            )?,
            ArchiveCategory::Registry => {
                let document: crate::registry::RegistryDocument = serde_json::from_slice(bytes)
                    .map_err(|_| {
                        failure(ctx, ErrorCode::ConfigSyntax, "archived registry invalid")
                    })?;
                if document.schema_version != 1
                    || document.snapshot.instance_id != self.app.installation.instance_id
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "foreign machine registry cannot replace this installation",
                    ));
                }
                let current = self.app.registry(ctx)?;
                for (id, entry) in &document.snapshot.entries {
                    if entry.ownership == hton_contracts::config::Ownership::Owned {
                        let existing = current.entries.get(id).ok_or_else(|| {
                            failure(
                                ctx,
                                ErrorCode::ResourceDrift,
                                "archived owned module has no qualified current native inventory",
                            )
                        })?;
                        if existing.ownership != entry.ownership
                            || existing.resources != entry.resources
                            || existing.installed_artifact_versions
                                != entry.installed_artifact_versions
                        {
                            return Err(failure(
                                ctx,
                                ErrorCode::ResourceDrift,
                                "archived owned native bindings differ; qualify an explicit restoration before writing registry",
                            ));
                        }
                    }
                    if entry.ownership == hton_contracts::config::Ownership::Associated
                        && current.entries.get(id) != Some(entry)
                    {
                        return Err(failure(
                            ctx,
                            ErrorCode::ApprovalRequired,
                            "archive cannot adopt or replay foreign resource association",
                        ));
                    }
                }
            }
            ArchiveCategory::PolicyFence
                if entry.location.components.first().map(String::as_str) == Some("runtime") =>
            {
                let state: wire::RuntimeState = wire::decode(bytes).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "archived runtime state invalid",
                    )
                })?;
                if state.instance_id.as_str() != self.app.installation.instance_id.as_str() {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationDenied,
                        "archived runtime belongs to another installation",
                    ));
                }
            }
            ArchiveCategory::PolicyFence => {
                super::identities::FileIdentityInventory::validate_archive_record(bytes, ctx)?
            }
            ArchiveCategory::Journal => {
                let _: Value = serde_json::from_slice(bytes).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "archived journal record invalid",
                    )
                })?;
            }
            ArchiveCategory::Certificate => {
                let _: Value = serde_json::from_slice(bytes).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::ConfigSyntax,
                        "archived certificate record invalid",
                    )
                })?;
            }
            ArchiveCategory::AcmeState => {
                if entry
                    .location
                    .components
                    .last()
                    .is_some_and(|name| name.ends_with(".conf"))
                {
                    validate_acme_data(bytes, ctx)?;
                }
            }
            _ => (),
        }
        Ok(())
    }
}
use hton_runtime_contract as wire;
impl BackupInventory for FsBackupInventory {
    fn snapshot(&self, kind: ArchiveKind, ctx: &OperationContext) -> Result<ArchiveManifest> {
        if kind == ArchiveKind::FullDurable {
            self.export_images(ctx)?;
        }
        let inventory = self.discovered(kind, ctx)?;
        let mut entries = Vec::new();
        let mut read_set = ReadSet {
            objects: BTreeMap::new(),
            absent_objects: BTreeSet::new(),
            secret_versions: BTreeMap::new(),
            coherence: ReadCoherence::Coherent,
        };
        let mut locations = BTreeMap::new();
        let mut total = 0u64;
        for (location, category) in &inventory {
            let (hash, size) = self.hash(location, ctx)?.ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "archive source disappeared",
                )
            })?;
            total = total
                .checked_add(size)
                .ok_or_else(|| failure(ctx, ErrorCode::OperationDenied, "archive size overflow"))?;
            if total > 32 * 1024 * 1024 * 1024 {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "archive total size exceeds bound",
                ));
            }
            let object_id = location.object_id().map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "archive logical identity invalid",
                )
            })?;
            locations.insert(object_id.clone(), location.clone());
            read_set.objects.insert(
                object_id.clone(),
                ReadDependency {
                    revision: hton_contracts::config::ContentRevision {
                        object_id: object_id.clone(),
                        digest: hash.clone(),
                    },
                    format_version: FormatVersion(1),
                },
            );
            entries.push(ArchiveEntry {
                object_id,
                location: location.clone(),
                category: *category,
                digest: hash,
                size,
                protection: ArchiveProtection::Private0600,
                format_version: FormatVersion(1),
            });
        }
        let second = self.discovered(kind, ctx)?;
        if second.len() != inventory.len() || second.iter().zip(&inventory).any(|(a, b)| a != b) {
            return Err(failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "archive inventory changed during snapshot",
            ));
        }
        for entry in &entries {
            if self.hash(&entry.location, ctx)? != Some((entry.digest.clone(), entry.size)) {
                return Err(failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "archive source changed during snapshot",
                ));
            }
        }
        *self.locations.lock().map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "archive inventory lock poisoned",
            )
        })? = locations;
        Ok(ArchiveManifest {
            schema_version: FormatVersion(1),
            instance_id: self.app.installation.instance_id.clone(),
            kind,
            read_set,
            entries,
            credential_fence: self.app.runtime(ctx)?.fences.credential_revision,
            created_at: UnixMillis(self.app.control.now_ms()),
        })
    }
    fn current(
        &self,
        objects: &[ObjectId],
        ctx: &OperationContext,
    ) -> Result<BTreeMap<ObjectId, Option<Sha256Digest>>> {
        let locations = self
            .locations
            .lock()
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "archive inventory lock poisoned",
                )
            })?
            .clone();
        let mut result = BTreeMap::new();
        for id in objects {
            let location = locations.get(id).ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "unknown archive logical object",
                )
            })?;
            let staged_runtime = location.scope == ArchiveScope::Work
                && location.components == ["runtime".to_owned(), "state.json".to_owned()];
            result.insert(
                id.clone(),
                if staged_runtime {
                    None
                } else {
                    self.hash(location, ctx)?.map(|(digest, _)| digest)
                },
            );
        }
        Ok(result)
    }
    fn read_chunk(
        &self,
        manifest: &ArchiveManifest,
        entry: &ArchiveEntry,
        offset: u64,
        max_bytes: usize,
        ctx: &OperationContext,
    ) -> Result<Vec<u8>> {
        self.check(ctx)?;
        if !manifest.entries.contains(entry) || max_bytes > CHUNK || offset > entry.size {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "backup source range invalid",
            ));
        }
        let directory = self.parent(&entry.location, false, ctx)?.ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "backup source directory disappeared",
            )
        })?;
        directory
            .read_range(
                entry
                    .location
                    .components
                    .last()
                    .expect("validated location"),
                offset,
                max_bytes,
                entry.size,
            )
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "backup source range changed",
                )
            })?
            .ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::ConfigReadConflict,
                    "backup source disappeared",
                )
            })
    }
    fn validate_restore(&self, manifest: &ArchiveManifest, ctx: &OperationContext) -> Result<()> {
        let mut locations = BTreeMap::new();
        for entry in &manifest.entries {
            if !entry.location.valid_for(entry.category)
                || entry.location.object_id().ok().as_ref() != Some(&entry.object_id)
                || entry.format_version != FormatVersion(1)
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "archive restore location/format invalid",
                ));
            }
            if locations
                .insert(entry.object_id.clone(), entry.location.clone())
                .is_some()
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "duplicate archive logical location",
                ));
            }
            if entry.category != ArchiveCategory::RequiredImage {
                let bytes = self.bytes(entry, ctx)?;
                self.validate_bytes(entry, &bytes, ctx)?;
            }
        }
        let mut overlay = BTreeMap::new();
        for entry in &manifest.entries {
            if entry.location.scope == ArchiveScope::Config
                && entry.category == ArchiveCategory::Configuration
                && entry
                    .location
                    .components
                    .last()
                    .is_some_and(|name| name.ends_with(".yaml"))
            {
                overlay.insert(entry.location.components.clone(), self.bytes(entry, ctx)?);
            }
        }
        let source = Overlay {
            base: config::FsConfigSource::open(&self.app.paths.config).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "restore base configuration unavailable",
                )
            })?,
            files: overlay,
        };
        let loaded = config::load(&source, &BTreeSet::from([FormatVersion(1)]));
        if !loaded.errors.is_empty() {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "archive config overlay fails schema/coherence checks",
            ));
        }
        let snapshot = loaded.snapshot.ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::ConfigSyntax,
                "archive configuration snapshot unavailable",
            )
        })?;
        let catalogue = crate::catalog::ModuleCatalogue::builtin().map_err(|_| {
            failure(
                ctx,
                ErrorCode::ModuleUnsupported,
                "trusted catalogue unavailable",
            )
        })?;
        if config::normalize(&snapshot, &catalogue).blocking {
            return Err(failure(
                ctx,
                ErrorCode::ConfigSemanticConflict,
                "archive configuration overlay incompatible with installed schemas",
            ));
        }
        *self.locations.lock().map_err(|_| {
            failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "archive inventory lock poisoned",
            )
        })? = locations;
        Ok(())
    }
    fn originals(
        &self,
        plan: &RestorePlan,
        ctx: &OperationContext,
    ) -> Result<Vec<ProtectedOriginal>> {
        let mut result = Vec::new();
        let mut total = 0usize;
        for entry in &plan.manifest.entries {
            if matches!(
                entry.category,
                ArchiveCategory::RequiredImage
                    | ArchiveCategory::AcceptedBundle
                    | ArchiveCategory::Journal
                    | ArchiveCategory::PolicyFence
            ) {
                continue;
            }
            if let Some(directory) = self.parent(&entry.location, false, ctx)? {
                if let Some(bytes) = directory
                    .read(
                        entry
                            .location
                            .components
                            .last()
                            .expect("validated location"),
                        1024 * 1024,
                    )
                    .map_err(|_| {
                        failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "restore original exceeds journal safety bound",
                        )
                    })?
                {
                    total += bytes.len();
                    if total > 1024 * 1024 {
                        return Err(failure(
                            ctx,
                            ErrorCode::OperationDenied,
                            "restore originals exceed bounded single transaction",
                        ));
                    }
                    result.push(ProtectedOriginal {
                        artifact_id: ArtifactId::new(format!(
                            "original.{}",
                            &config::digest(entry.object_id.as_str().as_bytes()).as_str()[7..]
                        ))
                        .expect("digest ID"),
                        digest: config::digest(&bytes),
                        was_present: true,
                        bytes,
                    });
                }
            }
        }
        Ok(result)
    }
    fn restore_object(
        &self,
        plan: &RestorePlan,
        entry: &ArchiveEntry,
        crypto: &dyn ArchiveCrypto,
        guard: &RestoreFence,
        transaction: &TransactionId,
        ctx: &OperationContext,
    ) -> Result<Sha256Digest> {
        ensure_mutating(ctx, self.app.control.as_ref())?;
        super::restore::verify_guard(&self.app, guard, ctx)?;
        if entry.category == ArchiveCategory::RequiredImage {
            let parent = self.parent(&entry.location, true, ctx)?.ok_or_else(|| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "image restore directory absent",
                )
            })?;
            let target = entry
                .location
                .components
                .last()
                .expect("validated location");
            if let Some((hash, size)) = self.hash(&entry.location, ctx)? {
                if hash != entry.digest || size != entry.size {
                    return Err(failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "existing immutable image archive differs",
                    ));
                }
            } else {
                let mut stream = parent.begin_stream(target, entry.size).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "image restore stream unavailable",
                    )
                })?;
                let mut offset = 0;
                let mut hasher = Sha256::new();
                while offset < entry.size {
                    super::restore::verify_guard(&self.app, guard, ctx)?;
                    let bytes = crypto.read_chunk(&plan.archive_id, entry, offset, CHUNK, ctx)?;
                    if bytes.is_empty() {
                        return Err(failure(
                            ctx,
                            ErrorCode::RecoveryRequired,
                            "image archive truncated",
                        ));
                    }
                    stream.write_chunk(&bytes).map_err(|_| {
                        failure(ctx, ErrorCode::IoUnavailable, "image restore stream failed")
                    })?;
                    hasher.update(&bytes);
                    offset += bytes.len() as u64;
                }
                if Sha256Digest::new(format!("sha256:{:x}", hasher.finalize())).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::OperationConflict,
                        "image restore hash encoding failed",
                    )
                })? != entry.digest
                {
                    return Err(failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "image restore fingerprint mismatch",
                    ));
                }
                stream.finish_new(target).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "image restore publication uncertain",
                    )
                })?;
            }
            let pins = crate::host::setup::locked_image_references(ctx)?;
            let pin = pins
                .values()
                .find(|pin| {
                    format!(
                        "image-{}.tar",
                        &config::digest(pin.as_str().as_bytes()).as_str()[7..]
                    ) == *target
                })
                .ok_or_else(|| {
                    failure(
                        ctx,
                        ErrorCode::ModuleUnsupported,
                        "archived image lacks trusted pinned source",
                    )
                })?;
            let private =
                PrivateDirectory::open(&self.app.paths.work.join("images")).map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::IoUnavailable,
                        "image restore capability invalid",
                    )
                })?;
            let blob = private.blob(target, 8 * 1024 * 1024 * 1024).map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::IoUnavailable,
                    "restored image blob unavailable",
                )
            })?;
            let native_budget = NativeBudget::crypto(
                Instant::now()
                    + Duration::from_millis(
                        ctx.deadline
                            .0
                            .saturating_sub(self.app.control.now_ms())
                            .min(300000),
                    ),
                65536,
            )
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationDeadline,
                    "image import budget invalid",
                )
            })?;
            self.tools
                .import_image(
                    pin,
                    &blob,
                    native_budget,
                    &Cancel {
                        app: &self.app,
                        ctx,
                    },
                )
                .map_err(|_| {
                    failure(
                        ctx,
                        ErrorCode::RecoveryRequired,
                        "restored pinned image cache verification failed",
                    )
                })?;
            super::restore::verify_guard(&self.app, guard, ctx)?;
            return Ok(entry.digest.clone());
        }
        let bytes = self.bytes(entry, ctx)?;
        if config::digest(&bytes) != entry.digest {
            return Err(failure(
                ctx,
                ErrorCode::RecoveryRequired,
                "restore plaintext fingerprint mismatch",
            ));
        }
        self.validate_bytes(entry, &bytes, ctx)?;
        let location = if entry.category == ArchiveCategory::Journal {
            ArchiveLocation {
                scope: ArchiveScope::Work,
                components: vec![
                    "journal".into(),
                    format!(
                        "archive-{}-{}.json",
                        &config::digest(transaction.as_str().as_bytes()).as_str()[7..23],
                        &config::digest(entry.object_id.as_str().as_bytes()).as_str()[7..]
                    ),
                ],
            }
        } else if entry.category == ArchiveCategory::PolicyFence
            && entry.location.components.first().map(String::as_str) == Some("runtime")
        {
            ArchiveLocation {
                scope: ArchiveScope::Work,
                components: vec![
                    "restore".into(),
                    transaction.as_str().into(),
                    "runtime-state.json".into(),
                ],
            }
        } else {
            entry.location.clone()
        };
        let parent = self.parent(&location, true, ctx)?.ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "restore destination unavailable",
            )
        })?;
        let name = location.components.last().expect("validated location");
        let locked = parent
            .lock(deadline(ctx, self.app.control.as_ref())?)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::OperationConflict,
                    "restore destination lock unavailable",
                )
            })?;
        let previous = locked.read(name, 4 * 1024 * 1024).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "restore destination baseline unavailable",
            )
        })?;
        if entry.category == ArchiveCategory::AcceptedBundle
            && previous.as_deref().is_some_and(|old| old != bytes)
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationConflict,
                "existing immutable accepted artifact differs",
            ));
        }
        if location == entry.location
            && plan.expected_objects.get(&entry.object_id)
                != Some(&previous.as_ref().map(|bytes| config::digest(bytes)))
        {
            return Err(failure(
                ctx,
                ErrorCode::PlanStale,
                "restore destination changed",
            ));
        }
        super::restore::verify_guard(&self.app, guard, ctx)?;
        locked
            .compare_exchange(name, previous.as_deref(), &bytes)
            .map_err(|_| {
                failure(
                    ctx,
                    ErrorCode::RecoveryRequired,
                    "restore object publication uncertain",
                )
            })?;
        Ok(entry.digest.clone())
    }
}
struct Overlay {
    base: config::FsConfigSource,
    files: BTreeMap<Vec<String>, Vec<u8>>,
}
impl ConfigSource for Overlay {
    fn list_modules(&self, limit: usize) -> std::result::Result<Vec<String>, SourceError> {
        self.list("modules", limit, self.base.list_modules(limit)?)
    }
    fn list_docker(&self, limit: usize) -> std::result::Result<Vec<String>, SourceError> {
        self.list("docker", limit, self.base.list_docker(limit)?)
    }
    fn read(
        &self,
        components: &[&str],
        limit: usize,
    ) -> std::result::Result<Option<Vec<u8>>, SourceError> {
        let key = components
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>();
        if let Some(bytes) = self.files.get(&key) {
            if bytes.len() > limit {
                return Err(SourceError::TooLarge);
            }
            Ok(Some(bytes.clone()))
        } else {
            self.base.read(components, limit)
        }
    }
}
impl Overlay {
    fn list(
        &self,
        kind: &str,
        limit: usize,
        base: Vec<String>,
    ) -> std::result::Result<Vec<String>, SourceError> {
        let mut names: BTreeSet<_> = base.into_iter().collect();
        for path in self.files.keys() {
            if path.len() == 2 && path[0] == kind {
                names.insert(path[1].clone());
            }
        }
        if names.len() > limit {
            return Err(SourceError::TooLarge);
        }
        Ok(names.into_iter().collect())
    }
}
fn validate_acme_data(bytes: &[u8], ctx: &OperationContext) -> Result<()> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| failure(ctx, ErrorCode::ConfigSyntax, "ACME state encoding invalid"))?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| {
            failure(
                ctx,
                ErrorCode::OperationDenied,
                "ACME state is not literal native data",
            )
        })?;
        if key.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            || !value.starts_with('\'')
            || !value.ends_with('\'')
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "ACME state contains executable syntax",
            ));
        }
        let inner = &value[1..value.len() - 1];
        if inner.contains('\'') || inner.contains('\n') || inner.contains('\r') {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "ACME state has unsupported quoting",
            ));
        }
        if (key.to_ascii_lowercase().contains("hook") || key == "Le_ReloadCmd") && !inner.is_empty()
        {
            return Err(failure(
                ctx,
                ErrorCode::OperationDenied,
                "ACME executable hooks are not restorable",
            ));
        }
    }
    Ok(())
}
