"""Standalone, user-maintained single-file profile repository.

``.secrets/`` is its own private Git repository (not a submodule) and is ignored
by this project. It holds one editable ``profile.yml`` with the complete
installation profile — root settings, module settings (enablement, SNI/domains),
secret values and optional web-root content — plus a generated ``images.lock.json``.
Import materializes the protected ``config/`` tree. This module never prints
secret values.
"""
from __future__ import annotations

import json
import os
from pathlib import Path
import stat
import sys

import yaml

PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROJECT_ROOT / "python/minimum"))
from storage import InstallError, Store, digest  # noqa: E402

REPOSITORY = ".secrets"
PROFILE_FILE = "profile.yml"
LOCK_FILE = "images.lock.json"
TEMPLATE_REPOSITORY = ".secrets.example"
PLACEHOLDER = "CHANGE-ME"
MAX_FILE = 1024 * 1024

# dotted profile path, description, default, config-store relative path
SECRET_FIELDS = (
    ("secrets.dns.internetbsApiKey",
     "InternetBS API key for DNS-01 wildcard certificates.",
     "CHANGE-ME-internetbs-api-key", "secrets/dns/internetbs-api-key"),
    ("secrets.dns.internetbsApiPassword",
     "InternetBS API password paired with the API key.",
     "CHANGE-ME-internetbs-api-password", "secrets/dns/internetbs-api-password"),
    ("secrets.amnezia.adminSshKey",
     "Ed25519 private key for the managed Amnezia host-access account.",
     "CHANGE-ME-admin-ssh-key", "secrets/amnezia/admin-ssh-key"),
    ("secrets.hysteria.password",
     "Hysteria2 client password.",
     "CHANGE-ME-hysteria-password", "secrets/hysteria/password"),
    ("secrets.outline.tcpPath",
     "Outline WSS TCP stream path (ordinary HTTP path, not a key).",
     "/assets/stream", "secrets/outline/tcp-path"),
    ("secrets.outline.udpPath",
     "Outline WSS UDP packet path (ordinary HTTP path, not a key).",
     "/assets/session", "secrets/outline/udp-path"),
)

SECTIONS = (
    ("root", "Root config: instance, site domains/SNI, edge port, certificate, DNS, client page."),
    ("modules", "Module settings: enablement, names/SNI, ports and mode."),
    ("secrets", "Secret values (DNS/ACME, managed admin key, Hysteria password, Outline paths)."),
    ("site", "Optional web-root content (indexHtml)."),
)


def repository_root(root) -> Path:
    return Path(root) / REPOSITORY


def profile_path(root) -> Path:
    return repository_root(root) / PROFILE_FILE


class _UniqueLoader(yaml.SafeLoader):
    pass


def _mapping(loader, node):
    loader.flatten_mapping(node)
    pairs = loader.construct_pairs(node)
    result = {}
    for key, value in pairs:
        if not isinstance(key, str) or key in result:
            raise ValueError("Invalid or duplicate YAML key")
        result[key] = value
    return result


_UniqueLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _mapping)


def _load(path: Path) -> dict:
    try:
        data = path.read_bytes()
    except OSError as error:
        raise InstallError("Profile is unavailable") from error
    if len(data) > MAX_FILE:
        raise InstallError("Profile exceeds its size bound")
    try:
        value = yaml.load(data, Loader=_UniqueLoader)
    except (yaml.YAMLError, ValueError, UnicodeError) as error:
        raise InstallError("Profile is not valid YAML") from error
    if not isinstance(value, dict):
        raise InstallError("Profile must be a mapping")
    return value


def _private_write(path: Path, data: bytes) -> None:
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    if path.is_symlink():
        raise InstallError("Secrets repository contains a symlink")
    try:
        info = path.lstat()
    except FileNotFoundError:
        info = None
    if info is not None and (not stat.S_ISREG(info.st_mode) or info.st_nlink != 1):
        raise InstallError("Secrets repository entry is not a regular file")
    temporary = path.parent / (".new-" + os.urandom(8).hex())
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        os.write(fd, data)
        os.fsync(fd)
    finally:
        os.close(fd)
    os.replace(temporary, path)
    os.chmod(path, 0o600)


