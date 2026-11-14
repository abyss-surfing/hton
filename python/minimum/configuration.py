"""Coherent configuration read and typed validation for the minimum profile.

The loader returns documents, content revisions and structured diagnostics even
when a target is malformed; validation never rewrites source bytes.  Typed
records mirror the fields frozen in ``docs/specs/sdd-config.md`` and are the only
shape consumed by the remaining minimum modules.
"""
from __future__ import annotations

import ipaddress
import re
from dataclasses import dataclass, field
from pathlib import Path

import clients
from storage import InstallError, Store, digest, parse_document

ROOT_DOCUMENT = "hton.yaml"
MODULE_DIRECTORY = "modules"
SUPPORTED_SCHEMA_VERSIONS = (1, 2)

MODULE_IDS = (
    "foreign-amnezia",
    "mtproto",
    "outline-wss",
    "hysteria",
    "amneziawg",
)

_LABEL = re.compile(r"^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$")
_UPSTREAM_ID = re.compile(r"^[a-z][a-z0-9-]{0,63}$")
_USERNAME = re.compile(r"^[a-z_][a-z0-9_-]{0,31}$")
_CLIENT_NAME = clients.NAME_PATTERN
_CONTAINER = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$")
_PAGE_SEGMENT = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._~-]{0,63}$")
_HTTP_PATH = re.compile(r"^/[A-Za-z0-9._~/-]{0,255}$")
_SECRET_REF = re.compile(r"^[a-z][a-z0-9-]*(?:/[a-z][a-z0-9-]*)+$")


class ConfigError(Exception):
    """A bounded, redacted configuration error."""


# ---------------------------------------------------------------------------
# Bounded name parsing
# ---------------------------------------------------------------------------

def canonical_name(value, *, allow_single: bool = False) -> str:
    """Return the canonical lower-case ASCII form of a bounded DNS name.

    Rejects URLs, ports, trailing dots, wildcards, embedded NUL and whitespace.
    IDNA is applied through the standard codec so a non-ASCII label is rejected
    or normalised deterministically, never silently widened.
    """
    if not isinstance(value, str) or not value or len(value) > 253:
        raise ConfigError("name must be a non-empty string of at most 253 bytes")
    if value != value.strip() or any(ch in value for ch in "\x00 \t\r\n"):
        raise ConfigError("name contains whitespace or NUL")
    if "://" in value or "/" in value or ":" in value or value.endswith("."):
        raise ConfigError("name must be a bare DNS name, not a URL, port or root")
    if "*" in value:
        raise ConfigError("wildcards are not allowed in configured names")
    try:
        ascii_name = value.encode("idna").decode("ascii").lower()
    except UnicodeError as error:
        raise ConfigError("name is not valid IDNA") from error
    labels = ascii_name.split(".")
    if not allow_single and len(labels) < 2:
        raise ConfigError("name must contain a dot")
    if len(ascii_name) > 253:
        raise ConfigError("name exceeds 253 bytes after IDNA")
    for label in labels:
        if not _LABEL.match(label):
            raise ConfigError("name contains an invalid label")
    return ascii_name


def _ipv4(value) -> str:
    if not isinstance(value, str):
        raise ConfigError("bootstrap address must be a literal IPv4 string")
    try:
        address = ipaddress.IPv4Address(value)
    except (ValueError, ipaddress.AddressValueError) as error:
        raise ConfigError("bootstrap address must be a literal IPv4 address") from error
    return str(address)


def _port(value, *, minimum: int = 1, maximum: int = 65535) -> int:
    if type(value) is not int or not minimum <= value <= maximum:
        raise ConfigError(f"port must be an integer in {minimum}..{maximum}")
    return value


# ---------------------------------------------------------------------------
# Diagnostics
# ---------------------------------------------------------------------------

def diagnostic(code, path, *, module=None, severity="error", blocking=True, hint=""):
    return {
        "code": code,
        "moduleId": module,
        "configPath": path,
        "severity": severity,
        "blocking": blocking,
        "repairHint": hint,
        "evidence": {},
    }


# ---------------------------------------------------------------------------
# Loaded read set
# ---------------------------------------------------------------------------

@dataclass
class Loaded:
    root: dict | None = None
    modules: dict = field(default_factory=dict)
    revisions: dict = field(default_factory=dict)
    diagnostics: list = field(default_factory=list)

    def revision(self, path: str) -> str | None:
        return self.revisions.get(path)


