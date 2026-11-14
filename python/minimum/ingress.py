"""Immutable site/edge artifacts, owned containers and real local probes."""
from __future__ import annotations

import base64
import copy
import json
import os
from pathlib import Path
import secrets
import socket
import ssl
import stat
import time

from docker import ROOT, NativeError, command, exact_id, inspect, native_config, run
from storage import InstallError, Store, digest


DEFAULT_SITE_ROOTS = (Path("/var/www/html"), Path("/var/www"))
INDEX_NAMES = ("index.html", "index.htm")
DENIAL_BODY = (b"<!DOCTYPE html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\"/>\n"
    b"<title>403 Forbidden</title></head>\n<body>\n<h1>Access denied</h1>\n"
    b"<p>You do not have permission to access this resource.</p>\n</body></html>\n")


def index_path(root: Path):
    """The first regular index file in a document root, or None."""
    for name in INDEX_NAMES:
        index = root / name
        if not index.is_symlink() and index.is_file():
            return index
    return None


def site_root(configured=None):
    """Return the website document root, or None for the placeholder.

    An explicitly configured root is authoritative: when it is unusable the
    install fails instead of silently serving the placeholder over a real site.
    """
    if configured is not None:
        if not isinstance(configured, str) or not configured.startswith("/"):
            raise InstallError("Configured website root must be an absolute path")
        root = Path(configured)
        if root.is_symlink() or root.resolve() != root or not root.is_dir():
            raise InstallError("Configured website root is missing, a symlink or not a directory")
        if index_path(root) is None:
            raise InstallError("Configured website root has no index.html")
        return root
    for root in DEFAULT_SITE_ROOTS:
        if root.is_symlink() or not root.is_dir():
            continue
        if index_path(root) is None:
            continue
        return root
    return None


def webroot(configured=None) -> bytes:
    if configured is None:
        return (b'<!DOCTYPE HTML PUBLIC "-//IETF//DTD HTML 2.0//EN">\n'
            b"<html><head>\n<title>403 Forbidden</title>\n</head><body>\n"
            b"<h1>Forbidden</h1>\n<p>You don't have permission to access this resource.</p>\n"
            b"</body></html>\n")
    root = site_root(configured)
    path = index_path(root)
    if path is None:
        raise InstallError("Website source changed before reading")
    if path.parent.resolve() != path.parent:
        raise InstallError("Website source directory contains a symlink")
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        info = os.fstat(fd)
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > 1024 * 1024:
            raise InstallError("Website source must be one regular file of at most 1 MiB")
        body = os.read(fd, 1024 * 1024 + 1)
        after = os.fstat(fd)
        if len(body) > 1024 * 1024 or (info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns) != (after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns):
            raise InstallError("Website source changed while reading")
        body.decode("utf-8")
        return body
    finally:
        os.close(fd)


