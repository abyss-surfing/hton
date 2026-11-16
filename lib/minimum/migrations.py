"""Ordered, idempotent minimum-installation migrations."""
from __future__ import annotations

import os
from pathlib import Path

import storage
from storage import InstallError

LEDGER = "migrations.json"


def _remove(root: Path, relative: str) -> None:
    path = root / relative
    if path.is_symlink() or path.is_file():
        path.unlink()
        return
    if path.exists():
        raise InstallError("Migration refused to remove a non-file: " + relative)


def _permissions(root: Path) -> None:
    if not root.exists():
        return
    if root.is_symlink() or not root.is_dir():
        raise InstallError("Migration refused an unsafe runtime root")
    dir_mode, file_mode = storage.modes()
    gid = storage.group_id()
    os.chmod(root, dir_mode)
    if gid is not None:
        os.chown(root, -1, gid)
    for current, dirs, files in os.walk(root, followlinks=False):
        base = Path(current)
        for name in dirs:
            path = base / name
            if path.is_symlink():
                raise InstallError("Migration refused a symlink directory")
            os.chmod(path, dir_mode)
            if gid is not None:
                os.chown(path, -1, gid)
        for name in files:
            path = base / name
            if path.is_symlink():
                raise InstallError("Migration refused a symlink file")
            os.chmod(path, file_mode)
            if gid is not None:
                os.chown(path, -1, gid)


def _remove_legacy_outline(work, config, state) -> None:
    _remove(work.root, "share/outline.yaml")
    # secrets/outline-secret is still referenced by an active outline receipt
    # until the old container is retired; protocol cleanup removes it then.
    if config is not None:
        _remove(config.root, "modules/foreign-outline.yaml")


def _runtime_permissions(work, config, state) -> None:
    _permissions(work.root)
    if config is not None:
        _permissions(config.root)


MIGRATIONS = (
    ("0001-remove-legacy-outline-material", _remove_legacy_outline),
    ("0002-runtime-permissions", _runtime_permissions),
)

_DESCRIPTIONS = {
    "0001-remove-legacy-outline-material": "remove preserved legacy outline material",
    "0002-runtime-permissions": "apply protected runtime root and file permissions",
}


def _ledger(work) -> dict:
    ledger = work.json(LEDGER, {})
    if not isinstance(ledger, dict) or not isinstance(ledger.get("done", []), list):
        raise InstallError("Migration ledger is invalid")
    done = ledger.get("done", [])
    if any(not isinstance(name, str) for name in done):
        raise InstallError("Migration ledger is invalid")
    return {"done": set(done)}


def plan(work, config, state) -> list:
    """Return the read-only source -> target migration plan.

    Each entry names one bounded minimum migration and whether it is still
    pending.  Planning never executes the function, writes the ledger or touches
    runtime files; it only binds the current ledger revision.
    """
    done = _ledger(work)["done"]
    return [
        {
            "name": name,
            "pending": name not in done,
            "description": _DESCRIPTIONS.get(name, ""),
        }
        for name, _function in MIGRATIONS
    ]


def apply(work, config, state, save=None) -> None:
    done = _ledger(work)["done"]
    for name, function in MIGRATIONS:
        if name in done:
            continue
        function(work, config, state)
        done.add(name)
        work.save(LEDGER, {"done": sorted(done)})
        if save:
            save()
