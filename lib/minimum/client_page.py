"""Client-only inventory snapshot and static bearer page rendering."""
from __future__ import annotations

import base64
import datetime
import html
import hashlib
import re
import secrets
from collections import Counter
from dataclasses import dataclass, field
from urllib.parse import unquote, urlsplit

import configuration
from storage import InstallError, digest

MAX_ENTRIES = 256
MAX_BYTES = 1024 * 1024
SUBSCRIPTION_PREFIX = "/subscriptions/"
SUBSCRIPTION_TOKEN = r"[A-Za-z0-9_-]{32}"
_SUBSCRIPTION_STATE = "subscription-tokens.json"
_SUBSCRIPTION_APPS = {"Incy": "incy://import/", "Happ": "happ://add/"}
RESERVED_PATHS = {"admin", "api", "acme", "share", "vpn", ".well-known"}

_SHARE_SCRIPT = """document.querySelectorAll('a[download]').forEach((link) => {
    if (!navigator.share || !navigator.canShare || typeof File !== 'function') return;
    const encoded = link.getAttribute('href').split(',')[1];
    const bytes = Uint8Array.from(atob(encoded), (char) => char.charCodeAt(0));
    const data = {files: [new File([bytes], link.download, {type: 'application/octet-stream'})]};
    if (!navigator.canShare(data)) return;
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = 'Share file';
    button.addEventListener('click', async () => {
        try {
            await navigator.share(data);
        } catch (error) {
            if (error.name !== 'AbortError') link.click();
        }
    });
    link.after(' ', button);
});"""
_SHARE_SCRIPT += """
document.querySelectorAll('a[data-import]').forEach((link) => {
    link.href = link.dataset.import + link.href;
});"""
_SHARE_HASH = base64.b64encode(hashlib.sha256(_SHARE_SCRIPT.encode()).digest()).decode()

