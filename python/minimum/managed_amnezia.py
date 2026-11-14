"""Managed native Amnezia configuration, provisioning and client inventory."""
from __future__ import annotations

import base64
import copy
import json
import os
import secrets
import shutil
import uuid
from dataclasses import dataclass

from storage import InstallError, digest

IMAGE_ROLE = "amnezia-xray"
CONTAINER_DIR = "/opt/amnezia/xray"
_DNS_PORT = 1053
NATIVE_PROTOCOL_LABEL = "XRay"


class ManagedAmneziaError(InstallError):
    """A bounded managed-Amnezia error; private key bytes never enter it."""


def _image_reference(value) -> bool:
    """Accept a pinned digest or a resolved mutable tag reference."""
    if not isinstance(value, str) or not value:
        return False
    if "@sha256:" in value:
        return True
    tail = value.rsplit("/", 1)[-1]
    return ":" in tail and not tail.endswith(":")


@dataclass(frozen=True)
class ManagedAmneziaPlan:
    mode: str
    container: str
    image: str | None
    server_names: tuple
    legacy_server_names: tuple
    host_port: int | None
    client_name: str | None
    admin: dict
    dest: str | None = None
    official: bool = False
    binary: str = "/usr/bin/xray"


def _field(value, name, default=None):
    if isinstance(value, dict):
        return value.get(name, default)
    return getattr(value, name, default)


def plan(foreign, pins=None, observations=None, receipts=None, dest=None) -> ManagedAmneziaPlan:
    """Pure managed plan from the validated module record and pinned image."""
    if foreign is None:
        raise ManagedAmneziaError("inputRequired: managed Amnezia settings are absent")
    mode = _field(foreign, "mode", "associate")
    pins = pins or {}
    if mode == "associate":
        container = _field(foreign, "container")
        if not container:
            raise ManagedAmneziaError("inputRequired: associated mode requires a selected container")
        facts = observations or {}
        if container in set(facts.get("managed_names", ())) and not receipts:
            raise ManagedAmneziaError("conflict: the named container is owned by another receipt")
        return ManagedAmneziaPlan("associate", container, None,
            tuple(_field(foreign, "server_names", ())),
            tuple(_field(foreign, "legacy_server_names", ())), None, None,
            _field(foreign, "admin", {}) or {})
    if mode != "provision":
        raise ManagedAmneziaError("unsupported: unknown managed Amnezia mode")
    image = pins.get(IMAGE_ROLE)
    official = False
    if not image:
        image = pins.get("xray")
        official = True
    if not _image_reference(image):
        raise ManagedAmneziaError("unsupported: no image reference for managed Amnezia (need amnezia-xray or xray in the image lock)")
    names = tuple(_field(foreign, "server_names", ()))
    if not names:
        raise ManagedAmneziaError("inputRequired: provision requires at least one own REALITY name")
    ports = _field(foreign, "host_ports", {}) or {}
    port = ports.get("tcp") if isinstance(ports, dict) else None
    if type(port) is not int or not 5000 <= port <= 6000:
        raise ManagedAmneziaError("inputRequired: provision requires a saved private host port")
    container = _field(foreign, "container") or IMAGE_ROLE
    return ManagedAmneziaPlan("provision", container, image, names,
        tuple(_field(foreign, "legacy_server_names", ())), port,
        _field(foreign, "client_name"), _field(foreign, "admin", {}) or {}, dest, official,
        "/usr/local/bin/xray" if official else "/usr/bin/xray")


# ---------------------------------------------------------------------------
# Key material and native server configuration
# ---------------------------------------------------------------------------

def _b64(raw: bytes) -> str:
    return base64.urlsafe_b64encode(raw).rstrip(b"=").decode()


def generate_key_material() -> dict:
    """Generate REALITY X25519 keys, a short id and the initial client UUID."""
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric.x25519 import X25519PrivateKey
    key = X25519PrivateKey.generate()
    private = key.private_bytes(serialization.Encoding.Raw,
        serialization.PrivateFormat.Raw, serialization.NoEncryption())
    public = key.public_key().public_bytes(serialization.Encoding.Raw,
        serialization.PublicFormat.Raw)
    return {
        "privateKey": _b64(private),
        "publicKey": _b64(public),
        "shortId": secrets.token_hex(4),
        "uuid": str(uuid.uuid4()),
    }