def configs(domain, names, vpn_port, site_port, body, root=None, mtproto=None, outline=None,
            selfsteal=None, chain_sni=None, native_selfsteal=False, site_bind=None, client_page=None,
            reality_names=(), test_tunnel=None):
    if native_selfsteal:
        if selfsteal:
            raise InstallError("Native self-steal and a separate REALITY container are mutually exclusive")
        if (chain_sni or domain) not in names:
            raise InstallError("Native self-steal requires the own SNI in the REALITY names")
    else:
        if domain in names:
            raise InstallError("Site and REALITY SNI must be distinct")
        if chain_sni and chain_sni in names:
            raise InstallError("Chain SNI must not collide with the old REALITY SNI")
    camouflage = mtproto["camouflage"] if mtproto and not chain_sni else None
    if camouflage and camouflage in [domain] + list(names):
        raise InstallError("MTProto camouflage must differ from the site and REALITY SNI")
    www = "www." + domain
    test_route = test_backend = ""
    test_name = None
    if test_tunnel is not None:
        if not isinstance(test_tunnel, dict):
            raise InstallError("Test tunnel settings must be a mapping")
        test_name = test_tunnel.get("sni")
        test_port = test_tunnel.get("port")
        if not isinstance(test_name, str) or not test_name:
            raise InstallError("Test tunnel requires a non-empty SNI")
        if type(test_port) is not int or not 1024 <= test_port <= 65535:
            raise InstallError("Test tunnel port must be 1024..65535")
        if test_name in [domain, www, *names, *([camouflage] if camouflage else []),
                *([chain_sni] if chain_sni else [])]:
            raise InstallError("Test tunnel SNI must be separate from every production SNI")
        test_route = f"    use_backend test_tunnel if {{ req.ssl_sni -i {test_name} }}\n"
        test_backend = f"backend test_tunnel\n    server tunnel 127.0.0.1:{test_port} check\n"
    allowed = " ".join(dict.fromkeys([domain, www, *names] + ([camouflage] if camouflage else [])
        + ([chain_sni] if chain_sni else []) + ([test_name] if test_name else [])))
    selfsteal_route = (f"    use_backend selfsteal if {{ req.ssl_sni -i {chain_sni} }}\n"
        if selfsteal and chain_sni else "")
    selfsteal_backend = (f"backend selfsteal\n"
        f"    server reality 127.0.0.1:{selfsteal['port']} check\n"
        f"    server site 127.0.0.1:{site_port} check backup\n"
        if selfsteal and chain_sni else "")
    native_backup = (f"    server site 127.0.0.1:{site_port} check backup\n"
        if native_selfsteal else "")
    mtproto_route = (f"    use_backend mtproto if {{ req.ssl_sni -i {camouflage} }}\n" if camouflage else "")
    mtproto_backend = (f"backend mtproto\n    server proxy 127.0.0.1:{mtproto['port']} check\n" if camouflage else "")
    haproxy = f"""global
    maxconn 512
    nbthread 1
defaults
    mode tcp
    timeout connect 5s
    timeout client 5m
    timeout server 5m
    timeout tunnel 1h
frontend public_tls
    bind 0.0.0.0:443
    tcp-request inspect-delay 5s
    acl hello req.ssl_hello_type 1
    acl allowed req.ssl_sni -i {allowed}
    tcp-request content reject if hello !allowed
    tcp-request content accept if hello allowed
    tcp-request content reject if WAIT_END
{selfsteal_route}{mtproto_route}{test_route}    use_backend amnezia if {{ req.ssl_sni -i {' '.join(names)} }}
    default_backend website
backend website
    server site 127.0.0.1:{site_port} check
backend amnezia
    server native 127.0.0.1:{vpn_port} check
{native_backup}{selfsteal_backend}{mtproto_backend}{test_backend}""".encode()
    if root is None:
        handle = [{"handler": "static_response", "headers": {"Content-Type": ["text/html; charset=utf-8"]}, "body": body.decode("utf-8"), "status_code": 200}]
    else:
        handle = [{"handler": "file_server", "root": "/site", "index_names": list(INDEX_NAMES), "hide": [".git", ".git/*"]}]
    hosts = list(dict.fromkeys([domain] + ([chain_sni] if chain_sni else [])))
    outline_host = chain_sni or domain
    routes = []
    if outline:
        upstream = {"handler": "reverse_proxy",
            "upstreams": [{"dial": "127.0.0.1:" + str(outline["webPort"])}]}
        for path in (outline["tcpPath"], outline["udpPath"]):
            routes.append({"match": [{"host": [outline_host], "path": [path]}],
                "handle": [upstream]})
    if client_page:
        page_path = "/" + str(client_page["path"])
        # Caddy's path matcher normalizes escapes and ignores case. Compare
        # the original request target so only the bearer path can match.
        uri = "string({http.request.orig_uri})"
        match_page = {"host": [domain], "expression": (
            f"{uri} == {json.dumps(page_path)} || "
            f"{uri}.startsWith({json.dumps(page_path + '?')})")}
        page_headers = client_page.get("headers") or {}
        page_handle = []
        if page_headers:
            page_handle.append({
                "handler": "headers",
                "response": {"set": {name: [value] for name, value in page_headers.items()}},
            })
        page_handle += [
            {"handler": "rewrite", "uri": "/index.html"},
            {"handler": "file_server", "root": "/client-page", "index_names": ["index.html"]},
        ]
        routes.append({
            "match": [dict(match_page, method=["GET", "HEAD"])],
            "handle": page_handle,
        })
        routes.append({
            "match": [match_page],
            "handle": [{"handler": "static_response", "status_code": 405}],
        })
    for reality_name in reality_names:
        routes.append({"match": [{"host": [reality_name]}],
            "handle": [{"handler": "static_response", "status_code": 403,
                "headers": {"Content-Type": ["text/html; charset=utf-8"]},
                "body": DENIAL_BODY.decode("utf-8")}]})
    routes.append({"match": [{"host": hosts}], "handle": handle})
    # www is a stable alias for the apex; the certificate is its only served
    # purpose besides verifying wildcard issuance.
    routes.append({"match": [{"host": [www]}],
        "handle": [{"handler": "static_response", "status_code": 308,
            "headers": {"Location": ["https://" + domain + "{http.request.uri}"]}}]})
    listen = [f"127.0.0.1:{site_port}"]
    if site_bind:
        listen.append(f"{site_bind}:{site_port}")
    loaded = [{"certificate": "/certs/fullchain.pem", "key": "/certs/key.pem"}]
    tls_names = list(dict.fromkeys([*hosts, www]))
    if reality_names:
        tls_names += [name for name in reality_names if name not in tls_names]
    caddy = {"admin": {"disabled": True}, "apps": {"tls": {"certificates": {"load_files": loaded}},
        "http": {"servers": {"site": {"listen": listen, "automatic_https": {"disable": True},
            "strict_sni_host": True,
            "tls_connection_policies": [{"match": {"sni": tls_names}}],
            "routes": routes}}}}}
    if root is not None:
        # Plain HTTP serves only the ACME webroot challenge and otherwise
        # redirects to HTTPS. It exists only with a real site root, so a
        # placeholder install keeps TCP/80 free for standalone issuance.
        caddy["apps"]["http"]["servers"]["http_redirect"] = {
            "listen": ["0.0.0.0:80"],
            "automatic_https": {"disable": True},
            "routes": [
                {"match": [{"path": ["/.well-known/acme-challenge/*"]}],
                 "handle": [{"handler": "file_server", "root": "/site",
                             "hide": [".git", ".git/*"]}]},
                {"handle": [{"handler": "static_response", "status_code": 308,
                             "headers": {"Location": [
                                 "https://{http.request.host}{http.request.uri}"]}}]},
            ],
        }
    return haproxy, json.dumps(caddy).encode()