def _parse(store: Store, name: str, diagnostics: list, module: str | None = None):
    raw = store.read(name)
    if raw is None:
        return None, None
    try:
        value = parse_document(raw)
    except InstallError:
        diagnostics.append(diagnostic(
            "config.unparseable", name, module=module,
            hint="Retain original bytes; repair the document before activation"))
        return None, digest(raw)
    return value, digest(raw)


def load(tree, supported_versions=SUPPORTED_SCHEMA_VERSIONS) -> Loaded:
    """Read the coherent configuration tree without mutating or repairing it."""
    store = Store(Path(tree))
    try:
        return _load(store, tuple(supported_versions))
    finally:
        store.close()


def _load(store: Store, supported_versions) -> Loaded:
    result = Loaded()
    root, revision = _parse(store, ROOT_DOCUMENT, result.diagnostics)
    result.root = root
    if revision is not None:
        result.revisions[ROOT_DOCUMENT] = revision
        if root is None:
            result.diagnostics.append(diagnostic(
                "config.root-unparseable", ROOT_DOCUMENT,
                hint="Retain original bytes; root must parse before activation"))
        elif not isinstance(root, dict):
            result.diagnostics.append(diagnostic(
                "config.root-not-mapping", ROOT_DOCUMENT))
        elif root.get("schemaVersion", 1) not in supported_versions:
            result.diagnostics.append(diagnostic(
                "config.root-version", ROOT_DOCUMENT,
                hint="Supported root versions: " + ", ".join(map(str, supported_versions))))
    else:
        result.diagnostics.append(diagnostic(
            "config.root-absent", ROOT_DOCUMENT, severity="info", blocking=False,
            hint="A fresh installation may create the root after review"))

    for module_id in MODULE_IDS:
        path = f"{MODULE_DIRECTORY}/{module_id}.yaml"
        value, revision = _parse(store, path, result.diagnostics, module=module_id)
        if revision is not None:
            result.revisions[path] = revision
        if value is not None:
            result.modules[module_id] = value
    return result


# ---------------------------------------------------------------------------
# Typed records
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class Listen:
    transport: str
    port: int
    address_families: tuple = ("ipv4",)
    scope: str | None = None


@dataclass(frozen=True)
class HostPolicy:
    wildcard_zone: str
    redirect_target: str
    unknown: str = "reject"


@dataclass(frozen=True)
class TestTunnel:
    """A test-only edge SNI forwarded to an SSH reverse-forwarded loopback port.

    The route is intentionally separate from every production protocol name and
    is only useful while an operator keeps a reverse SSH forward open.
    """

    sni: str
    port: int


@dataclass(frozen=True)
class Edge:
    domains: tuple
    listen: Listen
    host_policy: HostPolicy | None = None
    test_tunnel: TestTunnel | None = None


@dataclass(frozen=True)
class DnsUpstream:
    addresses: tuple
    port: int
    tls_name: str


@dataclass(frozen=True)
class Dns:
    upstream_mode: str
    upstreams: dict = field(default_factory=dict)
    upstream_order: tuple = ()
    fallback: str = "fail-closed"


@dataclass(frozen=True)
class ClientPage:
    path: str = ""
    names: dict = field(default_factory=dict)
    refresh_seconds: int = 60

    @property
    def enabled(self) -> bool:
        return bool(self.path)


@dataclass(frozen=True)
class Root:
    schema_version: int
    instance: dict
    installation: dict
    edge: Edge
    certificates: dict
    dns: Dns | None
    client_access_page: ClientPage
    features: dict
    connection_label: str | None = None


@dataclass(frozen=True)
class ForeignAmnezia:
    mode: str
    container: str | None
    server_names: tuple
    legacy_server_names: tuple
    host_ports: dict
    client_name: str | None
    admin: dict


@dataclass(frozen=True)
class Mtproto:
    backend: str
    camouflage_name: str | None


@dataclass(frozen=True)
class OutlineWss:
    server_name: str | None
    stream_path: str | dict | None
    packet_path: str | dict | None


@dataclass(frozen=True)
class Hysteria:
    hostname: str | None
    listen: Listen | None


@dataclass(frozen=True)
class AmneziaWG:
    hostname: str | None
    subnet: str | None
    listen: Listen | None