def native_material(data) -> dict:
    """Reusable REALITY keys and the complete client array from native bytes."""
    try:
        document = json.loads(data)
        inbound = document["inbounds"][0]
        reality = inbound["streamSettings"]["realitySettings"]
        private = reality["privateKey"]
        short_ids = reality["shortIds"]
        clients = inbound["settings"]["clients"]
    except (ValueError, KeyError, IndexError, TypeError) as error:
        raise ManagedAmneziaError("recoveryRequired: saved native material is unreadable") from error
    if not isinstance(private, str) or not private:
        raise ManagedAmneziaError("recoveryRequired: the saved REALITY private key is missing")
    if (not isinstance(short_ids, list) or not short_ids
            or any(not isinstance(item, str) or not item for item in short_ids)):
        raise ManagedAmneziaError("recoveryRequired: the saved REALITY short ids are invalid")
    if (not isinstance(clients, list) or not clients
            or any(not isinstance(item, dict) or not isinstance(item.get("id"), str) or not item["id"]
                for item in clients)):
        raise ManagedAmneziaError("recoveryRequired: the saved native client list is invalid")
    try:
        public = _public_key(private)
    except (ValueError, TypeError) as error:
        raise ManagedAmneziaError("recoveryRequired: the saved REALITY key pair is invalid") from error
    return {"privateKey": private, "publicKey": public, "shortId": short_ids[0],
        "shortIds": list(short_ids), "uuid": clients[0]["id"], "clients": list(clients)}


def saved_material(work):
    """Accepted native keys and client array from protected state.

    Recreation must reuse exactly these fields; a missing or malformed record
    blocks with recoveryRequired instead of rerolling keys or dropping clients.
    """
    data = work.read("amnezia/server.json")
    if data is None:
        return None
    return native_material(data)


def server_config(plan_value: ManagedAmneziaPlan, material: dict, dest=None,
                  dns_address="127.0.0.1") -> bytes:
    """Render the managed REALITY native server.json from typed settings."""
    if not plan_value.server_names:
        raise ManagedAmneziaError("inputRequired: a REALITY name is required")
    destination = dest or plan_value.dest or "127.0.0.1:9443"
    clients = material.get("clients") or [{"id": material["uuid"],
        "email": plan_value.client_name or "guest", "flow": "xtls-rprx-vision"}]
    inbound = {
        "tag": "reality",
        "listen": "0.0.0.0",
        "port": 443,
        "protocol": "vless",
        "settings": {
            "clients": copy.deepcopy(clients),
            "decryption": "none",
        },
        "streamSettings": {
            "network": "tcp",
            "security": "reality",
            "realitySettings": {
                "show": False,
                "dest": destination,
                "xver": 0,
                "serverNames": list(plan_value.server_names),
                "privateKey": material["privateKey"],
                "shortIds": list(material.get("shortIds") or [material["shortId"]]),
                # Xray 26.7+ defaults this gate above the Xray build shipped
                # by several Amnezia/sing-box clients; keep the managed
                # endpoint compatible without rotating any key material.
                "minClientVer": "1.0.0",
            },
        },
    }
    document = {
        "log": {"loglevel": "warning"},
        "inbounds": [inbound],
        "outbounds": [{"protocol": "freedom", "tag": "direct"}],
    }
    return (json.dumps(document, sort_keys=True, indent=2) + "\n").encode()


def _material_files(material: dict) -> dict:
    return {
        "xray_private.key": (material["privateKey"] + "\n").encode(),
        "xray_public.key": (material["publicKey"] + "\n").encode(),
        "xray_short_id.key": (material["shortId"] + "\n").encode(),
        "xray_uuid.key": (material["uuid"] + "\n").encode(),
    }


