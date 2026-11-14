#!/usr/bin/env python3
"""Minimum Core: trusted site and reversible loopback Amnezia installation."""
from __future__ import annotations

import argparse
import copy
import contextlib
import errno
import fcntl
import grp
import json
import os
from pathlib import Path
import re
import secrets
import signal
import socket
import ssl
import stat
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "lib"))

try:
    import yaml
    if int(str(getattr(yaml, "__version__", "0")).split(".")[0]) < 6:
        raise RuntimeError("PyYAML 6.x is required")
except (ImportError, RuntimeError) as error:
    raise SystemExit("ERROR: PyYAML 6.x is required on the host") from error

import access_links
import certificates
import client_page
import clients
import configuration
import docker
import ingress
import managed_amnezia
import migrations
import protocols
import publication
import storage
import updates
from storage import InstallError, Store, digest, document

SOURCE = Path(__file__).resolve().parents[2]
from install_config import ConfigError, deployment_root, load_settings
ROOT = deployment_root(SOURCE)
UNIT_NAMES = ("hton-minimum-recovery.service", "hton-minimum-recovery.timer",
    "hton-minimum-renew.service", "hton-minimum-renew.timer",
    "hton-minimum-usage.service", "hton-minimum-usage.timer",
    "hton-minimum-update.service", "hton-minimum-update.timer")
CLIENT_PAGE_UNITS = ("hton-minimum-client-page.service", "hton-minimum-client-page.timer")


def image_lock():
    shipped = (SOURCE / ".secrets.example/images.lock.json", SOURCE / "platform/images.lock.json")
    for path in (ROOT / ".secrets/images.lock.json", *shipped):
        if path.is_file():
            lock = json.loads(path.read_text())
            images = lock.get("images") if isinstance(lock, dict) else None
            if isinstance(images, dict) and "amneziawg" not in images:
                for baseline in shipped:
                    if baseline == path or not baseline.is_file():
                        continue
                    role = json.loads(baseline.read_text())["images"].get("amneziawg")
                    if role is not None:
                        images["amneziawg"] = role
                        break
            return lock
    raise InstallError("Image lock is unavailable")


def resolve_pins(lock, required, *, allow_cached=False):
    images = lock.get("images") if isinstance(lock, dict) else None
    if not isinstance(images, dict):
        raise InstallError("Image lock is invalid")
    pins = {}
    for name in required:
        entry = images.get(name)
        if not isinstance(entry, dict):
            raise InstallError("Image lock is missing " + name)
        reference = entry.get("reference")
        tag = entry.get("sourceTag")
        if not isinstance(reference, str) or "@sha256:" not in reference:
            raise InstallError("An image is not pinned")
        target = reference
        if isinstance(tag, str) and tag:
            target = reference.split("@", 1)[0] + ":" + tag
        try:
            docker.command("image", "pull", "--quiet", target, timeout=300)
        except InstallError:
            if not allow_cached:
                raise
            try:
                pins[name] = local_image_id(target)
            except InstallError:
                if target == reference:
                    raise
                pins[name] = local_image_id(reference)
        else:
            pins[name] = local_image_id(target)
    return pins


def local_image_id(reference):
    value = docker.command("image", "inspect", "--format", "{{.Id}}", reference).decode().strip()
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", value):
        raise InstallError("Docker did not return a valid immutable image identity")
    return value


def boot_id():
    return Path("/proc/sys/kernel/random/boot_id").read_text().strip()


def state_record(work):
    value = work.json("state.json")
    if value is None:
        return None
    if not isinstance(value, dict) or value.get("schemaVersion") != 1:
        raise InstallError("Unsupported minimum state version; original retained")
    for field, size in (("instance", 32), ("operation", 32), ("originalId", 64), ("nativeDigest", 64), ("binaryDigest", 64), ("originalFingerprint", 64)):
        text = value.get(field)
        if not isinstance(text, str) or len(text) != size or any(c not in "0123456789abcdef" for c in text):
            raise InstallError("Minimum state identity is invalid; original retained")
    for field in ("originalNativeDigest", "replacementNativeDigest", "nativeStructureDigest", "nativeClientsRequalified"):
        text = value.get(field)
        if text is not None and (not isinstance(text, str) or len(text) != 64 or any(c not in "0123456789abcdef" for c in text)):
            raise InstallError("Minimum native digest state is invalid; original retained")
    if type(value.get("port")) is not int or not 5000 <= value["port"] <= 6000:
        raise InstallError("Saved minimum VPN port is invalid; no reroll performed")
    for field in ("activeProtocols", "candidateProtocols"):
        selected = value.get(field)
        if selected is None:
            continue
        if not isinstance(selected, dict) or set(selected) - {"mtproto", "hysteria", "outline-wss", "amneziawg", "selfsteal"}:
            raise InstallError("Saved protocol ownership state is invalid; original retained")
        for role, identifier in selected.items():
            if not isinstance(identifier, str) or len(identifier) != 64 or any(c not in "0123456789abcdef" for c in identifier):
                raise InstallError("Saved protocol container identity is invalid; original retained")
    retired = value.get("retiredProtocols")
    if retired is not None and (not isinstance(retired, list) or len(retired) > 16
            or any(not isinstance(item, str) or len(item) != 64 or any(c not in "0123456789abcdef" for c in item) for item in retired)):
        raise InstallError("Saved retired protocol state is invalid; original retained")
    if value.get("protocols") is not None and not isinstance(value["protocols"], dict):
        raise InstallError("Saved protocol facts are invalid; original retained")
    if value.get("mtprotoPort") is not None and (type(value["mtprotoPort"]) is not int
            or not 40000 <= value["mtprotoPort"] <= 49999):
        raise InstallError("Saved MTProto internal port is invalid; original retained")
    if value.get("amneziawgPort") is not None and (type(value["amneziawgPort"]) is not int
            or not 5000 <= value["amneziawgPort"] <= 6000):
        raise InstallError("Saved AmneziaWG2 UDP port is invalid; original retained")
    return value


def lease_expired(state):
    return bool(state.get("lease")) and (state.get("leaseBoot") != boot_id()
        or state["lease"] <= time.time() or state.get("leaseMonotonic", 0) <= time.monotonic())


def state_fingerprint(state, value):
    return docker.fingerprint(value, state.get("fingerprintVersion", 1))


def fingerprint_matches(state, recorded, value):
    return state_fingerprint(state, value) == recorded


APPROVAL_VERSION = 2


def upgrade_state_fingerprints(config, work, state):
    """Re-baseline stored launch fingerprints and their derived consent scope.

    Fingerprint v2 omits Docker's daemon-issued default-bridge ID, which changes
    whenever the daemon recreates that network. The upgrade is allowed only after
    the live replacement is proven equivalent to the protected original launch
    record, so it cannot mask arbitrary drift.
    """
    if not state:
        return
    original = None
    changed = False
    if state.get("fingerprintVersion", 1) < docker.FINGERPRINT_VERSION:
        original = work.json("original.json")
        if not isinstance(original, dict):
            raise InstallError("Protected original launch record is missing; no re-baseline performed")
        state["fingerprintVersion"] = docker.FINGERPRINT_VERSION
        state["originalFingerprint"] = docker.fingerprint(original)
        if state.get("hasCommitted"):
            operation = Installation(config, work, state)
            operation.check_foreign(state["replacement"], replacement=True)
            state["replacementFingerprint"] = state_fingerprint(state, docker.inspect(state["replacement"]))
        changed = True
    if state.get("approvalVersion", 1) < APPROVAL_VERSION:
        if original is None:
            original = work.json("original.json")
        names = state.get("originalNativeNames")
        if not isinstance(original, dict) or not isinstance(names, list):
            raise InstallError("Saved remap approval cannot be resealed; no changes performed")
        approval_native = {"nativeDigest": state.get("originalNativeDigest", state["nativeDigest"]),
            "binaryDigest": state["binaryDigest"], "nativeVersion": state["nativeVersion"], "names": names}
        state["approval"] = consent_scope(original, approval_native, state["port"])
        state["approvalVersion"] = APPROVAL_VERSION
        changed = True
    if changed:
        work.save("state.json", state)


@contextlib.contextmanager
def installation_lock():
    _, file_mode = storage.modes()
    fd = os.open(ROOT / ".minimum.install.lock", os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_CLOEXEC, storage.LEGACY_FILE_MODE)
    try:
        gid = storage.group_id()
        if gid is not None:
            try:
                os.fchown(fd, -1, gid)
                os.fchmod(fd, file_mode)
            except OSError:
                pass
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_uid != 0 or info.st_mode & 0o022:
            raise InstallError("Installation lock is unsafe")
        deadline = time.monotonic() + 5
        while True:
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise InstallError("Another minimum install or renewal is running")
                time.sleep(0.05)
        yield
    finally:
        os.close(fd)


def process_identity(pid):
    base = Path("/proc") / str(pid)
    raw = (base / "stat").read_text()
    ticks = raw[raw.rfind(")") + 2:].split()[19]
    with (base / "cmdline").open("rb") as stream:
        argv = stream.read(16385)
    if len(argv) > 16384 or base.stat().st_uid != 0:
        raise InstallError("Recovery actor identity is unsupported")
    executable = (base / "exe").stat()
    return {"pid": pid, "startTicks": ticks, "argvDigest": digest(argv),
        "device": executable.st_dev, "inode": executable.st_ino, "group": os.getpgid(pid)}


def terminate_expired_actor(state):
    """Reclaim only a proven expired private installer, never the SSH shell."""
    if not lease_expired(state):
        raise InstallError("Recovery actor lease has not expired")
    actor = state.get("actor")
    if not actor or state.get("leaseBoot") != boot_id():
        return
    pid = actor["pid"]
    if pid == os.getpid():
        raise InstallError("Recovery cannot terminate its own process")
    try:
        fd = os.pidfd_open(pid)
    except ProcessLookupError:
        return
    try:
        if process_identity(pid) != actor or actor["group"] != pid:
            raise InstallError("Expired installer identity changed; refusing to signal it")
        children = Path(f"/proc/{pid}/task/{pid}/children").read_text().split()
        if len(children) > 32:
            raise InstallError("Expired installer child set exceeds bounds")
        for child in children:
            child_pid = int(child)
            try:
                child_fd = os.pidfd_open(child_pid)
            except ProcessLookupError:
                continue
            try:
                identity = process_identity(child_pid)
                executable = Path(os.readlink(f"/proc/{child_pid}/exe")).name
                if identity["group"] != child_pid or executable not in ("docker", "curl", "openssl", "systemctl"):
                    raise InstallError("Expired installer has an unqualified native child")
                signal.pidfd_send_signal(child_fd, signal.SIGKILL)
            finally:
                os.close(child_fd)
        signal.pidfd_send_signal(fd, signal.SIGKILL)
    finally:
        os.close(fd)


def interactive_install(args):
    return (getattr(args, "action", "install") == "install"
        and not getattr(args, "json", False) and not getattr(args, "quiet", False)
        and sys.stdin.isatty())


def prompt(label, default=""):
    try:
        with open("/dev/tty", "w", encoding="utf-8") as output, open("/dev/tty", "r", encoding="utf-8") as input_stream:
            output.write(label + (" [" + default + "]" if default else "") + ": ")
            output.flush()
            value = input_stream.readline(1025).strip()
    except OSError as error:
        raise InstallError("Installation input requires a readable and writable local TTY") from error
    if len(value) > 1024:
        raise InstallError("Installation input exceeds its bound")
    return value or default


def consent_scope(value, native, port):
    return digest(json.dumps([value["Id"], docker.fingerprint(value), native["nativeDigest"], native["binaryDigest"], native["nativeVersion"], native["names"], port], sort_keys=True).encode())[:16]


def consent(value, native, port):
    scope = consent_scope(value, native, port)
    answer = prompt(
        "\nAmnezia must move off port 443 so HAProxy can serve the website and VPN together.\n"
        + "Container: " + value["Name"].lstrip("/") + " (" + value["Id"] + ")\n"
        + "New internal address: 127.0.0.1:" + str(port) + "\n"
        + "Your VPN settings and keys will be kept. Existing VPN connections will briefly disconnect.\n"
        + "The original container will be kept for recovery.\n"
        + "Docker must be its only restart manager. Do not run other Amnezia management tools during setup.\n"
        + "Move Amnezia and continue? [y/N]")
    if answer.lower() not in ("y", "yes"):
        raise InstallError("Amnezia port change cancelled; explicit local confirmation is required")
    return scope


def load_site_inputs(root, args):
    bootstrap = load_settings(SOURCE).get("core", {})
    saved_core = root.get("core", {})
    domains = saved_core.get("edge", {}).get("domains") or []
    bootstrap_domains = bootstrap.get("edge", {}).get("domains") or []
    if not isinstance(domains, list) or not isinstance(bootstrap_domains, list) or len(domains) > 1 or (not domains and len(bootstrap_domains) > 1):
        raise InstallError("This minimum profile supports one site name per installation; additional domains require the multi-domain implementation and are not ignored")
    saved_domain = domains[0] if domains else None
    saved_email = saved_core.get("certificates", {}).get("email")
    default_domain = bootstrap_domains[0] if bootstrap_domains else None
    default_email = bootstrap.get("certificates", {}).get("email")
    domain = args.site_domain or saved_domain or default_domain
    email = args.certificate_email or saved_email or default_email
    if interactive_install(args):
        domain = domain or prompt("Site domain")
        email = email or prompt("Certificate account email")
    if not docker.valid_domain(domain) or not isinstance(email, str) or "@" not in email or len(email) > 254 or any(ord(c) < 33 for c in email):
        raise InstallError("A valid configured site domain and certificate account email are required")
    if saved_domain and saved_domain != domain or saved_email and saved_email != email:
        raise InstallError("Saved site settings differ; explicit configuration review is required")
    return domain, email


def save_site_inputs(config, root, domain, email):
    root.setdefault("schemaVersion", 1)
    installation = root.setdefault("installation", {})
    if installation.get("directory", str(ROOT)) != str(ROOT):
        raise InstallError("Saved installation directory differs; implicit relocation is forbidden")
    installation["directory"] = str(ROOT)
    core = root.setdefault("core", {})
    core.setdefault("edge", {})["domains"] = [domain]
    core.setdefault("certificates", {})["email"] = email
    config.save("hton.yaml", root)


