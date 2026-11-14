"""Bounded MTProto, Hysteria, Outline-WSS and AmneziaWG2 support.

This module owns declarative settings, immutable native artifacts and lifecycle
helpers for the optional protocols.  The installer transaction owns native
effects: helpers here only create/remove labelled containers through the shared
Docker transport and never touch a foreign resource.
"""
from __future__ import annotations

import base64
import copy
import hashlib
import json
from pathlib import Path
import re
import secrets
import socket
import time
from urllib.parse import quote

import docker
import ingress
from storage import InstallError, Store, digest, document

PROTOCOLS = ("mtproto", "hysteria", "outline-wss", "amneziawg")
ALIASES = {
    "mtproto": "mtproto",
    "mtg": "mtproto",
    "hysteria": "hysteria",
    "hysteria2": "hysteria",
    "outline": "outline-wss",
    "outline-wss": "outline-wss",
    "amneziawg": "amneziawg",
    "amneziawg2": "amneziawg",
    "awg": "amneziawg",
    "awg2": "amneziawg",
}
DEFAULT_CAMOUFLAGE = "www.microsoft.com"
DEFAULT_HYSTERIA_PORT = 8443
DEFAULT_OUTLINE_BINARY = "/opt/hton/bin/outline-ss-server"
INTERNAL_PORTS = (40000, 49999)
PUBLIC_UDP_PORTS = (5000, 6000)
OUTLINE_CIPHER = "chacha20-ietf-poly1305"
OUTLINE_SHA256 = "afeca868e0e65d658a53dace9128588e0a8049295b358dbdbe2015ad011513fe"
AMNEZIAWG_IMAGE_ROLE = "amneziawg"
AMNEZIAWG_INTERFACE = "hton0"
AMNEZIAWG_CONFIG_DIR = "/etc/amnezia/amneziawg"
AMNEZIAWG_CONFIG_FILE = AMNEZIAWG_INTERFACE + ".conf"
AMNEZIAWG_CONFIG_PATH = AMNEZIAWG_CONFIG_DIR + "/" + AMNEZIAWG_CONFIG_FILE
AMNEZIAWG_CONTAINER_PORT = 51820
AMNEZIAWG_SERVER_ADDRESS = "10.66.66.1/24"
AMNEZIAWG_SUBNET = "10.66.66.0/24"
AMNEZIAWG_MTU = 1280
AMNEZIAWG_DNS = "1.1.1.1"
AMNEZIAWG_CLIENT_FILE = "secrets/amneziawg-peers.json"


def parse_selection(value, option):
    if value is None:
        return None
    items = [item.strip().lower() for item in value.split(",")]
    if not items or any(not item for item in items) or len(items) > len(PROTOCOLS):
        raise InstallError(option + " needs a bounded comma-separated protocol list")
    result = []
    for item in items:
        key = ALIASES.get(item)
        if key is None:
            raise InstallError(option + " accepts only mtproto, hysteria, outline and amneziawg")
        if key in result:
            raise InstallError(option + " contains a duplicate protocol")
        result.append(key)
    return set(result)


def selected(root, with_value, without_value, state=None):
    features = root.get("features") or {}
    if not isinstance(features, dict):
        raise InstallError("Saved feature selection is invalid; original retained")
    for key, value in features.items():
        if key == "foreign-outline":
            # Superseded by the HTON-owned outline-wss; retain a readable old
            # config without activating the foreign container path.
            continue
        if key not in PROTOCOLS and key != "foreign-amnezia":
            raise InstallError("Desired composition selects unsupported module " + str(key))
        if not isinstance(value, dict) or set(value) - {"enabled"} or not isinstance(value.get("enabled", False), bool):
            raise InstallError("Saved feature selection for " + str(key) + " is invalid")
    chosen = bool(state and state.get("protocolsChosen"))
    active = {module for module in PROTOCOLS if features.get(module, {}).get("enabled") is True}
    if not chosen and with_value is None and not any(module in features for module in PROTOCOLS):
        enabled = set(PROTOCOLS)
    else:
        enabled = set(active)
    add = parse_selection(with_value, "--with")
    remove = parse_selection(without_value, "--without")
    if add and remove and add & remove:
        raise InstallError("--with and --without select the same protocol")
    if add:
        enabled |= add
    if remove:
        enabled -= remove
    return enabled


def module_document(config, module):
    name = "modules/" + module + ".yaml"
    raw = config.read(name) if config else None
    if raw is None:
        return {"schemaVersion": 1, "moduleId": module, "settings": {}}
    doc = document(config, name)
    if (not isinstance(doc, dict) or doc.get("schemaVersion") != 1 or doc.get("moduleId") != module
            or not isinstance(doc.get("settings"), dict) or set(doc) != {"schemaVersion", "moduleId", "settings"}):
        raise InstallError("Unsupported " + module + " settings document; original retained")
    return doc


def _positive_port(value, label):
    if type(value) is not int or not 1 <= value <= 65535:
        raise InstallError(label + " must be a port between 1 and 65535")
    return value


def _saved(state, module):
    facts = state.get("protocols") if isinstance(state, dict) else None
    if not isinstance(facts, dict):
        return {}
    value = facts.get(module)
    return value if isinstance(value, dict) else {}


def _confirm(confirm, label, old, new):
    # --yes is an ordinary-prompt shortcut, not consent to change a saved
    # protocol association, camouflage name or listener.
    if not confirm(label + ": " + str(old) + " -> " + str(new) + ". Continue? [y/N]"):
        raise InstallError(label + " declined; saved configuration retained")


def _port_free(port, values, state):
    for value in values.values():
        if (value.get("Config", {}).get("Labels") or {}).get("io.hton.minimum") == state.get("instance"):
            continue
        for bindings in (value.get("HostConfig", {}).get("PortBindings") or {}).values():
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


def _foreign_binding(port, values, state):
    for value in values.values():
        if (value.get("Config", {}).get("Labels") or {}).get("io.hton.minimum") == state.get("instance"):
            continue
        for bindings in (value.get("HostConfig", {}).get("PortBindings") or {}).values():
            if any(int(binding.get("HostPort") or 0) == port for binding in bindings or []):
                return True
    return False


def _mtproto_port(state, values):
    saved = state.get("mtprotoPort")
    if saved is not None:
        _positive_port(saved, "Saved MTProto port")
        # The saved listener may belong to our own running mtg container, so do
        # not bind-probe it; only a foreign Docker publication can force a stop.
        if _foreign_binding(saved, values, state):
            raise InstallError("Saved MTProto internal port is occupied; no reroll performed")
        return saved
    low, high = INTERNAL_PORTS
    choices = list(range(low, high + 1))
    secrets.SystemRandom().shuffle(choices)
    port = next((candidate for candidate in choices if _port_free(candidate, values, state)), None)
    if port is None:
        raise InstallError("No free MTProto internal port in 40000-49999")
    return port


def mtproto_port(state, values):
    """Return the persisted MTProto loopback port without generating secrets."""
    return _mtproto_port(state, values)


def _selfsteal_port(state, values):
    saved = state.get("selfstealPort")
    if saved is not None:
        _positive_port(saved, "Saved self-steal port")
        if _foreign_binding(saved, values, state):
            raise InstallError("Saved self-steal port is occupied; no reroll performed")
        return saved
    low, high = INTERNAL_PORTS
    choices = list(range(low, high + 1))
    secrets.SystemRandom().shuffle(choices)
    port = next((candidate for candidate in choices if _port_free(candidate, values, state)), None)
    if port is None:
        raise InstallError("No free self-steal port in 40000-49999")
    return port


def _reality_inbound(native, purpose):
    if not isinstance(native, dict):
        raise InstallError("Native REALITY config is unavailable for " + purpose)
    config = copy.deepcopy(native)
    inbounds = config.get("inbounds")
    if not isinstance(inbounds, list) or len(inbounds) != 1 or not isinstance(inbounds[0], dict):
        raise InstallError(purpose + " requires the single supported native REALITY inbound")
    inbound = inbounds[0]
    if inbound.get("protocol") != "vless" or inbound.get("streamSettings", {}).get("security") != "reality":
        raise InstallError(purpose + " requires the native VLESS REALITY inbound")
    if not inbound.get("settings", {}).get("clients"):
        raise InstallError(purpose + " requires the native REALITY clients")
    return config, inbound