@dataclass
class Validation:
    desired: Root | None = None
    modules: dict = field(default_factory=dict)
    diagnostics: list = field(default_factory=list)

    @property
    def blocking(self) -> bool:
        return any(item["blocking"] for item in self.diagnostics)

    @property
    def ok(self) -> bool:
        return self.desired is not None and not self.blocking

    def add(self, code, path, *, module=None, severity="error", blocking=True, hint=""):
        self.diagnostics.append(diagnostic(code, path, module=module,
            severity=severity, blocking=blocking, hint=hint))


# ---------------------------------------------------------------------------
# Validation
# ---------------------------------------------------------------------------

def validate(loaded: Loaded, catalogue=None, registry=None, approvals=None) -> Validation:
    """Return a typed candidate plus schema/module/global diagnostics."""
    result = Validation(diagnostics=list(loaded.diagnostics))
    if not isinstance(loaded.root, dict):
        result.add("config.root-missing", ROOT_DOCUMENT,
            hint="Install writes the reviewed root document")
        return result
    root = loaded.root
    edge = _validate_edge(root, result)
    certificates = _validate_certificates(root, result)
    dns = _validate_dns(root, result)
    page = _validate_client_page(root, result)
    features = _validate_features(root, result)
    label = _validate_instance_label(root, result)
    _check_cross_fields(edge, certificates, result)
    modules = _validate_modules(loaded.modules, result)
    if edge is not None and not result.blocking:
        result.desired = Root(
            schema_version=int(root.get("schemaVersion", 1)),
            instance=root.get("instance") if isinstance(root.get("instance"), dict) else {},
            installation=root.get("installation") if isinstance(root.get("installation"), dict) else {},
            edge=edge,
            certificates=certificates or {},
            dns=dns,
            client_access_page=page,
            features=features,
            connection_label=label,
        )
        result.modules = modules
    return result


def connection_label(instance) -> str | None:
    """Validated app connection display label from an instance mapping."""
    value = instance.get("label") if isinstance(instance, dict) else None
    if not isinstance(value, str):
        return None
    value = value.strip()
    if not value or len(value) > 64             or any(ord(character) < 32 or ord(character) == 127 for character in value):
        return None
    return value


def _validate_instance_label(root, result) -> str | None:
    instance = root.get("instance")
    raw = instance.get("label") if isinstance(instance, dict) else None
    if raw is None:
        return None
    value = connection_label(instance)
    if value is None:
        result.add("instance.label", "instance.label",
            hint="label must be 1-64 printable characters")
    return value


def _mapping(value) -> dict:
    return value if isinstance(value, dict) else {}


