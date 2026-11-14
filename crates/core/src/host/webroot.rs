//! Bounded owner of the public host webroot used by the ordinary Caddy site.
//!
//! This module never replaces site content. It creates the fixed default index
//! only when `/var/www/index.html` is absent, then reads exact bounded snapshots
//! through no-follow descriptors for install-time served-site verification.
use crate::{config, journal};
use hton_contracts::operations::{
    Error, ErrorCode, ExecutionMode, OperationContext, Result, Sha256Digest,
};
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags, RenameFlags};
use rustix::io::Errno;
use std::{
    fs::File,
    io::{Read, Write},
};

const MAX_INDEX_BYTES: usize = 1024 * 1024;
const DEFAULT_INDEX: &[u8] = include_bytes!("../../../../platform/web/mask/index.html");

/// Path and content identity for one read of the served index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SiteIndexIdentity {
    directory_device: u64,
    directory_inode: u64,
    file_device: u64,
    file_inode: u64,
    size: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

/// Exact non-secret site bytes. Debug intentionally omits website content.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct SiteIndex {
    bytes: Vec<u8>,
    digest: Sha256Digest,
    identity: SiteIndexIdentity,
}

impl std::fmt::Debug for SiteIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SiteIndex")
            .field("bytes", &"[site content]")
            .field("digest", &self.digest)
            .field("identity", &self.identity)
            .finish()
    }
}

impl SiteIndex {
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn content_digest(&self) -> &Sha256Digest {
        &self.digest
    }

    pub(crate) fn identity_digest(&self) -> Sha256Digest {
        let identity = &self.identity;
        config::digest(
            &serde_json::to_vec(&(
                identity.directory_device,
                identity.directory_inode,
                identity.file_device,
                identity.file_inode,
                identity.size,
                identity.modified_seconds,
                identity.modified_nanoseconds,
                identity.changed_seconds,
                identity.changed_nanoseconds,
            ))
            .expect("site identity tuple is serializable"),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SiteIndexObservation {
    Absent { identity_digest: Sha256Digest },
    Present(SiteIndex),
}

impl SiteIndexObservation {
    pub(crate) fn identity_digest(&self) -> Sha256Digest {
        match self {
            Self::Absent { identity_digest } => identity_digest.clone(),
            Self::Present(index) => index.identity_digest(),
        }
    }
}

fn failure(ctx: &OperationContext, code: ErrorCode, message: &'static str) -> Error {
    journal::failure(ctx, code, message)
}

fn io_failure(ctx: &OperationContext, error: Errno, message: &'static str) -> Error {
    let code = if matches!(error, Errno::LOOP | Errno::NOTDIR | Errno::ISDIR) {
        ErrorCode::ConfigSemanticConflict
    } else if error == Errno::EXIST {
        ErrorCode::ConfigReadConflict
    } else {
        ErrorCode::IoUnavailable
    };
    failure(ctx, code, message)
}

fn directory_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

fn open_var(ctx: &OperationContext) -> Result<File> {
    let root = File::from(
        fs::open("/", directory_flags(), Mode::empty())
            .map_err(|error| io_failure(ctx, error, "host webroot parent is unavailable"))?,
    );
    Ok(File::from(
        fs::openat(&root, "var", directory_flags(), Mode::empty())
            .map_err(|error| io_failure(ctx, error, "host webroot parent is unavailable"))?,
    ))
}

/// Open `/var/www` without following symlinks. On mutating install only, create
/// the directory itself if absent using the conventional public webroot mode.
fn open_webroot(create: bool, ctx: &OperationContext) -> Result<Option<(File, File)>> {
    let var = open_var(ctx)?;
    let www = match fs::openat(&var, "www", directory_flags(), Mode::empty()) {
        Ok(fd) => File::from(fd),
        Err(Errno::NOENT) if !create => return Ok(None),
        Err(Errno::NOENT) if create => {
            if ctx.execution_mode != ExecutionMode::Mutating
                || rustix::process::geteuid().as_raw() != 0
            {
                return Err(failure(
                    ctx,
                    ErrorCode::OperationDenied,
                    "creating the host webroot requires local root install authority",
                ));
            }
            let mode = Mode::RWXU | Mode::RGRP | Mode::XGRP | Mode::ROTH | Mode::XOTH;
            match fs::mkdirat(&var, "www", mode) {
                Ok(()) | Err(Errno::EXIST) => fs::fsync(&var)
                    .map_err(|error| io_failure(ctx, error, "host webroot creation failed"))?,
                Err(error) => {
                    return Err(io_failure(ctx, error, "host webroot creation failed"));
                }
            }
            File::from(
                fs::openat(&var, "www", directory_flags(), Mode::empty()).map_err(|error| {
                    io_failure(ctx, error, "host webroot is not a safe directory")
                })?,
            )
        }
        Err(error) => {
            return Err(io_failure(
                ctx,
                error,
                "host webroot is not a safe directory",
            ));
        }
    };
    let stat =
        fs::fstat(&www).map_err(|error| io_failure(ctx, error, "host webroot is unavailable"))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "host webroot is not a directory",
        ));
    }
    Ok(Some((var, www)))
}