def selfsteal_config(native, chain_sni, port, mtg_port):
    """Copy native REALITY keys/clients and terminate the chain in mtg."""
    config, inbound = _reality_inbound(native, "Self-steal")
    inbound["listen"] = "127.0.0.1"
    inbound["port"] = port
    stream = inbound.setdefault("streamSettings", {})
    reality = stream.setdefault("realitySettings", {})
    reality["serverNames"] = [chain_sni]
    reality["dest"] = "127.0.0.1:" + str(mtg_port)
    reality.pop("target", None)
    config["log"] = {"loglevel": "warning"}
    return json.dumps(config, indent=2).encode()


def native_selfsteal_config(native, names, dest):
    """Point the installed native REALITY listener at the internal site chain.

    Keys, short IDs, flow, clients and the listener port stay byte-identical.
    Only ``serverNames`` and ``dest`` change so exported Amnezia profiles use
    the owner's own SNI while unauthenticated bytes reach the local site.
    """
    if not isinstance(names, list) or not names or len(names) > 8:
        raise InstallError("Native self-steal requires one to eight server names")
    if any(not docker.valid_domain(name) for name in names):
        raise InstallError("Native self-steal server names must be valid DNS names")
    if not isinstance(dest, str):
        raise InstallError("Native self-steal destination is invalid")
    host, separator, port = dest.rpartition(":")
    if not separator or not host or not port.isdigit() or not 1 <= int(port) <= 65535:
        raise InstallError("Native self-steal destination must be host:port")
    try:
        socket.inet_aton(host)
    except OSError:
        if not docker.valid_domain(host):
            raise InstallError("Native self-steal destination host is invalid")
    config, inbound = _reality_inbound(native, "Native self-steal")
    reality = inbound["streamSettings"].setdefault("realitySettings", {})
    reality["serverNames"] = list(names)
    reality["dest"] = dest
    reality.pop("target", None)
    config["log"] = {"loglevel": "warning"}
    return json.dumps(config, indent=2).encode()


def _outline_web_port(state, values):
    saved = state.get("outlineWebPort")
    if saved is not None:
        _positive_port(saved, "Saved Outline web port")
        if _foreign_binding(saved, values, state):
            raise InstallError("Saved Outline web port is occupied; no reroll performed")
        return saved
    low, high = INTERNAL_PORTS
    choices = list(range(low, high + 1))
    secrets.SystemRandom().shuffle(choices)
    port = next((candidate for candidate in choices if _port_free(candidate, values, state)), None)
    if port is None:
        raise InstallError("No free Outline web port in 40000-49999")
    return port


def _outline_binary(args, saved):
    value = getattr(args, "outline_binary", None) or saved.get("binary") or DEFAULT_OUTLINE_BINARY
    if not value:
        raise InstallError("The pinned outline-ss-server binary path is unavailable")
    path = Path(value)
    if path.is_symlink() or not path.is_file():
        raise InstallError("Outline binary is not a regular file")
    if path.stat().st_size > 32 * 1024 * 1024:
        raise InstallError("Outline binary exceeds the supported size")
    mode = path.stat().st_mode
    if mode & 0o111 == 0 or mode & 0o444 == 0:
        raise InstallError("Outline binary must be readable and executable by the container")
    observed = hashlib.sha256(path.read_bytes()).hexdigest()
    if observed != OUTLINE_SHA256:
        raise InstallError("Outline binary digest differs from the pinned v1.9.2 build")
    return str(path), observed


def _outline_secret(work):
    def valid(value):
        return (isinstance(value, str) and len(value) == 43
                and all(character.isalnum() or character in "-_" for character in value))
    return _secret(work, "outline-secret", lambda: secrets.token_urlsafe(32), valid)


def _outline_path(work, name):
    def valid(value):
        return (isinstance(value, str) and len(value) == 49 and value.startswith("/")
                and all(character in "0123456789abcdef" for character in value[1:]))
    return _secret(work, name, lambda: "/" + secrets.token_hex(24), valid)


def _outline_users(work, native_server):
    existing = work.read("secrets/outline-users.json")
    if existing is not None:
        try:
            users = json.loads(existing)
        except (ValueError, UnicodeError) as error:
            raise InstallError("Saved Outline users file is invalid") from error
        if (not isinstance(users, list) or not users or len(users) > 256
                or any(not isinstance(user, dict) or set(user) != {"name", "secret"}
                    or not isinstance(user["name"], str) or not user["name"]
                    or len(user["name"]) > 64
                    or any(not (character.isalnum() or character in "._-") for character in user["name"])
                    or not isinstance(user["secret"], str) or len(user["secret"]) != 43
                    for user in users)
                or len({user["name"] for user in users}) != len(users)):
            raise InstallError("Saved Outline users file is invalid")
        return users
    names = []
    if isinstance(native_server, dict):
        for client in (native_server.get("inbounds") or [{}])[0].get("settings", {}).get("clients", []):
            suffix = str(client.get("id", ""))[-4:]
            if suffix:
                names.append("amnezia-" + suffix)
    seen, unique = set(), []
    for name in names or ["user-01"]:
        candidate, index = name, 2
        while candidate in seen:
            candidate, index = name + "-" + str(index), index + 1
        seen.add(candidate)
        unique.append(candidate)
    users = [{"name": name, "secret": secrets.token_urlsafe(32)} for name in unique]
    work.write("secrets/outline-users.json", (json.dumps(users, indent=2) + "\n").encode())
    return users


def _outline_metrics_port(state, values):
    saved = state.get("outlineMetricsPort")
    if saved is not None:
        _positive_port(saved, "Saved Outline metrics port")
        if _foreign_binding(saved, values, state):
            raise InstallError("Saved Outline metrics port is occupied; no reroll performed")
        return saved
    low, high = INTERNAL_PORTS
    choices = list(range(low, high + 1))
    secrets.SystemRandom().shuffle(choices)
    port = next((candidate for candidate in choices if _port_free(candidate, values, state)), None)
    if port is None:
        raise InstallError("No free Outline metrics port in 40000-49999")
    return port


def outline_config(secret, web_port, tcp_path, udp_path, users=None):
    entries = users or [{"name": "guest", "secret": secret}]
    keys = "".join(
        "      - id: " + user["name"] + "\n"
        + "        cipher: " + OUTLINE_CIPHER + "\n"
        + "        secret: \"" + user["secret"] + "\"\n"
        for user in entries)
    return ("web:\n"
        + "  servers:\n"
        + "    - id: hton\n"
        + "      listen:\n"
        + "        - \"127.0.0.1:" + str(web_port) + "\"\n"
        + "services:\n"
        + "  - listeners:\n"
        + "      - type: websocket-stream\n"
        + "        web_server: hton\n"
        + "        path: \"" + tcp_path + "\"\n"
        + "      - type: websocket-packet\n"
        + "        web_server: hton\n"
        + "        path: \"" + udp_path + "\"\n"
        + "    keys:\n" + keys).encode()


def outline_key(domain, value, secret=None, name=None):
    secret = secret or value["secret"]
    return ("transport:\n"
        "  $type: tcpudp\n"
        "  tcp:\n"
        "    $type: shadowsocks\n"
        "    endpoint:\n"
        "      $type: websocket\n"
        "      url: wss://" + domain + value["tcpPath"] + "\n"
        "    cipher: " + OUTLINE_CIPHER + "\n"
        "    secret: \"" + secret + "\"\n"
        "  udp:\n"
        "    $type: shadowsocks\n"
        "    endpoint:\n"
        "      $type: websocket\n"
        "      url: wss://" + domain + value["udpPath"] + "\n"
        "    cipher: " + OUTLINE_CIPHER + "\n"
        "    secret: \"" + secret + "\"\n").encode()


def _secret(work, name, generator, validate):
    existing = work.read("secrets/" + name)
    if existing is not None:
        try:
            value = existing.decode("ascii").strip()
        except UnicodeDecodeError as error:
            raise InstallError("Saved " + name + " has invalid encoding") from error
        if not validate(value):
            raise InstallError("Saved " + name + " is invalid; explicit review is required")
        return value
    value = generator()
    if not validate(value):
        raise InstallError("Generated " + name + " is invalid")
    work.write("secrets/" + name, value.encode() + b"\n")
    return value