def _validate_edge(root, result) -> Edge | None:
    core = _mapping(root.get("core"))
    raw = core.get("edge")
    if not isinstance(raw, dict):
        result.add("edge.missing", "core.edge")
        return None
    listen_value = raw.get("listen")
    listen = None
    if not isinstance(listen_value, dict):
        result.add("edge.listen.missing", "core.edge.listen")
    else:
        try:
            transport = listen_value.get("transport")
            if transport != "tcp":
                raise ConfigError("edge listen transport must be tcp")
            port = _port(listen_value.get("port"))
            families = listen_value.get("addressFamilies")
            if not isinstance(families, list) or not families \
                    or any(item not in ("ipv4", "ipv6") for item in families) \
                    or len(set(families)) != len(families):
                raise ConfigError("addressFamilies must be a non-empty unique list of ipv4/ipv6")
            scope = listen_value.get("scope")
            if scope is not None and scope not in ("public", "host-loopback", "project"):
                raise ConfigError("invalid listen scope")
            listen = Listen(transport, port, tuple(families), scope)
        except ConfigError as error:
            result.add("edge.listen.invalid", "core.edge.listen", hint=str(error))
    domains = raw.get("domains")
    canon_domains = ()
    if not isinstance(domains, list) or any(not isinstance(item, str) for item in domains):
        result.add("edge.domains.invalid", "core.edge.domains", hint="domains must be a list of names")
    else:
        try:
            canon = [canonical_name(item) for item in domains]
            if len(set(canon)) != len(canon):
                raise ConfigError("domains contain duplicates after canonicalisation")
            canon_domains = tuple(canon)
        except ConfigError as error:
            result.add("edge.domains.invalid", "core.edge.domains", hint=str(error))
    host_policy = None
    if "hostPolicy" in raw:
        policy = raw.get("hostPolicy")
        if not isinstance(policy, dict):
            result.add("edge.hostPolicy.invalid", "core.edge.hostPolicy", hint="must be a mapping")
        else:
            try:
                zone = canonical_name(policy.get("wildcardZone"))
                target = policy.get("redirectTarget")
                unknown = policy.get("unknown", "reject")
                if unknown != "reject":
                    raise ConfigError("hostPolicy.unknown must be 'reject'")
                if not isinstance(target, str) or not target.startswith("https://") or not target.endswith("/"):
                    raise ConfigError("redirectTarget must be an absolute https URL ending in '/'")
                if target.count("/") != 3:
                    raise ConfigError("redirectTarget must be an apex root URL without a path")
                host_policy = HostPolicy(zone, target, unknown)
            except ConfigError as error:
                result.add("edge.hostPolicy.invalid", "core.edge.hostPolicy", hint=str(error))
    test_tunnel = None
    if "testTunnel" in raw:
        value = raw.get("testTunnel")
        if not isinstance(value, dict):
            result.add("edge.testTunnel.invalid", "core.edge.testTunnel", hint="must be a mapping")
        else:
            enabled = value.get("enabled", True)
            if not isinstance(enabled, bool):
                result.add("edge.testTunnel.enabled", "core.edge.testTunnel.enabled",
                    hint="enabled must be a boolean")
            else:
                try:
                    sni = canonical_name(value.get("sni"))
                    port = _port(value.get("port"), minimum=1024)
                    if canon_domains and (
                        sni in canon_domains
                        or sni == "www." + canon_domains[0]
                        or (host_policy is not None and sni == "*." + host_policy.wildcard_zone)
                    ):
                        raise ConfigError("test SNI must be separate from the site names")
                    if enabled:
                        test_tunnel = TestTunnel(sni, port)
                except ConfigError as error:
                    result.add("edge.testTunnel.invalid", "core.edge.testTunnel", hint=str(error))
    if listen is None or not canon_domains:
        return None
    if host_policy is not None:
        apex = canon_domains[0]
        if host_policy.redirect_target != f"https://{apex}/":
            result.add("edge.hostPolicy.redirect-mismatch", "core.edge.hostPolicy.redirectTarget",
                hint="redirectTarget must equal the canonical apex root")
    return Edge(canon_domains, listen, host_policy, test_tunnel)


def _validate_certificates(root, result) -> dict | None:
    core = _mapping(root.get("core"))
    raw = core.get("certificates")
    if not isinstance(raw, dict):
        result.add("certificates.missing", "core.certificates")
        return None
    out = {}
    if raw.get("ca") != "letsencrypt":
        result.add("certificates.ca", "core.certificates.ca")
    challenge = raw.get("challenge")
    if challenge not in ("dns-01", "http-01"):
        result.add("certificates.challenge", "core.certificates.challenge")
    out["challenge"] = challenge
    email = raw.get("email")
    if not isinstance(email, str) or "@" not in email or len(email) > 254:
        result.add("certificates.email", "core.certificates.email")
    out["email"] = email
    if "wildcardDomain" in raw:
        try:
            out["wildcardDomain"] = canonical_name(raw.get("wildcardDomain"))
        except ConfigError as error:
            result.add("certificates.wildcardDomain", "core.certificates.wildcardDomain", hint=str(error))
    credentials = raw.get("credentials")
    if not isinstance(credentials, dict):
        result.add("certificates.credentials", "core.certificates.credentials")
    else:
        for key in ("apiKey", "apiPassword"):
            item = credentials.get(key)
            if not isinstance(item, dict) or not _SECRET_REF.match(str(item.get("secretRef", ""))):
                result.add("certificates.credentials.invalid", f"core.certificates.credentials.{key}")
        out["credentials"] = credentials
    return out


