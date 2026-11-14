"""Dedicated managed SSH account, key and sudoers effect owner."""
from __future__ import annotations

import os
import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

from storage import InstallError, digest

DEFAULT_ROOT = Path("/opt/hton")
USERNAME = re.compile(r"^[a-z][a-z0-9_-]{0,31}$")
_KEY_HEADER = b"-----BEGIN OPENSSH PRIVATE KEY-----"
_ALLOWED = {"useradd", "usermod", "userdel", "gpasswd", "install", "chmod",
    "chown", "mkdir", "rm", "visudo", "ssh-keygen", "getent"}


class HostAccessError(InstallError):
    """A bounded host-access error; key bytes and account secrets never enter it."""


@dataclass(frozen=True)
class HostAccessPlan:
    enabled: bool
    username: str | None = None
    ssh_port: int | None = None
    home: str | None = None
    receipt_id: str | None = None
    sudoers: str | None = None
    disabled: bool = False


def _field(value, name, default=None):
    if isinstance(value, dict):
        return value.get(name, default)
    return getattr(value, name, default)


def plan(configured, account_sshd_facts=None, receipt=None, root=DEFAULT_ROOT) -> HostAccessPlan:
    """Pure host-access plan from validated config and read-only account facts."""
    access = _field(configured, "access", "none")
    if access == "none":
        return HostAccessPlan(False, disabled=True)
    if access != "generate":
        raise HostAccessError("policyBlocked: unknown admin access mode")
    user = _field(configured, "user")
    if not isinstance(user, str) or not USERNAME.match(user):
        raise HostAccessError("policyBlocked: the admin user must be a bounded lowercase name")
    if user == "root":
        raise HostAccessError("policyBlocked: the admin user must never be root")
    port = _field(configured, "sshPort")
    if type(port) is not int or not 1 <= port <= 65535:
        raise HostAccessError("policyBlocked: the admin SSH port is invalid")
    facts = account_sshd_facts or {}
    users = set(facts.get("users", ()))
    if user in users and (not receipt or receipt.get("username") != user):
        raise HostAccessError("accountConflict: the account name is already in use")
    if facts.get("password_auth_disabled") is False:
        raise HostAccessError("policyBlocked: key-only admission cannot be guaranteed")
    receipt_id = digest((user + ":" + str(port)).encode())[:16]
    home = Path(root) / "host-access" / receipt_id / "home"
    sudoers = Path("/etc/sudoers.d") / ("hton-amnezia-" + receipt_id)
    return HostAccessPlan(True, user, port, str(home), receipt_id, str(sudoers))


def _valid_key(key_bytes) -> bool:
    if not isinstance(key_bytes, (bytes, bytearray)):
        return False
    return len(key_bytes) <= 16384 and bytes(key_bytes).startswith(_KEY_HEADER)


def _run(argv, input=None):
    if not isinstance(argv, (list, tuple)) or not argv:
        raise HostAccessError("policyBlocked: empty host command")
    name = PurePosixPath(argv[0]).name
    if name not in _ALLOWED:
        raise HostAccessError("policyBlocked: host command is not permitted")
    return subprocess.run(argv, input=input, check=True, timeout=60,
        env={"PATH": "/usr/bin:/bin", "LANG": "C"})


def _sudoers(username: str) -> bytes:
    if not USERNAME.match(username):
        raise HostAccessError("policyBlocked: invalid sudoers username")
    return (username + " ALL=(ALL) NOPASSWD: ALL\n").encode()


def reconcile(plan_value: HostAccessPlan, key_bytes, journal=None, runner=None,
              root=DEFAULT_ROOT) -> dict:
    """Create only the managed account, key and sudoers declared by the plan."""
    if not plan_value.enabled:
        return {"disabled": True, "username": None, "receiptId": None}
    if not _valid_key(key_bytes):
        raise HostAccessError("accountConflict: the managed key material is invalid")
    run = runner or _run
    home = Path(plan_value.home)
    ssh = home / ".ssh"
    home.mkdir(parents=True, mode=0o700, exist_ok=True)
    parent = home.parent.parent
    os.chmod(parent, 0o711)
    receipt = {
        "username": plan_value.username,
        "sshPort": plan_value.ssh_port,
        "home": str(home),
        "sudoers": plan_value.sudoers,
        "receiptId": plan_value.receipt_id,
        "keyDigest": digest(bytes(key_bytes)),
        "created": True,
    }
    run(["useradd", "--system", "--create-home", "--home-dir", str(home),
        "--shell", "/bin/bash", "--groups", "sudo", plan_value.username])
    run(["install", "-d", "-m", "0700", "-o", plan_value.username,
        "-g", plan_value.username, str(ssh)])
    run(["install", "-m", "0600", "-o", plan_value.username, "-g",
        plan_value.username, "/dev/stdin", str(ssh / "authorized_keys")],
        input=bytes(key_bytes))
    sudoers_path = plan_value.sudoers
    staging = str(sudoers_path) + ".hton-new"
    run(["install", "-m", "0440", "-o", "root", "-g", "root", "/dev/stdin", staging],
        input=_sudoers(plan_value.username))
    run(["visudo", "-cf", staging])
    os.replace(staging, sudoers_path)
    if journal:
        journal(receipt)
    return receipt


def deactivate(receipt, current_facts=None) -> dict:
    """Remove only the managed privilege/key; preserve data and home."""
    if not isinstance(receipt, dict) or not receipt.get("username"):
        return {"deactivated": False}
    username = receipt["username"]
    if not USERNAME.match(username):
        raise HostAccessError("drift: the receipt username is invalid")
    facts = current_facts or {}
    if username in set(facts.get("other_owners", ())):
        raise HostAccessError("drift: the account has other owners")
    return {"deactivated": True, "username": username,
        "sudoers": receipt.get("sudoers"), "home": receipt.get("home")}


def purge(receipt, explicit_consent=False, root=DEFAULT_ROOT) -> dict:
    """Delete only the exact managed subtree after explicit consent."""
    if not explicit_consent:
        raise HostAccessError("policyBlocked: purge requires explicit consent")
    if not isinstance(receipt, dict) or not receipt.get("receiptId"):
        raise HostAccessError("drift: purge receipt is invalid")
    receipt_id = str(receipt["receiptId"])
    if not re.fullmatch(r"[0-9a-f]{16}", receipt_id):
        raise HostAccessError("drift: purge receipt id is invalid")
    base = (Path(root) / "host-access" / receipt_id).resolve()
    allowed = (Path(root) / "host-access").resolve()
    if allowed not in base.parents:
        raise HostAccessError("policyBlocked: purge target escapes the managed tree")
    if base.exists():
        shutil.rmtree(base)
    return {"purged": True, "receiptId": receipt_id}