def configure_test_tunnel(root, args, domain):
    """Apply the explicit test-SNI reverse-tunnel block, if requested.

    The route only forwards a dedicated SNI to a loopback port supplied by an
    operator's reverse SSH forward. It is never derived from production names.
    """
    core = root.setdefault("core", {})
    edge = core.setdefault("edge", {})
    if getattr(args, "no_test_tunnel", False):
        edge.pop("testTunnel", None)
        return None
    sni = getattr(args, "test_sni", None)
    port = getattr(args, "test_tunnel_port", None)
    if sni is None and port is None:
        value = edge.get("testTunnel")
        if value is None:
            return None
        if not isinstance(value, dict):
            raise InstallError("Saved test tunnel settings are malformed; explicit review is required")
        if value.get("enabled", True) is False:
            return None
        return value
    if sni is None or port is None:
        raise InstallError("--test-sni and --test-tunnel-port must be given together")
    try:
        sni = configuration.canonical_name(sni)
    except configuration.ConfigError as error:
        raise InstallError("Test tunnel SNI must be a valid DNS name") from error
    if sni in (domain, "www." + domain):
        raise InstallError("Test tunnel SNI must be separate from the site names")
    try:
        port = int(port)
    except (TypeError, ValueError) as error:
        raise InstallError("Test tunnel port must be an integer in 1024..65535") from error
    if not 1024 <= port <= 65535:
        raise InstallError("Test tunnel port must be an integer in 1024..65535")
    value = {"sni": sni, "port": port}
    edge["testTunnel"] = value
    return value


def validate_test_tunnel(value, names):
    """Reject a test SNI that would shadow a production name before saving it."""
    if not value:
        return
    sni = value.get("sni") if isinstance(value, dict) else None
    reserved = {name for name in names if name}
    if sni in reserved:
        raise InstallError("Test tunnel SNI must be separate from every production SNI")


def available(port, values, allowed=()):
    for identifier, value in values.items():
        if identifier in allowed:
            continue
        for bindings in (value["HostConfig"].get("PortBindings") or {}).values():
            if any(int(binding.get("HostPort") or 0) == port for binding in bindings or []):
                return False
    sockets = []
    try:
        for kind in (socket.SOCK_STREAM, socket.SOCK_DGRAM):
            stream = socket.socket(socket.AF_INET, kind)
            sockets.append(stream)
            stream.bind(("127.0.0.1", port))
        return True
    except OSError:
        return False
    finally:
        for stream in sockets:
            stream.close()


def unit_preflight(work_root, extra=()):
    parent = Path("/etc/systemd/system")
    info = parent.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
        raise InstallError("System unit directory is unsafe")
    for name in (*UNIT_NAMES, *extra):
        path = parent / name
        if os.path.lexists(path) and (not path.is_symlink() or os.readlink(path) != str(work_root / "units" / name) or path.lstat().st_uid != 0):
            raise InstallError("A minimum-profile unit path is owned by another installation")


def install_units(work, client_page_enabled=False, refresh_seconds=60):
    extra = CLIENT_PAGE_UNITS if client_page_enabled else ()
    unit_preflight(work.root, extra)
    common = "[Service]\nType=oneshot\nUser=root\nGroup=root\nUMask=0077\nWorkingDirectory=" + str(ROOT) + "\nTimeoutStartSec=420\n"
    for role, schedule in (("recovery", "OnBootSec=15s\nOnUnitInactiveSec=15s"), ("renew", "OnCalendar=*-*-* 03,15:00:00\nRandomizedDelaySec=1h\nPersistent=true")):
        action = "recover" if role == "recovery" else "renew"
        service = f"hton-minimum-{role}.service"
        timer = f"hton-minimum-{role}.timer"
        work.write("units/" + service, ("[Unit]\nDescription=HTON minimum " + role + "\nAfter=docker.service\n" + common
            + "ExecStart=/usr/bin/python3 -E -s -B " + str(SOURCE / "python/minimum/install.py") + " " + action + "\n").encode())
        work.write("units/" + timer, ("[Unit]\nDescription=HTON minimum " + role + " schedule\n[Timer]\n" + schedule + "\nUnit=" + service + "\n[Install]\nWantedBy=timers.target\n").encode())
    work.write("units/hton-minimum-usage.service", ("[Unit]\nDescription=HTON minimum Outline usage\nAfter=docker.service\n" + common
        + "ExecStart=/usr/bin/python3 -E -s -B " + str(SOURCE / "scripts/usage") + " --collect\n").encode())
    work.write("units/hton-minimum-usage.timer", ("[Unit]\nDescription=HTON minimum Outline usage schedule\n[Timer]\n"
        "OnBootSec=2m\nOnUnitActiveSec=5m\nUnit=hton-minimum-usage.service\n[Install]\nWantedBy=timers.target\n").encode())
    work.write("units/hton-minimum-update.service", ("[Unit]\nDescription=HTON minimum source and image update\n"
        "After=docker.service network-online.target\nWants=network-online.target\n" + common
        + "TimeoutStartSec=3600\nExecStart=" + str(SOURCE / "scripts/update") + " --auto --quiet\n").encode())
    work.write("units/hton-minimum-update.timer", ("[Unit]\nDescription=HTON minimum weekly update check\n[Timer]\n"
        "OnCalendar=*-*-* 04:30:00\nRandomizedDelaySec=1h\nAccuracySec=10m\nPersistent=true\n"
        "Unit=hton-minimum-update.service\n[Install]\nWantedBy=timers.target\n").encode())
    if client_page_enabled:
        interval = refresh_seconds if type(refresh_seconds) is int and 10 <= refresh_seconds <= 3600 else 60
        work.write("units/hton-minimum-client-page.service", ("[Unit]\nDescription=HTON minimum client page refresh\nAfter=docker.service\n" + common
            + "ExecStart=/usr/bin/python3 -E -s -B " + str(SOURCE / "python/minimum/install.py") + " client-page-refresh\n").encode())
        work.write("units/hton-minimum-client-page.timer", ("[Unit]\nDescription=HTON minimum client page schedule\n[Timer]\n"
            "OnBootSec=2m\nOnUnitActiveSec=" + str(interval) + "s\nAccuracySec=5s\nUnit=hton-minimum-client-page.service\n[Install]\nWantedBy=timers.target\n").encode())
    docker.run(["/usr/bin/systemctl", "--system", "link", *[str(work.root / "units" / name) for name in (*UNIT_NAMES, *extra)]])
    docker.run(["/usr/bin/systemctl", "--system", "daemon-reload"])


def timer(role, enabled):
    docker.run(["/usr/bin/systemctl", "--system", "enable" if enabled else "disable", "--now", "hton-minimum-" + role + ".timer"])


def log(phase, outcome, detail=None):
    try:
        _log(phase, outcome, detail)
        return True
    except (OSError, InstallError):
        # Log pressure must never prevent safety recovery or working VPN traffic.
        return False


def _log_group():
    try:
        return grp.getgrnam("hton").gr_gid
    except KeyError:
        return 0


LOG_RETENTION_SECONDS = 7 * 24 * 60 * 60
LOG_MAX_BYTES = 1024 * 1024
LOG_ARCHIVES = 3


def _log_first_time(path):
    """Return the first JSON event timestamp, or None for an unreadable/empty file."""
    try:
        with open(path, "rb", buffering=0) as stream:
            line = stream.readline(4096)
        value = json.loads(line).get("time")
        return float(value) if isinstance(value, (int, float)) else None
    except (OSError, ValueError, AttributeError):
        return None


def _log_files(directory):
    for candidate in directory.glob("minimum.log*"):
        suffix = candidate.name[len("minimum.log"):]
        if suffix and not (suffix.startswith(".") and suffix[1:].isdigit()):
            continue
        try:
            mode = os.lstat(candidate).st_mode
        except OSError:
            continue
        if stat.S_ISREG(mode):
            yield candidate


def _rotate_log(path, gid, now):
    """Rotate the active log when it exceeds the size ceiling or its oldest entry is a week old."""
    try:
        size = os.lstat(path).st_size
    except OSError:
        return
    first = _log_first_time(path)
    if size <= LOG_MAX_BYTES and (first is None or now - first < LOG_RETENTION_SECONDS):
        return
    for number in range(LOG_ARCHIVES, 0, -1):
        source = path if number == 1 else path.with_name(path.name + "." + str(number - 1))
        if not source.exists() or source.is_symlink():
            continue
        target = path.with_name(path.name + "." + str(number))
        os.replace(source, target)
        try:
            os.chmod(target, 0o640)
            os.chown(target, 0, gid)
        except PermissionError:
            pass


def _prune_logs(directory, now):
    """Forget archived log files whose oldest entry (or mtime) is older than a week."""
    for candidate in _log_files(directory):
        if candidate.name == "minimum.log":
            continue
        first = _log_first_time(candidate)
        try:
            anchor = first if first is not None else os.lstat(candidate).st_mtime
        except OSError:
            continue
        if now - anchor >= LOG_RETENTION_SECONDS:
            try:
                candidate.unlink()
            except OSError:
                pass


def _log(phase, outcome, detail=None):
    parent = Path("/var/logs")
    parent.mkdir(mode=0o755, exist_ok=True)
    directory = parent / "hton"
    directory.mkdir(mode=0o750, exist_ok=True)
    if directory.resolve() != directory or directory.stat().st_uid != 0 or directory.stat().st_mode & 0o022:
        raise InstallError("HTON log directory is unsafe")
    gid = _log_group()
    try:
        os.chmod(directory, 0o750)
        os.chown(directory, 0, gid)
    except PermissionError:
        pass
    path = directory / "minimum.log"
    now = time.time()
    _rotate_log(path, gid, now)
    fd = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW, 0o640)
    try:
        try:
            os.fchmod(fd, 0o640)
            os.fchown(fd, 0, gid)
        except PermissionError:
            pass
        os.write(fd, json.dumps({"time": int(now), "component": "minimum", "phase": phase, "outcome": outcome, "diagnostic": detail}).encode() + b"\n")
    finally:
        os.close(fd)
    _prune_logs(directory, now)


