"""Protected, descriptor-relative minimum-profile state and settings."""
from __future__ import annotations

import contextlib
import fcntl
import grp
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import secrets
import stat
import time

GROUP_NAME = "hton"
LEGACY_DIR_MODE = 0o700
LEGACY_FILE_MODE = 0o600


class InstallError(Exception):
    """A bounded product error; native stdout and credentials never enter it."""


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def group_id():
    """The operator group that reads /opt/hton, or None when it is absent."""
    try:
        return grp.getgrnam(GROUP_NAME).gr_gid
    except KeyError:
        return None


def modes():
    """Protected directory/file modes; group-readable when the operator group exists."""
    if group_id() is None:
        return LEGACY_DIR_MODE, LEGACY_FILE_MODE
    return 0o750, 0o640


def safe_file(info, owner: int) -> bool:
    """A regular, singly linked, owner-held file with a protected mode."""
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_uid != owner:
        return False
    return stat.S_IMODE(info.st_mode) in (modes()[1], LEGACY_FILE_MODE)


def adopt_tree(root) -> None:
    """Make a runtime tree readable by the operator group.

    Directories become 0750 and regular files 0640, owned by the process and
    grouped to the operator group. Symlinks are left untouched and never
    followed. Nothing happens when the group does not exist.
    """
    root = Path(root)
    if not root.exists():
        return
    if root.is_symlink() or not root.is_dir():
        raise InstallError("Runtime permission root is unsafe")
    gid = group_id()
    if gid is None:
        return
    dir_mode, file_mode = modes()
    os.chown(root, -1, gid)
    os.chmod(root, dir_mode)
    for current, dirs, files in os.walk(root, followlinks=False):
        base = Path(current)
        for name in dirs:
            path = base / name
            if path.is_symlink():
                continue
            os.chown(path, -1, gid, follow_symlinks=False)
            os.chmod(path, dir_mode, follow_symlinks=False)
        for name in files:
            path = base / name
            if path.is_symlink():
                continue
            os.chown(path, -1, gid, follow_symlinks=False)
            os.chmod(path, file_mode, follow_symlinks=False)


def write_share_file(directory_fd, name, data, uid, gid):
    """Replace one private share artifact without modifying the old inode."""
    if PurePosixPath(name).name != name or name in ("", ".", ".."):
        raise InstallError("Invalid private share filename")
    temporary = ".new-" + secrets.token_hex(12)
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW
        | os.O_CLOEXEC, 0o600, dir_fd=directory_fd)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fchown(stream.fileno(), uid, gid)
            os.fchmod(stream.fileno(), 0o640)
            os.fsync(stream.fileno())
        os.replace(temporary, name, src_dir_fd=directory_fd, dst_dir_fd=directory_fd)
        os.fsync(directory_fd)
    finally:
        try:
            os.unlink(temporary, dir_fd=directory_fd)
        except FileNotFoundError:
            pass