HEADERS = {
    "Cache-Control": "no-store",
    "Referrer-Policy": "no-referrer",
    "X-Robots-Tag": "noindex, nofollow, noarchive",
    "X-Content-Type-Options": "nosniff",
    "Content-Security-Policy": "default-src 'none'; frame-ancestors 'none'; "
        f"base-uri 'none'; form-action 'none'; script-src 'sha256-{_SHARE_HASH}'; "
        "style-src 'unsafe-inline'",
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


def render(plan_value: ClientPagePlan, inventory, subscriptions=None) -> ClientPageArtifact:
    """Return escaped, bounded HTML plus its digest and privacy headers."""
    if not plan_value.enabled:
        raise ClientPageError("the client page is disabled")
    entries = [entry for entry in (inventory or ())
        if _field(entry, "status") == _ACTIVE and _field(entry, "payload") is not None]
    if any(_is_forbidden(entry) for entry in entries):
        raise ClientPageError("forbiddenCredential: an admin payload reached rendering")
    entries = entries[:MAX_ENTRIES]
    rows = []
    subscriptions = subscriptions or {}
    for entry in entries:
        label = _field(entry, "displayName") or _field(entry, "credentialId") or "client"
        protocol = _field(entry, "protocol") or "vpn"
        payload = _field(entry, "payload")
        if isinstance(payload, (bytes, bytearray)):
            payload = bytes(payload).decode("utf-8", "replace")
        content = str(payload)
        cell = "<pre>" + html.escape(content) + "</pre>"
        if protocol == "amneziawg":
            host = _field(entry, "siteHost")
            filename = "-".join(
                re.sub(r"[^\w.-]+", "-", str(part or "")).strip(".-_")[:64] or fallback
                for part, fallback in ((host, "server"), (label, "client"), (protocol, "vpn"))
            )
            encoded = base64.b64encode(content.encode("utf-8")).decode("ascii")
            cell = (
                '<a download="' + html.escape(filename) + '.conf" href="data:application/octet-stream;base64,'
                + encoded + '">Download .conf</a>')
        if subscriptions:
            for app, scheme in _SUBSCRIPTION_APPS.items():
                token = subscriptions.get((protocol, _field(entry, "credentialId"), app))
                if token:
                    path = SUBSCRIPTION_PREFIX + token + ".txt"
                    cell += (
                        f'<div><a data-import="{scheme}" href="{path}">Open in {app}</a>'
                        f' · <a href="{path}">{app} subscription URL</a></div>')
        rows.append(
            "<tr><td>" + html.escape(str(protocol))
            + "</td><td>" + html.escape(str(label))
            + "</td><td>" + cell + "</td></tr>")
    document = (
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">"
        "<meta name=\"robots\" content=\"noindex,nofollow,noarchive\">"
        "<meta name=\"referrer\" content=\"no-referrer\">"
        "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
        "<title>Client access</title>"
        "<style>body{font-family:system-ui,sans-serif;margin:2rem;color:#111}"
        "pre{white-space:pre-wrap;word-break:break-all;background:#f4f4f4;padding:.5rem}</style>"
        "</head><body><h1>Client access</h1>"
        "<p>Coverage: " + html.escape(plan_value.coverage)
        + ". Generated: " + html.escape(datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"))
        + "</p><table border=\"1\" cellpadding=\"4\">"
        "<thead><tr><th>Protocol</th><th>Client</th><th>Key</th></tr></thead><tbody>"
        + "".join(rows) + "</tbody></table><script>" + _SHARE_SCRIPT
        + "</script></body></html>").encode()
    if len(document) > MAX_BYTES:
        raise ClientPageError("clientCoverageIncomplete: rendered page exceeds its byte bound")
    return ClientPageArtifact(document, digest(document), dict(HEADERS))


def refresh(page_settings, inventory, store, save=None) -> dict:
    """Reconcile the accepted snapshot to the current inventory or withdraw it."""
    with store.lock("subscription-refresh.lock"):
        return _refresh(page_settings, inventory, store, save)


def _subscription_credentials(inventory):
    candidates = []
    for entry in inventory:
        payload = _field(entry, "payload")
        identifier = _field(entry, "credentialId")
        protocol = _field(entry, "protocol")
        if (_field(entry, "status") != _ACTIVE
                or not isinstance(identifier, str) or not identifier
                or not isinstance(payload, str)):
            continue
        if protocol == "amneziawg" and payload.startswith("[Interface]"):
            interface = payload.split("[Peer]", 1)[0]
            key = re.search(r"^PrivateKey[ \t]*=[ \t]*([^\s#]+)", interface, re.MULTILINE)
            if not key:
                continue
            auth = key[1]
            apps = ("Incy",)
        elif len(payload.splitlines()) == 1 and (
                (protocol == "vless" and payload.startswith("vless://"))
                or (protocol == "hysteria" and payload.startswith(("hysteria2://", "hy2://")))):
            try:
                auth, separator, _ = urlsplit(payload).netloc.rpartition("@")
            except ValueError:
                continue
            if not separator or not auth:
                continue
            auth = unquote(auth)
            if protocol == "vless":
                auth = auth.lower()
            apps = _SUBSCRIPTION_APPS
        else:
            continue
        candidates.append((protocol, identifier, digest(auth.encode()), apps, payload))
    identifiers = Counter((protocol, identifier) for protocol, identifier, *_ in candidates)
    keys = Counter((protocol, key) for protocol, _, key, *_ in candidates)
    for protocol, identifier, key, apps, payload in candidates:
        if identifiers[protocol, identifier] == keys[protocol, key] == 1:
            yield protocol, identifier, apps, (payload.rstrip("\n") + "\n").encode()



def _refresh(page_settings, inventory, store, save):
    inventory = list(inventory or ())
    previous = store.json(_SUBSCRIPTION_STATE, {})
    try:
        plan_value = plan(page_settings, inventory)
        subscriptions, files = {}, {}
        if plan_value.enabled:
            for protocol, identifier, apps, payload in _subscription_credentials(inventory):
                for app in apps:
                    key = digest(f"{plan_value.route_id}:{app}:{protocol}:{identifier}".encode())
                    token = previous.get(key) or secrets.token_urlsafe(24)
                    subscriptions[protocol, identifier, app] = token
                    files[key] = (token, payload)
        artifact = render(plan_value, inventory, subscriptions) if plan_value.enabled else None
    except ClientPageError:
        _reconcile_subscriptions(store, previous, {})
        _withdraw(store, save)
        return {"withdrawn": True, "reason": "invalid"}
    _reconcile_subscriptions(store, previous, files)
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


def _reconcile_subscriptions(store, previous, files):
    def filename(token):
        if not isinstance(token, str) or not re.fullmatch(SUBSCRIPTION_TOKEN, token):
            raise ClientPageError("Invalid saved subscription token")
        return "client-page" + SUBSCRIPTION_PREFIX + token + ".txt"

    current = {key: token for key, (token, _) in files.items()}
    if current != previous:
        for key in previous.keys() - current.keys():
            store.remove(filename(previous[key]))
        # Track new tokens before publishing their files so a retry can revoke them.
        store.save(_SUBSCRIPTION_STATE, current)
    for token, payload in files.values():
        path = filename(token)
        if store.read(path) != payload:
            store.write(path, payload)


def _withdraw(store, save=None) -> None:
    store.remove("client-page/index.html")
    store.save("client-page/receipt.json", {"enabled": False, "withdrawn": True})
    if save:
        save()