def _write_profile(root, data) -> None:
    _private_write(profile_path(root),
        yaml.safe_dump(data, sort_keys=False, allow_unicode=False).encode())


def _get(data, dotted):
    current = data
    for part in dotted.split("."):
        if not isinstance(current, dict) or part not in current:
            return None
        current = current[part]
    return current


def _set(data, dotted, value) -> None:
    parts = dotted.split(".")
    current = data
    for part in parts[:-1]:
        current = current.setdefault(part, {})
        if not isinstance(current, dict):
            raise InstallError("Profile section is not a mapping: " + part)
    current[parts[-1]] = value


def _config_read(config_root, relative):
    try:
        store = Store(Path(config_root))
    except (FileNotFoundError, OSError, InstallError):
        return None
    try:
        return store.read(relative)
    finally:
        store.close()


def images_lock(root) -> dict | None:
    for path in (repository_root(root) / LOCK_FILE,
                 PROJECT_ROOT / TEMPLATE_REPOSITORY / LOCK_FILE):
        if path.is_file():
            try:
                value = json.loads(path.read_text())
            except (ValueError, UnicodeError) as error:
                raise InstallError("Image lock is invalid JSON") from error
            if isinstance(value, dict):
                return value
    return None


def status(root) -> dict:
    root = Path(root)
    path = profile_path(root)
    entries = []
    repo_complete = True
    error = None
    data = None
    if path.is_file():
        try:
            data = _load(path)
        except InstallError as exc:
            error = str(exc)
    else:
        error = "profile.yml is missing"
    if data is None or data.get("schemaVersion") != 1:
        repo_complete = False
    for name, description in SECTIONS:
        present = isinstance((data or {}).get(name), dict) if name != "site" else (
            isinstance((data or {}).get("site"), dict))
        if name in ("root", "modules") and not present:
            repo_complete = False
        entries.append({"path": name, "description": description, "present": present})
    modules = (data or {}).get("modules")
    if isinstance(modules, dict):
        for module_id, entry in modules.items():
            enabled = entry.get("enabled") if isinstance(entry, dict) else None
            if not isinstance(enabled, bool):
                repo_complete = False
            entries.append({"path": "modules." + str(module_id) + ".enabled",
                "description": "Whether the module is installed/enabled.",
                "present": isinstance(enabled, bool), "enabled": enabled})
    challenge = _get(data or {}, "root.core.certificates.challenge")
    dns_optional = challenge == "http-01"
    for dotted, description, default, relative in SECRET_FIELDS:
        value = _get(data or {}, dotted)
        placeholder = not isinstance(value, str) or value.startswith(PLACEHOLDER)
        required = not (dns_optional and dotted.startswith("secrets.dns."))
        entries.append({"path": dotted, "description": description,
            "present": value is not None, "placeholder": placeholder,
            "target": relative, "required": required})
        if required and (value is None or placeholder):
            repo_complete = False
    lock = None
    try:
        lock = images_lock(root)
    except InstallError:
        lock = None
    config_complete = _config_read(root / "config", "hton.yaml") is not None
    return {"root": str(root), "profile": str(path), "error": error,
        "entries": entries, "repo_complete": repo_complete,
        "config_complete": config_complete, "images": lock is not None}