class Installation:
    def __init__(self, config: Store, work: Store, state, reporter=None):
        self.config, self.work, self.state = config, work, state
        self.reporter = reporter

    def save(self):
        self.work.save("state.json", self.state)

    def recovery_save(self):
        try:
            self.save()
        except OSError as error:
            if error.errno not in (errno.ENOSPC, errno.EDQUOT, errno.EROFS):
                raise
            # Exact inverse authority already exists in the durable pre-effect
            # record. Preserve it and recover native service even on a full disk.

    def phase(self, phase):
        if lease_expired(self.state):
            raise InstallError("Cutover lease expired; exact recovery is required")
        self.state["phase"] = phase
        self.save()
        log(phase, "begin")
        text = {"snapshot-intent": "Saving a recovery copy of the VPN.",
            "snapshot-export": "Saving the VPN backup to disk.", "native-validation": "Checking the VPN backup.",
            "replacement-create-intent": "Preparing the VPN on its internal port.",
            "original-stop": "Switching the VPN port. Connections may briefly drop.",
            "site-start": "Starting the HTTPS website.", "edge-start": "Starting traffic routing on port 443.",
            "route-verification": "Checking HTTPS and the VPN connection.",
            "acceptance": "Saving the working setup and automatic renewal."}.get(phase)
        if text and self.reporter: self.reporter(text)

    def original(self):
        value = self.work.json("original.json")
        if not value or not fingerprint_matches(self.state, self.state["originalFingerprint"], value):
            raise InstallError("Protected original launch record is corrupt")
        return value

    def original_native_digest(self):
        return self.state.get("originalNativeDigest") or self.state["nativeDigest"]

    def replacement_native_digest(self):
        return self.state.get("replacementNativeDigest") or self.state["nativeDigest"]

    def check_foreign(self, identifier, *, replacement=False):
        original = self.original()
        current = docker.inspect(identifier)
        if current["Image"] != (self.state["snapshotImage"] if replacement else original["Image"]):
            raise InstallError("Foreign image identity differs from protected custody")
        expected = docker.create_payload(original, self.state["snapshotImage"], self.state["port"]) if replacement else copy.deepcopy(original["Config"])
        expected_host = expected.pop("HostConfig") if replacement else copy.deepcopy(original["HostConfig"])
        expected.pop("NetworkingConfig", None)
        expected_host = docker.canonical_host_config(expected_host, normalize_restored_lists=True)
        if not docker.same_networks(original, current):
            raise InstallError("Native network attachments drifted; refusing overwrite")
        if original.get("NetworkSettings", {}).get("Networks"):
            docker.verify_networks(original)
        policy = current["HostConfig"]["RestartPolicy"]
        if policy not in (original["HostConfig"]["RestartPolicy"], {"Name": "no", "MaximumRetryCount": 0}):
            raise InstallError("Foreign restart policy drift; originals retained")
        actual_host = docker.canonical_host_config(current["HostConfig"], normalize_restored_lists=True)
        actual_host["RestartPolicy"] = expected_host["RestartPolicy"]
        if actual_host != expected_host or current["Config"] != expected:
            raise InstallError("Foreign launch/config drift; refusing overwrite")
        allowed = {original["Name"], "/" + self.state["originalParked"], "/" + self.state["replacementName"], "/" + self.state["replacementParked"]}
        if current["Name"] not in allowed or current["State"].get("Paused") or current["State"].get("Restarting") or current["State"].get("Dead"):
            raise InstallError("Foreign identity/runtime drift")
        expected_native = self.replacement_native_digest() if replacement else self.original_native_digest()
        if digest(docker.native_config(identifier)) != expected_native:
            raise InstallError("Native keys or configuration changed; originals retained")
        return current

    def capture(self, original):
        if "snapshotImage" not in self.state:
            self.phase("snapshot-intent")
            image = docker.command("container", "commit", original["Id"], self.state["snapshotTag"], timeout=180).decode().strip()
            if not image.startswith("sha256:") or len(image) != 71 or any(c not in "0123456789abcdef" for c in image[7:]):
                raise InstallError("Snapshot image identity is invalid")
            self.state["snapshotImage"] = image
            self.save()
        if not fingerprint_matches(self.state, self.state["originalFingerprint"], docker.inspect(original["Id"])) or digest(docker.native_config(original["Id"])) != self.original_native_digest():
            raise InstallError("Original changed during snapshot; no cutover performed")
        archive = self.work.root / "original-image.tar"
        if not self.state.get("snapshotArchive"):
            if archive.exists():
                info = archive.lstat()
                if not storage.safe_file(info, 0):
                    raise InstallError("Incomplete snapshot archive is unsafe")
                archive.unlink()
            self.phase("snapshot-export")
            self.state["snapshotArchive"] = docker.save_image(self.state["snapshotImage"], archive)
            self.save()
        import hashlib
        sha = hashlib.sha256()
        fd = os.open(archive, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        try:
            info = os.fstat(fd)
            if not storage.safe_file(info, 0) or info.st_size > 2 * 1024**3:
                raise InstallError("Snapshot archive ownership or bounds differ")
            while block := os.read(fd, 1024 * 1024):
                sha.update(block)
            if {"bytes": info.st_size, "digest": sha.hexdigest()} != self.state["snapshotArchive"]:
                raise InstallError("Protected snapshot archive is corrupt; original retained")
        finally:
            os.close(fd)

    def replacement(self):
        if not self.state.get("replacement"):
            self.phase("replacement-create-intent")
            collisions = [v for v in docker.inventory().values() if v["Name"] == "/" + self.state["replacementName"]]
            if collisions:
                if len(collisions) != 1:
                    raise InstallError("Replacement create outcome is ambiguous; original retained")
                identifier = collisions[0]["Id"]
                existing = self.check_foreign(identifier, replacement=True)
                if existing["State"]["Running"] or existing["HostConfig"]["RestartPolicy"]["Name"] != "no":
                    raise InstallError("Unrecorded replacement was activated outside its transaction")
            else:
                identifier = docker.create(self.state["replacementName"], docker.create_payload(self.original(), self.state["snapshotImage"], self.state["port"]))
            self.state["replacement"] = identifier
            self.save()
        value = self.check_foreign(self.state["replacement"], replacement=True)
        if value["State"]["Running"]:
            raise InstallError("Prepared replacement was started outside its transaction")

    def validate_snapshot(self):
        self.phase("native-validation")
        identifier = docker.exact_id(docker.command("container", "create", "--pull", "never", "--network", "none",
            "--read-only", "--user", "0:0", "--cap-drop", "ALL", "--cap-add", "DAC_READ_SEARCH", "--security-opt", "no-new-privileges:true",
            "--memory", "128m", "--cpus", "0.5", "--pids-limit", "64", "--log-driver", "none",
            "--label", "io.hton.minimum=" + self.state["instance"], "--label", "io.hton.role=validator",
            "--entrypoint", "/usr/bin/xray", self.state["snapshotImage"], "run", "-test", "-config", "/opt/amnezia/xray/server.json").decode().strip())
        self.state["validatorJob"] = identifier
        ingress.record_owned(self.work, self.state, identifier, "validator")
        self.save()
        try:
            docker.command("container", "start", "--attach", identifier)
            if docker.inspect(identifier)["State"]["ExitCode"] != 0:
                raise InstallError("Snapshot-backed native config validation failed")
        finally:
            ingress.owned(self.state, identifier, "validator", self.work)
            docker.command("container", "rm", "--force", identifier)
            self.state.pop("validatorJob", None)
            self.state["ownedReceipts"].pop(identifier, None)
            self.save()

    def validate_native_bytes(self, content):
        """Validate migrated native bytes in an isolated pinned Xray container."""
        relative = "native/server.json"
        self.work.write(relative, content)
        self.phase("native-validation")
        identifier = docker.exact_id(docker.command("container", "create", "--pull", "never", "--network", "none",
            "--read-only", "--user", "0:0", "--cap-drop", "ALL", "--cap-add", "DAC_READ_SEARCH", "--security-opt", "no-new-privileges:true",
            "--memory", "128m", "--cpus", "0.5", "--pids-limit", "64", "--log-driver", "none",
            "--label", "io.hton.minimum=" + self.state["instance"], "--label", "io.hton.role=validator",
            "--mount", "type=bind,src=" + str(self.work.root / "native") + ",dst=/config,readonly",
            "--entrypoint", "/usr/bin/xray", self.state["snapshotImage"], "run", "-test", "-config", "/config/server.json").decode().strip())
        self.state["validatorJob"] = identifier
        ingress.record_owned(self.work, self.state, identifier, "validator", [relative])
        self.save()
        try:
            docker.command("container", "start", "--attach", identifier)
            if docker.inspect(identifier)["State"]["ExitCode"] != 0:
                raise InstallError("Migrated native REALITY configuration validation failed")
        finally:
            ingress.owned(self.state, identifier, "validator", self.work)
            docker.command("container", "rm", "--force", identifier)
            self.state.pop("validatorJob", None)
            self.state["ownedReceipts"].pop(identifier, None)
            self.save()

    def migrate_native(self, migration, legacy=False):
        """Apply or revert the native REALITY SNI/dest configuration."""
        identifier = self.state.get("replacement")
        if not identifier:
            raise InstallError("Native migration requires the prepared replacement")
        current = self.check_foreign(identifier, replacement=True)
        old = migration["previous"]
        new = migration["content"]
        observed = digest(docker.native_config(identifier))
        if observed == migration["digest"]:
            self.state["replacementNativeDigest"] = migration["digest"]
            if self.state.get("hasCommitted"):
                self.state["nativeDigest"] = migration["digest"]
            if legacy:
                self.state.pop("nativeMigration", None)
                self.state.pop("nativeMigrationPending", None)
                self.state["legacySniRestored"] = migration["digest"]
            else:
                self.state["nativeMigration"] = {"names": migration["names"], "dest": migration["dest"],
                    "fromDigest": migration["previousDigest"], "toDigest": migration["digest"]}
                self.state.pop("nativeMigrationPending", None)
            self.state["nativeStructureDigest"] = docker.native_structure_digest(new)
            self.save()
            return
        if observed != migration["previousDigest"]:
            raise InstallError("Native configuration changed before migration")
        self.work.write("native/migrated.json" if legacy else "native/previous.json", old)
        self.validate_native_bytes(new)
        running = current["State"]["Running"]
        if running:
            self.phase("native-migration-stop")
            docker.command("container", "stop", "--time", "10", identifier)
        self.phase("native-sni-migration")
        docker.replace_native_config(identifier, self.work.root / "native/server.json")
        if digest(docker.native_config(identifier)) != migration["digest"]:
            raise InstallError("Migrated native configuration differs after transfer")
        self.state["replacementNativeDigest"] = migration["digest"]
        if self.state.get("hasCommitted"):
            self.state["nativeDigest"] = migration["digest"]
        self.state["nativeStructureDigest"] = docker.native_structure_digest(new)
        if legacy:
            self.state.pop("nativeMigration", None)
            self.state.pop("nativeMigrationPending", None)
            self.state["legacySniRestored"] = migration["digest"]
        else:
            self.state["nativeMigration"] = {"names": migration["names"], "dest": migration["dest"],
                "fromDigest": migration["previousDigest"], "toDigest": migration["digest"]}
            self.state.pop("nativeMigrationPending", None)
        self.save()
        if running:
            docker.command("container", "start", identifier)
            for _ in range(50):
                if docker.inspect(identifier)["State"].get("Running"):
                    return
                time.sleep(0.2)
            docker.replace_native_config(identifier, self.work.root / "native/previous.json")
            self.state["replacementNativeDigest"] = migration["previousDigest"]
            if self.state.get("hasCommitted"):
                self.state["nativeDigest"] = migration["previousDigest"]
            if not legacy:
                self.state.pop("nativeMigration", None)
            self.save()
            docker.command("container", "start", identifier)
            raise InstallError("Migrated native service did not stay running; previous configuration restored")

    def rollback(self):
        if os.getpgrp() != os.getpid():
            os.setsid()
        self.state["actor"] = process_identity(os.getpid())
        self.state["lease"] = int(time.time()) + 180
        self.state["leaseMonotonic"] = time.monotonic() + 180
        self.state["leaseBoot"] = boot_id()
        self.state["phase"] = "recovery"
        self.recovery_save()
        ingress.cleanup_temporary_jobs(self.work, self.state, self.recovery_save)
        protocols.rollback_candidates(self.work, self.state)
        for role, identifier in self.state.get("candidateIngress", {}).items():
            value = ingress.owned(self.state, identifier, role)
            docker.command("container", "update", "--restart=no", identifier)
            if value["State"]["Running"]:
                docker.command("container", "stop", "--time", "10", identifier)
        if self.state.get("hasCommitted"):
            for role, identifier in self.state.get("activeIngress", {}).items():
                value = ingress.owned(self.state, identifier, role)
                docker.command("container", "update", "--restart=always", identifier)
                if not value["State"]["Running"]:
                    docker.command("container", "start", identifier)
            protocols.restore_active(self.work, self.state)
            protocols.cleanup(self.work, self.state, self.recovery_save)
            for role, identifier in list(self.state.get("candidateIngress", {}).items()):
                ingress.owned(self.state, identifier, role, self.work)
                docker.command("container", "rm", identifier)
                self.state["candidateIngress"].pop(identifier)
                self.state["ownedReceipts"].pop(identifier, None)
            self.state["candidateIngress"] = {}
            self.state["candidateProtocols"] = {}
            self.state["certificate"] = self.state["activeCertificate"]
            self.state["phase"] = "committed"
            self.state["lease"] = 0
            self.recovery_save()
            return
        if self.state.get("replacement"):
            identifier = self.state["replacement"]
            value = self.check_foreign(identifier, replacement=True)
            docker.command("container", "update", "--restart=no", identifier)
            if value["State"]["Running"]:
                docker.command("container", "stop", "--time", "10", identifier)
            if value["Name"] != "/" + self.state["replacementParked"]:
                docker.command("container", "rename", identifier, self.state["replacementParked"])
        original = self.original()
        identifier = original["Id"]
        value = self.check_foreign(identifier)
        if value["Name"] != original["Name"]:
            docker.command("container", "rename", identifier, original["Name"].lstrip("/"))
        policy = original["HostConfig"]["RestartPolicy"]
        restart = policy["Name"]
        if restart == "on-failure":
            restart += ":" + str(policy["MaximumRetryCount"])
        docker.command("container", "update", "--restart", restart, identifier)
        if original["State"]["Running"] and not docker.inspect(identifier)["State"]["Running"]:
            docker.command("container", "start", identifier)
        if not fingerprint_matches(self.state, self.state["originalFingerprint"], docker.inspect(identifier)):
            raise InstallError("Original restoration identity differs; recovery material retained")
        self.state["phase"] = "restored"
        self.state["lease"] = 0
        self.state["candidateIngress"] = {}
        self.recovery_save()

    def cutover(self, body, names, domain, pins, protocol_plan, verify_native=True, chain_sni=None, native_migration=None, native_restore=None):
        # All TTY input is complete; recovery never signals the SSH process group.
        if os.getpgrp() != os.getpid():
            os.setsid()
        self.state["actor"] = process_identity(os.getpid())
        self.state["lease"], self.state["leaseBoot"] = int(time.time()) + 180, boot_id()
        self.state["leaseMonotonic"] = time.monotonic() + 180
        self.save()
        timer("recovery", True)
        try:
            if native_migration is not None:
                self.migrate_native(native_migration)
            if not self.state.get("hasCommitted"):
                original = self.original()
                current = self.check_foreign(original["Id"])
                repeated = docker.qualify(current)
                if repeated["nativeDigest"] != self.original_native_digest() or repeated["binaryDigest"] != self.state["binaryDigest"]:
                    raise InstallError("Native qualification changed before cutover")
                self.phase("original-disable-restart")
                docker.command("container", "update", "--restart=no", original["Id"])
                self.phase("original-stop")
                docker.command("container", "stop", "--time", "10", original["Id"])
                self.check_foreign(original["Id"])
                self.phase("original-rename")
                docker.command("container", "rename", original["Id"], self.state["originalParked"])
                self.phase("replacement-rename")
                docker.command("container", "rename", self.state["replacement"], original["Name"].lstrip("/"))
                self.phase("replacement-start")
                docker.command("container", "start", self.state["replacement"])
            for role, identifier in self.state.get("activeIngress", {}).items():
                ingress.owned(self.state, identifier, role)
                self.phase("prior-ingress-stop")
                docker.command("container", "update", "--restart=no", identifier)
                docker.command("container", "stop", "--time", "10", identifier)
            protocols.stop_active(self.work, self.state)
            protocols.start_chain(self.work, self.state, protocol_plan)
            if native_restore is not None:
                self.migrate_native(native_restore, legacy=True)
            for role in ("site", "edge"):
                self.phase(role + "-start")
                identifier = self.state["candidateIngress"][role]
                ingress.owned(self.state, identifier, role)
                docker.command("container", "start", identifier)
            self.phase("route-verification")
            for _ in range(20):
                try:
                    ingress.website(domain, 443, body)
                    break
                except (OSError, ssl.SSLError):
                    time.sleep(0.25)
            else:
                raise InstallError("Trusted website TLS verification failed")
            self.check_foreign(self.state["replacement"], replacement=True)
            if verify_native:
                ingress.authentication(self.work, self.state, pins, self.state["replacement"], names, self.save)
                self.state["nativeVerified"] = True
            elif not self.state.get("hasCommitted") or not self.state.get("nativeVerified"):
                raise InstallError("Automatic renewal has no retained native handshake proof")
            self.phase("protocol-start")
            protocols.start_candidates(self.work, self.state)
            protocols.probe(self.work, self.state, pins, protocol_plan, domain)
            if chain_sni:
                for _ in range(20):
                    try:
                        ingress.website(chain_sni, 443, body)
                        break
                    except (OSError, ssl.SSLError):
                        time.sleep(0.25)
                else:
                    raise InstallError("Trusted chain website TLS verification failed")
                if verify_native:
                    ingress.authentication(self.work, self.state, pins, self.state["replacement"],
                        names, self.save, server_name=chain_sni)
            self.phase("acceptance")
            protocols.promote(self.work, self.state)
            for identifier in self.state["candidateIngress"].values():
                docker.command("container", "update", "--restart=always", identifier)
            policy = self.original()["HostConfig"]["RestartPolicy"]
            restart = policy["Name"] + ((":" + str(policy["MaximumRetryCount"])) if policy["Name"] == "on-failure" else "")
            docker.command("container", "update", "--restart", restart, self.state["replacement"])
            self.state.setdefault("retainedIngress", []).append(self.state.get("activeIngress", {}))
            self.state["activeIngress"] = self.state.pop("candidateIngress")
            self.state["activeCertificate"] = self.state["certificate"]
            self.state.pop("pendingCertificate", None)
            self.state["generation"] = self.state.pop("pendingGeneration")
            protocols.accept(self.state)
            self.state.setdefault("originalNativeDigest", self.state["nativeDigest"])
            self.state["nativeDigest"] = self.replacement_native_digest()
            self.state["hasCommitted"], self.state["phase"], self.state["lease"] = True, "committed", 0
            self.state["replacementFingerprint"] = state_fingerprint(self.state, docker.inspect(self.state["replacement"]))
            self.save()
            timer("recovery", False)
            timer("renew", True)
            log("committed", "succeeded")
        except BaseException:
            self.rollback()
            raise


def progress(args, text):
    if not args.quiet:
        print("STEP: " + text, file=sys.stderr)


def _entry_from_artifact(protocol, credential_id, display, artifacts):
    content = artifacts[0]["content"] if artifacts else None
    if isinstance(content, (bytes, bytearray)):
        content = bytes(content).decode("utf-8", "replace")
    return {"credentialId": credential_id, "protocol": protocol, "displayName": display,
        "status": "active", "payload": content,
        "sourceRevision": digest(str(content).encode())}


def protocol_page_entries(protocol_plan, domain, names=None):
    labels = names if isinstance(names, dict) else {}
    entries = []

    def label(credential_id, default):
        value = labels.get(credential_id)
        return value if isinstance(value, str) and value else default

    outline = (protocol_plan or {}).get("outline")
    if outline:
        users = outline.get("users") or [{"name": "guest", "secret": outline.get("secret")}]
        for user in users:
            credential_id = "outline-" + str(user["name"])
            display = label(credential_id, str(user["name"]))
            try:
                artifacts = access_links.encode({"kind": "guest", "protocol": "outline-wss",
                    "credentialId": credential_id,
                    "displayName": display, "host": outline.get("host", domain),
                    "secret": user["secret"], "tcpPath": outline["tcpPath"],
                    "udpPath": outline["udpPath"]})
            except (access_links.IncompatibleExport, KeyError):
                continue
            entries.append(_entry_from_artifact("outline-wss",
                credential_id, display, artifacts))
    awg = (protocol_plan or {}).get("amneziawg")
    if awg:
        for peer in awg.get("peers") or ():
            name = str(peer.get("name") or "guest")
            credential_id = "amneziawg-" + name
            display = label(credential_id, name)
            try:
                artifacts = access_links.encode({"kind": "guest", "protocol": "amneziawg",
                    "credentialId": credential_id, "displayName": display,
                    "host": awg.get("domain", domain), "payload": awg["clients"][name]})
            except (access_links.IncompatibleExport, KeyError):
                continue
            entries.append(_entry_from_artifact("amneziawg", credential_id, display, artifacts))
    mtproto = (protocol_plan or {}).get("mtproto")
    if mtproto and domain:
        try:
            artifacts = access_links.encode({"kind": "guest", "protocol": "mtproto",
                "credentialId": "mtproto", "displayName": label("mtproto", "MTProto"),
                "host": domain, "secret": mtproto["secret"], "port": 443})
            entries.append(_entry_from_artifact("mtproto", "mtproto",
                label("mtproto", "MTProto"), artifacts))
        except (access_links.IncompatibleExport, KeyError):
            pass
    hysteria = (protocol_plan or {}).get("hysteria")
    if hysteria and domain:
        try:
            artifacts = access_links.encode({"kind": "guest", "protocol": "hysteria",
                "credentialId": "hysteria", "displayName": label("hysteria", "Hysteria"),
                "host": hysteria.get("domain", domain), "secret": hysteria["password"],
                "port": hysteria["port"]})
            entries.append(_entry_from_artifact("hysteria", "hysteria",
                label("hysteria", "Hysteria"), artifacts))
        except (access_links.IncompatibleExport, KeyError):
            pass
    return entries


def refresh_client_page(config, work, state, protocol_plan=None):
    try:
        root = document(config, "hton.yaml") or {}
    except InstallError:
        root = {}
    page = configuration.client_page(root)
    if not page.enabled:
        return client_page.refresh(page, (), work)
    stored = work.json("client-page/inventory.json")
    protocol_entries = list(stored) if isinstance(stored, list) else []
    if protocol_plan is not None:
        domain = state.get("siteDomain")
        if domain:
            protocol_entries = protocol_page_entries(protocol_plan, domain, page.names)
            work.save("client-page/inventory.json", protocol_entries)
    identifier = (state.get("container")
        or (state.get("replacement") if state.get("hasCommitted") else state.get("originalId")))
    if not identifier:
        # The refresh timer can be running with legacy state absent.
        try:
            managed = managed_state_record(work)
        except InstallError:
            managed = None
        identifier = (managed or {}).get("container")
    native = []
    if identifier:
        try:
            native = managed_amnezia.client_inventory(docker.native_config(identifier),
                page.names, link_label=configuration.connection_label(root.get("instance")))
        except (InstallError, OSError):
            native = [{"credentialId": "native-amnezia", "protocol": "vless",
                "status": "unavailable", "payload": None}]
    return client_page.refresh(page, protocol_entries + native, work)


def apex_server_names(domain, names):
    """OR-63: new installations use the apex, not protocol subdomains."""
    names = list(names)
    if not names or domain in names:
        return names
    if all(name.endswith("." + domain) for name in names):
        return [domain]
    return names


def reality_certificate_names(domain, names):
    """REALITY names the trusted site certificate must cover.

    Third-party legacy names cannot be issued by our ACME account; the site
    domain is already the primary name. OR-38/OR-44 require the remaining
    REALITY hosts to present a trusted certificate, never a self-signed one.
    """
    return [name for name in names
        if name != domain and name.endswith("." + domain)]


def resolve_site_root(document_root, domain, args):
    """Resolve the website root, asking once about a detected existing site.

    An explicit `core.site.root` is authoritative. Otherwise a detected
    `/var/www/html` or `/var/www` site is adopted after one interactive question
    (default yes); the decision is persisted so later runs never ask again. A
    declined choice stores `core.site.useExisting: false` and serves the
    placeholder. Quiet, JSON and renewal runs never prompt and adopt the detected
    site; with no detected site the placeholder is used.
    """
    core = document_root.setdefault("core", {}) if isinstance(document_root, dict) else {}
    if not isinstance(core, dict):
        raise InstallError("core must be a mapping")
    site = core.setdefault("site", {})
    if not isinstance(site, dict):
        raise InstallError("core.site must be a mapping")
    configured = site.get("root")
    if configured not in (None, ""):
        return ingress.site_root(configured)
    if site.get("useExisting") is False:
        return None
    detected = ingress.site_root(None)
    if detected is None:
        return None
    adopt = True
    if interactive_install(args) and not getattr(args, "yes", False):
        answer = prompt("Use the existing website at " + str(detected)
            + " as the default site on " + domain + "? [Y/n]", "Y")
        adopt = answer.strip().lower() in ("", "y", "yes")
    if adopt:
        site["root"] = str(detected)
        return detected
    site["useExisting"] = False
    return None


_CLIENT_PAGE_BLOCK = re.compile(r"^(\s*)clientAccessPage\s*:\s*(?:#.*)?$")
_PROFILE_PATH_LINE = re.compile(r"^(\s*)path\s*:.*$")
_SAFE_SHARE_NAME = re.compile(r"^\w[\w.-]{0,127}\Z")


def ensure_client_page_path(root, args, config):
    """Enable automatic publication unless the profile explicitly opted out.

    A present nonempty ``path`` is authoritative. A present empty ``path`` is a
    deliberate opt-out. An absent key means first-install auto-publication, so a
    fresh profile receives one CSPRNG segment. ``--rotate-client-page`` replaces
    an existing path without prompting.
    """
    if not isinstance(root, dict):
        return False
    core = root.setdefault("core", {})
    if not isinstance(core, dict):
        return False
    page = core.get("clientAccessPage")
    if page is None:
        page = {}
    if not isinstance(page, dict):
        return False
    rotate = bool(getattr(args, "rotate_client_page", False))
    current = page.get("path")
    if "path" in page and not isinstance(current, str):
        return False
    if isinstance(current, str) and current and not rotate:
        return False
    if not rotate and "path" in page and current == "":
        return False
    if interactive_install(args) and not getattr(args, "yes", False):
        answer = prompt("Publish the client credentials at a generated secret URL? [Y/n]", "Y")
        if answer.strip().lower() in ("n", "no"):
            page["path"] = ""
            core["clientAccessPage"] = page
            config.save("hton.yaml", root)
            return True
    page["path"] = client_page.generate_path()
    page.setdefault("names", {})
    page.setdefault("refreshSeconds", 60)
    core["clientAccessPage"] = page
    config.save("hton.yaml", root)
    return True


def _share_markdown(title, payload):
    return ("# " + str(title) + "\n\n```\n" + str(payload).strip() + "\n```\n").encode()


def _share_section(title, payload, fence=""):
    return ("## " + str(title) + "\n\n```" + fence + "\n"
        + str(payload).strip() + "\n```\n\n")


def secret_share_artifacts(protocol_plan, domain, native_entries=(), page_path="", names=None):
    """One private Markdown bundle per user with every credential they hold."""
    labels = names if isinstance(names, dict) else {}
    users = {}
    order = []
    shared = []

    def display(credential_id, fallback):
        value = labels.get(credential_id)
        return value if isinstance(value, str) and value else fallback

    def user(key, label):
        if key not in users:
            users[key] = {"label": label, "sections": []}
            order.append(key)
        return users[key]

    for entry in native_entries or ():
        if not isinstance(entry, dict) or entry.get("status") != "active":
            continue
        payload = entry.get("payload")
        if not isinstance(payload, str) or not payload:
            continue
        credential_id = str(entry.get("credentialId") or "guest")
        label = display(credential_id, str(entry.get("displayName") or credential_id))
        user(credential_id, label)["sections"].append(("Amnezia REALITY", payload, ""))
    for entry in protocol_page_entries(protocol_plan, domain, labels):
        payload = entry.get("payload")
        if not isinstance(payload, str) or not payload:
            continue
        protocol = entry.get("protocol")
        credential_id = str(entry.get("credentialId") or "guest")
        if protocol == "outline-wss":
            identity = credential_id[len("outline-"):] if credential_id.startswith("outline-") else credential_id
            user(identity, display(credential_id, identity))["sections"].append(
                ("Outline", payload, "yaml"))
        elif protocol == "amneziawg":
            identity = credential_id[len("amneziawg-"):] if credential_id.startswith("amneziawg-") else credential_id
            user(identity, display(credential_id, identity))["sections"].append(
                ("AmneziaWG", payload, ""))
        elif protocol == "mtproto":
            shared.append(("MTProto", payload, ""))
        elif protocol == "hysteria":
            shared.append(("Hysteria", payload, ""))

    page = ""
    artifacts = {}
    if page_path and domain:
        page = ("## Client access page\n\nhttps://" + domain + "/" + page_path
            + "\n\nThis URL is a bearer secret. Keep this repository private.\n")
        artifacts["client-page.md"] = ("# Client access page\n\nhttps://" + domain + "/"
            + page_path + "\n\nThis URL is a bearer secret. Keep this repository private.\n").encode()

    for key in order:
        value = users[key]
        sections = value["sections"] + shared
        if not sections:
            continue
        text = "# " + value["label"] + "\n\n"
        for section in sections:
            text += _share_section(section[0], section[1], section[2])
        if page:
            text += page
        artifacts[clients.share_name(key)] = text.encode()
    if not order and shared:
        text = "# Server credentials\n\n"
        for section in shared:
            text += _share_section(section[0], section[1], section[2])
        if page:
            text += page
        artifacts["server.md"] = text.encode()
    return artifacts


def _install_source_root():
    raw = os.environ.get("HTON_INSTALL_SOURCE")
    if not raw:
        return SOURCE
    candidate = Path(raw)
    if not candidate.is_absolute() or candidate.is_symlink() or not candidate.is_dir():
        raise InstallError("Installed source root is invalid")
    return candidate.resolve()


def _secret_share_destinations():
    runtime = ROOT / ".secrets"
    destinations = []
    if runtime.is_symlink():
        raise InstallError("The private secrets directory is a symlink")
    if not runtime.exists():
        runtime.mkdir(mode=0o750)
        gid = storage.group_id()
        if gid is not None:
            os.chown(runtime, -1, gid)
    destinations.append(runtime)
    source = _install_source_root() / ".secrets"
    if not source.is_symlink() and source.is_dir():
        if source.resolve() != runtime.resolve():
            destinations.append(source)
    return destinations


def _write_share_tree(base, artifacts, stale):
    if base.is_symlink() or not base.is_dir():
        raise InstallError("The private secrets directory is unsafe")
    info = base.stat()
    directory = base / "share"
    if directory.is_symlink():
        raise InstallError("The private share directory is a symlink")
    if not directory.exists():
        directory.mkdir(mode=0o750)
    elif not directory.is_dir():
        raise InstallError("The private share path is not a directory")
    os.chown(directory, info.st_uid, info.st_gid)
    os.chmod(directory, 0o750)
    directory_fd = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
        | os.O_CLOEXEC)
    try:
        present = set(os.listdir(directory_fd))
        legacy = {name for name in present
            if (name.startswith("amnezia-") and name.endswith(".md"))
            or (name.startswith("outline-") and name.endswith(".yaml"))
            or name in ("mtproto.md", "hysteria.md")}
        for name in sorted((set(stale) | legacy) - set(artifacts)):
            if not isinstance(name, str) or not _SAFE_SHARE_NAME.match(name):
                continue
            try:
                os.unlink(name, dir_fd=directory_fd)
            except FileNotFoundError:
                pass
        for name, data in sorted(artifacts.items()):
            if not _SAFE_SHARE_NAME.match(name):
                raise InstallError("A generated share artifact name is invalid")
            storage.write_share_file(directory_fd, name, data, info.st_uid, info.st_gid)
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)