fn create_default_index(directory: &File, ctx: &OperationContext) -> Result<()> {
    let mut random = [0_u8; 12];
    getrandom::fill(&mut random).map_err(|_| {
        failure(
            ctx,
            ErrorCode::IoUnavailable,
            "site placeholder entropy unavailable",
        )
    })?;
    let suffix: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
    let temporary = format!(".hton-index-{suffix}.tmp");
    let mode = Mode::RUSR | Mode::WUSR | Mode::RGRP | Mode::ROTH;
    let fd = fs::openat(
        directory,
        &temporary,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        mode,
    )
    .map_err(|error| io_failure(ctx, error, "site placeholder staging failed"))?;
    let mut file = File::from(fd);
    let result = (|| {
        file.write_all(DEFAULT_INDEX).map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "site placeholder write failed",
            )
        })?;
        file.sync_all().map_err(|_| {
            failure(
                ctx,
                ErrorCode::IoUnavailable,
                "site placeholder sync failed",
            )
        })?;
        match fs::renameat_with(
            directory,
            &temporary,
            directory,
            "index.html",
            RenameFlags::NOREPLACE,
        ) {
            Ok(()) => fs::fsync(directory)
                .map_err(|error| io_failure(ctx, error, "site placeholder publication failed")),
            Err(Errno::EXIST) => Ok(()),
            Err(error) => Err(io_failure(
                ctx,
                error,
                "site placeholder publication failed",
            )),
        }
    })();
    if result.is_err() || fs::statat(directory, &temporary, AtFlags::SYMLINK_NOFOLLOW).is_ok() {
        let _ = fs::unlinkat(directory, &temporary, AtFlags::empty());
    }
    result
}

/// Read one existing `/var/www/index.html`. Missing content returns `None`;
/// symlinks, non-regular files, hardlinks, empty files and oversized files fail.
pub(crate) fn inspect_index(ctx: &OperationContext) -> Result<SiteIndexObservation> {
    let var = open_var(ctx)?;
    let directory = match fs::openat(&var, "www", directory_flags(), Mode::empty()) {
        Ok(fd) => File::from(fd),
        Err(Errno::NOENT) => {
            return Ok(SiteIndexObservation::Absent {
                identity_digest: absent_identity_digest(&var, None, ctx)?,
            });
        }
        Err(error) => {
            return Err(io_failure(
                ctx,
                error,
                "host webroot is not a safe directory",
            ));
        }
    };
    let stat = fs::statat(&directory, "index.html", AtFlags::SYMLINK_NOFOLLOW);
    match stat {
        Err(Errno::NOENT) => Ok(SiteIndexObservation::Absent {
            identity_digest: absent_identity_digest(&var, Some(&directory), ctx)?,
        }),
        Ok(stat) if FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile => {
            read_index_at(&var, &directory, ctx).map(SiteIndexObservation::Present)
        }
        Ok(_) => Err(failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "site index must be a regular file",
        )),
        Err(error) => Err(io_failure(
            ctx,
            error,
            "site index is unavailable or unsafe",
        )),
    }
}

fn absent_identity_digest(
    var: &File,
    www: Option<&File>,
    ctx: &OperationContext,
) -> Result<Sha256Digest> {
    let var_stat = fs::fstat(var)
        .map_err(|error| io_failure(ctx, error, "host webroot parent cannot be inspected"))?;
    let www_identity = www
        .map(|directory| {
            fs::fstat(directory)
                .map(|stat| (stat.st_dev, stat.st_ino))
                .map_err(|error| io_failure(ctx, error, "host webroot cannot be inspected"))
        })
        .transpose()?;
    let bytes = serde_json::to_vec(&(
        "site-index-absent-v1",
        var_stat.st_dev,
        var_stat.st_ino,
        www_identity,
        "index.html",
    ))
    .map_err(|_| {
        failure(
            ctx,
            ErrorCode::IoUnavailable,
            "site index basis cannot be encoded",
        )
    })?;
    Ok(config::digest(&bytes))
}