def _validate_dns(root, result) -> Dns | None:
    core = _mapping(root.get("core"))
    raw = core.get("dns")
    if raw is None:
        return None
    if not isinstance(raw, dict):
        result.add("dns.invalid", "core.dns")
        return None
    mode = raw.get("upstreamMode")
    if mode not in ("tls-forward", "recursive"):
        result.add("dns.upstreamMode", "core.dns.upstreamMode")
        return None
    upstreams_raw = raw.get("upstreams", {})
    if not isinstance(upstreams_raw, dict):
        result.add("dns.upstreams", "core.dns.upstreams")
        return None
    upstreams = {}
    for key, item in upstreams_raw.items():
        path = f"core.dns.upstreams.{key}"
        if not _UPSTREAM_ID.match(str(key)):
            result.add("dns.upstream.id", path)
            continue
        if not isinstance(item, dict):
            result.add("dns.upstream.invalid", path)
            continue
        try:
            addresses = item.get("addresses")
            if not isinstance(addresses, list) or not addresses:
                raise ConfigError("addresses must be a non-empty list")
            canonical = tuple(_ipv4(address) for address in addresses)
            port = _port(item.get("port"))
            tls_name = canonical_name(item.get("tlsName"))
        except ConfigError as error:
            result.add("dns.upstream.invalid", path, hint=str(error))
            continue
        upstreams[key] = DnsUpstream(canonical, port, tls_name)
    order = raw.get("upstreamOrder")
    fallback = raw.get("fallback", "fail-closed")
    if mode == "tls-forward":
        if not isinstance(order, list) or not order:
            result.add("dns.upstreamOrder.required", "core.dns.upstreamOrder",
                hint="tls-forward requires a non-empty ordered upstream list")
            order = ()
        elif any(not isinstance(key, str) or key not in upstreams for key in order) or len(set(order)) != len(order):
            result.add("dns.upstreamOrder.invalid", "core.dns.upstreamOrder")
            order = ()
        else:
            order = tuple(order)
        if fallback != "fail-closed":
            result.add("dns.fallback", "core.dns.fallback",
                hint="no hidden plaintext or OS fallback is allowed")
        if not upstreams:
            result.add("dns.upstreams.empty", "core.dns.upstreams")
    else:
        if order is not None and (not isinstance(order, list) or any(not isinstance(key, str) for key in order)):
            result.add("dns.upstreamOrder.invalid", "core.dns.upstreamOrder")
            order = ()
        else:
            order = tuple(order or ())
    return Dns(mode, upstreams, order, fallback)


def _validate_client_page(root, result) -> ClientPage:
    core = _mapping(root.get("core"))
    raw = core.get("clientAccessPage")
    if raw is None:
        return ClientPage()
    if not isinstance(raw, dict):
        result.add("clientPage.invalid", "core.clientAccessPage")
        return ClientPage()
    path = raw.get("path", "")
    if not isinstance(path, str):
        result.add("clientPage.path", "core.clientAccessPage.path")
        path = ""
    elif path:
        if "/" in path or path in (".", "..") or not _PAGE_SEGMENT.match(path):
            result.add("clientPage.path.invalid", "core.clientAccessPage.path",
                hint="path must be a single case-sensitive URL segment")
            path = ""
    names = raw.get("names", {})
    if not isinstance(names, dict):
        result.add("clientPage.names", "core.clientAccessPage.names")
        names = {}
    else:
        for key, value in names.items():
            if not isinstance(key, str) or not key or not isinstance(value, str):
                result.add("clientPage.name.invalid", "core.clientAccessPage.names", hint="names must map identifiers to strings")
                break
    refresh = raw.get("refreshSeconds", 60)
    if type(refresh) is not int or not 10 <= refresh <= 3600:
        result.add("clientPage.refreshSeconds", "core.clientAccessPage.refreshSeconds")
        refresh = 60
    return ClientPage(path, dict(names), refresh)


def client_page(root) -> ClientPage:
    """Parse only the client-page settings, independent of other diagnostics."""
    result = Validation()
    page = _validate_client_page(_mapping(root), result)
    return ClientPage() if result.blocking else page


def _validate_features(root, result) -> dict:
    features = root.get("features", {})
    if not isinstance(features, dict):
        result.add("features.invalid", "features")
        return {}
    out = {}
    for key, value in features.items():
        if not isinstance(value, dict) or set(value) - {"enabled"}:
            result.add("features.invalid", f"features.{key}",
                hint="a feature object carries only 'enabled'")
            continue
        if not isinstance(value.get("enabled"), bool):
            result.add("features.enabled", f"features.{key}.enabled")
            continue
        out[key] = {"enabled": value["enabled"]}
    return out