def _seed(root) -> dict:
    root = Path(root)
    repository_root(root).mkdir(mode=0o700, parents=True, exist_ok=True)
    os.chmod(repository_root(root), 0o700)
    template = PROJECT_ROOT / TEMPLATE_REPOSITORY / PROFILE_FILE
    if profile_path(root).is_file():
        data = _load(profile_path(root))
    elif template.is_file():
        data = _load(template)
    else:
        data = {"schemaVersion": 1, "root": {}, "modules": {}, "secrets": {}}
    changed = False
    for dotted, _description, default, _relative in SECRET_FIELDS:
        if not isinstance(_get(data, dotted), str):
            _set(data, dotted, default)
            changed = True
    if not isinstance(data.get("root"), dict):
        data["root"] = {}
        changed = True
    if not isinstance(data.get("modules"), dict):
        data["modules"] = {}
        changed = True
    for module_id, entry in list(data["modules"].items()):
        if not isinstance(entry, dict):
            continue
        if "settings" not in entry:
            entry = {"enabled": True, "settings": {key: value for key, value in entry.items() if key != "enabled"}}
            data["modules"][module_id] = entry
            changed = True
        if not isinstance(entry.get("enabled"), bool):
            entry["enabled"] = True
            changed = True
    if changed or not profile_path(root).is_file():
        _write_profile(root, data)
    lock = repository_root(root) / LOCK_FILE
    if not lock.is_file():
        source = PROJECT_ROOT / TEMPLATE_REPOSITORY / LOCK_FILE
        if source.is_file():
            _private_write(lock, source.read_bytes())
    return data


def init(root, assume_yes=False, prompt=None) -> dict:
    root = Path(root)
    data = _seed(root)
    if not assume_yes:
        if prompt is None:
            prompt = _tty_prompt
        _describe(root)
        print("Press Enter to accept (Y), or type n to edit further.", file=sys.stderr)
        while True:
            answer = prompt("Profile complete? [Y/n]: ").strip().lower()
            if answer in ("", "y", "yes", "д", "да"):
                break
            if answer in ("n", "no", "н", "нет"):
                print("Edit the profile, then press Enter to accept.", file=sys.stderr)
                continue
            print("Answer y (accept) or n (edit).", file=sys.stderr)
    return {"root": str(root), "profile": str(profile_path(root)), "status": status(root)}


def _describe(root) -> None:
    lines = ["Complete profile: " + str(profile_path(root)), "",
        "Edit this single YAML file, then confirm. Keep this repository private.", ""]
    for name, description in SECTIONS:
        lines.append(name + ": " + description)
    for dotted, description, _default, _relative in SECRET_FIELDS:
        value = _get(_load(profile_path(root)), dotted)
        state = "filled" if isinstance(value, str) and not value.startswith(PLACEHOLDER) else "placeholder"
        lines.append(dotted + " (" + state + "): " + description)
    print("\n".join(lines), file=sys.stderr)


def _tty_prompt(label: str) -> str:
    try:
        with open("/dev/tty", "r", encoding="utf-8") as stream:
            print(label, end="", file=sys.stderr, flush=True)
            return stream.readline(4096)
    except OSError as error:
        raise InstallError("Secrets confirmation requires a local TTY") from error


def _write_yaml(store, name, value) -> None:
    store.write(name, yaml.safe_dump(value, sort_keys=False, allow_unicode=False).encode())


def set_value(root, dotted: str, value: str) -> dict:
    """Set one dotted profile value and persist the private file."""
    root = Path(root)
    if not profile_path(root).is_file():
        _seed(root)
    data = _load(profile_path(root))
    _set(data, dotted, value)
    _write_profile(root, data)
    return status(root)