def launch_digest(value, *, legacy_nullable=False, normalize_restored_lists=True):
    normalized = copy.deepcopy(value)
    from docker import canonical_host_config
    normalized["HostConfig"] = canonical_host_config(normalized["HostConfig"],
        normalize_restored_lists=normalize_restored_lists)
    if legacy_nullable and "OomKillDisable" in normalized["HostConfig"] and normalized["HostConfig"]["OomKillDisable"] is False:
        normalized["HostConfig"]["OomKillDisable"] = None
    policy = normalized["HostConfig"]["RestartPolicy"]
    if policy not in ({"Name": "no", "MaximumRetryCount": 0}, {"Name": "always", "MaximumRetryCount": 0}):
        raise InstallError("Owned restart policy is outside its admitted scope")
    normalized["HostConfig"]["RestartPolicy"] = {"Name": "no", "MaximumRetryCount": 0}
    from docker import fingerprint
    return fingerprint(normalized)


def record_owned(work, state, container, role, files=()):
    value = inspect(exact_id(container))
    labels = value["Config"].get("Labels") or {}
    instance = state.get("instance") if isinstance(state, dict) else None
    if not instance:
        raise InstallError("Owned runtime state lacks instance; refusing to inspect it")
    if labels.get("io.hton.minimum") != instance or labels.get("io.hton.role") != role:
        raise InstallError("Created resource identity differs from its intended ownership")
    claims = []
    for relative in files:
        data = work.read(relative)
        if data is None:
            raise InstallError("Admitted native artifact disappeared")
        claims.append({"path": relative, "digest": digest(data)})
    state.setdefault("ownedReceipts", {})[container] = {"role": role, "launchVersion": 2, "launch": launch_digest(value), "files": claims}