def _check_cross_fields(edge, certificates, result) -> None:
    if edge is None or certificates is None:
        return
    wildcard = certificates.get("wildcardDomain")
    if edge.host_policy is not None and wildcard:
        if edge.host_policy.wildcard_zone != wildcard:
            result.add("config.wildcard-mismatch", "core.edge.hostPolicy.wildcardZone",
                hint="wildcard zone and certificate wildcardDomain must match")
        if certificates.get("challenge") != "dns-01":
            result.add("config.wildcard-challenge", "core.certificates.challenge",
                hint="a wildcard profile requires dns-01")


def _validate_modules(documents, result) -> dict:
    modules = {}
    if "foreign-amnezia" in documents:
        value = _validate_foreign(documents["foreign-amnezia"], result)
        if value is not None:
            modules["foreign-amnezia"] = value
    if "mtproto" in documents:
        value = _validate_mtproto(documents["mtproto"], result)
        if value is not None:
            modules["mtproto"] = value
    if "outline-wss" in documents:
        value = _validate_outline(documents["outline-wss"], result)
        if value is not None:
            modules["outline-wss"] = value
    if "hysteria" in documents:
        value = _validate_hysteria(documents["hysteria"], result)
        if value is not None:
            modules["hysteria"] = value
    if "amneziawg" in documents:
        value = _validate_amneziawg(documents["amneziawg"], result)
        if value is not None:
            modules["amneziawg"] = value
    return modules


def _module_settings(document_value, module_id, result) -> dict | None:
    if not isinstance(document_value, dict):
        result.add("module.not-mapping", f"modules/{module_id}.yaml", module=module_id)
        return None
    if document_value.get("schemaVersion", 1) not in SUPPORTED_SCHEMA_VERSIONS:
        result.add("module.version", f"modules/{module_id}.yaml", module=module_id)
        return None
    if document_value.get("moduleId") != module_id:
        result.add("module.identity", f"modules/{module_id}.yaml", module=module_id,
            hint="moduleId must match the file identity")
        return None
    settings = document_value.get("settings", {})
    if not isinstance(settings, dict):
        result.add("module.settings", f"modules/{module_id}.yaml", module=module_id)
        return None
    return settings


def _name_list(value, path, module, result, *, required=False):
    if value is None:
        if required:
            result.add("module.name.required", path, module=module)
        return ()
    if not isinstance(value, list):
        result.add("module.name.invalid", path, module=module)
        return ()
    try:
        canonical = tuple(canonical_name(item) for item in value)
    except ConfigError as error:
        result.add("module.name.invalid", path, module=module, hint=str(error))
        return ()
    if len(set(canonical)) != len(canonical):
        result.add("module.name.duplicate", path, module=module)
    return canonical


def _validate_foreign(document_value, result) -> ForeignAmnezia | None:
    module = "foreign-amnezia"
    settings = _module_settings(document_value, module, result)
    if settings is None:
        return None
    mode = settings.get("mode", "associate")
    if mode not in ("associate", "provision"):
        result.add("module.mode", f"modules/{module}.yaml:settings.mode", module=module)
        return None
    container = settings.get("container")
    if container is not None and (not isinstance(container, str) or not _CONTAINER.match(container)):
        result.add("module.container", f"modules/{module}.yaml:settings.container", module=module,
            hint="container must be a bounded Docker name")
        container = None
    server_names = _name_list(settings.get("serverNames"), f"modules/{module}.yaml:settings.serverNames", module, result)
    compatibility = settings.get("compatibility", {})
    legacy = ()
    if compatibility is not None:
        if not isinstance(compatibility, dict):
            result.add("module.compatibility", f"modules/{module}.yaml:settings.compatibility", module=module)
        else:
            legacy = _name_list(compatibility.get("legacyServerNames"),
                f"modules/{module}.yaml:settings.compatibility.legacyServerNames", module, result)
    host_ports = settings.get("hostPorts")
    ports = {}
    if host_ports is not None:
        if not isinstance(host_ports, dict) or set(host_ports) - {"tcp", "udp"}:
            result.add("module.hostPorts", f"modules/{module}.yaml:settings.hostPorts", module=module)
        else:
            try:
                tcp = _port(host_ports.get("tcp"), minimum=5000, maximum=6000)
                udp = _port(host_ports.get("udp"), minimum=5000, maximum=6000)
                if tcp != udp:
                    raise ConfigError("host tcp and udp must share one allocated value")
                ports = {"tcp": tcp, "udp": udp}
            except ConfigError as error:
                result.add("module.hostPorts.invalid", f"modules/{module}.yaml:settings.hostPorts",
                    module=module, hint=str(error))
    client_name = None
    admin = {}
    if mode == "provision":
        provision = settings.get("provision", {})
        if not isinstance(provision, dict):
            result.add("module.provision", f"modules/{module}.yaml:settings.provision", module=module)
        else:
            name = provision.get("clientName")
            if name is not None:
                if not isinstance(name, str) or not _CLIENT_NAME.match(name):
                    result.add("module.clientName", f"modules/{module}.yaml:settings.provision.clientName", module=module)
                else:
                    client_name = name
        admin_value = settings.get("admin", {})
        if not isinstance(admin_value, dict):
            result.add("module.admin", f"modules/{module}.yaml:settings.admin", module=module)
        else:
            access = admin_value.get("access", "none")
            if access not in ("generate", "none"):
                result.add("module.admin.access", f"modules/{module}.yaml:settings.admin.access", module=module)
            user = admin_value.get("user")
            if user is not None and (not isinstance(user, str) or not _USERNAME.match(user)):
                result.add("module.admin.user", f"modules/{module}.yaml:settings.admin.user", module=module)
                user = None
            ssh_port = admin_value.get("sshPort")
            if ssh_port is not None:
                try:
                    ssh_port = _port(ssh_port)
                except ConfigError as error:
                    result.add("module.admin.sshPort", f"modules/{module}.yaml:settings.admin.sshPort",
                        module=module, hint=str(error))
                    ssh_port = None
            if access == "generate" and (user is None or ssh_port is None):
                result.add("module.admin.required", f"modules/{module}.yaml:settings.admin", module=module,
                    hint="generate requires a configured user and sshPort")
            admin = {"access": access, "user": user, "sshPort": ssh_port}
    return ForeignAmnezia(mode, container, server_names, legacy, ports, client_name, admin)