def _updated_profile_text(text, page_path):
    lines = text.splitlines()
    output = []
    in_page = False
    indent = 0
    inserted = False
    for line in lines:
        if not in_page:
            match = _CLIENT_PAGE_BLOCK.match(line)
            output.append(line)
            if match:
                in_page = True
                indent = len(match.group(1))
            continue
        stripped = line.strip()
        current_indent = len(line) - len(line.lstrip())
        if stripped and current_indent <= indent:
            if not inserted:
                output.append(" " * (indent + 2) + "path: " + json.dumps(page_path))
                inserted = True
            in_page = False
            output.append(line)
            continue
        match = _PROFILE_PATH_LINE.match(line)
        if match and current_indent == indent + 2:
            output.append(" " * (indent + 2) + "path: " + json.dumps(page_path))
            inserted = True
            continue
        output.append(line)
    if in_page and not inserted:
        output.append(" " * (indent + 2) + "path: " + json.dumps(page_path))
        inserted = True
    if not inserted:
        return None
    return "\n".join(output) + ("\n" if text.endswith("\n") else "")


def _write_preserving_owner(path, data):
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
        raise InstallError("A private profile file is not regular")
    temporary = path.parent / (".new-" + secrets.token_hex(12))
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW
        | os.O_CLOEXEC, 0o600)
    try:
        view = memoryview(data)
        while view:
            view = view[os.write(fd, view):]
        os.fsync(fd)
        os.fchown(fd, info.st_uid, info.st_gid)
        os.fchmod(fd, stat.S_IMODE(info.st_mode))
    finally:
        os.close(fd)
    try:
        os.replace(temporary, path)
    except BaseException:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise


