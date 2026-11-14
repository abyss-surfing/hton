//! Linux directory capabilities and durable publication, without domain policy.
#![forbid(unsafe_code)]
#[cfg(not(target_os = "linux"))]
compile_error!("HTON filesystem support currently requires Linux");

use rustix::fs::{self, AtFlags, FileType, FlockOperation, Mode, OFlags, RenameFlags};
use rustix::io::Errno;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_DIRECTORY_ENTRIES: usize = 4096;
const LOCK_NAME: &str = ".hton.lock";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidName,
    UnsafePath,
    NotFound,
    NotRegular,
    TooLarge,
    Conflict,
    Deadline,
    Permission,
    Io,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidName => "invalid storage name",
            Self::UnsafePath => "unsafe storage path",
            Self::NotFound => "storage object not found",
            Self::NotRegular => "unsupported storage object type",
            Self::TooLarge => "storage limit exceeded",
            Self::Conflict => "storage object changed",
            Self::Deadline => "storage lock deadline exceeded",
            Self::Permission => "storage permission policy rejected",
            Self::Io => "storage operation failed; reconcile before retrying",
        })
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

fn error(err: Errno) -> Error {
    match err {
        Errno::NOENT => Error::NotFound,
        Errno::LOOP | Errno::NOTDIR => Error::UnsafePath,
        Errno::ACCESS | Errno::PERM => Error::Permission,
        Errno::EXIST => Error::Conflict,
        _ => Error::Io,
    }
}
fn name(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 255
        || value == "."
        || value == ".."
        || !value
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"._-".contains(&v))
    {
        return Err(Error::InvalidName);
    }
    Ok(())
}
fn data_name(value: &str) -> Result<()> {
    name(value)?;
    if value == LOCK_NAME || value.starts_with(".hton-tmp-") {
        return Err(Error::InvalidName);
    }
    Ok(())
}
fn directory_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

