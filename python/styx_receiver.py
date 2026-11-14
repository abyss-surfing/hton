"""Compile an authenticated, destination-restricted Styx HTTPS CONNECT receiver."""
from __future__ import annotations

import argparse
import ipaddress
import json
import os
from pathlib import Path
import re
import sys
import tempfile


_LABEL = re.compile(r"[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\Z")
_PROTOCOLS = {"vless", "vmess", "trojan"}
_MAX_BYTES = 1024 * 1024


def _record(value, fields, message):
    if not isinstance(value, dict) or set(value) != set(fields):
        raise ValueError(message)


def _label(value):
    if not isinstance(value, str) or not _LABEL.fullmatch(value):
        raise ValueError("styx.invalidIdentifier")
    return value


def _port(value):
    if type(value) is not int or not 1 <= value <= 65535:
        raise ValueError("styx.invalidPort")
    return value


def _ip(value, *, listener=False):
    try:
        if not isinstance(value, str) or "%" in value:
            raise ValueError
        address = ipaddress.ip_address(value)
        if address.is_multicast or (not listener and address.is_unspecified):
            raise ValueError
    except ValueError:
        raise ValueError("styx.invalidAddress") from None
    return str(address)


def _list(value):
    if not isinstance(value, list) or not 1 <= len(value) <= 1024:
        raise ValueError("styx.invalidCollection")
    return value


def _path(value):
    if not isinstance(value, str) or not value or any(ord(c) < 32 for c in value):
        raise ValueError("styx.invalidTlsPath")
    if not Path(value).is_absolute():
        raise ValueError("styx.invalidTlsPath")
    return value


def compile_receiver(spec: dict) -> dict:
    """Return sing-box JSON data; never mutate input or access the environment."""
    _record(spec, ("version", "transport", "listen", "listen_port", "tls", "users", "backends"),
            "styx.invalidReceiverFields")
    if type(spec["version"]) is not int or spec["version"] != 1:
        raise ValueError("styx.unsupportedVersion")
    if spec["transport"] != "https-connect":
        raise ValueError("styx.unsupportedTransport")
    listen = _ip(spec["listen"], listener=True)
    listen_port = _port(spec["listen_port"])
    _record(spec["tls"], ("certificate_path", "key_path"), "styx.invalidTlsFields")
    tls = {"enabled": True,
           "certificate_path": _path(spec["tls"]["certificate_path"]),
           "key_path": _path(spec["tls"]["key_path"])}
    backends = {}
    for backend in _list(spec["backends"]):
        _record(backend, ("id", "protocol", "address", "port"), "styx.invalidBackendFields")
        name = _label(backend["id"])
        if name in backends:
            raise ValueError("styx.duplicateBackend")
        protocol = backend["protocol"]
        if not isinstance(protocol, str) or protocol not in _PROTOCOLS:
            raise ValueError("styx.invalidProtocol")
        address, port = _ip(backend["address"]), _port(backend["port"])
        if port == listen_port and (address == listen or
                (ipaddress.ip_address(listen).is_unspecified and
                 ipaddress.ip_address(address).is_loopback)):
            raise ValueError("styx.receiverLoop")
        backends[name] = (protocol, address, port)

    users, rules, names, passwords = [], [], set(), set()
    for user in _list(spec["users"]):
        _record(user, ("username", "password", "backends"), "styx.invalidUserFields")
        name = _label(user["username"])
        password = user["password"]
        if not isinstance(password, str) or not 16 <= len(password) <= 1024 or any(
                not 32 <= ord(c) <= 126 for c in password):
            raise ValueError("styx.invalidPassword")
        if name in names or password in passwords:
            raise ValueError("styx.duplicateCredential")
        names.add(name)
        passwords.add(password)
        grants = set()
        for backend in _list(user["backends"]):
            backend = _label(backend)
            if backend not in backends or backend in grants:
                raise ValueError("styx.invalidBackendGrant")
            grants.add(backend)
            protocol, address, port = backends[backend]
            rules.append({"inbound": ["styx-in"], "auth_user": [name],
                          "domain": [protocol + "." + backend + ".styx.invalid"], "port": [443],
                          "network": "tcp", "action": "route", "outbound": "styx-backend",
                          "override_address": address, "override_port": port})
            if len(rules) > 4096:
                raise ValueError("styx.tooManyGrants")
        users.append({"username": name, "password": password})

    rules.append({"action": "reject"})
    return {"log": {"level": "warn"},
            "inbounds": [{"type": "http", "tag": "styx-in", "listen": listen,
                          "listen_port": listen_port, "users": users, "tls": tls}],
            "outbounds": [{"type": "direct", "tag": "styx-backend"}],
            "route": {"rules": rules}}


def _unique_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("styx.duplicateJsonField")
        value[key] = item
    return value


def _write_private(path, data):
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=".styx-", suffix=".json", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as stream:
            os.fchmod(stream.fileno(), 0o600)
            json.dump(data, stream, indent=2)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description="Compile a private Styx receiver configuration; start nothing.")
    parser.add_argument("--spec", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        if args.spec.resolve() == args.output.resolve():
            raise ValueError("styx.inputOutputConflict")
        with args.spec.open("rb") as stream:
            raw = stream.read(_MAX_BYTES + 1)
        if len(raw) > _MAX_BYTES:
            raise ValueError("styx.inputTooLarge")
        try:
            spec = json.loads(raw, object_pairs_hook=_unique_object)
        except (json.JSONDecodeError, UnicodeDecodeError, RecursionError):
            raise ValueError("styx.invalidJson") from None
        config = compile_receiver(spec)
        _write_private(args.output, config)
    except ValueError as error:
        print(str(error), file=sys.stderr)
        return 2
    except OSError:
        print("styx.fileOperationFailed", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