def _update_profile_client_page(base, page_path):
    profile = base / "profile.yml"
    if profile.is_symlink() or not profile.is_file():
        return
    original = profile.read_bytes().decode("utf-8")
    updated = _updated_profile_text(original, page_path)
    if updated is not None and updated != original:
        _write_preserving_owner(profile, updated.encode())


def publish_secret_share_artifacts(config, work, plan_value, domain, container, page_path=""):
    """Persist per-user guest artifacts and the bearer URL in private .secrets/share."""
    root = (document(config, "hton.yaml") if config is not None else None) or {}
    page = (root.get("core") or {}).get("clientAccessPage") or {}
    names = page.get("names") if isinstance(page, dict) else {}
    native = []
    link_label = configuration.connection_label(root.get("instance"))
    if container:
        try:
            native = managed_amnezia.client_inventory(docker.native_config(container), names,
                link_label=link_label)
        except (InstallError, OSError):
            native = []
    artifacts = secret_share_artifacts(plan_value, domain, native, page_path, names)
    previous = work.json("secret-share.json")
    stale = set(previous.get("files") or ()) if isinstance(previous, dict) else set()
    destinations = _secret_share_destinations()
    for base in destinations:
        _write_share_tree(base, artifacts, stale - set(artifacts))
        _update_profile_client_page(base, page_path)
    work.save("secret-share.json", {"files": sorted(artifacts)})


def native_share_links(work, container, link_label=None):
    """Guest REALITY links and their protected share files for the native listener."""
    if not container:
        return {}, {}
    try:
        entries = managed_amnezia.client_inventory(docker.native_config(container),
            link_label=link_label)
    except (InstallError, OSError):
        return {}, {}
    exportable = [entry for entry in entries if entry.get("payload")]
    links, shares = {}, {}
    for entry in exportable:
        name = str(entry.get("credentialId") or "guest")
        slug = clients.slug(name)
        label = "Amnezia" if len(exportable) == 1 else "Amnezia " + str(entry.get("displayName") or name)
        relative = "share/amnezia-" + slug + ".txt"
        links[label] = entry["payload"]
        work.write(relative, (entry["payload"] + "\n").encode())
        shares[label] = str(work.root / relative)
    return links, shares


def ready_result(domain, port, challenge, native_state, publication_result, plan, work,
                 native_links=None, native_shares=None):
    protocols_value = protocols.result(plan)
    if native_links:
        protocols_value["amnezia"] = {"listener": "reality/tcp/443", "guestLinks": len(native_links)}
    result = {"status": "ready", "ready": True, "site": domain,
        "amneziaBackend": "127.0.0.1:" + str(port), "certificate": "trusted",
        "nativeAuthentication": native_state,
        "externalDns": "matched" if publication_result else "unverified",
        "certificateChallenge": challenge, "protocols": protocols_value}
    shares = protocols.save_shares(work, plan, domain)
    shares.update(native_shares or {})
    if shares:
        result["shareFiles"] = shares
    return result


def normalize_runtime_permissions(work, config=None):
    """Make runtime state readable by the operator group after every install."""
    targets = [work.root]
    if config is not None:
        targets.append(config.root)
    parents = work.root.parents
    if len(parents) > 1:
        targets.extend([parents[1] / "work", parents[1] / ".secrets",
            parents[1] / "host-access"])
    seen = set()
    for target in targets:
        key = str(target)
        if key in seen:
            continue
        seen.add(key)
        storage.adopt_tree(target)


def emit_ready(args, domain, port, challenge, native_state, publication_result, plan, work, container,
               config=None):
    """Emit the ready summary: site, client page URL and the share directory."""
    normalize_runtime_permissions(work, config)
    root = document(config, "hton.yaml") if config is not None else None
    link_label = configuration.connection_label((root or {}).get("instance"))
    native_links, native_shares = native_share_links(work, container, link_label)
    page_url = None
    share_directory = None
    if config is not None and getattr(args, "action", "install") in ("install", "update"):
        root = root or {}
        page = (root.get("core") or {}).get("clientAccessPage") or {}
        page_path = page.get("path") if isinstance(page, dict) else ""
        page_path = page_path if isinstance(page_path, str) else ""
        publish_secret_share_artifacts(config, work, plan, domain, container, page_path)
        if page_path:
            page_url = "https://" + domain + "/" + page_path
        share_directory = str(ROOT / ".secrets/share")
    if getattr(args, "action", "install") in ("install", "update"):
        try:
            updates.record(work, getattr(args, "action", "install"))
        except (OSError, InstallError):
            # The schedule stamp is advisory; a write failure must not fail a
            # working installation. The next run treats a missing stamp as due.
            pass
    emit(args, ready_result(domain, port, challenge, native_state, publication_result, plan, work,
        native_links=native_links, native_shares=native_shares),
        page_url=page_url, share_directory=share_directory)


def emit(args, result, page_url=None, share_directory=None):
    if args.quiet: return
    if args.json:
        print(json.dumps(result))
    elif result.get("ready"):
        print("OK: Setup complete.")
        print("Website: https://" + result["site"])
        if page_url:
            print("Client access page: " + page_url)
        if share_directory:
            mode = ("0640; readable by the hton group" if storage.group_id() is not None
                else "0600; root-only (no hton group)")
            print("Client files: " + share_directory + " (" + mode + ").")
        print("HTTPS certificate: trusted. Renewal: automatic.")
        print("VPN: ready.")
        for label, value in (result.get("protocols") or {}).items():
            if label == "modules" and value:
                print("Protocols: " + ", ".join(value) + ".")
            elif label == "mtproto":
                print("MTProto: listening for camouflage " + value["camouflage"] + ".")
            elif label == "hysteria":
                print("Hysteria: UDP " + str(value["port"]) + ".")
            elif label == "outline":
                print("Outline: own WSS backend on loopback " + str(value["webPort"]) + ".")
            elif label == "amnezia":
                print("Amnezia: REALITY on TCP 443 (guest links: " + str(value["guestLinks"]) + ").")
    else:
        print(json.dumps(result))


def managed_state_record(work):
    value = work.json("managed.json")
    if value is None:
        return None
    if not isinstance(value, dict) or value.get("schemaVersion") != 1 or value.get("kind") != "managed":
        raise InstallError("Unsupported managed state; original retained")
    for field, size in (("instance", 32), ("operation", 32)):
        text = value.get(field)
        if not isinstance(text, str) or len(text) != size or any(c not in "0123456789abcdef" for c in text):
            raise InstallError("Managed state identity is invalid; original retained")
    if type(value.get("port")) is not int or not 5000 <= value["port"] <= 6000:
        raise InstallError("Saved managed port is invalid; no reroll performed")
    if value.get("phase") not in ("prepared", "starting", "committed", "rolled_back"):
        raise InstallError("Saved managed phase is invalid; original retained")
    return value


def rollback_managed(work, state):
    for role, identifier in list((state.get("candidateIngress") or {}).items()):
        try:
            ingress.owned(state, identifier, role, work)
            docker.command("container", "update", "--restart=no", identifier)
            if docker.inspect(identifier)["State"]["Running"]:
                docker.command("container", "stop", "--time", "10", identifier)
        except InstallError:
            pass
    protocols.rollback_candidates(work, state)
    previous = state.get("previousNative")
    if previous:
        candidate = state.get("container")
        if candidate and candidate != previous["id"]:
            try:
                docker.command("container", "rm", "--force", docker.exact_id(candidate))
            except docker.NativeError as error:
                if error.reason != "The container no longer exists.":
                    raise
        identifier = docker.exact_id(previous["id"])
        if state.get("suspendedNative"):
            docker.command("container", "update", "--restart=" + previous["restart"], identifier)
            if previous["running"] and not docker.inspect(identifier)["State"]["Running"]:
                docker.command("container", "start", identifier)
        state.update({"container": identifier, "imageDigest": previous["imageDigest"],
            "configDigest": previous["configDigest"], "port": previous["port"]})
        state.pop("previousNative")
        state.pop("suspendedNative", None)
        state.pop("resumeNative", None)
    elif state.get("container"):
        try:
            running = docker.inspect(state["container"])["State"]["Running"]
            resume = state.pop("resumeNative", False)
            if running and not resume:
                docker.command("container", "update", "--restart=no", state["container"])
                docker.command("container", "stop", "--time", "10", state["container"])
            elif resume and not running:
                docker.command("container", "start", state["container"])
        except InstallError:
            pass
    for identifier, role in list((state.get("suspendedIngress") or {}).items()):
        try:
            ingress.owned(state, identifier, role, work)
            docker.command("container", "update", "--restart=always", identifier)
            if not docker.inspect(identifier)["State"]["Running"]:
                docker.command("container", "start", identifier)
        except InstallError:
            pass
    state.pop("suspendedIngress", None)
    try:
        protocols.restore_active(work, state)
    except InstallError:
        pass
    state["phase"] = "committed" if state.get("generation") and state.get("activeIngress") else "rolled_back"
    state["lease"] = 0
    state.pop("actor", None)
    work.save("managed.json", state)