/// An anchored descriptor. Debug deliberately exposes neither paths nor contents.
pub struct SecureDir {
    fd: File,
}
impl std::fmt::Debug for SecureDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecureDir([capability])")
    }
}
impl SecureDir {
    pub fn open(path: &Path) -> Result<Self> {
        Self::walk(path, false)
    }
    pub fn open_private(path: &Path) -> Result<Self> {
        let dir = Self::open(path)?;
        dir.require_private()?;
        Ok(dir)
    }
    /// Explicit initialization. Existing permissions are never silently changed.
    pub fn create(path: &Path) -> Result<Self> {
        let dir = Self::walk(path, true)?;
        dir.require_private()?;
        Ok(dir)
    }
    fn walk(path: &Path, create: bool) -> Result<Self> {
        if path.as_os_str().is_empty() {
            return Err(Error::UnsafePath);
        }
        let start = if path.is_absolute() { "/" } else { "." };
        let mut dir = Self {
            fd: File::from(fs::open(start, directory_flags(), Mode::empty()).map_err(error)?),
        };
        for component in path.components() {
            match component {
                Component::RootDir | Component::CurDir => {}
                Component::Normal(value) => {
                    let value = value.to_str().ok_or(Error::UnsafePath)?;
                    name(value)?;
                    match dir.child(value) {
                        Ok(child) => dir = child,
                        Err(Error::NotFound) if create => {
                            match fs::mkdirat(&dir.fd, value, Mode::RWXU) {
                                Ok(()) => fs::fsync(&dir.fd).map_err(error)?,
                                Err(Errno::EXIST) => {}
                                Err(err) => return Err(error(err)),
                            }
                            dir = dir.child(value)?;
                            dir.require_private()?;
                        }
                        Err(err) => return Err(err),
                    }
                }
                _ => return Err(Error::UnsafePath),
            }
        }
        Ok(dir)
    }
    fn require_private(&self) -> Result<()> {
        let stat = fs::fstat(&self.fd).map_err(error)?;
        if stat.st_uid != rustix::process::geteuid().as_raw() || stat.st_mode & 0o077 != 0 {
            return Err(Error::Permission);
        }
        Ok(())
    }
    pub fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            fd: self.fd.try_clone().map_err(|_| Error::Io)?,
        })
    }
    pub fn child(&self, component: &str) -> Result<Self> {
        name(component)?;
        Ok(Self {
            fd: File::from(
                fs::openat(&self.fd, component, directory_flags(), Mode::empty()).map_err(error)?,
            ),
        })
    }
    pub fn create_child(&self, component: &str) -> Result<Self> {
        data_name(component)?;
        self.require_private()?;
        match fs::mkdirat(&self.fd, component, Mode::RWXU) {
            Ok(()) => fs::fsync(&self.fd).map_err(error)?,
            Err(Errno::EXIST) => {}
            Err(err) => return Err(error(err)),
        }
        let child = self.child(component)?;
        child.require_private()?;
        Ok(child)
    }
    /// Opens no special files, follows no symlinks, and never creates a file.
    /// New immutable large blob only; the caller checks its operation deadline per chunk.
    pub fn begin_stream(&self, target: &str, max_file_bytes: u64) -> Result<StreamWriter> {
        data_name(target)?;
        self.require_private()?;
        if max_file_bytes == 0 || max_file_bytes > 8 * 1024 * 1024 * 1024 {
            return Err(Error::TooLarge);
        }
        let temp_name = format!(
            ".hton-tmp-stream-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let directory = self.try_clone()?;
        let fd = fs::openat(
            &self.fd,
            &temp_name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(error)?;
        Ok(StreamWriter {
            directory,
            file: Some(File::from(fd)),
            temp_name,
            target: target.to_owned(),
            bytes: 0,
            max_file_bytes,
            failed: false,
            published: false,
        })
    }
    /// Fsync a completed external writer's private regular file; no path following.
    pub fn seal_external_file(&self, component: &str, max_file_bytes: u64) -> Result<u64> {
        data_name(component)?;
        self.require_private()?;
        if max_file_bytes > 8 * 1024 * 1024 * 1024 {
            return Err(Error::TooLarge);
        }
        let file = File::from(
            fs::openat(
                &self.fd,
                component,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(error)?,
        );
        let stat = fs::fstat(&file).map_err(error)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
            || stat.st_nlink != 1
            || stat.st_uid != rustix::process::geteuid().as_raw()
            || stat.st_mode & 0o077 != 0
        {
            return Err(Error::NotRegular);
        }
        if stat.st_size < 0 || stat.st_size as u64 > max_file_bytes {
            return Err(Error::TooLarge);
        }
        fs::fsync(&file).map_err(error)?;
        let current = fs::statat(&self.fd, component, AtFlags::SYMLINK_NOFOLLOW).map_err(error)?;
        if current.st_dev != stat.st_dev
            || current.st_ino != stat.st_ino
            || current.st_size != stat.st_size
            || current.st_mtime != stat.st_mtime
            || current.st_mtime_nsec != stat.st_mtime_nsec
            || current.st_ctime != stat.st_ctime
            || current.st_ctime_nsec != stat.st_ctime_nsec
        {
            return Err(Error::Conflict);
        }
        fs::fsync(&self.fd).map_err(error)?;
        Ok(stat.st_size as u64)
    }
    pub fn read(&self, component: &str, limit: usize) -> Result<Option<Vec<u8>>> {
        Ok(self
            .read_segment(component, 0, limit, limit as u64, false, false)?
            .map(|(_, bytes)| bytes))
    }
    /// Input private-file read; no source fsync/chmod or permission repair.
    pub fn read_private(&self, component: &str, limit: usize) -> Result<Option<Vec<u8>>> {
        Ok(self
            .read_segment(component, 0, limit, limit as u64, false, true)?
            .map(|(_, bytes)| bytes))
    }
    pub fn file_size(&self, component: &str, max_file_bytes: u64) -> Result<Option<u64>> {
        Ok(self
            .read_segment(component, 0, 0, max_file_bytes, true, false)?
            .map(|(size, _)| size))
    }
    pub fn file_size_private(&self, component: &str, max_file_bytes: u64) -> Result<Option<u64>> {
        Ok(self
            .read_segment(component, 0, 0, max_file_bytes, true, true)?
            .map(|(size, _)| size))
    }
    pub fn read_private_range(
        &self,
        component: &str,
        offset: u64,
        max_bytes: usize,
        max_file_bytes: u64,
    ) -> Result<Option<Vec<u8>>> {
        Ok(self
            .read_segment(component, offset, max_bytes, max_file_bytes, false, true)?
            .map(|(_, bytes)| bytes))
    }
    pub fn read_range(
        &self,
        component: &str,
        offset: u64,
        max_bytes: usize,
        max_file_bytes: u64,
    ) -> Result<Option<Vec<u8>>> {
        Ok(self
            .read_segment(component, offset, max_bytes, max_file_bytes, false, false)?
            .map(|(_, bytes)| bytes))
    }
    fn read_segment(
        &self,
        component: &str,
        offset: u64,
        limit: usize,
        max_file_bytes: u64,
        metadata_only: bool,
        require_private: bool,
    ) -> Result<Option<(u64, Vec<u8>)>> {
        data_name(component)?;
        if limit > MAX_FILE_BYTES || max_file_bytes > 8 * 1024 * 1024 * 1024 {
            return Err(Error::TooLarge);
        }
        match fs::statat(&self.fd, component, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) if FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile => {}
            Ok(_) => return Err(Error::NotRegular),
            Err(Errno::NOENT) => return Ok(None),
            Err(err) => return Err(error(err)),
        }
        let fd = match fs::openat(
            &self.fd,
            component,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(Errno::NOENT) => return Ok(None),
            Err(err) => return Err(error(err)),
        };
        let mut file = File::from(fd);
        let before = fs::fstat(&file).map_err(error)?;
        if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile || before.st_nlink != 1
        {
            return Err(Error::NotRegular);
        }
        if require_private
            && (before.st_uid != rustix::process::geteuid().as_raw() || before.st_mode & 0o077 != 0)
        {
            return Err(Error::Permission);
        }
        if before.st_size < 0 || before.st_size as u64 > max_file_bytes {
            return Err(Error::TooLarge);
        }
        let size = before.st_size as u64;
        if offset > size {
            return Err(Error::Conflict);
        }
        let count = (size - offset).min(limit as u64) as usize;
        let mut bytes = Vec::with_capacity(count);
        if !metadata_only {
            file.seek(SeekFrom::Start(offset)).map_err(|_| Error::Io)?;
            (&mut file)
                .take(count as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| Error::Io)?;
            if bytes.len() != count {
                return Err(Error::Conflict);
            }
        }
        let after = fs::fstat(&file).map_err(error)?;
        if before.st_size != after.st_size
            || before.st_mtime != after.st_mtime
            || before.st_mtime_nsec != after.st_mtime_nsec
            || before.st_ctime != after.st_ctime
            || before.st_ctime_nsec != after.st_ctime_nsec
            || after.st_nlink != 1
        {
            return Err(Error::Conflict);
        }
        // Detect path replacement during the read, even when the opened inode stayed stable.
        let current = fs::statat(&self.fd, component, AtFlags::SYMLINK_NOFOLLOW).map_err(error)?;
        if current.st_dev != after.st_dev || current.st_ino != after.st_ino {
            return Err(Error::Conflict);
        }
        Ok(Some((size, bytes)))
    }
    pub fn list(&self, limit: usize) -> Result<Vec<String>> {
        if limit > MAX_DIRECTORY_ENTRIES {
            return Err(Error::TooLarge);
        }
        let mut names = Vec::new();
        let mut entries = fs::Dir::read_from(&self.fd).map_err(error)?;
        for (scanned, entry) in (&mut entries).enumerate() {
            if scanned > MAX_DIRECTORY_ENTRIES + 2 {
                return Err(Error::TooLarge);
            }
            let entry = entry.map_err(error)?;
            let value = entry.file_name().to_str().map_err(|_| Error::InvalidName)?;
            if value == "."
                || value == ".."
                || value == LOCK_NAME
                || value.starts_with(".hton-tmp-")
            {
                continue;
            }
            name(value)?;
            if names.len() == limit {
                return Err(Error::TooLarge);
            }
            names.push(value.to_owned());
        }
        names.sort_unstable();
        Ok(names)
    }
    pub fn lock(&self, deadline: Instant) -> Result<LockedDir> {
        self.require_private()?;
        if Instant::now() >= deadline {
            return Err(Error::Deadline);
        }
        let lock = File::from(
            fs::openat(
                &self.fd,
                LOCK_NAME,
                OFlags::RDWR
                    | OFlags::CREATE
                    | OFlags::NOFOLLOW
                    | OFlags::CLOEXEC
                    | OFlags::NONBLOCK,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(error)?,
        );
        let stat = fs::fstat(&lock).map_err(error)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
            || stat.st_nlink != 1
            || stat.st_uid != rustix::process::geteuid().as_raw()
            || stat.st_mode & 0o077 != 0
        {
            return Err(Error::Permission);
        }
        loop {
            if Instant::now() >= deadline {
                return Err(Error::Deadline);
            }
            match fs::flock(&lock, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => {
                    return Ok(LockedDir {
                        dir: self.try_clone()?,
                        lock,
                    });
                }
                Err(Errno::WOULDBLOCK | Errno::INTR) => {
                    std::thread::sleep(
                        Duration::from_millis(5)
                            .min(deadline.saturating_duration_since(Instant::now())),
                    );
                }
                Err(err) => return Err(error(err)),
            }
        }
    }
}

/// Serializes cooperating publishers. Drop closes the advisory lock descriptor.
pub struct LockedDir {
    dir: SecureDir,
    lock: File,
}
impl std::fmt::Debug for LockedDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LockedDir([capability])")
    }
}
impl LockedDir {
    /// Empty private generated directories only; never a recursive deletion API.
    pub fn remove_empty_child(&self, component: &str) -> Result<()> {
        data_name(component)?;
        self.dir.require_private()?;
        let child = self.dir.child(component)?;
        child.require_private()?;
        let guard = child.lock(Instant::now() + Duration::from_secs(1))?;
        let mut entries = fs::Dir::read_from(&child.fd).map_err(error)?;
        for entry in &mut entries {
            let entry = entry.map_err(error)?;
            let name = entry.file_name().to_str().map_err(|_| Error::InvalidName)?;
            if !matches!(name, "." | ".." | LOCK_NAME) {
                return Err(Error::Conflict);
            }
        }
        let lock_open = fs::fstat(&guard.lock).map_err(error)?;
        let lock_current =
            fs::statat(&child.fd, LOCK_NAME, AtFlags::SYMLINK_NOFOLLOW).map_err(error)?;
        if lock_open.st_dev != lock_current.st_dev || lock_open.st_ino != lock_current.st_ino {
            return Err(Error::Conflict);
        }
        fs::unlinkat(&child.fd, LOCK_NAME, AtFlags::empty()).map_err(error)?;
        let opened = fs::fstat(&child.fd).map_err(error)?;
        let current =
            fs::statat(&self.dir.fd, component, AtFlags::SYMLINK_NOFOLLOW).map_err(error)?;
        if opened.st_dev != current.st_dev || opened.st_ino != current.st_ino {
            return Err(Error::Conflict);
        }
        fs::unlinkat(&self.dir.fd, component, AtFlags::REMOVEDIR).map_err(error)?;
        fs::fsync(&self.dir.fd).map_err(error)
    }
    pub fn directory(&self) -> &SecureDir {
        &self.dir
    }
    pub fn read(&self, name: &str, limit: usize) -> Result<Option<Vec<u8>>> {
        self.dir.read(name, limit)
    }
    pub fn list(&self, limit: usize) -> Result<Vec<String>> {
        self.dir.list(limit)
    }
    pub fn compare_exchange(
        &self,
        component: &str,
        expected: Option<&[u8]>,
        replacement: &[u8],
    ) -> Result<()> {
        data_name(component)?;
        self.dir.require_private()?;
        if replacement.len() > MAX_FILE_BYTES || expected.is_some_and(|v| v.len() > MAX_FILE_BYTES)
        {
            return Err(Error::TooLarge);
        }
        if self.dir.read(component, MAX_FILE_BYTES)?.as_deref() != expected {
            return Err(Error::Conflict);
        }
        let mut temp = None;
        for _ in 0..128 {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let temp_name = format!(".hton-tmp-{}-{sequence}", std::process::id());
            match fs::openat(
                &self.dir.fd,
                temp_name.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            ) {
                Ok(fd) => {
                    temp = Some((temp_name, File::from(fd)));
                    break;
                }
                Err(Errno::EXIST) => continue,
                Err(err) => return Err(error(err)),
            }
        }
        let (temp_name, mut file) = temp.ok_or(Error::Conflict)?;
        let result = (|| {
            file.write_all(replacement).map_err(|_| Error::Io)?;
            file.sync_all().map_err(|_| Error::Io)?;
            // Recheck supported editor preconditions immediately before publication.
            if self.dir.read(component, MAX_FILE_BYTES)?.as_deref() != expected {
                return Err(Error::Conflict);
            }
            if expected.is_none() {
                fs::renameat_with(
                    &self.dir.fd,
                    temp_name.as_str(),
                    &self.dir.fd,
                    component,
                    RenameFlags::NOREPLACE,
                )
                .map_err(error)?;
            } else {
                fs::renameat(&self.dir.fd, temp_name.as_str(), &self.dir.fd, component)
                    .map_err(error)?;
            }
            fs::fsync(&self.dir.fd).map_err(error)?;
            Ok(())
        })();
        if result.is_err() {
            // Unpublished temporary bytes are never treated as a durable domain record.
            let _ = fs::unlinkat(&self.dir.fd, temp_name.as_str(), AtFlags::empty());
        }
        result
    }
    pub fn remove(&self, component: &str, expected: &[u8]) -> Result<()> {
        data_name(component)?;
        self.dir.require_private()?;
        if self.dir.read(component, MAX_FILE_BYTES)?.as_deref() != Some(expected) {
            return Err(Error::Conflict);
        }
        fs::unlinkat(&self.dir.fd, component, AtFlags::empty()).map_err(error)?;
        fs::fsync(&self.dir.fd).map_err(error)
    }
}
impl Drop for LockedDir {
    fn drop(&mut self) {
        let _ = fs::flock(&self.lock, FlockOperation::Unlock);
    }
}