def import_secrets(root) -> dict:
    """Materialize the whole profile into the protected config store."""
    root = Path(root)
    data = _load(profile_path(root))
    if data.get("schemaVersion") != 1:
        raise InstallError("Profile schemaVersion must be 1")
    root_document = data.get("root")
    modules = data.get("modules")
    if not isinstance(root_document, dict) or not isinstance(modules, dict):
        raise InstallError("Profile requires root and modules mappings")
    store = Store(root / "config", create=True)
    try:
        imported = []
        features = {}
        for module_id, entry in modules.items():
            if not isinstance(module_id, str) or not isinstance(entry, dict):
                raise InstallError("Profile module entry is invalid")
            enabled = entry.get("enabled")
            settings = entry.get("settings", {})
            if not isinstance(enabled, bool):
                raise InstallError("Profile module requires enabled: true|false: " + module_id)
            if not isinstance(settings, dict):
                raise InstallError("Profile module settings must be a mapping: " + module_id)
            relative = "modules/" + module_id + ".yaml"
            _write_yaml(store, relative, {"schemaVersion": 1, "moduleId": module_id,
                "settings": settings})
            imported.append(relative)
            features[module_id] = {"enabled": enabled}
        root_document = dict(root_document)
        root_document["features"] = features
        site = data.get("site") if isinstance(data.get("site"), dict) else {}
        site_root = site.get("root")
        if site_root not in (None, ""):
            if not isinstance(site_root, str) or not site_root.startswith("/"):
                raise InstallError("site.root must be an absolute path")
            core = root_document.setdefault("core", {})
            if not isinstance(core, dict):
                raise InstallError("Profile root.core must be a mapping")
            site_settings = core.setdefault("site", {})
            if not isinstance(site_settings, dict):
                raise InstallError("Profile root.core.site must be a mapping")
            site_settings["root"] = site_root
        _write_yaml(store, "hton.yaml", root_document)
        imported.insert(0, "hton.yaml")
        lock = images_lock(root)
        if lock is not None:
            store.write("locks/images.lock.json",
                (json.dumps(lock, sort_keys=True, indent=2) + "\n").encode())
            imported.append("locks/images.lock.json")
        placeholders = []
        values = {}
        for dotted, _description, _default, relative in SECRET_FIELDS:
            value = _get(data, dotted)
            if not isinstance(value, str) or not value:
                raise InstallError("Profile secret is missing: " + dotted)
            if value.startswith(PLACEHOLDER):
                placeholders.append(relative)
            store.write(relative, value.encode() + b"\n")
            imported.append(relative)
            values[relative] = value
        store.save("secrets/minimum-internetbs.json", {
            "apiKey": values["secrets/dns/internetbs-api-key"],
            "apiPassword": values["secrets/dns/internetbs-api-password"],
        })
        site = data.get("site") if isinstance(data.get("site"), dict) else {}
        index = site.get("indexHtml")
        if isinstance(index, str) and index:
            store.write("site/index.html", index.encode() if index.endswith("\n") else (index + "\n").encode())
            imported.append("site/index.html")
        profile = profile_path(root)
        receipt = {"imported": imported, "placeholders": placeholders,
            "digest": digest(json.dumps(imported, sort_keys=True).encode()),
            "profileDigest": digest(profile.read_bytes()) if profile.is_file() else None}
        store.save("secrets/import-receipt.json", receipt)
        return receipt
    finally:
        store.close()


def _import_receipt(root):
    try:
        store = Store(Path(root) / "config")
    except (FileNotFoundError, OSError, InstallError):
        return None
    try:
        return store.json("secrets/import-receipt.json")
    finally:
        store.close()


def ensure(root, interactive: bool, prompt=None) -> dict:
    root = Path(root)
    profile = profile_path(root)
    current = status(root)
    profile_digest = digest(profile.read_bytes()) if profile.is_file() else None
    if _config_read(root / "config", "hton.yaml") is not None:
        receipt = _import_receipt(root)
        recorded = receipt.get("profileDigest") if isinstance(receipt, dict) else None
        if profile_digest is None or recorded == profile_digest:
            return {"status": "present", **current}
        if current["repo_complete"]:
            return {"status": "imported", "receipt": import_secrets(root)}
        raise InstallError("Secrets profile changed but still has placeholders; fill secrets.dns.internetbsApiKey and secrets.dns.internetbsApiPassword in .secrets/profile.yml")
    if profile.is_file() and current["repo_complete"]:
        return {"status": "imported", "receipt": import_secrets(root)}
    if not interactive:
        raise InstallError("Secrets profile is not configured; run scripts/secrets init --root " + str(root))
    init(root, prompt=prompt)
    return {"status": "initialized", "receipt": import_secrets(root)}