def _mtg_secret(work, image, camouflage):
    expected = camouflage.encode("ascii").hex()

    def valid(value):
        return (isinstance(value, str) and len(value) <= 512
                and value[:2] in ("ee", "dd")
                and all(character in "0123456789abcdef" for character in value[2:])
                and value[2:].lower().endswith(expected))

    def generate():
        return docker.command("run", "--rm", "--pull", "never", "--network", "none", "--read-only",
            "--user", "0:0", "--cap-drop", "ALL", "--security-opt", "no-new-privileges:true",
            "--entrypoint", "/mtg", image, "generate-secret", "--hex", camouflage, limit=65536).decode().strip()

    legacy = work.read("secrets/mtproto-secret")
    if legacy is not None:
        try:
            value = legacy.decode("ascii").strip()
        except UnicodeDecodeError as error:
            raise InstallError("Saved mtproto-secret has invalid encoding") from error
        if valid(value):
            return value, "secrets/mtproto-secret"
    relative = "secrets/mtproto-secret-" + digest(camouflage.encode())[:12]
    existing = work.read(relative)
    if existing is not None:
        try:
            value = existing.decode("ascii").strip()
        except UnicodeDecodeError as error:
            raise InstallError("Saved MTProto secret has invalid encoding") from error
        if valid(value):
            return value, relative
    value = generate()
    if not valid(value):
        raise InstallError("Generated MTProto secret does not match its camouflage domain")
    work.write(relative, value.encode() + b"\n")
    return value, relative


def _hysteria_password(work):
    def valid(value):
        return isinstance(value, str) and 32 <= len(value) <= 128 and all(
            character in "0123456789abcdef" for character in value)
    return _secret(work, "hysteria-password", lambda: secrets.token_hex(32), valid)


# ---------------------------------------------------------------------------
# AmneziaWG2: userspace UDP endpoint, per-peer keys and .conf export
# ---------------------------------------------------------------------------

def _awg_port(state, values):
    saved = state.get("amneziawgPort")
    if saved is not None:
        _awg_public_port(saved)
        if _foreign_binding(saved, values, state):
            raise InstallError("Saved AmneziaWG2 UDP port is occupied; no reroll performed")
        return saved
    low, high = PUBLIC_UDP_PORTS
    choices = list(range(low, high + 1))
    secrets.SystemRandom().shuffle(choices)
    port = next((candidate for candidate in choices if _port_free(candidate, values, state)), None)
    if port is None:
        raise InstallError("No free AmneziaWG2 UDP port in 5000-6000")
    return port


def _awg_public_port(value):
    low, high = PUBLIC_UDP_PORTS
    if type(value) is not int or not low <= value <= high:
        raise InstallError("AmneziaWG2 public UDP port must be in 5000-6000")
    return value


def _awg_keypair():
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric.x25519 import X25519PrivateKey
    raw = bytearray(secrets.token_bytes(32))
    raw[0] &= 248
    raw[31] &= 127
    raw[31] |= 64
    key = X25519PrivateKey.from_private_bytes(bytes(raw))
    public = key.public_key().public_bytes(serialization.Encoding.Raw,
        serialization.PublicFormat.Raw)
    return base64.b64encode(bytes(raw)).decode(), base64.b64encode(public).decode()


def _awg_public_key(private):
    if not _awg_key(private):
        raise InstallError("AmneziaWG2 private key is invalid")
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric.x25519 import X25519PrivateKey
    raw = base64.b64decode(private, validate=True)
    key = X25519PrivateKey.from_private_bytes(raw)
    public = key.public_key().public_bytes(serialization.Encoding.Raw,
        serialization.PublicFormat.Raw)
    return base64.b64encode(public).decode()


def _awg_key(value):
    if not isinstance(value, str) or len(value) != 44 or not value.endswith("="):
        return False
    try:
        raw = base64.b64decode(value, validate=True)
    except (ValueError, TypeError):
        return False
    return len(raw) == 32


def _awg_preshared():
    return base64.b64encode(secrets.token_bytes(32)).decode()


def _awg_params():
    rng = secrets.SystemRandom()
    jmin = rng.randint(32, 80)
    jmax = jmin + rng.randint(8, 48)
    params = {
        "Jc": rng.randint(4, 12),
        "Jmin": jmin,
        "Jmax": jmax,
        "S1": rng.randint(24, 96),
        "S2": rng.randint(24, 96),
        "S3": rng.randint(24, 96),
        "S4": rng.randint(24, 96),
    }
    cursor = 1
    for index in range(1, 5):
        start = cursor + rng.randint(1000, 100000)
        end = start + rng.randint(100, 5000)
        params["H" + str(index)] = str(start) + "-" + str(end)
        cursor = end
    return params


def _awg_valid_params(value):
    keys = {"Jc", "Jmin", "Jmax", "S1", "S2", "S3", "S4",
        "H1", "H2", "H3", "H4"}
    if not isinstance(value, dict) or set(value) != keys:
        return False
    for key in ("Jc", "Jmin", "Jmax", "S1", "S2", "S3", "S4"):
        if type(value[key]) is not int or not 0 <= value[key] <= 65535:
            return False
    if value["Jmin"] > value["Jmax"] or value["Jmax"] >= AMNEZIAWG_MTU:
        return False
    if not 1 <= value["Jc"] <= 24:
        return False
    previous = 0
    for key in ("H1", "H2", "H3", "H4"):
        match = re.fullmatch(r"([0-9]{1,10})(?:-([0-9]{1,10}))?", str(value[key]))
        if not match:
            return False
        start = int(match.group(1))
        end = int(match.group(2) or start)
        if not 1 <= start <= end <= 4294967295 or start <= previous:
            return False
        previous = end
    return True


def _awg_peer_name(value):
    if not isinstance(value, str) or not 1 <= len(value) <= 64:
        raise InstallError("AmneziaWG2 peer name is invalid")
    if any(ord(character) < 32 or ord(character) == 127 for character in value):
        raise InstallError("AmneziaWG2 peer name is invalid")
    return value


def _awg_names(native_server):
    """Reuse the native client identities so one user keeps every protocol."""
    names = []
    if isinstance(native_server, dict):
        try:
            clients = native_server["inbounds"][0]["settings"]["clients"]
        except (KeyError, IndexError, TypeError):
            clients = []
        if not isinstance(clients, list):
            clients = []
        for item in clients:
            if not isinstance(item, dict):
                continue
            name = item.get("email")
            if not isinstance(name, str) or not name:
                identifier = item.get("id")
                name = "client-" + digest(str(identifier).encode())[:8] if identifier else "guest"
            try:
                names.append(_awg_peer_name(name))
            except InstallError:
                continue
    unique = list(dict.fromkeys(names)) or ["guest"]
    if len(unique) > 253:
        raise InstallError("AmneziaWG2 supports at most 253 peers")
    return unique


def _awg_address(index):
    return "10.66.66." + str(index + 2)