def run_managed(config, work, root, module_doc, args, values=None, pins_override=None):
    """Dedicated journaled transaction for a fresh managed REALITY container."""
    settings = (module_doc or {}).get("settings") or {}
    foreign = {
        "mode": settings.get("mode", "associate"),
        "container": settings.get("container"),
        "server_names": tuple(settings.get("serverNames") or ()),
        "legacy_server_names": tuple((settings.get("compatibility") or {}).get("legacyServerNames") or ()),
        "host_ports": settings.get("hostPorts") or {},
        "client_name": (settings.get("provision") or {}).get("clientName"),
        "admin": settings.get("admin") or {},
    }
    if work is None:
        parent = Store(ROOT / "work", create=True)
        parent.write("minimum/owner", b"hton-minimum-v1\n")
        work = Store(ROOT / "work/minimum")
    config = config or Store(ROOT / "config", create=True)
    state = managed_state_record(work)
    if state is None:
        domain, email = load_site_inputs(root, args)
        state = {"schemaVersion": 1, "kind": "managed", "instance": secrets.token_hex(16),
            "operation": secrets.token_hex(16), "container": None, "imageDigest": None,
            "configDigest": None, "port": None, "phase": "prepared", "lease": 0,
            "sitePort": 9443, "protocols": {}, "siteDomain": domain, "certificateEmail": email}
    state.setdefault("instance", secrets.token_hex(16))
    state.setdefault("ownedReceipts", {})
    domain = state["siteDomain"]
    email = state["certificateEmail"]
    test_tunnel = configure_test_tunnel(root, args, domain)
    enabled = protocols.selected(root, args.with_features, args.without_features, state=state)
    if pins_override is not None:
        pins = dict(pins_override)
    else:
        lock = image_lock()
        roles = ["haproxy", "caddy", "acme", "xray"]
        if managed_amnezia.IMAGE_ROLE in lock.get("images", {}):
            roles.append(managed_amnezia.IMAGE_ROLE)
        for protocol, image_role in (("mtproto", "mtg"), ("hysteria", "hysteria"),
                ("outline-wss", "alpine"), ("amneziawg", "amneziawg")):
            if protocol in enabled:
                roles.append(image_role)
        pins = resolve_pins(lock, roles, allow_cached=args.action == "install")

    def save():
        work.save("managed.json", state)

    interactive = interactive_install(args)
    if interactive:
        apex = apex_server_names(domain, foreign["server_names"])
        if apex != list(foreign["server_names"]):
            # OR-63: drop protocol subdomains only on an interactive install;
            # renewal/update keep the accepted names.
            foreign["server_names"] = tuple(apex)
            if not getattr(args, "quiet", False) and not getattr(args, "json", False):
                print("REALITY SNI: " + domain + " (subdomain names dropped; "
                    "client links must be reissued).", file=sys.stderr)

    plan = managed_amnezia.plan(foreign, pins=pins)
    existing = state.get("container")
    live_material = None
    replacing = False
    recreate = False
    if existing:
        current = None
        native_bytes = None
        try:
            current = docker.inspect(existing)
            entry = (current.get("Config") or {}).get("Entrypoint")
            if entry is not None and entry != [plan.binary]:
                recreate = True
            image_id = current.get("Image")
            if image_id:
                expected_id = plan.image if plan.image.startswith("sha256:") else local_image_id(plan.image)
                if image_id != expected_id:
                    recreate = True
            if domain:
                hosts = current.get("HostConfig", {}).get("ExtraHosts") or []
                if not any(str(item).startswith(domain + ":") for item in hosts):
                    recreate = True
                try:
                    native_bytes = docker.native_config(existing)
                    native = json.loads(native_bytes)
                    reality = native["inbounds"][0]["streamSettings"]["realitySettings"]
                    if reality.get("dest") != domain + ":" + str(state["sitePort"]):
                        recreate = True
                    if list(reality.get("serverNames") or []) != list(plan.server_names):
                        recreate = True
                    if reality.get("minClientVer") != "1.0.0":
                        recreate = True
                except docker.NativeError as error:
                    if error.reason not in ("A required file or folder is missing.",
                            "The container no longer exists."):
                        raise
                    recreate = True
                except (ValueError, KeyError, IndexError, TypeError) as error:
                    raise InstallError("Managed native configuration is unreadable; existing service retained") from error
        except docker.NativeError as error:
            if error.reason != "The container no longer exists.":
                raise
            recreate = True
        if recreate:
            if native_bytes is not None:
                live_material = managed_amnezia.native_material(native_bytes)
            else:
                live_material = managed_amnezia.saved_material(work)
                if live_material is None:
                    raise InstallError("Managed recreation requires retained native key and client material")
            if current is not None:
                restart = (current["HostConfig"].get("RestartPolicy") or {}).get("Name", "no")
                if restart not in ("no", "always"):
                    raise InstallError("Managed native restart policy is outside its admitted scope")
                state["previousNative"] = {"id": existing,
                    "running": current["State"]["Running"], "restart": restart,
                    "imageDigest": state.get("imageDigest"), "configDigest": state.get("configDigest"),
                    "port": state["port"],
                    "liveDigest": digest(native_bytes) if native_bytes is not None else None}
                replacing = True
    if not existing or recreate:
        try:
            receipt = managed_amnezia.prepare(plan, work, site_port=state["sitePort"],
                domain=domain, material=live_material)
        except BaseException:
            state.pop("previousNative", None)
            raise
        state.update({"container": receipt["replacement"],
            "imageDigest": digest(plan.image.encode()), "configDigest": receipt["configDigest"],
            "port": plan.host_port, "phase": "prepared"})
        save()
    container = state["container"]
    try:
        state["resumeNative"] = bool(docker.inspect(container)["State"]["Running"])
    except InstallError:
        state["resumeNative"] = False
    save()
    try:
        if replacing:
            bridge_gateway = docker.bridge_gateway()
        else:
            docker.command("container", "start", container)
            value = docker.inspect(container)
            docker.qualify(value, plan.host_port, allow_image_volumes=plan.official, binary=plan.binary)
            bridge_gateway = docker.native_bridge_gateway(value)
        values = values if values is not None else docker.inventory()

        ensure_client_page_path(root, args, config)
        certificate_defaults = load_settings(SOURCE).get("core", {}).get("certificates", {})
        challenge = certificates.select_policy(root, state, config, args.dns_credentials,
            defaults=certificate_defaults)
        certificate_settings = root.setdefault("core", {}).setdefault("certificates", {})
        certificate_settings.update({"ca": "letsencrypt", "challenge": challenge})
        if challenge == "http-01":
            certificate_settings.pop("provider", None)
        else:
            certificate_settings.setdefault("provider", "internetbs")
        config.save("hton.yaml", root)
        reality_names = reality_certificate_names(domain, plan.server_names)
        request_names = ["www." + domain, *reality_names]
        wildcard = (challenge == "dns-01"
            and certificate_settings.get("wildcardDomain") == domain)
        certificate = certificates.ensure(config, work, state, pins, domain, email,
            args.dns_credentials, save, interactive=interactive,
            challenge=challenge, http_settings=certificate_settings.get("http", {}),
            names=list(request_names), wildcard=wildcard)

        configured_site = resolve_site_root(root, domain, args)
        body = ingress.webroot(str(configured_site) if configured_site is not None else None)
        native_server = json.loads(docker.native_config(container))
        mtg_port = protocols.mtproto_port(state, values) if "mtproto" in enabled else None
        protocol_plan = protocols.plan(config, work, state, pins, domain, list(plan.server_names),
            enabled, args, values, ask=prompt if interactive else (lambda _label, default: default),
            confirm=lambda label: interactive and prompt(label, "N").lower() in ("y", "yes"),
            native_server=native_server, chain_sni=None, site_port=state["sitePort"],
            migrate_native=False, bridge_gateway=bridge_gateway, mtg_port=mtg_port)
        validate_test_tunnel(test_tunnel, [
            domain, "www." + domain, *plan.server_names,
            *([protocol_plan["mtproto"]["camouflage"]] if protocol_plan.get("mtproto") else []),
        ])
        protocols.record_state(state, protocol_plan)
        save()

        settings_doc = module_doc if isinstance(module_doc, dict) else {}
        settings_doc.setdefault("settings", {}).update({
            "container": plan.container, "serverNames": list(plan.server_names),
            "hostPorts": {"tcp": plan.host_port, "udp": plan.host_port}, "mode": "provision"})
        config.save("modules/foreign-amnezia.yaml", settings_doc)
        root.setdefault("schemaVersion", 1)
        root.setdefault("instance", {"name": "hton"})
        root.setdefault("installation", {}).update({"systemd": True})
        root.setdefault("installation", {}).setdefault("pathShim", False)
        core = root.setdefault("core", {})
        core.setdefault("edge", {}).setdefault("listen", {}).update(
            {"transport": "tcp", "port": 443, "scope": "public", "addressFamilies": ["ipv4"]})
        core["edge"]["domains"] = [domain]
        root.setdefault("features", {})["foreign-amnezia"] = {"enabled": True}
        protocols.save_config(config, root, protocol_plan)
        config.save("hton.yaml", root)

        page_value = core.get("clientAccessPage") or {}
        page_enabled = isinstance(page_value.get("path"), str) and bool(page_value.get("path"))
        page_refresh = page_value.get("refreshSeconds", 60) if isinstance(page_value, dict) else 60
        protocols.validate(work, state, pins, protocol_plan, certificate)
        protocols.prepare(work, state, pins, protocol_plan, certificate, save)
        page_snapshot = refresh_client_page(config, work, state, protocol_plan)
        client_page_arg = ({"path": page_snapshot["path"], "headers": page_snapshot["headers"]}
            if page_snapshot.get("enabled") else None)
        # A REALITY name equal to the site domain nests the listener behind
        # HAProxy: the site is served by the native fallback to Caddy.
        native_selfsteal = domain in plan.server_names
        ingress.prepare(work, state, pins, certificate, domain, list(plan.server_names),
            plan.host_port, state["sitePort"], body, save,
            mtproto=protocol_plan["mtproto"], outline=protocol_plan["outline"],
            selfsteal=protocol_plan["selfsteal"], chain_sni=None,
            native_selfsteal=native_selfsteal,
            site_bind=bridge_gateway, client_page=client_page_arg,
            reality_names=tuple(reality_names),
            site_document_root=str(configured_site) if configured_site is not None else None,
            test_tunnel=test_tunnel)
    except BaseException:
        rollback_managed(work, state)
        raise

    suspended = {}
    for entry in [state.get("activeIngress") or {}, *(state.get("retainedIngress") or [])]:
        if not isinstance(entry, dict):
            continue
        for role, identifier in entry.items():
            if role in ("site", "edge") and isinstance(identifier, str) and identifier:
                suspended[identifier] = role
    state["suspendedIngress"] = suspended
    state["phase"] = "starting"
    state["lease"] = int(time.time()) + 180
    state["leaseBoot"] = boot_id()
    state["leaseMonotonic"] = time.monotonic() + 180
    if os.getpgrp() != os.getpid():
        os.setsid()
    state["actor"] = process_identity(os.getpid())
    save()
    try:
        for identifier, role in suspended.items():
            ingress.owned(state, identifier, role, work)
            docker.command("container", "update", "--restart=no", identifier)
            if docker.inspect(identifier)["State"]["Running"]:
                docker.command("container", "stop", "--time", "10", identifier)
        protocols.stop_active(work, state)
    except BaseException:
        rollback_managed(work, state)
        raise

    try:
        if replacing:
            previous = state["previousNative"]
            previous_id = docker.exact_id(previous["id"])
            if previous["liveDigest"] is not None and digest(docker.native_config(previous_id)) != previous["liveDigest"]:
                raise InstallError("Managed native clients changed during replacement preparation")
            state["suspendedNative"] = True
            save()
            docker.command("container", "update", "--restart=no", previous_id)
            if docker.inspect(previous_id)["State"]["Running"]:
                docker.command("container", "stop", "--time", "10", previous_id)
        docker.command("container", "start", container)
        if replacing:
            value = docker.inspect(container)
            docker.qualify(value, plan.host_port, allow_image_volumes=plan.official, binary=plan.binary)
            if docker.native_bridge_gateway(value) != bridge_gateway:
                raise InstallError("Managed native bridge gateway changed during preparation")
        protocols.start_chain(work, state, protocol_plan)
        protocols.start_candidates(work, state)
        for role in ("site", "edge"):
            identifier = state["candidateIngress"][role]
            ingress.owned(state, identifier, role)
            docker.command("container", "start", identifier)
        for _ in range(20):
            try:
                ingress.website(domain, 443, body)
                break
            except (OSError, ssl.SSLError):
                time.sleep(0.25)
        else:
            raise InstallError("Managed trusted website TLS verification failed")
        ingress.authentication(work, state, pins, container, list(plan.server_names), save)
        protocols.probe(work, state, pins, protocol_plan, domain)
        protocols.promote(work, state)
        for identifier in state["candidateIngress"].values():
            docker.command("container", "update", "--restart=always", identifier)
        docker.command("container", "update", "--restart=always", container)
        accepted = copy.deepcopy(state)
        accepted["activeIngress"] = accepted.pop("candidateIngress")
        retired_ingress = accepted.pop("suspendedIngress", {})
        accepted["retainedIngress"] = [{role: identifier} for identifier, role in retired_ingress.items()]
        accepted["activeCertificate"] = accepted.get("certificate")
        protocols.accept(accepted)
        accepted.pop("resumeNative", None)
        accepted.pop("suspendedNative", None)
        previous = accepted.pop("previousNative", None)
        if previous:
            accepted.setdefault("retiredNative", []).append(previous["id"])
        accepted["generation"] = digest(body + (certificate / "fullchain.pem").read_bytes()
            + json.dumps(pins, sort_keys=True).encode() + protocol_plan["generation"].encode()
            + json.dumps(test_tunnel, sort_keys=True).encode()
            + (str(page_value.get("path", "")) if page_enabled else "").encode())
        accepted.update(phase="committed", lease=0, protocolsChosen=True)
        accepted.pop("actor", None)
        work.save("managed.json", accepted)
    except BaseException:
        rollback_managed(work, state)
        raise
    state = accepted
    protocols.cleanup(work, state, save)
    for entry in state["retainedIngress"][:]:
        role, identifier = next(iter(entry.items()))
        try:
            ingress.owned(state, identifier, role, work)
            docker.command("container", "rm", "--force", identifier)
        except InstallError:
            continue
        state["ownedReceipts"].pop(identifier, None)
        state["retainedIngress"].remove(entry)
    for identifier in state.get("retiredNative", [])[:]:
        try:
            docker.command("container", "rm", docker.exact_id(identifier))
        except docker.NativeError as error:
            if error.reason != "The container no longer exists.":
                continue
        state["retiredNative"].remove(identifier)
    save()

    install_units(work, client_page_enabled=page_enabled, refresh_seconds=page_refresh)
    timer("recovery", True)
    timer("renew", True)
    timer("usage", True)
    timer("update", True)
    if page_enabled:
        timer("client-page", True)
    refresh_client_page(config, work, state, protocol_plan)
    emit_ready(args, domain, plan.host_port, challenge, "verified", None, protocol_plan, work, container,
        config)
    return 0