def _validate_mtproto(document_value, result) -> Mtproto | None:
    module = "mtproto"
    settings = _module_settings(document_value, module, result)
    if settings is None:
        return None
    backend = settings.get("backend", "mtg")
    if backend not in ("mtg", "telemt"):
        result.add("module.backend", f"modules/{module}.yaml:settings.backend", module=module)
        return None
    camouflage = settings.get("camouflageName")
    if camouflage is not None:
        try:
            camouflage = canonical_name(camouflage)
        except ConfigError as error:
            result.add("module.camouflageName", f"modules/{module}.yaml:settings.camouflageName",
                module=module, hint=str(error))
            camouflage = None
    return Mtproto(backend, camouflage)


def _http_path(value, path, module, result):
    if value is None:
        return None
    if isinstance(value, dict):
        if set(value) != {"secretRef"} or not _SECRET_REF.match(str(value.get("secretRef", ""))):
            result.add("module.path.secretRef", path, module=module)
            return None
        return {"secretRef": value["secretRef"]}
    if not isinstance(value, str) or not _HTTP_PATH.match(value):
        result.add("module.path.invalid", path, module=module,
            hint="path must be an absolute HTTP path")
        return None
    return value


def _validate_outline(document_value, result) -> OutlineWss | None:
    module = "outline-wss"
    settings = _module_settings(document_value, module, result)
    if settings is None:
        return None
    server_name = settings.get("serverName")
    if server_name is not None:
        try:
            server_name = canonical_name(server_name)
        except ConfigError as error:
            result.add("module.serverName", f"modules/{module}.yaml:settings.serverName",
                module=module, hint=str(error))
            server_name = None
    stream = _http_path(settings.get("streamPath"), f"modules/{module}.yaml:settings.streamPath", module, result)
    packet = _http_path(settings.get("packetPath"), f"modules/{module}.yaml:settings.packetPath", module, result)
    return OutlineWss(server_name, stream, packet)