def _awg_material(work, names):
    """Load or create the server key, obfuscation profile and peer keys."""
    raw = work.read(AMNEZIAWG_CLIENT_FILE)
    if raw is None:
        material = {"schemaVersion": 1, "serverPrivateKey": None,
            "serverPublicKey": None, "params": _awg_params(), "peers": []}
    else:
        try:
            material = json.loads(raw)
        except (ValueError, UnicodeError) as error:
            raise InstallError("Saved AmneziaWG2 peer file is invalid; explicit review is required") from error
        if (not isinstance(material, dict) or material.get("schemaVersion") != 1
                or set(material) != {"schemaVersion", "serverPrivateKey", "serverPublicKey", "params", "peers"}
                or not isinstance(material.get("peers"), list)
                or not _awg_key(material.get("serverPrivateKey"))
                or not _awg_key(material.get("serverPublicKey"))
                or not _awg_valid_params(material.get("params"))):
            raise InstallError("Saved AmneziaWG2 peer file is invalid; explicit review is required")
        if material.get("serverPublicKey") != _awg_public_key(material.get("serverPrivateKey")):
            raise InstallError("Saved AmneziaWG2 server key pair is invalid; explicit review is required")
    if material["serverPrivateKey"] is None:
        material["serverPrivateKey"], material["serverPublicKey"] = _awg_keypair()
    saved = {}
    for peer in material["peers"]:
        address = peer.get("address") if isinstance(peer, dict) else None
        match = re.fullmatch(r"10\.66\.66\.([0-9]{1,3})", address) if isinstance(address, str) else None
        if (not isinstance(peer, dict)
                or set(peer) != {"name", "privateKey", "publicKey", "presharedKey", "address"}
                or not _awg_key(peer.get("privateKey")) or not _awg_key(peer.get("publicKey"))
                or not _awg_key(peer.get("presharedKey"))
                or peer.get("publicKey") != _awg_public_key(peer.get("privateKey"))
                or match is None or not 2 <= int(match.group(1)) <= 254):
            raise InstallError("Saved AmneziaWG2 peer is invalid; explicit review is required")
        _awg_peer_name(peer["name"])
        if peer["name"] in saved:
            raise InstallError("Saved AmneziaWG2 peer file contains duplicate names")
        saved[peer["name"]] = peer
    peers, used = [], set()
    for name in names:
        peer = saved.get(name)
        if peer is None:
            private, public = _awg_keypair()
            peer = {"name": name, "privateKey": private, "publicKey": public,
                "presharedKey": _awg_preshared(), "address": None}
        address = peer.get("address")
        if address is not None:
            if address in used:
                raise InstallError("Saved AmneziaWG2 peer addresses collide; explicit review is required")
            used.add(address)
        peers.append(peer)
    for peer in peers:
        if peer.get("address") is None:
            candidate = next((_awg_address(index) for index in range(253)
                if _awg_address(index) not in used), None)
            if candidate is None:
                raise InstallError("AmneziaWG2 peer address pool is exhausted")
            peer["address"] = candidate
            used.add(candidate)
    material["peers"] = peers
    if raw is None or json.loads(raw) != material:
        work.write(AMNEZIAWG_CLIENT_FILE, (json.dumps(material, indent=2) + "\n").encode())
    return material


def _awg_interface_lines(material):
    params = material["params"]
    lines = ["PrivateKey = " + material["serverPrivateKey"]]
    lines.append("Address = " + AMNEZIAWG_SERVER_ADDRESS)
    return lines, params


def _awg_common_params(params):
    keys = ("Jc", "Jmin", "Jmax", "S1", "S2", "S3", "S4", "H1", "H2", "H3", "H4")
    return [key + " = " + str(params[key]) for key in keys]


def awg_server_config(material):
    if not _awg_valid_params(material.get("params")):
        raise InstallError("AmneziaWG2 obfuscation parameters are invalid")
    lines = ["[Interface]"]
    interface, params = _awg_interface_lines(material)
    lines += interface
    lines += ["ListenPort = " + str(AMNEZIAWG_CONTAINER_PORT), "MTU = " + str(AMNEZIAWG_MTU)]
    lines += _awg_common_params(params)
    for peer in material["peers"]:
        lines += ["", "[Peer]", "# " + peer["name"],
            "PublicKey = " + peer["publicKey"],
            "PresharedKey = " + peer["presharedKey"],
            "AllowedIPs = " + peer["address"] + "/32"]
    return ("\n".join(lines) + "\n").encode()


def awg_client_config(material, peer, domain, port):
    if not docker.valid_domain(domain):
        raise InstallError("AmneziaWG2 endpoint host is invalid")
    _positive_port(port, "AmneziaWG2 UDP port")
    lines = ["[Interface]",
        "PrivateKey = " + peer["privateKey"],
        "Address = " + peer["address"] + "/32",
        "DNS = " + AMNEZIAWG_DNS,
        "MTU = " + str(AMNEZIAWG_MTU)]
    lines += _awg_common_params(material["params"])
    lines += ["", "[Peer]",
        "PublicKey = " + material["serverPublicKey"],
        "PresharedKey = " + peer["presharedKey"],
        "AllowedIPs = 0.0.0.0/0",
        "Endpoint = " + domain + ":" + str(port),
        "PersistentKeepalive = 25"]
    return ("\n".join(lines) + "\n").encode()


def awg_client_text(payload):
    """Strictly accept only a client .conf, never the server configuration."""
    if isinstance(payload, str):
        try:
            payload = payload.encode("ascii")
        except UnicodeEncodeError as error:
            raise InstallError("AmneziaWG2 client configuration is not ASCII") from error
    if not isinstance(payload, (bytes, bytearray)) or len(payload) > 65536 or b"\0" in payload:
        raise InstallError("AmneziaWG2 client configuration is invalid")
    try:
        text = bytes(payload).decode("ascii", "strict")
    except UnicodeDecodeError as error:
        raise InstallError("AmneziaWG2 client configuration is not ASCII") from error
    section, sections, seen = None, [], {}
    interface_keys = {"PrivateKey", "Address", "DNS", "MTU",
        "Jc", "Jmin", "Jmax", "S1", "S2", "S3", "S4", "H1", "H2", "H3", "H4"}
    peer_keys = {"PublicKey", "PresharedKey", "AllowedIPs", "Endpoint", "PersistentKeepalive"}
    for line in text.splitlines():
        line = line.split("#", 1)[0].strip()
        if not line:
            continue
        if line in ("[Interface]", "[Peer]"):
            section = line[1:-1]
            sections.append(section)
            seen[section] = set()
            continue
        key, separator, value = line.partition("=")
        key, value = key.strip(), value.strip()
        if not separator or not value or section is None or key in seen[section]:
            raise InstallError("AmneziaWG2 client configuration is invalid")
        if key not in (interface_keys if section == "Interface" else peer_keys):
            raise InstallError("AmneziaWG2 client configuration contains an unsupported field")
        seen[section].add(key)
        if key in ("PrivateKey", "PublicKey", "PresharedKey"):
            if not _awg_key(value):
                raise InstallError("AmneziaWG2 client key is invalid")
        elif key == "Endpoint":
            host, _, port = value.rpartition(":")
            if not host or not port.isdigit() or not 1 <= int(port) <= 65535:
                raise InstallError("AmneziaWG2 client endpoint is invalid")
        elif key == "AllowedIPs":
            if any(not re.fullmatch(r"[0-9a-fA-F:./]+", item.strip())
                    for item in value.split(",")):
                raise InstallError("AmneziaWG2 client allowed IPs are invalid")
    if sections != ["Interface", "Peer"]:
        raise InstallError("AmneziaWG2 client configuration must have one interface and one peer")
    if not {"PrivateKey", "Address"} <= seen["Interface"]:
        raise InstallError("AmneziaWG2 client configuration lacks its interface key or address")
    if not {"PublicKey", "AllowedIPs", "Endpoint"} <= seen["Peer"]:
        raise InstallError("AmneziaWG2 client configuration lacks its peer endpoint")
    if "ListenPort" in seen["Interface"]:
        raise InstallError("A server configuration cannot be exported as a guest artifact")
    return bytes(payload)


def mtg_config(secret, port, fronting_port=None, bind_address="127.0.0.1"):
    content = ('secret = "' + secret + '"\n'
            + 'bind-to = "' + bind_address + ':' + str(port) + '"\n'
            + 'concurrency = 8192\n')
    if fronting_port:
        content += ('[domain-fronting]\n'
            + 'host = "127.0.0.1"\n'
            + 'port = ' + str(fronting_port) + '\n')
    return content.encode()


def hysteria_config(domain, port, password):
    return ("listen: :" + str(port) + "\n"
            + "tls:\n  cert: /certs/fullchain.pem\n  key: /certs/key.pem\n"
            + "auth:\n  type: password\n  password: \"" + password + "\"\n"
            + "masquerade:\n  type: proxy\n  proxy:\n"
            + "    url: https://" + domain + "/\n    rewriteHost: true\n").encode()


def _write_artifact(work, directory, filename, content):
    name = directory + "/" + filename
    previous = work.read(name)
    if previous is not None and previous != content:
        raise InstallError("Immutable protocol artifact changed; original retained")
    if previous is None:
        work.write(name, content)
    return name


