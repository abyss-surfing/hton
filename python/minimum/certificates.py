"""Pinned HTTP/DNS ACME issue/reuse and trusted certificate validation."""
from __future__ import annotations

import json
import getpass
import os
from pathlib import Path
import secrets
import ssl
import stat
import socket
import http_challenge
import warnings

from docker import command, run
from storage import InstallError, Store, digest


def validate(chain: Path, key: Path, domain: str, remaining=86400):
    if chain.is_symlink() or key.is_symlink():
        raise InstallError("Certificate input must not be a symlink")
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(chain, key, password=lambda: b"")
    leaf = run(["/usr/bin/openssl", "x509", "-in", str(chain), "-outform", "PEM"])
    run(["/usr/bin/openssl", "x509", "-in", str(chain), "-checkend", str(remaining), "-noout"])
    run(["/usr/bin/openssl", "verify", "-CAfile", "/etc/ssl/certs/ca-certificates.crt", "-verify_hostname", domain,
        "-untrusted", str(chain), "/dev/stdin"], data=leaf)


def credentials(config: Store, source: str | None, *, interactive=True):
    saved = config.json("secrets/minimum-internetbs.json")
    if source:
        path = Path(source)
        if not path.is_absolute() or path.resolve() != path:
            raise InstallError("DNS credential input must be an absolute canonical private file")
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        try:
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_mode & 0o077 or info.st_uid not in (0, int(os.environ.get("SUDO_UID", "0"))):
                raise InstallError("DNS credential input must be private and owned by root or the invoking sudo user")
            raw = os.read(fd, 65537)
            if len(raw) > 65536:
                raise InstallError("DNS credential input exceeds its bound")
            incoming = json.loads(raw)
        finally:
            os.close(fd)
        if saved is not None and saved != incoming:
            raise InstallError("Saved DNS credentials differ; explicit rotation is required")
        saved = incoming
    if saved is None and interactive:
        try:
            with warnings.catch_warnings():
                warnings.simplefilter("error", getpass.GetPassWarning)
                saved = {"apiKey": getpass.getpass("InternetBS API key (stored privately): "),
                    "apiPassword": getpass.getpass("InternetBS API password (stored privately): ")}
        except (OSError, getpass.GetPassWarning) as error:
            raise InstallError("DNS credentials require a local TTY or --dns-credentials PRIVATE_JSON_FILE") from error
    if not isinstance(saved, dict) or set(saved) != {"apiKey", "apiPassword"} or any(
        not isinstance(value, str) or not value or len(value) > 4096 or any(ord(c) < 32 or c in "\r\n\x00" for c in value)
        for value in saved.values()
    ):
        raise InstallError("DNS credentials required: provide a private JSON file with apiKey and apiPassword using --dns-credentials; never send secrets in chat")
    if any(value.startswith("CHANGE-ME") for value in saved.values()):
        raise InstallError("DNS credentials are still placeholders; fill secrets.dns.internetbsApiKey and secrets.dns.internetbsApiPassword in .secrets/profile.yml, then rerun scripts/secrets import")
    config.save("secrets/minimum-internetbs.json", saved)
    return saved


def output_directory(work, relative):
    parts = Path(relative).parts
    if len(parts) != 2 or parts[0] != "certificates" or len(parts[1]) != 24 or any(c not in "0123456789abcdef" for c in parts[1]):
        raise InstallError("Certificate custody reference is invalid")
    with work.parent(relative + "/fullchain.pem"):
        pass
    return work.root / relative


def validate_names(chain, key, names, remaining=86400):
    for name in names:
        validate(chain, key, name, remaining)


def validate_wildcard(chain, domain):
    from cryptography import x509

    try:
        leaf = x509.load_pem_x509_certificate(chain.read_bytes())
        names = leaf.extensions.get_extension_for_class(x509.SubjectAlternativeName)
    except (OSError, ValueError, x509.ExtensionNotFound) as error:
        raise InstallError("Issued certificate does not contain a valid SAN extension") from error
    if "*." + domain.lower() not in {name.lower() for name in names.value.get_values_for_type(x509.DNSName)}:
        raise InstallError("Issued certificate does not contain the requested wildcard name")