def _payload(work, plan_value: ManagedAmneziaPlan, material: dict, config: bytes):
    base = work.root / "amnezia/payload"
    if base.exists():
        shutil.rmtree(base)
    directory = base / "opt/amnezia/xray"
    directory.mkdir(parents=True, exist_ok=True)
    files = _material_files(material)
    files["server.json"] = config
    for name, content in files.items():
        path = directory / name
        path.write_bytes(content)
        os.chmod(path, 0o644)
    for path in (base, base / "opt", base / "opt/amnezia", directory):
        os.chmod(path, 0o755)
    return base, {name: "amnezia/payload/opt/amnezia/xray/" + name for name in files}


def prepare(plan_value: ManagedAmneziaPlan, work, journal=None, dest=None,
            dns_address="127.0.0.1", material=None, save=None, site_port=9443, domain=None) -> dict:
    """Generate once, stage protected native files, then create the container."""
    if plan_value.mode != "provision":
        raise ManagedAmneziaError("unsupported: prepare requires provision mode")
    import docker
    if not _image_reference(plan_value.image):
        raise ManagedAmneziaError("unsupported: the managed Amnezia image reference is invalid")
    material = material or saved_material(work)
    if material is None:
        material = generate_key_material()
    for key in ("privateKey", "publicKey", "shortId", "uuid"):
        if not isinstance(material.get(key), str) or not material[key]:
            raise ManagedAmneziaError("recoveryRequired: managed key material is incomplete")
    arguments = ["container", "create", "--pull", "never", "--network", "bridge",
        "--restart", "no", "--cap-drop", "ALL", "--security-opt", "no-new-privileges:true",
        "--publish", "127.0.0.1:" + str(plan_value.host_port) + ":443/tcp",
        "--publish", "127.0.0.1:" + str(plan_value.host_port) + ":443/udp",
        "--entrypoint", plan_value.binary, plan_value.image,
        "-config", CONTAINER_DIR + "/server.json"]
    if domain:
        arguments[2:2] = ["--add-host", domain + ":host-gateway"]
    identifier = docker.exact_id(docker.command(*arguments).decode().strip())
    gateway = docker.bridge_gateway()
    destination = dest or ((domain + ":" + str(site_port)) if domain else (gateway + ":" + str(site_port)))
    config = server_config(plan_value, material, dest=destination, dns_address=dns_address)
    work.write("amnezia/server.json", config)
    payload, staged = _payload(work, plan_value, material, config)
    docker.command("cp", "-a", str(payload) + "/.", identifier + ":/")
    if digest(docker.native_config(identifier)) != digest(config):
        raise ManagedAmneziaError("recoveryRequired: the staged native config differs after transfer")
    receipt = {
        "mode": "provision",
        "container": plan_value.container,
        "replacement": identifier,
        "image": plan_value.image,
        "official": plan_value.official,
        "binary": plan_value.binary,
        "names": list(plan_value.server_names),
        "hostPort": plan_value.host_port,
        "files": staged,
        "configDigest": digest(config),
        "publicKey": material["publicKey"],
        "uuid": material["uuid"],
        "shortId": material["shortId"],
    }
    work.save("amnezia/receipt.json", receipt)
    if journal:
        journal(receipt)
    if save:
        save()
    return receipt


def reconcile(plan_value: ManagedAmneziaPlan, receipt, current_native) -> dict:
    """Verify the accepted managed receipt against current native bytes."""
    if not isinstance(receipt, dict) or not receipt.get("replacement"):
        raise ManagedAmneziaError("drift: the managed receipt is missing")
    if isinstance(current_native, (bytes, bytearray)):
        current = bytes(current_native)
    else:
        raise ManagedAmneziaError("drift: native bytes are unavailable")
    if digest(current) != receipt.get("configDigest"):
        raise ManagedAmneziaError("drift: the managed native configuration changed")
    return {"reconciled": True, "container": receipt["container"],
        "configDigest": receipt["configDigest"]}


# ---------------------------------------------------------------------------
# Client inventory
# ---------------------------------------------------------------------------