def owned(state, container, role, work=None):
    value = inspect(exact_id(container))
    labels = value["Config"].get("Labels") or {}
    instance = state.get("instance") if isinstance(state, dict) else None
    if not instance:
        raise InstallError("Owned runtime state lacks instance; refusing to inspect it")
    if labels.get("io.hton.minimum") != instance or labels.get("io.hton.role") != role:
        raise InstallError("Owned runtime identity differs; refusing to stop or replace it")
    receipt = state.get("ownedReceipts", {}).get(container)
    if not receipt or receipt["role"] != role:
        raise InstallError("Owned native launch has no matching receipt; refusing mutation")
    version = receipt.get("launchVersion", 1)
    if type(version) is not int or version not in (1, 2):
        raise InstallError("Owned native launch receipt version is unsupported")
    matches = launch_digest(value) == receipt["launch"]
    if not matches and version == 1:
        matches = launch_digest(value, legacy_nullable=True) == receipt["launch"]
    if not matches:
        raise InstallError("Owned native launch drifted; refusing mutation")
    if receipt["files"]:
        store = work or Store(ROOT / "work/minimum")
        try:
            for claim in receipt["files"]:
                data = store.read(claim["path"])
                if data is None or digest(data) != claim["digest"]:
                    raise InstallError("Admitted native artifact drifted; original retained")
        finally:
            if work is None:
                store.close()
    return value


def cleanup_temporary_jobs(work, state, save):
    from docker import inventory
    for field, role in (("clientJob", "client"), ("validatorJob", "validator")):
        identifier = state.get(field)
        if not identifier:
            continue
        receipt = state.get("ownedReceipts", {}).get(identifier)
        if not receipt or receipt["role"] != role:
            raise InstallError("Interrupted job has no exact owned creation receipt")
        if identifier in inventory():
            owned(state, identifier, role, work)
            command("container", "rm", "--force", identifier)
        state.pop(field, None)
        state["ownedReceipts"].pop(identifier, None)
        save()