def plan(config, work, state, pins, domain, native_names, enabled, args, values,
         ask, confirm, native_server=None, chain_sni=None, site_port=None,
         migrate_native=False, bridge_gateway=None, mtg_port=None):
    """Return a validated, secret-bearing plan; no native service is changed."""
    if chain_sni is not None:
        if not docker.valid_domain(chain_sni):
            raise InstallError("Chain SNI must be a valid lower-case DNS name")
        if chain_sni != domain and not chain_sni.endswith("." + domain):
            raise InstallError("Chain SNI must be the site domain or its subdomain")
        if "mtproto" not in enabled:
            raise InstallError("Chain SNI requires MTProto activation")
    documents = {module: module_document(config, module) for module in PROTOCOLS}
    result = {"enabled": sorted(enabled), "documents": documents, "mtproto": None,
        "hysteria": None, "outline": None, "amneziawg": None, "selfsteal": None}
    facts = {"mtproto": None, "hysteria": None, "outline-wss": None,
        "amneziawg": None, "selfsteal": None}

    if "mtproto" in enabled:
        settings = documents["mtproto"]["settings"]
        backend = settings.get("backend", "mtg")
        if backend != "mtg":
            raise InstallError("The minimum profile supports only the mtg MTProto backend")
        saved_fact = _saved(state, "mtproto")
        previous = settings.get("camouflageName")
        camouflage = (chain_sni or getattr(args, "mtproto_camouflage", None)
            or previous or saved_fact.get("camouflage") or domain)
        if not camouflage:
            if getattr(args, "json", False):
                raise InstallError("MTProto camouflage domain is unavailable")
            camouflage = ask("MTProto camouflage domain", DEFAULT_CAMOUFLAGE)
        if not docker.valid_domain(camouflage):
            raise InstallError("MTProto camouflage must be a valid lower-case DNS name")
        if (camouflage in native_names and camouflage != chain_sni) or (camouflage == domain and not chain_sni):
            raise InstallError("MTProto camouflage must differ from the site and REALITY SNI")
        old = previous or saved_fact.get("camouflage")
        if old and camouflage != old:
            _confirm(confirm, "Change MTProto camouflage", old, camouflage)
        settings.update({"backend": "mtg", "camouflageName": camouflage})
        port = mtg_port if mtg_port is not None else _mtproto_port(state, values)
        _positive_port(port, "MTProto port")
        bind = "127.0.0.1"
        if migrate_native and chain_sni:
            if not isinstance(bridge_gateway, str) or not bridge_gateway:
                raise InstallError("Native self-steal requires the Docker bridge gateway")
            try:
                socket.inet_aton(bridge_gateway)
            except OSError:
                raise InstallError("Docker bridge gateway must be an IPv4 address")
            bind = bridge_gateway
        fronting = (state.get("sitePort") or site_port or 9443) if chain_sni else None
        secret, secret_file = _mtg_secret(work, pins["mtg"], camouflage)
        content = mtg_config(secret, port, fronting, bind)
        directory = "protocols/mtproto-" + digest(content)[:24]
        filename = _write_artifact(work, directory, "mtg.toml", content)
        result["mtproto"] = {"camouflage": camouflage, "port": port, "bind": bind,
            "secret": secret, "server": chain_sni or domain, "directory": directory,
            "files": [filename], "secretFile": secret_file,
            "digest": digest(content + secret.encode())}
        facts["mtproto"] = {"camouflage": camouflage, "port": port,
            "secretDigest": digest(secret.encode())}
        if bind != "127.0.0.1":
            facts["mtproto"]["bind"] = bind

    if "hysteria" in enabled:
        settings = documents["hysteria"]["settings"]
        saved_fact = _saved(state, "hysteria")
        listen = settings.get("listen") if isinstance(settings.get("listen"), dict) else {}
        previous = listen.get("port")
        requested = (getattr(args, "hysteria_port", None) or previous
            or saved_fact.get("port") or DEFAULT_HYSTERIA_PORT)
        port = _positive_port(requested, "Hysteria UDP port")
        old = previous or saved_fact.get("port")
        if old and port != old:
            _confirm(confirm, "Change Hysteria UDP port", old, port)
        if settings.get("hostname") and settings["hostname"] != domain:
            raise InstallError("Saved Hysteria hostname differs from the site domain; explicit review is required")
        settings.update({"listen": {"transport": "udp", "port": port, "scope": "public",
            "addressFamilies": ["ipv4"]}, "hostname": domain})
        password = _hysteria_password(work)
        content = hysteria_config(domain, port, password)
        directory = "protocols/hysteria-" + digest(content)[:24]
        filename = _write_artifact(work, directory, "config.yaml", content)
        result["hysteria"] = {"domain": domain, "port": port, "password": password,
            "directory": directory, "files": [filename], "secretFile": "secrets/hysteria-password",
            "digest": digest(content + password.encode())}
        facts["hysteria"] = {"hostname": domain, "port": port,
            "passwordDigest": digest(password.encode())}

    if "outline-wss" in enabled:
        settings = documents["outline-wss"]["settings"]
        saved_fact = _saved(state, "outline-wss")
        binary, binary_digest = _outline_binary(args, saved_fact)
        users = _outline_users(work, native_server)
        secret = users[0]["secret"]
        tcp_path = _outline_path(work, "outline-tcp-path")
        udp_path = _outline_path(work, "outline-udp-path")
        web_port = _outline_web_port(state, values)
        metrics_port = _outline_metrics_port(state, values)
        content = outline_config(secret, web_port, tcp_path, udp_path, users)
        directory = "protocols/outline-" + digest(content)[:24]
        filename = _write_artifact(work, directory, "outline.yaml", content)
        settings.update({"streamPath": {"secretRef": "outline/tcp-path"},
            "packetPath": {"secretRef": "outline/udp-path"}})
        result["outline"] = {"secret": secret, "users": users, "tcpPath": tcp_path,
            "udpPath": udp_path, "webPort": web_port, "metricsPort": metrics_port,
            "binary": binary, "host": chain_sni or domain, "directory": directory,
            "files": [filename, "secrets/outline-users.json",
                "secrets/outline-tcp-path", "secrets/outline-udp-path"],
            "secretFile": "secrets/outline-users.json",
            "digest": digest(content + json.dumps(users, sort_keys=True).encode())}
        facts["outline-wss"] = {"webPort": web_port, "metricsPort": metrics_port,
            "binary": binary, "binaryDigest": binary_digest,
            "userDigests": {user["name"]: digest(user["secret"].encode()) for user in users},
            "tcpPathDigest": digest(tcp_path.encode()), "udpPathDigest": digest(udp_path.encode())}

    if "amneziawg" in enabled:
        settings = documents["amneziawg"]["settings"]
        image = pins.get(AMNEZIAWG_IMAGE_ROLE)
        if not image:
            raise InstallError("The image lock has no pinned AmneziaWG2 image")
        listen = settings.get("listen") if isinstance(settings.get("listen"), dict) else {}
        previous = listen.get("port")
        if previous is not None:
            _awg_public_port(previous)
            if _foreign_binding(previous, values, state):
                raise InstallError("Saved AmneziaWG2 UDP port is occupied; no reroll performed")
            recorded = _saved(state, "amneziawg").get("port")
            if recorded is not None and recorded != previous:
                _confirm(confirm, "Change AmneziaWG2 UDP port", recorded, previous)
            port = previous
        else:
            port = _awg_port(state, values)
        if settings.get("hostname") and settings["hostname"] != domain:
            raise InstallError("Saved AmneziaWG2 hostname differs from the site domain; explicit review is required")
        names = _awg_names(native_server)
        material = _awg_material(work, names)
        server = awg_server_config(material)
        directory = "protocols/amneziawg-" + digest(server + json.dumps(material["peers"], sort_keys=True).encode())[:24]
        filename = _write_artifact(work, directory, AMNEZIAWG_CONFIG_FILE, server)
        clients = {peer["name"]: awg_client_config(material, peer, domain, port)
            for peer in material["peers"]}
        settings.update({"backend": "amneziawg-go",
            "listen": {"transport": "udp", "port": port, "scope": "public",
                "addressFamilies": ["ipv4"]},
            "hostname": domain, "subnet": AMNEZIAWG_SUBNET})
        result["amneziawg"] = {"port": port, "interface": AMNEZIAWG_INTERFACE,
            "server": server, "clients": clients, "peers": material["peers"],
            "directory": directory, "files": [filename], "domain": domain,
            "digest": digest(server + json.dumps(material["peers"], sort_keys=True).encode())}
        facts["amneziawg"] = {"port": port, "interface": AMNEZIAWG_INTERFACE,
            "serverDigest": digest(server),
            "peerDigests": {peer["name"]: digest(clients[peer["name"]]) for peer in material["peers"]}}

    if migrate_native:
        if not isinstance(bridge_gateway, str) or not bridge_gateway:
            raise InstallError("Native self-steal requires the Docker bridge gateway")
        own_name = chain_sni or domain
        names = list(dict.fromkeys([own_name, *native_names]))
        if chain_sni:
            if not result.get("mtproto"):
                raise InstallError("Chain SNI requires the MTProto plan")
            dest = bridge_gateway + ":" + str(result["mtproto"]["port"])
        else:
            dest = bridge_gateway + ":" + str(site_port or state.get("sitePort") or 9443)
        content = native_selfsteal_config(native_server, names, dest)
        result["native"] = {"names": names, "dest": dest,
            "content": content, "digest": digest(content)}
        facts["native"] = {"names": names, "dest": dest}

    if chain_sni and not migrate_native:
        if not result.get("mtproto"):
            raise InstallError("Chain SNI requires the MTProto plan")
        if not isinstance(native_server, dict):
            raise InstallError("Native REALITY config is unavailable for self-steal")
        port = _selfsteal_port(state, values)
        content = selfsteal_config(native_server, chain_sni, port, result["mtproto"]["port"])
        directory = "protocols/selfsteal-" + digest(content)[:24]
        filename = _write_artifact(work, directory, "xray.json", content)
        result["selfsteal"] = {"port": port, "chainSni": chain_sni,
            "directory": directory, "files": [filename],
            "digest": digest(content)}
        facts["selfsteal"] = {"port": port, "chainSni": chain_sni,
            "digest": digest(content)}

    result["facts"] = facts
    result["generation"] = digest(json.dumps(facts, sort_keys=True).encode())
    return result