def _validate_hysteria(document_value, result) -> Hysteria | None:
    module = "hysteria"
    settings = _module_settings(document_value, module, result)
    if settings is None:
        return None
    hostname = settings.get("hostname")
    if hostname is not None:
        try:
            hostname = canonical_name(hostname)
        except ConfigError as error:
            result.add("module.hostname", f"modules/{module}.yaml:settings.hostname",
                module=module, hint=str(error))
            hostname = None
    listen = None
    raw = settings.get("listen")
    if raw is not None:
        try:
            if not isinstance(raw, dict):
                raise ConfigError("listen must be a mapping")
            if raw.get("transport") != "udp":
                raise ConfigError("hysteria listen transport must be udp")
            port = _port(raw.get("port"))
            families = raw.get("addressFamilies")
            if not isinstance(families, list) or not families or any(item not in ("ipv4", "ipv6") for item in families):
                raise ConfigError("addressFamilies must be a non-empty list")
            scope = raw.get("scope")
            if scope is not None and scope not in ("public", "host-loopback", "project"):
                raise ConfigError("invalid listen scope")
            listen = Listen("udp", port, tuple(families), scope)
        except ConfigError as error:
            result.add("module.listen.invalid", f"modules/{module}.yaml:settings.listen",
                module=module, hint=str(error))
    return Hysteria(hostname, listen)


def _validate_amneziawg(document_value, result) -> AmneziaWG | None:
    module = "amneziawg"
    settings = _module_settings(document_value, module, result)
    if settings is None:
        return None
    if settings.get("backend", "amneziawg-go") != "amneziawg-go":
        result.add("module.backend", f"modules/{module}.yaml:settings.backend", module=module)
        return None
    hostname = settings.get("hostname")
    if hostname is not None:
        try:
            hostname = canonical_name(hostname)
        except ConfigError as error:
            result.add("module.hostname", f"modules/{module}.yaml:settings.hostname",
                module=module, hint=str(error))
            hostname = None
    subnet = settings.get("subnet")
    if subnet is not None:
        try:
            network = ipaddress.ip_network(subnet, strict=True)
            if network.version != 4 or network.prefixlen > 30:
                raise ValueError
            subnet = str(network)
        except (TypeError, ValueError):
            result.add("module.subnet.invalid", f"modules/{module}.yaml:settings.subnet",
                module=module, hint="subnet must be one IPv4 /30 or larger network")
            subnet = None
    listen = None
    raw = settings.get("listen")
    if raw is not None:
        try:
            if not isinstance(raw, dict):
                raise ConfigError("listen must be a mapping")
            if raw.get("transport") != "udp":
                raise ConfigError("amneziawg listen transport must be udp")
            if raw.get("scope") != "public":
                raise ConfigError("amneziawg listen scope must be public")
            port = _port(raw.get("port"))
            families = raw.get("addressFamilies")
            if not isinstance(families, list) or not families or any(item not in ("ipv4", "ipv6") for item in families):
                raise ConfigError("addressFamilies must be a non-empty list")
            listen = Listen("udp", port, tuple(families), "public")
        except ConfigError as error:
            result.add("module.listen.invalid", f"modules/{module}.yaml:settings.listen",
                module=module, hint=str(error))
    return AmneziaWG(hostname, subnet, listen)


# ---------------------------------------------------------------------------
# Name inventory
# ---------------------------------------------------------------------------

def resolved_names(validation: Validation) -> list:
    """Derive the single name inventory from a valid root/module candidate."""
    desired = validation.desired
    if desired is None:
        return []
    inventory = []
    seen = set()

    def add(name, kind, source):
        if name and name not in seen:
            seen.add(name)
            inventory.append({"name": name, "kind": kind, "source": source})

    for domain in desired.edge.domains:
        add(domain, "site", "core.edge.domains")
    if desired.edge.host_policy is not None:
        add("*." + desired.edge.host_policy.wildcard_zone, "wildcard", "core.edge.hostPolicy")
    if desired.edge.test_tunnel is not None:
        add(desired.edge.test_tunnel.sni, "test-tunnel", "core.edge.testTunnel")
    amnezia = validation.modules.get("foreign-amnezia")
    if amnezia is not None:
        for name in amnezia.server_names:
            add(name, "reality", "foreign-amnezia.serverNames")
        for name in amnezia.legacy_server_names:
            add(name, "legacy", "foreign-amnezia.compatibility.legacyServerNames")
    mtproto = validation.modules.get("mtproto")
    if mtproto is not None and mtproto.camouflage_name:
        add(mtproto.camouflage_name, "mtproto", "mtproto.camouflageName")
    outline = validation.modules.get("outline-wss")
    if outline is not None and outline.server_name:
        add(outline.server_name, "outline", "outline-wss.serverName")
    hysteria = validation.modules.get("hysteria")
    if hysteria is not None and hysteria.hostname:
        add(hysteria.hostname, "hysteria", "hysteria.hostname")
    return inventory
