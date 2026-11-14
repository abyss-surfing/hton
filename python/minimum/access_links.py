"""Pure typed guest export encoding shared by inventory and client-page owners."""
from __future__ import annotations

import json
from urllib.parse import quote

import configuration
import protocols
from storage import InstallError

_FORBIDDEN_MARKERS = (
    "privatekey", "private_key", "sshprivatekey", "ssh-", "password",
    "rootpassword", "adminvpn", "serverprivatekey", "apikey", "apipassword",
    "sudoers", "-----begin",
)


class IncompatibleExport(InstallError):
    """A requested export is not a guest-safe artifact; no secret bytes escape."""


def _reject_admin_bytes(payload: bytes) -> None:
    lowered = payload.lower()
    for marker in _FORBIDDEN_MARKERS:
        if marker.encode() in lowered:
            raise IncompatibleExport("admin or secret fields are not permitted in a guest export")


def _require_guest(request) -> None:
    kind = request.get("kind")
    if kind != "guest":
        raise IncompatibleExport("only guest artifacts can be encoded for the client page")


def _host(request) -> str:
    try:
        return configuration.canonical_name(request.get("host"))
    except configuration.ConfigError as error:
        raise IncompatibleExport("export host is invalid") from error


def encode(request) -> list:
    """Encode one typed guest request into protected artifact bytes.

    The request is discriminated by ``kind``: only ``guest`` is accepted. A
    caller-supplied native payload is passed through only after a secret/admin
    marker scan; the codec never invents a full-access envelope.
    """
    if not isinstance(request, dict):
        raise IncompatibleExport("export request must be a mapping")
    _require_guest(request)
    protocol = request.get("protocol")
    credential_id = str(request.get("credentialId") or "guest")
    display = str(request.get("displayName") or credential_id)
    host = _host(request)
    artifacts = []
    if protocol == "outline-wss":
        secret = request.get("secret")
        tcp_path = request.get("tcpPath")
        udp_path = request.get("udpPath")
        if not isinstance(secret, str) or not secret:
            raise IncompatibleExport("outline export has no key")
        if not isinstance(tcp_path, str) or not isinstance(udp_path, str):
            raise IncompatibleExport("outline export has no configured paths")
        content = protocols.outline_key(host,
            {"tcpPath": tcp_path, "udpPath": udp_path, "secret": secret},
            secret=secret, name=display)
        artifacts.append({"name": "share/outline-" + credential_id + ".yaml",
            "content": content, "credentialId": credential_id, "protocol": protocol})
    elif protocol == "mtproto":
        secret = request.get("secret")
        if not isinstance(secret, str) or not secret:
            raise IncompatibleExport("mtproto export has no secret")
        port = request.get("port", 443)
        if type(port) is not int or not 1 <= port <= 65535:
            raise IncompatibleExport("mtproto export port is invalid")
        link = ("tg://proxy?server=" + host + "&port=" + str(port)
            + "&secret=" + quote(secret))
        artifacts.append({"name": "share/mtproto-" + credential_id + ".txt",
            "content": (link + "\n").encode(), "credentialId": credential_id,
            "protocol": protocol})
    elif protocol == "hysteria":
        password = request.get("secret")
        if not isinstance(password, str) or not password:
            raise IncompatibleExport("hysteria export has no password")
        port = request.get("port", 443)
        if type(port) is not int or not 1 <= port <= 65535:
            raise IncompatibleExport("hysteria export port is invalid")
        link = ("hysteria2://" + quote(password) + "@" + host + ":" + str(port)
            + "/?sni=" + host + "#" + quote(display))
        artifacts.append({"name": "share/hysteria-" + credential_id + ".txt",
            "content": (link + "\n").encode(), "credentialId": credential_id,
            "protocol": protocol})
    elif protocol == "amneziawg":
        payload = request.get("payload")
        content = protocols.awg_client_text(payload)
        identity = credential_id[len("amneziawg-"):] \
            if credential_id.startswith("amneziawg-") else credential_id
        artifacts.append({"name": "share/amneziawg-" + identity + ".conf",
            "content": content, "credentialId": credential_id, "protocol": protocol})
    elif protocol in ("amnezia", "vless"):
        payload = request.get("payload")
        if payload is None:
            raise IncompatibleExport("native client export requires an accepted payload")
        if isinstance(payload, str):
            payload = payload.encode()
        if not isinstance(payload, (bytes, bytearray)):
            raise IncompatibleExport("native client export payload has an invalid type")
        payload = bytes(payload)
        _reject_admin_bytes(payload)
        artifacts.append({"name": "share/" + protocol + "-" + credential_id + ".vpn",
            "content": payload, "credentialId": credential_id, "protocol": protocol})
    else:
        raise IncompatibleExport("unsupported export protocol")
    return artifacts