/// Private streamed publication. No arbitrary file handle or mutable-overwrite API.
pub struct StreamWriter {
    directory: SecureDir,
    file: Option<File>,
    temp_name: String,
    target: String,
    bytes: u64,
    max_file_bytes: u64,
    failed: bool,
    published: bool,
}
impl StreamWriter {
    pub fn write_chunk(&mut self, bytes: &[u8]) -> Result<()> {
        let Some(next) = self.bytes.checked_add(bytes.len() as u64) else {
            self.failed = true;
            return Err(Error::TooLarge);
        };
        if self.failed || bytes.len() > MAX_FILE_BYTES || next > self.max_file_bytes {
            self.failed = true;
            return Err(Error::TooLarge);
        }
        if self
            .file
            .as_mut()
            .ok_or(Error::Io)?
            .write_all(bytes)
            .is_err()
        {
            self.failed = true;
            return Err(Error::Io);
        }
        self.bytes = next;
        Ok(())
    }
    pub fn finish_new(mut self, target: &str) -> Result<u64> {
        data_name(target)?;
        if self.failed || target != self.target {
            return Err(Error::InvalidName);
        }
        let file = self.file.as_ref().ok_or(Error::Io)?;
        fs::fsync(file).map_err(error)?;
        fs::renameat_with(
            &self.directory.fd,
            &self.temp_name,
            &self.directory.fd,
            target,
            RenameFlags::NOREPLACE,
        )
        .map_err(error)?;
        self.published = true;
        fs::fsync(&self.directory.fd).map_err(error)?;
        Ok(self.bytes)
    }
}
impl Drop for StreamWriter {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::unlinkat(&self.directory.fd, &self.temp_name, AtFlags::empty());
        }
    }
}