def prepare(work, state, pins, certificate, domain, names, port, site_port, body, save, mtproto=None, outline=None, selfsteal=None, chain_sni=None, native_selfsteal=False, site_bind=None, client_page=None, reality_names=(), site_document_root=None, test_tunnel=None):
    root = site_root(site_document_root) if site_document_root is not None else None
    ha, site = configs(domain, names, port, site_port, body, root, mtproto=mtproto, outline=outline,
        selfsteal=selfsteal, chain_sni=chain_sni, native_selfsteal=native_selfsteal, site_bind=site_bind,
        client_page=client_page, reality_names=reality_names, test_tunnel=test_tunnel)
    token = digest(ha + site + (certificate / "fullchain.pem").read_bytes())[:24]
    directory = "artifacts/" + token
    for filename, content in (("haproxy.cfg", ha), ("caddy.json", site)):
        previous = work.read(directory + "/" + filename)
        if previous is not None and previous != content:
            raise InstallError("Immutable ingress artifact changed; original retained")
        if previous is None:
            work.write(directory + "/" + filename, content)
    artifact = work.root / directory
    from docker import inventory
    present = inventory() if state.get("candidateIngress") else {}
    for role, identifier in list(state.get("candidateIngress", {}).items()):
        if identifier not in present:
            if identifier not in state.get("ownedReceipts", {}):
                raise InstallError("Abandoned ingress lacks its exact creation receipt")
            state["candidateIngress"].pop(role)
            state["ownedReceipts"].pop(identifier)
            save()
            continue
        value = owned(state, identifier, role, work)
        if value["State"]["Running"]:
            raise InstallError("Uncommitted ingress is running; exact recovery is required")
        command("container", "rm", identifier)
        state["ownedReceipts"].pop(identifier, None)
        state["candidateIngress"].pop(role)
        save()
    state["candidateIngress"] = {}
    save()
    containers = {}
    for role, image, filename, entrypoint, args in (
        ("site", pins["caddy"], "caddy.json", "caddy", ["run", "--config", "/etc/hton/caddy.json"]),
        ("edge", pins["haproxy"], "haproxy.cfg", "haproxy", ["-W", "-db", "-f", "/etc/hton/haproxy.cfg"]),
    ):
        native = ["validate", "--config", "/etc/hton/caddy.json"] if role == "site" else ["-c", "-f", "/etc/hton/haproxy.cfg"]
        mounts = ["--mount", f"type=bind,src={artifact},dst=/etc/hton,readonly"]
        if role == "site":
            mounts += ["--mount", f"type=bind,src={certificate},dst=/certs,readonly"]
            if root is not None:
                mounts += ["--mount", f"type=bind,src={root},dst=/site,readonly"]
            if client_page:
                mounts += ["--mount", f"type=bind,src={work.root / 'client-page'},dst=/client-page,readonly"]
        validation = exact_id(command("container", "create", "--pull", "never", "--network", "none", "--read-only", "--log-driver", "none",
            "--user", "0:0", "--tmpfs", "/data:rw,nosuid,noexec,size=16777216", "--tmpfs", "/config:rw,nosuid,noexec,size=16777216",
            "--label", "io.hton.minimum=" + state["instance"], "--label", "io.hton.role=validator", *mounts,
            "--entrypoint", entrypoint, image, *native).decode().strip())
        state["validatorJob"] = validation
        record_owned(work, state, validation, "validator")
        save()
        try:
            command("container", "start", "--attach", validation)
            if inspect(validation)["State"]["ExitCode"] != 0:
                raise InstallError("Native ingress validation failed")
        finally:
            owned(state, validation, "validator", work)
            command("container", "rm", "--force", validation)
            state.pop("validatorJob", None)
            state["ownedReceipts"].pop(validation, None)
            save()
        identifier = exact_id(command("container", "create", "--pull", "never", "--network", "host", "--read-only",
            "--user", "0:0", "--tmpfs", "/data:rw,nosuid,noexec,size=16777216", "--tmpfs", "/config:rw,nosuid,noexec,size=16777216",
            "--restart", "no", "--cap-drop", "ALL", "--cap-add", "NET_BIND_SERVICE", "--security-opt", "no-new-privileges:true",
            "--tmpfs", "/tmp:rw,nosuid,noexec,size=16777216", "--log-driver", "none",
            "--label", "io.hton.minimum=" + state["instance"], "--label", "io.hton.role=" + role, *mounts,
            "--entrypoint", entrypoint, image, *args).decode().strip())
        containers[role] = identifier
        state.setdefault("candidateIngress", {})[role] = identifier
        files = [directory + "/" + filename]
        if role == "site":
            certificate_relative = certificate.relative_to(work.root).as_posix()
            files += [certificate_relative + "/fullchain.pem", certificate_relative + "/key.pem"]
        record_owned(work, state, identifier, role, files)
        save()
    return containers


def website(domain, port, expected):
    context = ssl.create_default_context()
    with socket.create_connection(("127.0.0.1", port), timeout=5) as stream:
        with context.wrap_socket(stream, server_hostname=domain) as secure:
            secure.settimeout(5)
            secure.sendall((f"GET / HTTP/1.1\r\nHost: {domain}\r\nConnection: close\r\n\r\n").encode())
            chunks, size = [], 0
            while block := secure.recv(65536):
                size += len(block)
                if size > 2 * 1024 * 1024:
                    raise InstallError("Website verification response exceeds bounds")
                chunks.append(block)
    response = b"".join(chunks)
    headers, _, body = response.partition(b"\r\n\r\n")
    if headers.split(b"\r\n")[0].split()[1:2] != [b"200"]:
        raise InstallError("Website did not return HTTP 200")
    if b"transfer-encoding: chunked" in headers.lower():
        decoded = bytearray()
        while body:
            line, _, body = body.partition(b"\r\n")
            count = int(line.split(b";")[0], 16)
            if not count:
                break
            decoded.extend(body[:count])
            body = body[count + 2:]
        body = bytes(decoded)
    if body != expected:
        raise InstallError("HTTPS body differs from the immutable website artifact (served "
            + str(len(body)) + " bytes, expected " + str(len(expected)) + ")")