def _public_key(private_b64: str) -> str:
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric.x25519 import X25519PrivateKey
    padded = private_b64 + "=" * (-len(private_b64) % 4)
    raw = base64.urlsafe_b64decode(padded.encode())
    key = X25519PrivateKey.from_private_bytes(raw)
    return _b64(key.public_key().public_bytes(serialization.Encoding.Raw,
        serialization.PublicFormat.Raw))


def vless_link(uuid_value: str, host: str, label: str, public_key: str,
               short_id: str, port: int = 443) -> str:
    """Standard REALITY VLESS client link built only from public client fields."""
    from urllib.parse import quote
    query = ("encryption=none&flow=xtls-rprx-vision&security=reality&sni=" + quote(host)
        + "&fp=chrome&pbk=" + quote(public_key) + "&sid=" + quote(short_id) + "&type=tcp")
    return ("vless://" + uuid_value + "@" + host + ":" + str(port) + "?"
        + query + "#" + quote(label))


def _native_document(accepted_native) -> dict:
    if isinstance(accepted_native, (bytes, bytearray)):
        try:
            document = json.loads(bytes(accepted_native))
        except (ValueError, UnicodeError) as error:
            raise ManagedAmneziaError("clientCoverageIncomplete: native config is not JSON") from error
    elif isinstance(accepted_native, dict):
        document = accepted_native
    else:
        raise ManagedAmneziaError("clientCoverageIncomplete: native config is unavailable")
    if not isinstance(document, dict):
        raise ManagedAmneziaError("clientCoverageIncomplete: native config is not a mapping")
    return document


def _client_list(accepted_native) -> list:
    document = _native_document(accepted_native)
    try:
        clients = document["inbounds"][0]["settings"]["clients"]
    except (KeyError, IndexError, TypeError) as error:
        raise ManagedAmneziaError("clientCoverageIncomplete: native client list is absent") from error
    if not isinstance(clients, list):
        raise ManagedAmneziaError("clientCoverageIncomplete: native client list is invalid")
    return clients


def _reality_export_fields(document) -> tuple:
    try:
        reality = document["inbounds"][0]["streamSettings"]["realitySettings"]
    except (KeyError, IndexError, TypeError):
        return None, None, None
    private = reality.get("privateKey")
    names = reality.get("serverNames") or []
    shorts = reality.get("shortIds") or []
    public = None
    if isinstance(private, str) and private:
        try:
            public = _public_key(private)
        except (ValueError, TypeError):
            public = None
    host = names[0] if names and isinstance(names[0], str) else None
    short = shorts[0] if shorts and isinstance(shorts[0], str) else None
    return public, host, short


def connection_fragment(label):
    """App connection name: configured label plus the native protocol name."""
    value = label.strip() if isinstance(label, str) else ""
    return (value + " " + NATIVE_PROTOCOL_LABEL).strip()


def client_inventory(accepted_native, configured_labels=None, name_policy=None,
                     link_label=None) -> list:
    """Return typed native-client entries with public guest links when derivable."""
    labels = configured_labels or {}
    document = _native_document(accepted_native)
    clients = _client_list(document)
    public, host, short = _reality_export_fields(document)
    entries = []
    seen = set()
    for client in clients:
        if not isinstance(client, dict):
            continue
        identifier = client.get("email") or client.get("id")
        if not isinstance(identifier, str) or not identifier:
            continue
        if identifier in seen:
            continue
        seen.add(identifier)
        configured = labels.get(identifier)
        if isinstance(configured, str) and configured:
            display = fragment = configured
        else:
            display = identifier
            fragment = connection_fragment(link_label)
        payload = None
        uuid_value = client.get("id")
        if public and host and short and isinstance(uuid_value, str) and uuid_value:
            payload = vless_link(uuid_value, host, fragment, public, short)
        entries.append({
            "credentialId": identifier,
            "protocol": "vless",
            "displayName": display,
            "status": "active",
            "payload": payload,
            "sourceRevision": digest(identifier.encode()),
        })
    return entries
