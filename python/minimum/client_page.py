"""Client-only inventory snapshot and static bearer page rendering."""
from __future__ import annotations

import datetime
import html
import re
import secrets
from dataclasses import dataclass, field

import configuration
from storage import InstallError, digest

MAX_ENTRIES = 256
MAX_BYTES = 1024 * 1024
RESERVED_PATHS = {"admin", "api", "acme", "share", "vpn", ".well-known"}

HEADERS = {
    "Cache-Control": "no-store",
    "Referrer-Policy": "no-referrer",
    "X-Robots-Tag": "noindex, nofollow, noarchive",
    "X-Content-Type-Options": "nosniff",
    "Content-Security-Policy": "default-src 'none'; frame-ancestors 'none'; "
        "base-uri 'none'; form-action 'none'",
}

_PAGE_SEGMENT = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._~-]{0,63}$")
_FORBIDDEN_KEYS = {
    "privateKey", "private_key", "sshPrivateKey", "password", "rootPassword",
    "adminVpn", "serverPrivateKey", "apiKey", "apiPassword", "sudoers",
}
_ACTIVE = "active"


class ClientPageError(InstallError):
    """A bounded client-page error; secret paths and payload bytes never enter it."""


@dataclass(frozen=True)
class ClientPagePlan:
    enabled: bool
    path: str = ""
    route_id: str = ""
    coverage: str = "complete"
    refresh_seconds: int = 60
    missing: tuple = ()


@dataclass(frozen=True)
class ClientPageArtifact:
    html: bytes
    digest: str
    headers: dict = field(default_factory=lambda: dict(HEADERS))


def _field(value, name, default=None):
    if isinstance(value, dict):
        return value.get(name, default)
    return getattr(value, name, default)


def _valid_path(path: str) -> bool:
    return bool(path) and bool(_PAGE_SEGMENT.match(path)) and path not in (".", "..")


def generate_path() -> str:
    """Return a CSPRNG secret path with at least 128 bits of entropy."""
    while True:
        value = secrets.token_urlsafe(24)
        if _valid_path(value) and value not in RESERVED_PATHS:
            return value


def _is_forbidden(entry) -> bool:
    if _field(entry, "kind") == "admin":
        return True
    for key in _field(entry, "fields", ()) or ():
        if str(key) in _FORBIDDEN_KEYS:
            return True
    payload = _field(entry, "payload")
    if isinstance(payload, str):
        lowered = payload.lower()
        if "private key" in lowered or "-----begin" in lowered or "ssh-" in lowered:
            return True
    return False


def plan(page_settings, inventory, host_policy=None, reserved_paths=()) -> ClientPagePlan:
    """Pure plan from validated settings and a typed client inventory."""
    path = _field(page_settings, "path", "")
    if not path:
        return ClientPagePlan(False)
    if not isinstance(path, str) or not _valid_path(path):
        raise ClientPageError("invalidPath: the page path is not a single safe segment")
    if path in RESERVED_PATHS:
        raise ClientPageError("pathConflict: the page path collides with a reserved route")
    for reserved in reserved_paths:
        if path.casefold() == str(reserved).casefold():
            raise ClientPageError("pathConflict: the page path collides with a published route")
    refresh = _field(page_settings, "refresh_seconds", _field(page_settings, "refreshSeconds", 60))
    if type(refresh) is not int or not 10 <= refresh <= 3600:
        raise ClientPageError("invalidPath: refreshSeconds is out of range")
    entries = list(inventory or ())
    missing = []
    active = 0
    for entry in entries:
        if _is_forbidden(entry):
            raise ClientPageError("forbiddenCredential: an admin or secret payload cannot be published")
        if _field(entry, "status") != _ACTIVE or _field(entry, "payload") is None:
            missing.append(str(_field(entry, "credentialId", "unknown")))
        else:
            active += 1
    coverage = "complete" if active == len(entries) and entries else ("empty" if not entries else "partial")
    if len(entries) > MAX_ENTRIES:
        raise ClientPageError("clientCoverageIncomplete: too many inventory entries")
    return ClientPagePlan(True, path, digest(path.encode())[:16], coverage,
        refresh, tuple(missing))


def render(plan_value: ClientPagePlan, inventory) -> ClientPageArtifact:
    """Return escaped, bounded HTML plus its digest and privacy headers."""
    if not plan_value.enabled:
        raise ClientPageError("the client page is disabled")
    entries = [entry for entry in (inventory or ())
        if _field(entry, "status") == _ACTIVE and _field(entry, "payload") is not None]
    if any(_is_forbidden(entry) for entry in entries):
        raise ClientPageError("forbiddenCredential: an admin payload reached rendering")
    entries = entries[:MAX_ENTRIES]
    rows = []
    for entry in entries:
        label = _field(entry, "displayName") or _field(entry, "credentialId") or "client"
        protocol = _field(entry, "protocol") or "vpn"
        payload = _field(entry, "payload")
        if isinstance(payload, (bytes, bytearray)):
            payload = bytes(payload).decode("utf-8", "replace")
        rows.append(
            "<tr><td>" + html.escape(str(protocol))
            + "</td><td>" + html.escape(str(label))
            + "</td><td><pre>" + html.escape(str(payload)) + "</pre></td></tr>")
    document = (
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">"
        "<meta name=\"robots\" content=\"noindex,nofollow,noarchive\">"
        "<meta name=\"referrer\" content=\"no-referrer\">"
        "<title>Client access</title>"
        "<style>body{font-family:system-ui,sans-serif;margin:2rem;color:#111}"
        "pre{white-space:pre-wrap;word-break:break-all;background:#f4f4f4;padding:.5rem}</style>"
        "</head><body><h1>Client access</h1>"
        "<p>Coverage: " + html.escape(plan_value.coverage)
        + ". Generated: " + html.escape(datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"))
        + "</p><table border=\"1\" cellpadding=\"4\">"
        "<thead><tr><th>Protocol</th><th>Client</th><th>Key</th></tr></thead><tbody>"
        + "".join(rows) + "</tbody></table></body></html>").encode()
    if len(document) > MAX_BYTES:
        raise ClientPageError("clientCoverageIncomplete: rendered page exceeds its byte bound")
    return ClientPageArtifact(document, digest(document), dict(HEADERS))


def refresh(page_settings, inventory, store, save=None) -> dict:
    """Reconcile the accepted snapshot to the current inventory or withdraw it."""
    try:
        plan_value = plan(page_settings, inventory)
        artifact = render(plan_value, inventory) if plan_value.enabled else None
    except ClientPageError:
        _withdraw(store, save)
        return {"withdrawn": True, "reason": "invalid"}
    if not plan_value.enabled:
        _withdraw(store, save)
        return {"withdrawn": True, "reason": "disabled"}
    relative = "client-page/index.html"
    store.write(relative, artifact.html)
    receipt = {
        "enabled": True,
        "routeId": plan_value.route_id,
        "coverage": plan_value.coverage,
        "missing": list(plan_value.missing),
        "refreshSeconds": plan_value.refresh_seconds,
        "artifact": relative,
        "digest": artifact.digest,
        "headers": artifact.headers,
    }
    # The bearer path is needed by the caller to configure the route, but it
    # must not enter the persisted JSON receipt (OR-49/T-12).
    store.save("client-page/receipt.json", receipt)
    if save:
        save()
    return dict(receipt, path=plan_value.path)


def _withdraw(store, save=None) -> None:
    store.remove("client-page/index.html")
    store.save("client-page/receipt.json", {"enabled": False, "withdrawn": True})
    if save:
        save()