def ensure_secrets(args):
    import importlib.util
    path = SOURCE / "lib/secrets_store.py"
    if not path.is_file():
        raise InstallError("Secrets profile helper is not delivered")
    spec = importlib.util.spec_from_file_location("secrets_store", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.ensure(ROOT, interactive=interactive_install(args))


def main(argv=None):
    parser = argparse.ArgumentParser(description="Install the minimum trusted-site and Amnezia routing profile")
    parser.add_argument("action", nargs="?", choices=("install", "recover", "renew", "update", "client-page-refresh"), default="install")
    parser.add_argument("--site-domain")
    parser.add_argument("--certificate-email")
    parser.add_argument("--amnezia-container")
    parser.add_argument("--with", dest="with_features")
    parser.add_argument("--without", dest="without_features")
    parser.add_argument("--mtproto-camouflage")
    parser.add_argument("--outline-binary")
    parser.add_argument("--chain-sni")
    parser.add_argument("--hysteria-port", type=int)
    parser.add_argument("--dns-credentials")
    parser.add_argument("--test-sni")
    parser.add_argument("--test-tunnel-port", type=int)
    parser.add_argument("--no-test-tunnel", action="store_true")
    parser.add_argument("--rotate-client-page", action="store_true")
    parser.add_argument("--yes", action="store_true")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--quiet", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--auto", action="store_true")
    args = parser.parse_args(argv)
    if args.auto and args.action != "update":
        parser.error("--auto is only valid for the update action")
    if args.dry_run:
        print(json.dumps({"status": "preview", "operation": "install.minimum", "writes": False,
            "scope": ["trusted-site", "persisted-loopback-amnezia-remap",
                "exact-SNI-routing", "optional-mtproto-hysteria-outline-amneziawg"],
            "siteDomain": args.site_domain, "nativeResource": "unverified", "certificate": "unverified", "ready": False}))
        return 0
    if os.geteuid() != 0:
        raise InstallError("Minimum installation requires local sudo")
    if not (ROOT / ".hton-source-deploy-managed").is_file():
        raise InstallError("Minimum Core must run from the active protected /opt/hton deployment")
    if SOURCE != ROOT:
        raise InstallError("Minimum Core must run from the exact protected /opt/hton deployment")
    if (ROOT / "work/registry.json").exists():
        raise InstallError("An existing full-R1 registry requires its own migration; minimum install will not overwrite it")
    if args.action == "update":
        if not (ROOT / "work/minimum").exists():
            raise InstallError("No installed minimum profile to update; run scripts/install first")
        if args.auto:
            args.quiet = True
            schedule = Store(ROOT / "work/minimum")
            try:
                ready = updates.due(schedule)
            finally:
                schedule.close()
            if not ready:
                if args.json:
                    print(json.dumps({"status": "recent", "operation": "update.minimum", "writes": False}))
                elif not args.quiet:
                    print("Recent install/update; automatic update is not due.", file=sys.stderr)
                return 0
    log("started", "begin")
    if args.action == "client-page-refresh":
        if not (ROOT / "work/minimum").exists() or not (ROOT / "config").exists():
            return 0
        config = Store(ROOT / "config")
        work = Store(ROOT / "work/minimum")
        try:
            state = state_record(work) or {}
            refresh_client_page(config, work, state)
        finally:
            config.close()
            work.close()
        return 0
    if args.action == "recover":
        if not (ROOT / "work/minimum").exists():
            return 0
        config = Store(ROOT / "config")
        work = Store(ROOT / "work/minimum")
        state = state_record(work)
        if state and lease_expired(state):
            terminate_expired_actor(state)
            with work.lock("effect.lock"):
                state = state_record(work)
                if state and lease_expired(state):
                    Installation(config, work, state).rollback()
        elif state is None:
            managed = managed_state_record(work)
            if managed and lease_expired(managed):
                terminate_expired_actor(managed)
                with installation_lock():
                    managed = managed_state_record(work)
                    if managed and lease_expired(managed):
                        rollback_managed(work, managed)
        return 0
    config = Store(ROOT / "config") if (ROOT / "config").exists() else None
    work = Store(ROOT / "work/minimum") if (ROOT / "work/minimum").exists() else None
    if work and work.read("owner") != b"hton-minimum-v1\n":
        raise InstallError("Minimum state ownership differs; refusing adoption")
    if args.action in ("install", "renew", "update"):
        ensure_secrets(args)
        config = Store(ROOT / "config") if (ROOT / "config").exists() else config
    if work:
        normalize_runtime_permissions(work, config)
    early_module = document(config, "modules/foreign-amnezia.yaml") if config else None
    managed_mode = (isinstance(early_module, dict)
        and isinstance(early_module.get("settings"), dict)
        and early_module["settings"].get("mode") == "provision")
    with installation_lock():
        state = state_record(work) if work else None
        managed_state = managed_state_record(work) if work else None
        if managed_mode and state is None:
            if managed_state and (managed_state.get("lease") or managed_state.get("previousNative")):
                rollback_managed(work, managed_state)
                raise InstallError("Interrupted managed cutover was restored; repeat installation to resume")
            if args.action in ("renew", "update") and not (managed_state and managed_state.get("phase") == "committed"):
                raise InstallError("Automatic renewal/update requires a committed managed installation")
            if docker.run(["/usr/bin/systemctl", "--system", "is-enabled", "docker.service"]).strip() != b"enabled":
                raise InstallError("Docker boot startup is required: run sudo systemctl enable --now docker.service, then repeat scripts/install")
            progress(args, "Checking the managed VPN service.")
            return run_managed(config, work, document(config, "hton.yaml") or {},
                early_module, args, values=docker.inventory() if work else None)
        if args.action in ("renew", "update") and (not state or not state.get("hasCommitted")):
            raise InstallError("Automatic renewal/update requires a committed minimum installation")
        if state and not state.get("lease"):
            upgrade_state_fingerprints(config, work, state)
        if state and state.get("lease"):
            with work.lock("effect.lock"):
                Installation(config, work, state).rollback()
            raise InstallError("Interrupted cutover was restored; repeat installation to resume")
        if docker.run(["/usr/bin/systemctl", "--system", "is-enabled", "docker.service"]).strip() != b"enabled":
            raise InstallError("Docker boot startup is required: run sudo systemctl enable --now docker.service, then repeat scripts/install")
        progress(args, "Checking the VPN service.")
        values = docker.inventory()
        if state:
            identifier = state["replacement"] if state.get("hasCommitted") else state["originalId"]
            value = values.get(identifier)
            if value is None:
                raise InstallError("Recorded native resource disappeared; originals retained")
            if args.amnezia_container and args.amnezia_container not in (value["Id"], value["Name"].lstrip("/"), state["originalId"]):
                raise InstallError("Requested native association differs from the saved installation")
            expected = state.get("replacementFingerprint") if state.get("hasCommitted") else state["originalFingerprint"]
            if not fingerprint_matches(state, expected, value):
                raise InstallError("Native resource drift; existing settings retained")
        else:
            candidates = [v for v in values.values() if (v["Name"].lstrip("/") == args.amnezia_container or v["Id"] == args.amnezia_container)] if args.amnezia_container else [v for v in values.values() if v["Name"] == "/amnezia-xray"]
            if len(candidates) != 1:
                raise InstallError("Select exactly one installed Amnezia Xray with --amnezia-container NAME_OR_FULL_ID")
            value = candidates[0]
        native = docker.qualify(value, state["port"] if state and state.get("hasCommitted") else 443)
        if state:
            state.setdefault("originalNativeDigest", state["nativeDigest"])
        expected_native = state["nativeDigest"] if state and state.get("hasCommitted") else (state.get("originalNativeDigest", state["nativeDigest"]) if state else native["nativeDigest"])
        if state and native["nativeDigest"] != expected_native:
            current_bytes = docker.native_config(value["Id"])
            structure = docker.native_structure_digest(current_bytes)
            if not state.get("hasCommitted") or state.get("nativeStructureDigest") != structure:
                raise InstallError("Saved native configuration changed; exact requalification is required")
            if interactive_install(args):
                answer = prompt(
                    "\nThe Amnezia client list changed; keys, REALITY names and the listener are unchanged.\n"
                    + "Accept the updated client list and continue? [y/N]")
                if answer.lower() not in ("y", "yes"):
                    raise InstallError("Native client-list update was not confirmed; no changes performed")
            elif args.action in ("install", "update"):
                raise InstallError("Native client list changed; run scripts/install to review it before updating")
            state["nativeDigest"] = native["nativeDigest"]
            state["nativeClientsRequalified"] = native["nativeDigest"]
            work.save("state.json", state)
            expected_native = native["nativeDigest"]
        elif state and state.get("hasCommitted") and "nativeStructureDigest" not in state:
            state["nativeStructureDigest"] = docker.native_structure_digest(docker.native_config(value["Id"]))
        if state and native["binaryDigest"] != state["binaryDigest"]:
            raise InstallError("Saved native executable changed; exact requalification is required")
        if not state or not state.get("hasCommitted"):
            docker.public_holder(value)
            # Normal local TLS route observation, not a public DNS/IP identity
            # query. A trusted camouflage handshake also excludes an existing
            # target-to-entry routing loop before any remap effects.
            context = ssl.create_default_context()
            with socket.create_connection(("127.0.0.1", 443), timeout=5) as stream:
                with context.wrap_socket(stream, server_hostname=native["names"][0]):
                    pass
        root = (document(config, "hton.yaml") if config else None) or {}
        if root.get("schemaVersion", 1) != 1:
            raise InstallError("Unsupported root configuration version; original retained")
        if root.get("installation", {}).get("systemd") is False:
            raise InstallError("Minimum installation requires its independent recovery units; saved systemd setting was retained")
        config = config or Store(ROOT / "config", create=True)
        ensure_client_page_path(root, args, config)
        page_value = (root.get("core") or {}).get("clientAccessPage") or {}
        page_enabled = isinstance(page_value, dict) and isinstance(page_value.get("path"), str) and bool(page_value.get("path"))
        page_refresh = page_value.get("refreshSeconds", 60) if isinstance(page_value, dict) else 60
        enabled_protocols = protocols.selected(root, args.with_features, args.without_features, state=state)
        saved_core = root.get("core", {})
        saved_domain = (saved_core.get("edge", {}).get("domains") or [None])[0]
        saved_email = saved_core.get("certificates", {}).get("email")
        if state:
            if saved_domain and saved_domain != state["siteDomain"] or saved_email and saved_email != state["certificateEmail"]:
                raise InstallError("Saved site configuration changed; explicit review is required")
            saved_domain = saved_domain or state["siteDomain"]
            saved_email = saved_email or state["certificateEmail"]
        metadata = saved_core.get("certificates", {})

        domain, email = load_site_inputs(root, args)
        test_tunnel = configure_test_tunnel(root, args, domain)
        chain_sni = (getattr(args, "chain_sni", None)
            or (state.get("chainSni") if state else None)
            or (domain if "mtproto" in enabled_protocols else None))
        if chain_sni:
            if not docker.valid_domain(chain_sni):
                raise InstallError("Chain SNI must be a valid lower-case DNS name")
            if chain_sni != domain and not chain_sni.endswith("." + domain):
                raise InstallError("Chain SNI must be the site domain or its subdomain")
            if "mtproto" not in enabled_protocols:
                raise InstallError("Chain SNI requires MTProto activation")
        if state and (domain != state["siteDomain"] or email != state["certificateEmail"]):
            raise InstallError("Site inputs differ from the saved installation; original configuration retained")
        if chain_sni and state and state.get("chainSni") is not None and chain_sni != state["chainSni"]:
            raise InstallError("Saved chain SNI differs; explicit review is required")
        if domain in native["names"] and not (state and state.get("nativeMigration")):
            raise InstallError("Site and native REALITY SNI overlap without a recorded migration")
        module_bytes = config.read("modules/foreign-amnezia.yaml") if config else None
        module = (document(config, "modules/foreign-amnezia.yaml") if config else None) or {"schemaVersion": 1, "moduleId": "foreign-amnezia", "settings": {}}
        if module.get("schemaVersion") != 1 or module.get("moduleId") != "foreign-amnezia":
            raise InstallError("Unsupported native settings document; original retained")
        settings = module["settings"]
        if settings.get("container") and settings["container"] not in (value["Name"].lstrip("/"), value["Id"], state["originalId"] if state else value["Id"]):
            raise InstallError("Saved native selector differs; explicit association review is required")
        ports = settings.get("hostPorts")
        port = ports.get("tcp") if ports else None
        if state:
            if not ports:
                if state.get("configurationCommitted") or (digest(module_bytes) if module_bytes is not None else None) != state.get("portBasis"):
                    raise InstallError("Saved service port configuration disappeared or changed; no reroll performed")
                port = state["port"]
            elif port != state["port"]:
                raise InstallError("Service port differs from the exact installation assignment; no implicit change")
        if ports and (not isinstance(port, int) or not 5000 <= port <= 6000 or ports.get("udp") != port):
            raise InstallError("Saved VPN port state is inconsistent; no reroll performed")
        if not port:
            choices = list(range(5000, 6001))
            secrets.SystemRandom().shuffle(choices)
            port = next((p for p in choices if available(p, values)), None)
            if port is None:
                raise InstallError("No free VPN backend port in 5000-6000")
        elif not state or not state.get("hasCommitted"):
            if not available(port, values, (state.get("replacement"),) if state else ()):
                raise InstallError("Saved VPN port is occupied; no reroll performed")
        site_port = state.get("sitePort", 9443) if state else 9443
        if not state and not available(site_port, values):
            raise InstallError("Loopback site port 9443 is occupied")
        configured_site = resolve_site_root(root, domain, args)
        body = ingress.webroot(str(configured_site) if configured_site is not None else None)
        unit_preflight(ROOT / "work/minimum")
        if settings.get("serverNames") and settings["serverNames"] != native["names"] and not (state and (state.get("nativeMigration") or state.get("nativeMigrationPending"))):
            raise InstallError("Saved native SNI differs; no implicit update")
        identifier = state["replacement"] if state and state.get("replacement") else value["Id"]
        current_native_bytes = docker.native_config(identifier)
        try:
            bridge_gateway = docker.native_bridge_gateway(value)
        except InstallError:
            bridge_gateway = None
        own_name = chain_sni or domain
        mtg_port = protocols.mtproto_port(state or {}, values) if "mtproto" in enabled_protocols else None
        current_names = native["names"]
        native_names = current_names
        desired_names = list(dict.fromkeys([own_name, *current_names]))
        target_dest = ((bridge_gateway + ":" + str(mtg_port)) if chain_sni and bridge_gateway
            else ((bridge_gateway + ":" + str(site_port)) if bridge_gateway else None))
        desired_bytes = None
        if target_dest:
            desired_bytes = protocols.native_selfsteal_config(
                json.loads(current_native_bytes), desired_names, target_dest)
        already_migrated = desired_bytes is not None and desired_bytes == current_native_bytes
        migrate_native = False
        native_restore = None
        interactive = interactive_install(args)
        if state and state.get("nativeMigration") and not interactive:
            migrate_native = already_migrated
        elif state and state.get("nativeMigration"):
            previous = work.read("native/previous.json")
            if previous is None:
                previous = docker.native_config(state["originalId"])
            try:
                restored_reality = json.loads(previous)["inbounds"][0]["streamSettings"]["realitySettings"]
                restored_names = restored_reality["serverNames"]
            except (ValueError, KeyError, IndexError, TypeError):
                raise InstallError("Saved native rollback material is invalid; explicit review is required")
            answer = prompt(
                "\nThe native REALITY listener currently uses the own SNI; profiles that still use the legacy SNI stop working.\n"
                + "Restore the previous native configuration for backward compatibility? [Y/n]", "Y")
            if answer.lower() not in ("n", "no"):
                native_names = restored_names
                native_restore = {"content": previous, "digest": digest(previous),
                    "previous": current_native_bytes, "previousDigest": digest(current_native_bytes),
                    "names": restored_names, "dest": restored_reality.get("dest")}
                desired_bytes = None
            else:
                migrate_native = already_migrated
        elif already_migrated:
            migrate_native = True
        elif desired_bytes is not None and interactive:
            answer = prompt(
                "\nSwitching the native REALITY listener to the own SNI breaks profiles that still use the legacy SNI.\n"
                + "Names: " + ", ".join(desired_names) + "\n"
                + "Type DROP-LEGACY-SNI to migrate now; press Enter to keep backward compatibility.")
            migrate_native = answer == "DROP-LEGACY-SNI"
        if not migrate_native and native_restore is None:
            native_names = current_names
            desired_bytes = None
        if not state and not interactive:
            raise InstallError("Amnezia port change requires explicit approval in an interactive install")
        config = config or Store(ROOT / "config", create=True)
        save_site_inputs(config, root, domain, email)
        certificate_defaults = load_settings(SOURCE).get("core", {}).get("certificates", {})
        challenge = certificates.select_policy(root, state, config, args.dns_credentials, defaults=certificate_defaults)
        certificate_settings = root["core"]["certificates"]
        if "http" not in certificate_settings and "http" in certificate_defaults:
            certificate_settings["http"] = copy.deepcopy(certificate_defaults["http"])
        certificate_settings.update({"ca": "letsencrypt", "challenge": challenge})
        if challenge == "http-01":
            certificate_settings.pop("provider", None)
        else:
            certificate_settings["provider"] = "internetbs"
        config.save("hton.yaml", root)
        publication_result = None
        if interactive and challenge == "http-01":
            progress(args, "Checking public DNS.")
            publication_result = publication.check(domain, root["core"]["edge"], interactive=True, prompt=prompt)
        if not state:
            approval = consent(value, native, port)
            work_parent = Store(ROOT / "work", create=True)
            work_parent.write("minimum/owner", b"hton-minimum-v1\n")
            work = work or Store(ROOT / "work/minimum")
            token = secrets.token_hex(16)
            state = {"schemaVersion": 1, "instance": secrets.token_hex(16), "operation": token,
                "originalId": value["Id"], "originalFingerprint": docker.fingerprint(value), "nativeDigest": native["nativeDigest"],
                "fingerprintVersion": docker.FINGERPRINT_VERSION,
                "approvalVersion": APPROVAL_VERSION,
                "originalNativeDigest": native["nativeDigest"],
                "originalNativeNames": native["names"],
                "nativeStructureDigest": docker.native_structure_digest(docker.native_config(value["Id"])),
                "binaryDigest": native["binaryDigest"], "nativeVersion": native["nativeVersion"],
                "siteDomain": domain, "certificateEmail": email,
                "portBasis": digest(module_bytes) if module_bytes is not None else None,
                "configurationCommitted": False,
                "port": port, "sitePort": site_port, "approval": approval, "phase": "prepared", "lease": 0,
                "snapshotTag": "hton-minimum-original:" + token, "replacementName": "hton-minimum-replacement-" + token,
                "replacementParked": "hton-minimum-retained-" + token, "originalParked": "hton-minimum-original-" + token}
            work.save("original.json", value)
            work.save("state.json", state)
        operation = Installation(config, work, state, reporter=lambda text: progress(args, text))
        if "originalNativeNames" not in state:
            original_config = json.loads(docker.native_config(state["originalId"]))
            state["originalNativeNames"] = original_config["inbounds"][0]["streamSettings"]["realitySettings"]["serverNames"]
        state["certificateChallenge"] = challenge
        if publication_result:
            state["publication"] = publication_result
        operation.save()
        approval_native = {"nativeDigest": state.get("originalNativeDigest", state["nativeDigest"]),
            "binaryDigest": state["binaryDigest"], "nativeVersion": state["nativeVersion"],
            "names": state["originalNativeNames"]}
        if state["approval"] != consent_scope(operation.original(), approval_native, state["port"]):
            raise InstallError("Exact saved remap approval differs from the current installation scope")
        pins_raw = image_lock()
        required_images = ["haproxy", "caddy", "acme", "xray"]
        if "mtproto" in enabled_protocols:
            required_images.append("mtg")
        if "hysteria" in enabled_protocols:
            required_images.append("hysteria")
        if "outline-wss" in enabled_protocols:
            required_images.append("alpine")
        if "amneziawg" in enabled_protocols:
            required_images.append("amneziawg")
        pins = resolve_pins(pins_raw, required_images, allow_cached=args.action == "install")
        migrations.apply(work, config, state, operation.save)
        native_server = json.loads(docker.native_config(state.get("replacement") or value["Id"]))
        protocol_plan = protocols.plan(config, work, state, pins, domain, native_names,
            enabled_protocols, args, values, ask=prompt if interactive else (lambda _label, default: default),
            confirm=lambda label: interactive and prompt(label, "N").lower() in ("y", "yes"),
            native_server=native_server, chain_sni=chain_sni, site_port=site_port,
            migrate_native=migrate_native, bridge_gateway=bridge_gateway, mtg_port=mtg_port)
        plan_native = protocol_plan.get("native") if migrate_native else None
        active_names = plan_native["names"] if plan_native else native_names
        native_migration = None
        if plan_native and plan_native["content"] != current_native_bytes:
            native_migration = {"content": plan_native["content"], "digest": plan_native["digest"],
                "previous": current_native_bytes, "previousDigest": digest(current_native_bytes),
                "names": plan_native["names"], "dest": plan_native["dest"]}
            state["nativeMigrationPending"] = {"names": plan_native["names"], "dest": plan_native["dest"],
                "fromDigest": digest(current_native_bytes), "toDigest": plan_native["digest"]}
        if not native_migration and native_restore is None:
            state.pop("nativeMigrationPending", None)
        if chain_sni:
            state["chainSni"] = chain_sni
        protocols.record_state(state, protocol_plan)
        operation.save()
        validate_test_tunnel(test_tunnel, [
            domain, "www." + domain, *active_names,
            *([chain_sni] if chain_sni else []),
            *([protocol_plan["mtproto"]["camouflage"]] if protocol_plan.get("mtproto") else []),
        ])
        settings.update({"container": value["Name"].lstrip("/"), "serverNames": active_names, "hostPorts": {"tcp": port, "udp": port}})
        config.save("modules/foreign-amnezia.yaml", module)
        root.update({"schemaVersion": 1, "instance": root.get("instance", {"name": "hton"})})
        root.setdefault("installation", {}).update({"systemd": True})
        root["installation"].setdefault("pathShim", False)
        core = root.setdefault("core", {})
        core["edge"].setdefault("listen", {}).update({"transport": "tcp", "port": 443, "scope": "public", "addressFamilies": ["ipv4"]})
        root.setdefault("features", {})["foreign-amnezia"] = {"enabled": True}
        protocols.save_config(config, root, protocol_plan)
        config.save("hton.yaml", root)
        state["configurationCommitted"] = True
        operation.save()
        ingress.cleanup_temporary_jobs(work, state, operation.save)
        if state.get("hasCommitted"):
            original = operation.check_foreign(state["originalId"])
            if original["Name"] != "/" + state["originalParked"] or original["State"]["Running"] or original["HostConfig"]["RestartPolicy"]["Name"] != "no":
                raise InstallError("Retained original restart/name state drifted; no changes performed")
        progress(args, "Checking the HTTPS certificate and automatic renewal.")
        legacy_names = ["www." + domain]
        if chain_sni and chain_sni != domain:
            legacy_names.append(chain_sni)
        legacy_wildcard = (challenge == "dns-01"
            and root["core"]["certificates"].get("wildcardDomain") == domain)
        certificate = certificates.ensure(config, work, state, pins, domain, email, args.dns_credentials, operation.save,
            interactive=interactive, challenge=challenge, http_settings=root["core"]["certificates"].get("http", {}),
            names=legacy_names, wildcard=legacy_wildcard)
        if not args.quiet: print("OK: HTTPS certificate checked.", file=sys.stderr)
        protocols.validate(work, state, pins, protocol_plan, certificate)
        operation.save()
        generation = digest(body + (certificate / "fullchain.pem").read_bytes()
            + json.dumps(pins, sort_keys=True).encode() + protocol_plan["generation"].encode()
            + json.dumps(test_tunnel, sort_keys=True).encode()
            + (str(page_value.get("path", "")) if page_enabled else "").encode())
        if state.get("hasCommitted") and state.get("generation") == generation:
            for role in ("site", "edge"):
                admitted = ingress.owned(state, state["activeIngress"][role], role, work)
                if not admitted["State"]["Running"]:
                    raise InstallError("Accepted ingress is stopped; restore its exact Docker container before retrying")
            ingress.website(domain, 443, body)
            if chain_sni:
                ingress.website(chain_sni, 443, body)
            protocols.verify_active(work, state, pins, protocol_plan, domain)
            if args.action != "renew":
                ingress.authentication(work, state, pins, state["replacement"], active_names, operation.save)
                if chain_sni:
                    ingress.authentication(work, state, pins, state["replacement"], active_names,
                        operation.save, server_name=chain_sni)
            if args.action in ("install", "update"):
                state["recoverySource"] = str(SOURCE)
                operation.save()
                install_units(work, client_page_enabled=page_enabled, refresh_seconds=page_refresh)
                timer("recovery", False)
                timer("renew", True)
                timer("usage", True)
                timer("update", True)
                if page_enabled: timer("client-page", True)
            state["protocolsChosen"] = True
            operation.save()
            refresh_client_page(config, work, state, protocol_plan)
            emit_ready(args, domain, port, challenge, "retained" if args.action == "renew" else "verified",
                publication_result, protocol_plan, work, state.get("replacement") or state.get("originalId"),
                config)
            return 0
        if not state.get("hasCommitted"):
            operation.capture(operation.original())
            operation.validate_snapshot()
            operation.replacement()
        state["pendingGeneration"] = generation
        operation.save()
        protocols.prepare(work, state, pins, protocol_plan, certificate, operation.save)
        page_snapshot = refresh_client_page(config, work, state, protocol_plan) if config else {"enabled": False}
        client_page_arg = ({"path": page_snapshot["path"], "headers": page_snapshot["headers"]}
            if page_snapshot.get("enabled") else None)
        ingress.prepare(work, state, pins, certificate, domain, active_names, port, site_port, body,
            operation.save, mtproto=None if chain_sni else protocol_plan["mtproto"],
            outline=protocol_plan["outline"], selfsteal=protocol_plan["selfsteal"],
            chain_sni=chain_sni, native_selfsteal=bool(plan_native),
            site_bind=(bridge_gateway if plan_native and not chain_sni else None),
            client_page=client_page_arg,
            site_document_root=str(configured_site) if configured_site is not None else None,
            test_tunnel=test_tunnel)
        state["recoverySource"] = str(SOURCE)
        operation.save()
        install_units(work, client_page_enabled=page_enabled, refresh_seconds=page_refresh)
        timer("renew", True)
        timer("usage", True)
        timer("update", True)
        if page_enabled: timer("client-page", True)
        with work.lock("effect.lock"):
            operation.cutover(body, active_names, domain, pins, protocol_plan,
                verify_native=args.action != "renew", chain_sni=chain_sni,
                native_migration=native_migration, native_restore=native_restore)
        state["protocolsChosen"] = True
        operation.save()
        refresh_client_page(config, work, state, protocol_plan)
        emit_ready(args, domain, port, challenge, "retained" if args.action == "renew" else "verified",
            publication_result, protocol_plan, work, state.get("replacement") or state.get("originalId"),
            config)
        return 0