def save_config(config, root, plan_value):
    for module in PROTOCOLS:
        config.save("modules/" + module + ".yaml", plan_value["documents"][module])
    features = root.setdefault("features", {})
    for module in PROTOCOLS:
        features.setdefault(module, {})["enabled"] = module in plan_value["enabled"]


def record_state(state, plan_value):
    state["protocols"] = plan_value["facts"]
    if plan_value["mtproto"]:
        state["mtprotoPort"] = plan_value["mtproto"]["port"]
    if plan_value.get("outline"):
        state["outlineWebPort"] = plan_value.get("outline")["webPort"]
        state["outlineMetricsPort"] = plan_value.get("outline")["metricsPort"]
    if plan_value.get("amneziawg"):
        state["amneziawgPort"] = plan_value["amneziawg"]["port"]
    if plan_value.get("selfsteal"):
        state["selfstealPort"] = plan_value.get("selfsteal")["port"]
    if plan_value.get("native"):
        state["nativeNames"] = plan_value["native"]["names"]
        state["nativeDest"] = plan_value["native"]["dest"]


def _container(work, state, role, image, entrypoint, arguments, mounts, files,
               capabilities=(), network="host", devices=(), publish=(), sysctls=(),
               environment=(), tmpfs=()):
    argv = ["container", "create", "--pull", "never", "--network", network, "--read-only",
        "--user", "0:0", "--cap-drop", "ALL"]
    argv += list(capabilities)
    argv += ["--security-opt", "no-new-privileges:true", "--restart", "no",
        "--tmpfs", "/tmp:rw,nosuid,noexec,size=16777216", "--log-driver", "none",
        "--label", "io.hton.minimum=" + state["instance"], "--label", "io.hton.role=" + role]
    for value in tmpfs:
        argv += ["--tmpfs", value]
    for device in devices:
        argv += ["--device", device]
    for mapping in publish:
        argv += ["--publish", mapping]
    for key, value in sysctls:
        argv += ["--sysctl", key + "=" + str(value)]
    for value in environment:
        argv += ["--env", value]
    for source, target in mounts:
        argv += ["--mount", "type=bind,src=" + str(source) + ",dst=" + target + ",readonly"]
    argv += ["--entrypoint", entrypoint, image, *arguments]
    identifier = docker.exact_id(docker.command(*argv, limit=65536).decode().strip())
    ingress.record_owned(work, state, identifier, role, files)
    return identifier


def _temporary(work, state, role, image, entrypoint, arguments, mounts, files=(), timeout=30,
               attach=True, settle=0.0, network="host", capabilities=(), devices=(),
               sysctls=(), environment=(), tmpfs=()):
    identifier = _container(work, state, role, image, entrypoint, arguments, mounts, files,
        network=network, capabilities=capabilities, devices=devices, sysctls=sysctls,
        environment=environment, tmpfs=tmpfs)
    state["validatorJob"] = identifier
    try:
        if attach:
            docker.command("container", "start", "--attach", identifier, timeout=timeout)
        else:
            docker.command("container", "start", identifier, timeout=timeout)
            if settle:
                time.sleep(settle)
        return docker.inspect(identifier)
    finally:
        ingress.owned(state, identifier, role, work)
        docker.command("container", "rm", "--force", identifier)
        state.pop("validatorJob", None)
        state["ownedReceipts"].pop(identifier, None)


def _mtg_validation_artifact(work, value):
    """Return a validation mount for a bridge-bound mtg config.

    A networkless validator cannot bind the host bridge address, so the
    structure is validated with the same bytes bound to loopback instead.
    """
    bind = value.get("bind", "127.0.0.1")
    if bind == "127.0.0.1":
        return value["directory"], value["files"], None
    source = work.root / value["directory"] / "mtg.toml"
    data = source.read_bytes()
    needle = ('bind-to = "' + bind + ":" + str(value["port"]) + '"').encode()
    if needle not in data:
        raise InstallError("MTProto validation source differs from its plan")
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    content = data.replace(needle, ('bind-to = "127.0.0.1:' + str(port) + '"').encode())
    directory = "native/validate-mtg-" + secrets.token_hex(8)
    relative = directory + "/mtg.toml"
    work.write(relative, content)
    return directory, [relative], relative


def validate(work, state, pins, plan_value, certificate):
    if plan_value["mtproto"]:
        value = plan_value["mtproto"]
        # mtg doctor's SNI-DNS check is unreliable on hosts whose resolver
        # returns CNAME chains; a real start proves config and listener instead.
        directory, files, temporary = _mtg_validation_artifact(work, value)
        try:
            observed = _temporary(work, state, "validator", pins["mtg"], "/mtg",
                ["run", "/etc/hton/mtg.toml"],
                [(work.root / directory, "/etc/hton")], files,
                timeout=10, attach=False, settle=1.5, network="none")
        finally:
            if temporary:
                path = work.root / temporary
                path.unlink(missing_ok=True)
                path.parent.rmdir()
        if not observed["State"].get("Running"):
            raise InstallError("MTProto native configuration validation failed")
    if plan_value["hysteria"]:
        value = plan_value["hysteria"]
        observed = _temporary(work, state, "validator", pins["hysteria"], "hysteria",
            ["server", "-c", "/etc/hysteria/config.yaml"],
            [(work.root / value["directory"], "/etc/hysteria"), (certificate, "/certs")],
            value["files"], timeout=10, attach=False, settle=1.5, network="none")
        if not observed["State"].get("Running"):
            raise InstallError("Hysteria native configuration validation failed")
    if plan_value.get("outline"):
        value = plan_value.get("outline")
        observed = _temporary(work, state, "validator", pins["alpine"],
            "/outline/outline-ss-server", ["-config", "/etc/hton/outline.yaml",
                "-metrics", "127.0.0.1:" + str(value["metricsPort"])],
            [(work.root / value["directory"], "/etc/hton"),
                (value["binary"], "/outline/outline-ss-server")],
            value["files"], timeout=10, attach=False, settle=1.5, network="none")
        if not observed["State"].get("Running"):
            raise InstallError("Outline native configuration validation failed")
    if plan_value.get("amneziawg"):
        value = plan_value["amneziawg"]
        observed = _temporary(work, state, "validator", pins[AMNEZIAWG_IMAGE_ROLE],
            "/bin/sh", ["-c", "awg-quick up " + AMNEZIAWG_CONFIG_PATH],
            [(work.root / value["directory"], AMNEZIAWG_CONFIG_DIR)],
            value["files"], timeout=20, attach=True, network="none",
            capabilities=["--cap-add", "NET_ADMIN"], devices=["/dev/net/tun:/dev/net/tun"],
            environment=["WG_QUICK_USERSPACE_IMPLEMENTATION=amneziawg-go"],
            tmpfs=["/run:rw,nosuid,noexec,size=16777216"])
        if observed["State"].get("ExitCode") != 0:
            raise InstallError("AmneziaWG2 native configuration validation failed")
    if plan_value.get("selfsteal"):
        value = plan_value.get("selfsteal")
        observed = _temporary(work, state, "validator", pins["xray"], "xray",
            ["run", "-config", "/etc/xray/xray.json"],
            [(work.root / value["directory"], "/etc/xray")], value["files"],
            timeout=10, attach=False, settle=1.5, network="none")
        if not observed["State"].get("Running"):
            raise InstallError("Self-steal REALITY configuration validation failed")


