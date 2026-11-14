"""Typed native Amnezia (VLESS REALITY) client-list operations.

Pure functions over the native `server.json` document: no Docker, no files,
no credentials in errors. The caller owns IO and restart.
"""
from __future__ import annotations

import copy
import hashlib
import json
import re
import uuid

NAME_PATTERN = re.compile(r"^[0-9A-Za-zА-Яа-яЁё][0-9A-Za-zА-Яа-яЁё_.-]{0,31}\Z")
PROTECTED = frozenset({"guest"})
FLOW = "xtls-rprx-vision"


class ClientError(Exception):
    """A bounded client-operation error; payloads never enter the message."""


def _clients(document):
    try:
        clients = document["inbounds"][0]["settings"]["clients"]
    except (KeyError, IndexError, TypeError) as error:
        raise ClientError("native client list is absent") from error
    if not isinstance(clients, list) or not clients:
        raise ClientError("native client list is empty or invalid")
    if any(not isinstance(item, dict) for item in clients):
        raise ClientError("native client list is invalid")
    return clients


def document(payload):
    """Validated copy of the native config (bytes or mapping)."""
    if isinstance(payload, (bytes, bytearray)):
        try:
            value = json.loads(bytes(payload))
        except (ValueError, UnicodeError) as error:
            raise ClientError("native config is not JSON") from error
    elif isinstance(payload, dict):
        value = copy.deepcopy(payload)
    else:
        raise ClientError("native config is unavailable")
    if not isinstance(value, dict):
        raise ClientError("native config is not a mapping")
    _clients(value)
    return value


def names(value):
    """Ordered client identifiers (email, falling back to id)."""
    return [str(item.get("email") or item.get("id")) for item in _clients(value)]


def identifier(value, name):
    """UUID of an existing client; the document is not modified."""
    name = validate(name)
    for item in _clients(value):
        if str(item.get("email") or item.get("id")) == name:
            found = item.get("id")
            if not isinstance(found, str) or not found:
                raise ClientError("client identifier is invalid: " + name)
            return found
    raise ClientError("client not found: " + name)


def validate(name):
    if not isinstance(name, str) or not NAME_PATTERN.match(name):
        raise ClientError("client name must start with a letter or digit and "
                          "use letters (Latin/Cyrillic, any case), digits, "
                          "'_', '.', '-' (max 32)")
    return name


def add(value, name):
    """Append a client and return its new UUID; the document is mutated."""
    name = validate(name)
    items = _clients(value)
    if name in names(value):
        raise ClientError("client already exists: " + name)
    identifier = str(uuid.uuid4())
    items.append({"id": identifier, "flow": FLOW, "email": name})
    return identifier


def remove(value, name):
    """Remove a client; protected and last-client removal are refused."""
    name = validate(name)
    if name in PROTECTED:
        raise ClientError("client is protected: " + name)
    items = _clients(value)
    kept = [item for item in items
            if str(item.get("email") or item.get("id")) != name]
    if len(kept) == len(items):
        raise ClientError("client not found: " + name)
    if not kept:
        raise ClientError("refusing to remove the last client")
    value["inbounds"][0]["settings"]["clients"] = kept


def encode(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + "\n").encode()


def slug(name):
    cleaned = "".join(ch if ch.isalnum() or ch in "-_" else "-"
                      for ch in name.lower()).strip("-")
    return cleaned or "client"


def share_name(name):
    base = slug(name)
    if base != name or base in {"client-page", "server"}:
        # The separator cannot occur in a plain slug. Keep the name stable
        # even if another user is added or removed later.
        base += "." + hashlib.sha256(name.encode()).hexdigest()[:16]
    return base + ".md"


def markdown(name, link):
    """Operator-facing share artifact: Markdown with the link in a fence."""
    return "# Amnezia — " + name + "\n\n```\n" + link + "\n```\n"


def reality_fields(value):
    """(privateKey, host, shortId) needed to build a client link."""
    try:
        reality = value["inbounds"][0]["streamSettings"]["realitySettings"]
        return reality["privateKey"], reality["serverNames"][0], reality["shortIds"][0]
    except (KeyError, IndexError, TypeError) as error:
        raise ClientError("native REALITY settings are incomplete") from error