/// Ensure `/var/www/index.html` exists, seeding the shipped placeholder only
/// when the file is absent. Existing content, including an empty index, is never
/// overwritten; an empty index remains a readiness blocker until the owner edits it.
pub(crate) fn ensure_index(
    ctx: &OperationContext,
    expected: &SiteIndexObservation,
) -> Result<SiteIndex> {
    if ctx.execution_mode != ExecutionMode::Mutating || rustix::process::geteuid().as_raw() != 0 {
        return Err(failure(
            ctx,
            ErrorCode::OperationDenied,
            "preparing the host webroot requires local root install authority",
        ));
    }
    let current = inspect_index(ctx)?;
    if !same_observation(&current, expected) {
        return Err(failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "host webroot changed after install preflight",
        ));
    }
    if let SiteIndexObservation::Present(index) = current {
        return Ok(index);
    }
    let (var, directory) = open_webroot(true, ctx)?.expect("create mode opens or creates webroot");
    match fs::statat(&directory, "index.html", AtFlags::SYMLINK_NOFOLLOW) {
        Err(Errno::NOENT) => create_default_index(&directory, ctx)?,
        Ok(_) => {
            return Err(failure(
                ctx,
                ErrorCode::ConfigReadConflict,
                "site index appeared after install preflight; original retained",
            ));
        }
        Err(error) => {
            return Err(io_failure(
                ctx,
                error,
                "site index is unavailable or unsafe",
            ));
        }
    }
    match inspect_index(ctx)? {
        SiteIndexObservation::Present(index) => Ok(index),
        SiteIndexObservation::Absent { .. } => Err(failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "site placeholder was not published",
        )),
    }
}

fn same_observation(left: &SiteIndexObservation, right: &SiteIndexObservation) -> bool {
    match (left, right) {
        (
            SiteIndexObservation::Absent {
                identity_digest: left,
            },
            SiteIndexObservation::Absent {
                identity_digest: right,
            },
        ) => left == right,
        (SiteIndexObservation::Present(left), SiteIndexObservation::Present(right)) => {
            left.digest == right.digest && left.identity_digest() == right.identity_digest()
        }
        _ => false,
    }
}

fn read_index_at(var: &File, directory: &File, ctx: &OperationContext) -> Result<SiteIndex> {
    let fd = match fs::openat(
        directory,
        "index.html",
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(error) => {
            return Err(io_failure(
                ctx,
                error,
                "site index is unavailable or unsafe",
            ));
        }
    };
    let mut file = File::from(fd);
    let before = fs::fstat(&file)
        .map_err(|error| io_failure(ctx, error, "site index cannot be inspected"))?;
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile || before.st_nlink != 1 {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "site index must be a regular unlinked file",
        ));
    }
    if before.st_size <= 0 {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "site index is empty and was preserved",
        ));
    }
    if before.st_size as u64 > MAX_INDEX_BYTES as u64 {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "site index exceeds the bounded served-site size",
        ));
    }
    let mut bytes = Vec::with_capacity(before.st_size as usize);
    (&mut file)
        .take(MAX_INDEX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| failure(ctx, ErrorCode::IoUnavailable, "site index read failed"))?;
    if bytes.len() != before.st_size as usize {
        return Err(failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "site index changed during read",
        ));
    }
    let after = fs::fstat(&file)
        .map_err(|error| io_failure(ctx, error, "site index cannot be rechecked"))?;
    let current = fs::statat(directory, "index.html", AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|error| io_failure(ctx, error, "site index path changed during read"))?;
    if before.st_dev != after.st_dev
        || before.st_ino != after.st_ino
        || before.st_size != after.st_size
        || before.st_mtime != after.st_mtime
        || before.st_mtime_nsec != after.st_mtime_nsec
        || before.st_ctime != after.st_ctime
        || before.st_ctime_nsec != after.st_ctime_nsec
        || after.st_nlink != 1
        || FileType::from_raw_mode(current.st_mode) != FileType::RegularFile
        || current.st_nlink != 1
        || current.st_dev != after.st_dev
        || current.st_ino != after.st_ino
    {
        return Err(failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "site index changed during read",
        ));
    }
    let dir_stat = fs::fstat(directory)
        .map_err(|error| io_failure(ctx, error, "host webroot cannot be rechecked"))?;
    let current_directory = File::from(
        fs::openat(var, "www", directory_flags(), Mode::empty())
            .map_err(|error| io_failure(ctx, error, "host webroot path changed during read"))?,
    );
    let current_dir_stat = fs::fstat(&current_directory)
        .map_err(|error| io_failure(ctx, error, "host webroot path cannot be rechecked"))?;
    if dir_stat.st_dev != current_dir_stat.st_dev || dir_stat.st_ino != current_dir_stat.st_ino {
        return Err(failure(
            ctx,
            ErrorCode::ConfigReadConflict,
            "host webroot path changed during read",
        ));
    }
    let identity = SiteIndexIdentity {
        directory_device: dir_stat.st_dev,
        directory_inode: dir_stat.st_ino,
        file_device: after.st_dev,
        file_inode: after.st_ino,
        size: after.st_size as u64,
        modified_seconds: after.st_mtime,
        modified_nanoseconds: after.st_mtime_nsec,
        changed_seconds: after.st_ctime,
        changed_nanoseconds: after.st_ctime_nsec,
    };
    if std::str::from_utf8(&bytes).is_err() {
        return Err(failure(
            ctx,
            ErrorCode::ConfigSemanticConflict,
            "site index must contain valid UTF-8",
        ));
    }
    Ok(SiteIndex {
        digest: config::digest(&bytes),
        bytes,
        identity,
    })
}