def accept_output(output, names):
    for name in ("fullchain.pem", "key.pem"):
        fd = os.open(output / name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        try:
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_uid != 0 or info.st_size > 262144:
                raise InstallError("Issuer produced unsafe certificate files")
            os.fchmod(fd, 0o600)
            os.fsync(fd)
        finally:
            os.close(fd)
    trust_names = tuple(names) if isinstance(names, (tuple, list)) else (names,)
    validate_names(output / "fullchain.pem", output / "key.pem", trust_names)


def cached_output(work, relative, domain):
    """Reuse a complete trusted native issuer cache before requesting issuance."""
    base = work.root / "issuer" / "certs" / (domain + "_ecc")
    for parent in (base.parent.parent, base.parent, base):
        if not parent.exists():
            return False
        info = parent.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
            raise InstallError("Native issuer cache directory is unsafe")
    content = {}
    for source, destination in (("fullchain.cer", "fullchain.pem"), (domain + ".key", "key.pem")):
        try:
            fd = os.open(base / source, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        except FileNotFoundError:
            return False
        try:
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_nlink != 1 or info.st_mode & 0o022 or info.st_size > 262144:
                raise InstallError("Native issuer cache file is unsafe")
            content[destination] = os.read(fd, 262145)
        finally:
            os.close(fd)
    for name, data in content.items():
        work.write(relative + "/" + name, data)
    directory = output_directory(work, relative)
    try:
        validate(directory / "fullchain.pem", directory / "key.pem", domain, renewal_window(directory / "fullchain.pem"))
    except (InstallError, OSError, ssl.SSLError):
        return False
    return True


# Core invokes the native issuer only when trusted material enters its renewal
# window. --days 1 makes native eligibility compatible with short certificate
# lifetimes; valid-certificate and cache reuse prevent daily issuance.
def renewal_window(chain):
    dates = run(["/usr/bin/openssl", "x509", "-in", str(chain), "-noout", "-dates"]).decode("ascii")
    values = dict(line.split("=", 1) for line in dates.splitlines())
    lifetime = ssl.cert_time_to_seconds(values["notAfter"]) - ssl.cert_time_to_seconds(values["notBefore"])
    if lifetime <= 0:
        raise InstallError("Certificate validity interval is invalid")
    return max(86400, min(30 * 86400, int(lifetime / 3)))


def select_policy(root, state, config, dns_source=None, *, defaults=None):
    defaults = defaults or {}
    metadata = {**defaults, **root.get("core", {}).get("certificates", {})}
    if metadata.get("ca", "letsencrypt") != "letsencrypt":
        raise InstallError("This issuer adapter currently supports Let's Encrypt; no alternate CA was selected silently")
    challenge = metadata.get("challenge", "dns-01" if dns_source else "http-01")
    legacy = (state and not state.get("hasCommitted") and not state.get("certificateChallenge")
        and not state.get("certificate") and not state.get("issuerJob") and not dns_source
        and defaults.get("challenge") != "dns-01" and not config.json("secrets/minimum-internetbs.json"))
    if challenge == "dns-01" and legacy:
        challenge = "http-01"
    if challenge not in ("http-01", "dns-01"):
        raise InstallError("Certificate challenge must be http-01 or dns-01")
    if state and state.get("certificateChallenge", challenge) != challenge:
        raise InstallError("Saved certificate challenge differs; explicit migration is required")
    if challenge == "dns-01" and metadata.get("provider", "internetbs") != "internetbs":
        raise InstallError("The selected DNS API adapter is not implemented; use HTTP-01 without DNS credentials or install the required provider adapter")
    if challenge == "http-01" and dns_source:
        raise InstallError("DNS credentials were supplied for HTTP-01; remove --dns-credentials or explicitly select dns-01")
    return challenge


def require_http_port(port=80):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        try:
            listener.bind(("0.0.0.0", port))
        except OSError as error:
            raise InstallError("HTTP-01 requires a free local TCP/80. Another service owns it; configure challenge routing or select a DNS API adapter") from error


def issuer_arguments(work, state, image, token, output, domain, email, challenge, *, http=None, names=(), wildcard=False):
    arguments = ["container", "create", "--pull", "never", "--name", "hton-minimum-acme-" + token,
        "--user", "0:0", "--label", "io.hton.minimum=" + state["instance"], "--label", "io.hton.role=acme",
        "--log-driver", "none", "--memory", "128m", "--pids-limit", "64", "--read-only", "--cap-drop", "ALL",
        "--security-opt", "no-new-privileges:true", "--env", "AUTO_UPGRADE=0",
        "--tmpfs", "/tmp:rw,nosuid,noexec,size=16777216",
        "--mount", "type=bind,src=" + str(work.root / "issuer") + ",dst=/acme.sh",
        "--mount", "type=bind,src=" + str(output) + ",dst=/output"]
    if challenge == "http-01":
        arguments += ["--network", "host"]
        if http and http.get("mode") == "webroot":
            directory = Path(http["webroot"]) / ".well-known/acme-challenge"
            arguments += ["--tmpfs", "/challenge:rw,nosuid,noexec,size=1048576",
                "--mount", "type=bind,src=" + str(directory) + ",dst=/challenge/.well-known/acme-challenge"]
            validation = ["--webroot", "/challenge"]
        else:
            arguments += ["--cap-add", "NET_BIND_SERVICE"]
            validation = ["--standalone", "--listen-v4", "--httpport", "80"]
    elif challenge == "dns-01":
        arguments += ["--env-file", str(work.root / "issuer/dns.env")]
        validation = ["--dns", "dns_internetbs", "--dnssleep", "120"]
    else:
        raise InstallError("Unsupported certificate challenge")
    arguments += ["--entrypoint", "/usr/local/bin/acme.sh", image, "--issue", "--home", "/acmebin",
        "--config-home", "/acme.sh", "--cert-home", "/acme.sh/certs", *validation,
        "--server", "letsencrypt", "--accountemail", email, "--keylength", "ec-256", "--days", "1",
        "--fullchain-file", "/output/fullchain.pem", "--key-file", "/output/key.pem", "-d", domain]
    if wildcard:
        arguments += ["-d", "*." + domain]
    for name in names:
        arguments += ["-d", name]
    return arguments


def ensure(config: Store, work: Store, state: dict, pins: dict, domain: str, email: str, source: str | None, save, *, interactive=True, challenge="http-01", http_settings=None, names=(), wildcard=False):
    names = tuple(names)
    trust_names = (domain, *names)
    from ingress import record_owned, owned
    from docker import inventory
    if state.get("issuerJob"):
        identifier = state["issuerJob"]
        receipt = state.get("ownedReceipts", {}).get(identifier)
        if not receipt or receipt["role"] != "acme":
            raise InstallError("Interrupted issuer has no exact creation receipt")
        output = output_directory(work, state["issuerOutput"])
        if identifier in inventory():
            owned(state, identifier, "acme", work)
            try:
                command("container", "wait", identifier, timeout=300)
                accept_output(output, trust_names)
                if wildcard:
                    validate_wildcard(output / "fullchain.pem", domain)
                state["certificate"] = state["issuerOutput"]
                state["pendingCertificate"] = state["issuerOutput"]
                save()
            finally:
                owned(state, identifier, "acme", work)
                command("container", "rm", "--force", identifier)
                state.pop("issuerJob", None)
                state.pop("issuerOutput", None)
                state["ownedReceipts"].pop(identifier, None)
                save()
        else:
            accept_output(output, trust_names)
            if wildcard:
                validate_wildcard(output / "fullchain.pem", domain)
            state["certificate"] = state["issuerOutput"]
            state["pendingCertificate"] = state.pop("issuerOutput")
            state.pop("issuerJob", None)
            state["ownedReceipts"].pop(identifier, None)
            save()
    selected = state.get("pendingCertificate") or state.get("certificate")
    if selected:
        directory = output_directory(work, selected)
        chain, key = directory / "fullchain.pem", directory / "key.pem"
        try:
            validate_names(chain, key, trust_names, renewal_window(chain))
            if wildcard:
                validate_wildcard(chain, domain)
            state["certificate"] = selected
            save()
            return chain.parent
        except (InstallError, OSError, ssl.SSLError):
            pass
    token = secrets.token_hex(12)
    relative = "certificates/" + token
    # Protected provider material and issuer account state are separate from YAML.
    work.write("issuer/owner", b"hton-minimum-issuer-v1\n")
    work.write(relative + "/pending", b"pending\n")
    issuer = work.root / "issuer"
    output = work.root / relative
    if not names and not wildcard and cached_output(work, relative, domain):
        state["certificate"] = relative
        state["pendingCertificate"] = relative
        save()
        return output
    http = None
    if http_settings is not None and not isinstance(http_settings, dict):
        raise InstallError("core.certificates.http must be a configuration mapping")
    if challenge == "http-01":
        http = http_challenge.select(domain, state.get("httpChallenge"), (http_settings or {}).get("webroot"))
        if http["mode"] == "standalone":
            require_http_port()
        else:
            with http_challenge.open_directory(Path(http["webroot"]), owner=http.get("ownerUid")):
                pass
        state["httpChallenge"] = http
        save()
    elif challenge == "dns-01":
        secret = credentials(config, source, interactive=interactive)
        work.write("issuer/dns.env", ("INTERNETBS_API_KEY=" + secret["apiKey"] + "\nINTERNETBS_API_PASSWORD=" + secret["apiPassword"] + "\n").encode())
    arguments = issuer_arguments(work, state, pins["acme"], token, output, domain, email, challenge, http=http, names=names, wildcard=wildcard)
    created = command(*arguments).decode().strip()
    from docker import exact_id, inspect
    created = exact_id(created)
    state["issuerJob"] = created
    state["issuerOutput"] = relative
    record_owned(work, state, created, "acme")
    save()
    try:
        # Native issuer output can contain provider diagnostics; never print it.
        command("container", "start", "--attach", created, timeout=300, allowed=(0, 1, 2))
        finished = inspect(created)["State"]
        if finished["Running"]:
            raise InstallError("Certificate job did not terminate")
        if finished["ExitCode"] not in (0, 2):
            if challenge == "http-01":
                raise InstallError("Let's Encrypt HTTP-01 failed. Check public DNS, inbound TCP/80, firewall/NAT forwarding and CAA records. Amnezia has not been remapped by certificate issuance")
            raise InstallError("DNS-01 issuance failed; check the explicitly configured DNS API account and domain authority")
        if not (output / "fullchain.pem").exists() and not names:
            cached_output(work, relative, domain)
        accept_output(output, trust_names)
        if wildcard:
            validate_wildcard(output / "fullchain.pem", domain)
        state["certificate"] = relative
        state["pendingCertificate"] = relative
        save()
        return output
    finally:
        owned(state, created, "acme", work)
        command("container", "rm", "--force", created)
        state.pop("issuerJob", None)
        state.pop("issuerOutput", None)
        state["ownedReceipts"].pop(created, None)
        save()