def cleanup(work, state, save):
    for identifier in list(state.get("retiredProtocols", [])):
        if identifier not in docker.inventory():
            state["retiredProtocols"].remove(identifier)
            state.get("ownedReceipts", {}).pop(identifier, None)
            save()
            continue
        receipt = (state.get("ownedReceipts") or {}).get(identifier)
        if not receipt:
            raise InstallError("Retired protocol container has no exact ownership receipt")
        role = receipt.get("role")
        if role not in ("mtproto", "hysteria", "outline-wss", "amneziawg", "selfsteal"):
            raise InstallError("Retired protocol container identity is invalid")
        ingress.owned(state, identifier, role, work)
        if docker.inspect(identifier)["State"]["Running"]:
            docker.command("container", "update", "--restart=no", identifier)
            docker.command("container", "stop", "--time", "10", identifier)
        docker.command("container", "rm", identifier)
        state["retiredProtocols"].remove(identifier)
        state.get("ownedReceipts", {}).pop(identifier, None)
        save()
    for role, identifier in list((state.get("candidateProtocols") or {}).items()):
        if role not in ("mtproto", "hysteria", "outline-wss", "amneziawg", "selfsteal"):
            raise InstallError("Candidate protocol role is invalid")
        if identifier not in docker.inventory():
            if identifier not in (state.get("ownedReceipts") or {}):
                raise InstallError("Abandoned protocol lacks its exact creation receipt")
            state["candidateProtocols"].pop(role)
            state.get("ownedReceipts", {}).pop(identifier)
            save()
            continue
        value = ingress.owned(state, identifier, role, work)
        if value["State"]["Running"]:
            raise InstallError("Uncommitted protocol is running; exact recovery is required")
        docker.command("container", "rm", identifier)
        state["candidateProtocols"].pop(role)
        state.get("ownedReceipts", {}).pop(identifier, None)
        save()
    _prune_obsolete_protocol_secrets(work, state)


def _prune_obsolete_protocol_secrets(work, state):
    referenced = {
        claim.get("path")
        for receipt in (state.get("ownedReceipts") or {}).values()
        for claim in receipt.get("files", [])
    }
    candidates = [work.root / "secrets/outline-secret",
        work.root / "secrets/mtproto-secret"]
    for path in candidates:
        relative = path.relative_to(work.root).as_posix()
        if relative in referenced or not path.exists():
            continue
        if path.is_symlink() or path.is_file():
            path.unlink()


def prepare(work, state, pins, plan_value, certificate, save):
    cleanup(work, state, save)
    state["candidateProtocols"] = {}
    save()
    if plan_value["mtproto"]:
        value = plan_value["mtproto"]
        identifier = _container(work, state, "mtproto", pins["mtg"], "/mtg",
            ["run", "/etc/hton/mtg.toml"],
            [(work.root / value["directory"], "/etc/hton")],
            value["files"] + [value["secretFile"]])
        state["candidateProtocols"]["mtproto"] = identifier
        save()
    if plan_value["hysteria"]:
        value = plan_value["hysteria"]
        identifier = _container(work, state, "hysteria", pins["hysteria"], "hysteria",
            ["server", "-c", "/etc/hysteria/config.yaml"],
            [(work.root / value["directory"], "/etc/hysteria"), (certificate, "/certs")],
            value["files"] + [value["secretFile"]], capabilities=["--cap-add", "NET_BIND_SERVICE"])
        state["candidateProtocols"]["hysteria"] = identifier
        save()
    if plan_value.get("amneziawg"):
        value = plan_value["amneziawg"]
        command = ("set -eu; "
            "awg-quick up " + AMNEZIAWG_CONFIG_PATH + "; "
            "iptables -w -A FORWARD -i " + value["interface"] + " -j ACCEPT; "
            "iptables -w -A FORWARD -o " + value["interface"] + " -j ACCEPT; "
            "iptables -w -t nat -A POSTROUTING -s " + AMNEZIAWG_SUBNET + " -o eth0 -j MASQUERADE; "
            "exec tail -f /dev/null")
        identifier = _container(work, state, "amneziawg", pins[AMNEZIAWG_IMAGE_ROLE],
            "/bin/sh", ["-c", command],
            [(work.root / value["directory"], AMNEZIAWG_CONFIG_DIR)],
            value["files"], network="bridge",
            capabilities=["--cap-add", "NET_ADMIN", "--cap-add", "NET_RAW"],
            devices=["/dev/net/tun:/dev/net/tun"],
            publish=[f"0.0.0.0:{value['port']}:{AMNEZIAWG_CONTAINER_PORT}/udp"],
            sysctls=[("net.ipv4.ip_forward", 1)],
            environment=["WG_QUICK_USERSPACE_IMPLEMENTATION=amneziawg-go"],
            tmpfs=["/run:rw,nosuid,noexec,size=16777216"])
        state["candidateProtocols"]["amneziawg"] = identifier
        save()
    if plan_value.get("outline"):
        value = plan_value.get("outline")
        identifier = _container(work, state, "outline-wss", pins["alpine"],
            "/outline/outline-ss-server", ["-config", "/etc/hton/outline.yaml",
                "-metrics", "127.0.0.1:" + str(value["metricsPort"])],
            [(work.root / value["directory"], "/etc/hton"),
                (value["binary"], "/outline/outline-ss-server")],
            value["files"])
        state["candidateProtocols"]["outline-wss"] = identifier
        save()
    if plan_value.get("selfsteal"):
        value = plan_value.get("selfsteal")
        identifier = _container(work, state, "selfsteal", pins["xray"], "xray",
            ["run", "-config", "/etc/xray/xray.json"],
            [(work.root / value["directory"], "/etc/xray")], value["files"])
        state["candidateProtocols"]["selfsteal"] = identifier
        save()


def stop_active(work, state):
    for role, identifier in (state.get("activeProtocols") or {}).items():
        ingress.owned(state, identifier, role, work)
        value = docker.inspect(identifier)
        docker.command("container", "update", "--restart=no", identifier)
        if value["State"]["Running"]:
            docker.command("container", "stop", "--time", "10", identifier)


def start_candidates(work, state):
    for role, identifier in (state.get("candidateProtocols") or {}).items():
        ingress.owned(state, identifier, role, work)
        docker.command("container", "start", identifier)


def start_chain(work, state, plan_value):
    """Start the internal chain before HAProxy sends the own SNI to it.

    Native self-steal forwards unauthenticated bytes to mtg; the legacy
    self-steal container is itself a HAProxy primary backend. Either must be
    running before route verification, otherwise the site probe fails while
    the native/self-steal backend is still healthy at TCP level.
    """
    roles = []
    if plan_value.get("native") and plan_value.get("mtproto"):
        roles.append("mtproto")
    if plan_value.get("selfsteal"):
        roles.append("selfsteal")
        if plan_value.get("mtproto"):
            roles.append("mtproto")
    if not roles:
        return
    for role in roles:
        identifier = (state.get("candidateProtocols") or {}).get(role)
        if not identifier:
            raise InstallError("Chain dependency has no prepared candidate")
        ingress.owned(state, identifier, role, work)
        docker.command("container", "start", identifier)
        deadline = time.monotonic() + 10
        while not docker.inspect(identifier)["State"].get("Running"):
            if time.monotonic() >= deadline:
                raise InstallError("Chain dependency did not stay running")
            time.sleep(0.2)


def rollback_candidates(work, state):
    for role, identifier in (state.get("candidateProtocols") or {}).items():
        value = ingress.owned(state, identifier, role, work)
        docker.command("container", "update", "--restart=no", identifier)
        if value["State"]["Running"]:
            docker.command("container", "stop", "--time", "10", identifier)


def restore_active(work, state):
    for role, identifier in (state.get("activeProtocols") or {}).items():
        value = ingress.owned(state, identifier, role, work)
        docker.command("container", "update", "--restart=always", identifier)
        if not value["State"]["Running"]:
            docker.command("container", "start", identifier)