def failure_detail(error):
    detail = {"kind": type(error).__name__}
    if isinstance(error, OSError) and type(error.errno) is int:
        detail["errno"] = error.errno
    if isinstance(error, docker.NativeError):
        detail.update({"operation": error.operation, "exitCode": error.code, "reason": error.reason})
    trace = error.__traceback__
    while trace:
        code = trace.tb_frame.f_code
        filename = Path(code.co_filename)
        if filename.parent.name in ("minimum", "lib") and filename.suffix == ".py":
            detail.update({"module": filename.stem, "function": code.co_name, "line": trace.tb_lineno})
            if isinstance(error, docker.NativeError) and filename.stem != "docker":
                detail["caller"] = filename.stem + "." + code.co_name
        trace = trace.tb_next
    return detail


def failure_message(error):
    if isinstance(error, InstallError) and str(error).strip():
        return str(error)
    detail = failure_detail(error)
    location = ""
    if "module" in detail:
        location = " at " + detail["module"] + "." + detail["function"] + ":" + str(detail["line"])
    reason = {FileNotFoundError: "A required file or program is missing", PermissionError: "Setup cannot read or use a required file",
        ssl.SSLCertVerificationError: "The HTTPS certificate could not be verified"}.get(type(error), "Setup could not finish a required check")
    return reason + " (" + detail["kind"] + location + "). Setup is not complete."


def report_failure(error):
    marker = "ERROR"
    if sys.stderr.isatty() and "NO_COLOR" not in os.environ and os.environ.get("TERM") != "dumb":
        marker = "\033[1;31mERROR\033[0m"
    message = str(error) if isinstance(error, ConfigError) else failure_message(error)
    detail = failure_detail(error)
    if not message or not message.strip():
        location = ""
        if "module" in detail:
            location = " at " + detail["module"] + "." + detail["function"] + ":" + str(detail["line"])
        message = type(error).__name__ + location
    print("\n" + marker + ": Setup stopped.", file=sys.stderr)
    print(message, file=sys.stderr)
    print("Detail: " + " ".join(f"{key}={detail[key]}" for key in
        ("kind", "module", "function", "line", "operation", "exitCode", "reason") if key in detail),
        file=sys.stderr)
    print("Log: /var/logs/hton/minimum.log", file=sys.stderr)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        # No traceback/native output or structured secret objects in product logs.
        detail = failure_detail(error)
        detail["message"] = failure_message(error)
        log("failed", type(error).__name__, detail)
        report_failure(error)
        raise SystemExit(1)