def authentication(work, state, pins, original, names, save, server_name=None):
    """Real pinned Xray client; private config/UUID never enter argv or logs."""
    native = json.loads(native_config(original))
    probe_name = server_name or names[0]
    inbound = native["inbounds"][0]
    reality = inbound["streamSettings"]["realitySettings"]
    client = inbound["settings"]["clients"][0]
    private = base64.urlsafe_b64decode(reality["privateKey"] + "=" * (-len(reality["privateKey"]) % 4))
    if len(private) != 32:
        raise InstallError("Unsupported native REALITY key encoding")
    public = run(["/usr/bin/openssl", "pkey", "-inform", "DER", "-pubout", "-outform", "DER"], data=bytes.fromhex("302e020100300506032b656e04220420") + private)
    public = base64.urlsafe_b64encode(public[-32:]).decode().rstrip("=")
    with socket.socket() as available:
        available.bind(("127.0.0.1", 0))
        port = available.getsockname()[1]
    token = secrets.token_hex(12)
    user = {"id": client["id"], "encryption": "none"}
    if client.get("flow"):
        user["flow"] = client["flow"]
    config = {"log": {"loglevel": "none"}, "inbounds": [{"listen": "127.0.0.1", "port": port, "protocol": "socks", "settings": {"auth": "noauth", "udp": False}}],
        "outbounds": [{"protocol": "vless", "settings": {"vnext": [{"address": "127.0.0.1", "port": 443, "users": [user]}]},
            "streamSettings": {"network": "tcp", "security": "reality", "realitySettings": {"fingerprint": "chrome", "serverName": probe_name, "publicKey": public, "shortId": reality["shortIds"][0], "spiderX": "/"}}}]}
    relative = "clients/" + token + "/client.json"
    work.save(relative, config)
    identifier = exact_id(command("container", "create", "--pull", "never", "--network", "host", "--read-only", "--log-driver", "none", "--cap-drop", "ALL",
        "--user", "0:0", "--tmpfs", "/etc/xray:rw,nosuid,noexec,size=1048576", "--tmpfs", "/var/log/xray:rw,nosuid,noexec,size=1048576",
        "--label", "io.hton.minimum=" + state["instance"], "--label", "io.hton.role=client",
        "--mount", f"type=bind,src={work.root / ('clients/' + token)},dst=/client,readonly",
        "--entrypoint", "xray", pins["xray"], "run", "-config", "/client/client.json").decode().strip())
    state["clientJob"] = identifier
    record_owned(work, state, identifier, "client", [relative])
    save()
    try:
        command("container", "start", identifier)
        for _ in range(30):
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=0.2):
                    break
            except OSError:
                time.sleep(0.1)
        # A freshly started REALITY server needs several seconds before it can
        # complete the authenticated tunnel with the camouflage target. Probe
        # with a bounded readiness budget instead of failing on the first race.
        deadline = time.monotonic() + 30
        last_reason = ""
        while True:
            try:
                status = run(["/usr/bin/curl", "--disable", "--silent", "--show-error", "--max-time", "15", "--connect-timeout", "5", "--noproxy", "", "--proxy", f"socks5h://127.0.0.1:{port}",
                    "--max-filesize", "1048576",
                    "--proto", "=http", "--output", "/dev/null", "--write-out", "%{http_code}", "http://example.com/"], timeout=20).decode()
            except NativeError as error:
                status = ""
                last_reason = error.reason
            if status.isdigit() and 100 <= int(status) < 500:
                break
            client_state = inspect(identifier)["State"]
            if not client_state.get("Running") and client_state.get("ExitCode") not in (0, None):
                raise InstallError("Xray client exited during authentication (exit " + str(client_state.get("ExitCode")) + ")")
            if time.monotonic() >= deadline:
                detail = (" (" + last_reason + ")") if last_reason else ""
                raise InstallError("Authenticated native VPN request failed" + detail)
            time.sleep(1)
    finally:
        owned(state, identifier, "client", work)
        command("container", "rm", "--force", identifier)
        state.pop("clientJob", None)
        state["ownedReceipts"].pop(identifier, None)
        save()