def promote(work, state):
    """Make candidates durable; may fail, so state still has candidateProtocols."""
    for role, identifier in (state.get("candidateProtocols") or {}).items():
        ingress.owned(state, identifier, role, work)
        docker.command("container", "update", "--restart=always", identifier)


def accept(state):
    """Pure state transition after every native promotion succeeded."""
    prior = state.get("activeProtocols") or {}
    retired = [identifier for role, identifier in prior.items()
        if (state.get("candidateProtocols") or {}).get(role) != identifier]
    if retired:
        state["retiredProtocols"] = sorted(set(state.get("retiredProtocols") or []) | set(retired))
    state["activeProtocols"] = state.pop("candidateProtocols", {})


def _probe_mtproto(state, value):
    deadline = time.monotonic() + 10
    while True:
        try:
            with socket.create_connection((value.get("bind", "127.0.0.1"), value["port"]), timeout=1):
                return
        except OSError:
            if time.monotonic() >= deadline:
                raise InstallError("MTProto backend did not accept a connection")
            time.sleep(0.2)


def _probe_awg(state, value):
    identifier = (state.get("candidateProtocols") or {}).get("amneziawg") \
        or (state.get("activeProtocols") or {}).get("amneziawg")
    if not identifier:
        raise InstallError("AmneziaWG2 has no prepared native container")
    observed = docker.inspect(identifier)
    if not observed["State"].get("Running"):
        raise InstallError("AmneziaWG2 service did not stay running")
    listen_port = docker.command("container", "exec", identifier, "awg", "show",
        value["interface"], "listen-port", limit=32).strip()
    if listen_port != str(AMNEZIAWG_CONTAINER_PORT).encode():
        raise InstallError("AmneziaWG2 interface is not listening on the published target port")


def _probe_hysteria(work, state, pins, value, domain):
    with socket.socket() as available:
        available.bind(("127.0.0.1", 0))
        port = available.getsockname()[1]
    token = secrets.token_hex(12)
    directory = "clients/" + token
    config = ("server: 127.0.0.1:" + str(value["port"]) + "\n"
        + "auth: \"" + value["password"] + "\"\n"
        + "tls:\n  sni: " + domain + "\n"
        + "socks5:\n  listen: 127.0.0.1:" + str(port) + "\n").encode()
    work.write(directory + "/client.yaml", config)
    identifier = _container(work, state, "client", pins["hysteria"], "hysteria",
        ["client", "-c", "/client/client.yaml"], [(work.root / directory, "/client")],
        [directory + "/client.yaml"])
    state["clientJob"] = identifier
    try:
        docker.command("container", "start", identifier)
        deadline = time.monotonic() + 30
        while True:
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                    break
            except OSError:
                if time.monotonic() >= deadline:
                    raise InstallError("Hysteria client did not expose its private SOCKS listener")
                time.sleep(0.1)
        while True:
            try:
                status = docker.run(["/usr/bin/curl", "--disable", "--silent", "--show-error",
                    "--max-time", "15", "--connect-timeout", "5", "--noproxy", "",
                    "--proxy", "socks5h://127.0.0.1:" + str(port), "--max-filesize", "1048576",
                    "--proto", "=https", "--output", "/dev/null", "--write-out", "%{http_code}",
                    "https://" + domain + "/"], timeout=20).decode()
            except docker.NativeError:
                status = ""
            if status.isdigit() and 100 <= int(status) < 500:
                return
            if time.monotonic() >= deadline:
                raise InstallError("Authenticated Hysteria request failed")
            time.sleep(1)
    finally:
        ingress.owned(state, identifier, "client", work)
        docker.command("container", "rm", "--force", identifier)
        state.pop("clientJob", None)
        state.get("ownedReceipts", {}).pop(identifier, None)


def _probe_outline(value):
    deadline = time.monotonic() + 10
    while True:
        try:
            with socket.create_connection(("127.0.0.1", value["webPort"]), timeout=1):
                return
        except OSError:
            if time.monotonic() >= deadline:
                raise InstallError("Outline web listener did not accept a connection")
            time.sleep(0.2)


def _probe_selfsteal(value):
    deadline = time.monotonic() + 10
    while True:
        try:
            with socket.create_connection(("127.0.0.1", value["port"]), timeout=1):
                return
        except OSError:
            if time.monotonic() >= deadline:
                raise InstallError("Self-steal REALITY listener did not accept a connection")
            time.sleep(0.2)


def probe(work, state, pins, plan_value, domain):
    if plan_value["mtproto"]:
        _probe_mtproto(state, plan_value["mtproto"])
    if plan_value["hysteria"]:
        _probe_hysteria(work, state, pins, plan_value["hysteria"], domain)
    if plan_value.get("outline"):
        _probe_outline(plan_value.get("outline"))
    if plan_value.get("amneziawg"):
        _probe_awg(state, plan_value["amneziawg"])
    if plan_value.get("selfsteal"):
        _probe_selfsteal(plan_value.get("selfsteal"))


def verify_active(work, state, pins, plan_value, domain):
    for role, identifier in (state.get("activeProtocols") or {}).items():
        value = ingress.owned(state, identifier, role, work)
        if not value["State"]["Running"]:
            raise InstallError("Accepted " + role + " service is stopped; restore it before retrying")
    probe(work, state, pins, plan_value, domain)


def share_links(plan_value, domain):
    links = {}
    if plan_value["mtproto"]:
        server = plan_value["mtproto"].get("server", domain)
        links["MTProto"] = ("tg://proxy?server=" + server + "&port=443&secret="
            + quote(plan_value["mtproto"]["secret"]))
    if plan_value["hysteria"]:
        links["Hysteria"] = ("hysteria2://" + quote(plan_value["hysteria"]["password"])
            + "@" + domain + ":" + str(plan_value["hysteria"]["port"])
            + "/?sni=" + domain + "#Hysteria2")
    return links


def save_shares(work, plan_value, domain):
    paths = {}
    for label, link in share_links(plan_value, domain).items():
        name = "share/" + label.lower() + ".txt"
        work.write(name, (link + "\n").encode())
        paths[label] = str(work.root / name)
    if plan_value.get("outline"):
        value = plan_value.get("outline")
        users = value.get("users") or [{"name": "guest", "secret": value["secret"]}]
        summary = []
        for user in users:
            name = "share/outline-" + user["name"] + ".yaml"
            work.write(name, outline_key(value.get("host", domain), value,
                secret=user["secret"], name=user["name"]))
            paths["Outline " + user["name"]] = str(work.root / name)
            summary.append(user["name"] + ": " + str(work.root / name))
        summary_name = "share/outline-users.txt"
        work.write(summary_name, ("\n".join(summary) + "\n").encode())
        paths["Outline users"] = str(work.root / summary_name)
    if plan_value.get("amneziawg"):
        value = plan_value["amneziawg"]
        for peer in value["peers"]:
            name = "share/amneziawg-" + _awg_slug(peer["name"]) + ".conf"
            work.write(name, value["clients"][peer["name"]])
            paths["AmneziaWG " + peer["name"]] = str(work.root / name)
    return paths


def _awg_slug(name):
    cleaned = "".join(character if character.isalnum() or character in "-_" else "-"
        for character in str(name).lower()).strip("-")
    return cleaned or "peer"


def result(plan_value):
    value = {"modules": plan_value["enabled"]}
    if plan_value["mtproto"]:
        value["mtproto"] = {"camouflage": plan_value["mtproto"]["camouflage"],
            "backend": "127.0.0.1:" + str(plan_value["mtproto"]["port"])}
    if plan_value["hysteria"]:
        value["hysteria"] = {"hostname": plan_value["hysteria"]["domain"],
            "port": plan_value["hysteria"]["port"]}
    if plan_value.get("outline"):
        value["outline"] = {"webPort": plan_value.get("outline")["webPort"],
            "paths": 2, "state": "own-wss"}
    if plan_value.get("amneziawg"):
        value["amneziawg"] = {"listener": "udp/" + str(plan_value["amneziawg"]["port"]),
            "peers": len(plan_value["amneziawg"]["peers"]),
            "backend": "amneziawg-go/3.1"}
    if plan_value.get("selfsteal"):
        value["selfsteal"] = {"chainSni": plan_value.get("selfsteal")["chainSni"],
            "port": plan_value.get("selfsteal")["port"]}
    return value