class Store:
    def __init__(self, root: Path, *, create: bool = False):
        self.root = root
        self.owner = os.geteuid()
        self.group = group_id()
        self.dir_mode, self.file_mode = modes()
        if create:
            root.mkdir(mode=LEGACY_DIR_MODE, parents=False, exist_ok=True)
            self._adopt(root, directory=True)
        if root.resolve() != root:
            raise InstallError("Protected storage path contains a symlink")
        self.fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
        try:
            self._directory(self.fd)
        except BaseException:
            os.close(self.fd)
            raise

    def close(self):
        os.close(self.fd)

    def _adopt(self, target, *, directory: bool, fd=None):
        """Give protected state the operator group's read access.

        Falls back to legacy root-only modes when the group cannot be applied,
        so a host without the group keeps working with its previous modes.
        """
        if self.group is None:
            return
        mode = self.dir_mode if directory else self.file_mode
        try:
            if fd is None:
                os.chown(target, -1, self.group, follow_symlinks=False)
                os.chmod(target, mode, follow_symlinks=False)
            else:
                os.fchown(fd, -1, self.group)
                os.fchmod(fd, mode)
        except OSError:
            legacy = LEGACY_DIR_MODE if directory else LEGACY_FILE_MODE
            if fd is None:
                os.chmod(target, legacy, follow_symlinks=False)
            else:
                os.fchmod(fd, legacy)
            self.group = None
            self.dir_mode, self.file_mode = LEGACY_DIR_MODE, LEGACY_FILE_MODE

    def _directory(self, fd):
        info = os.fstat(fd)
        if info.st_uid != self.owner:
            raise InstallError("Protected directory ownership or mode differs")
        mode = stat.S_IMODE(info.st_mode)
        if mode not in (self.dir_mode, LEGACY_DIR_MODE):
            raise InstallError("Protected directory ownership or mode differs")
        if mode == self.dir_mode and self.group is not None and info.st_gid != self.group:
            raise InstallError("Protected directory ownership or mode differs")

    def _safe(self, info):
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_uid != self.owner:
            return False
        return stat.S_IMODE(info.st_mode) in (self.file_mode, LEGACY_FILE_MODE)

    @contextlib.contextmanager
    def parent(self, name: str, *, create: bool = False):
        parts = PurePosixPath(name).parts
        if not parts or name.startswith("/") or any(p in (".", "..") for p in parts):
            raise InstallError("Invalid protected relative path")
        current = os.dup(self.fd)
        try:
            for part in parts[:-1]:
                created = False
                if create:
                    try:
                        os.mkdir(part, LEGACY_DIR_MODE, dir_fd=current)
                        created = True
                        os.fsync(current)
                    except FileExistsError:
                        pass
                child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=current)
                try:
                    if created:
                        self._adopt(None, directory=True, fd=child)
                    self._directory(child)
                except BaseException:
                    os.close(child)
                    raise
                os.close(current)
                current = child
            yield current, parts[-1]
        finally:
            os.close(current)

    def read(self, name: str, limit: int = 4 * 1024 * 1024) -> bytes | None:
        try:
            with self.parent(name) as (parent, leaf):
                fd = os.open(leaf, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=parent)
        except FileNotFoundError:
            return None
        try:
            info = os.fstat(fd)
            if not self._safe(info):
                raise InstallError("Protected file ownership, mode or type differs")
            chunks, count = [], 0
            while block := os.read(fd, min(65536, limit + 1 - count)):
                count += len(block)
                if count > limit:
                    raise InstallError("Protected file exceeds its size limit")
                chunks.append(block)
            return b"".join(chunks)
        finally:
            os.close(fd)

    def write(self, name: str, data: bytes):
        with self.parent(name, create=True) as (parent, leaf):
            try:
                existing = os.stat(leaf, dir_fd=parent, follow_symlinks=False)
            except FileNotFoundError:
                existing = None
            if existing is not None and not self._safe(existing):
                raise InstallError("Protected write target ownership, mode or type differs")
            temporary = ".new-" + secrets.token_hex(12)
            fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, LEGACY_FILE_MODE, dir_fd=parent)
            try:
                self._adopt(None, directory=False, fd=fd)
                view = memoryview(data)
                while view:
                    view = view[os.write(fd, view):]
                os.fsync(fd)
                os.close(fd)
                fd = -1
                os.replace(temporary, leaf, src_dir_fd=parent, dst_dir_fd=parent)
                os.fsync(parent)
            finally:
                if fd >= 0:
                    os.close(fd)
                try:
                    os.unlink(temporary, dir_fd=parent)
                except FileNotFoundError:
                    pass

    def json(self, name: str, default=None):
        data = self.read(name)
        if data is None:
            return default
        try:
            return strict_json(data)
        except (ValueError, UnicodeError) as error:
            raise InstallError("Protected state is invalid; original retained") from error

    def save(self, name: str, value):
        self.write(name, json.dumps(value, sort_keys=True, indent=2, allow_nan=False).encode() + b"\n")

    def remove(self, name: str):
        try:
            with self.parent(name) as (parent, leaf):
                info = os.stat(leaf, dir_fd=parent, follow_symlinks=False)
                if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_uid != self.owner:
                    raise InstallError("Protected removal target ownership or type differs")
                os.unlink(leaf, dir_fd=parent)
                os.fsync(parent)
        except FileNotFoundError:
            return

    @contextlib.contextmanager
    def lock(self, name="operation.lock", timeout=5):
        with self.parent(name, create=True) as (parent, leaf):
            fd = os.open(leaf, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_CLOEXEC, LEGACY_FILE_MODE, dir_fd=parent)
        try:
            self._adopt(None, directory=False, fd=fd)
            info = os.fstat(fd)
            if not self._safe(info):
                raise InstallError("Protected lock is unsafe")
            deadline = time.monotonic() + timeout
            while True:
                try:
                    fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except BlockingIOError:
                    if time.monotonic() >= deadline:
                        raise InstallError("Another installation or recovery is running")
                    time.sleep(0.05)
            yield
        finally:
            os.close(fd)


def document(store: Store, name: str):
    data = store.read(name)
    return parse_document(data) if data is not None else None


def parse_document(data: bytes):
    try:
        return strict_json(data)
    except ValueError:
        try:
            import yaml
        except ImportError as error:
            raise InstallError("Existing YAML requires PyYAML; original retained") from error
        try:
            class UniqueLoader(yaml.SafeLoader):
                pass
            def mapping(loader, node):
                loader.flatten_mapping(node)
                pairs = loader.construct_pairs(node)
                result = {}
                for key, value in pairs:
                    if not isinstance(key, str) or key in result:
                        raise ValueError("Invalid or duplicate YAML key")
                    result[key] = value
                return result
            UniqueLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, mapping)
            value = yaml.load(data, Loader=UniqueLoader)
            json.dumps(value, allow_nan=False)
        except (yaml.YAMLError, ValueError, TypeError, RecursionError) as error:
            raise InstallError("Existing YAML requires a valid document and PyYAML; original retained") from error
        if not isinstance(value, dict):
            raise InstallError("Configuration must be a mapping")
        return value


def strict_json(data):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("Duplicate JSON key")
            result[key] = value
        return result
    def invalid(value):
        raise ValueError("Non-finite JSON number")
    def finite(value):
        parsed = float(value)
        if not math.isfinite(parsed):
            invalid(value)
        return parsed
    return json.loads(data, object_pairs_hook=pairs, parse_constant=invalid, parse_float=finite)
