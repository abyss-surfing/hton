"""Fixed local Docker transport and exact native launch observations."""
from __future__ import annotations

import copy
import hashlib
import http.client
import io
import ipaddress
import json
import re
import os
from pathlib import Path, PurePosixPath
import selectors
import signal
import socket
import stat
import subprocess
import sys
import tarfile
import time
from urllib.parse import urlencode

import storage
from storage import InstallError, Store, digest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "lib"))
from install_config import deployment_root
ROOT = deployment_root(Path(__file__).resolve().parents[2])

DOCKER = ["/usr/bin/docker", "--host", "unix:///var/run/docker.sock", "--config", str(ROOT / "work/docker-client")]


class NativeError(InstallError):
    """Only fixed labels/reasons and numeric status; no process secrets."""
    def __init__(self, operation, code, reason):
        self.operation, self.code, self.reason = operation, code, reason
        status = " (exit " + str(code) + ")" if code is not None else ""
        super().__init__(operation + " failed" + status + ". " + reason)


def operation_name(argv):
    if argv[:len(DOCKER)] == DOCKER:
        command = tuple(argv[len(DOCKER):len(DOCKER)+2])
        labels = {("container", "create"): "Create container", ("container", "start"): "Run container task",
            ("container", "inspect"): "Read container settings", ("container", "cp"): "Read container file",
            ("container", "commit"): "Save VPN recovery image", ("container", "wait"): "Wait for container task",
            ("container", "rm"): "Remove completed task", ("container", "top"): "Read VPN process list",
            ("container", "exec"): "Check VPN program", ("image", "pull"): "Download container image",
            ("image", "inspect"): "Read image details", ("network", "inspect"): "Read network settings"}
        return labels.get(command, "Docker command")
    tool = argv[0]
    if tool == "/usr/bin/openssl":
        if len(argv)>1 and argv[1]=="verify": return "Check HTTPS certificate trust and domain"
        if "-checkend" in argv: return "Check certificate expiry"
        if "-dates" in argv: return "Read certificate dates"
        return "Check certificate or key"
    return {"/usr/bin/curl": "Network request", "/usr/bin/systemctl": "Update service settings",
        "/usr/bin/ss": "Check network ports"}.get(tool, "Program")


def command_reason(argv, code, data):
    text = data.decode("utf-8", errors="replace").lower()
    known = (("read-only file system", "The program tried to write to a read-only folder."),
        ("permission denied", "Access denied. The program could not read or use a required file."),
        ("operation not permitted", "The container does not have a required permission."),
        ("no space left on device", "The disk is full."),
        ("address already in use", "Another service is using the required port."),
        ("port is already allocated", "Another service is using the required port."),
        ("no such object", "The container no longer exists."),
        ("no such container", "The container no longer exists."),
        ("no such file or directory", "A required file or folder is missing."),
        ("out of memory", "There is not enough memory."))
    for pattern, message in known:
        if pattern in text: return message
    if argv[0]=="/usr/bin/curl":
        return {6:"The network name could not be resolved.",7:"The server could not be reached.",
            22:"The server returned an HTTP error.",28:"The network request timed out.",
            35:"The secure connection failed.",60:"The remote certificate could not be verified."}.get(code,"The network request did not complete.")
    if argv[0]=="/usr/bin/openssl" and len(argv)>1 and argv[1]=="verify":
        return "The certificate chain, domain name or validity check failed."
    return "The command did not finish successfully."


def run(argv, *, timeout=30, data=None, allowed=(0,), limit=4 * 1024 * 1024):
    """Drain finite output; kill the complete process group on timeout/overflow."""
    child = subprocess.Popen(argv, stdin=subprocess.PIPE if data is not None else subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
        env={"PATH": "/usr/bin:/bin", "LANG": "C", "HOME": "/nonexistent"})
    chunks, count, offset = [], 0, 0
    errors, error_count = [], 0
    operation = operation_name(argv)
    selector = selectors.DefaultSelector()
    os.set_blocking(child.stdout.fileno(), False)
    selector.register(child.stdout, selectors.EVENT_READ)
    os.set_blocking(child.stderr.fileno(), False)
    selector.register(child.stderr, selectors.EVENT_READ)
    if data is not None:
        os.set_blocking(child.stdin.fileno(), False)
        selector.register(child.stdin, selectors.EVENT_WRITE)
    deadline = time.monotonic() + timeout
    try:
        while selector.get_map():
            if time.monotonic() >= deadline:
                raise NativeError(operation, None, "The command took too long.")
            for key, _ in selector.select(0.1):
                if key.fileobj is child.stdin:
                    if offset == len(data):
                        selector.unregister(child.stdin)
                        child.stdin.close()
                    else:
                        offset += os.write(child.stdin.fileno(), data[offset:offset + 8192])
                elif key.fileobj is child.stderr:
                    block = os.read(child.stderr.fileno(), 65536)
                    if not block:
                        selector.unregister(child.stderr)
                    else:
                        error_count += len(block)
                        if error_count > 65536:
                            raise NativeError(operation, None, "The command returned too much error data.")
                        errors.append(block)
                else:
                    block = os.read(child.stdout.fileno(), 65536)
                    if not block:
                        selector.unregister(child.stdout)
                    else:
                        count += len(block)
                        if count > limit:
                            raise NativeError(operation, None, "The command returned too much data.")
                        chunks.append(block)
        try:
            code = child.wait(timeout=max(0.01, deadline - time.monotonic()))
        except subprocess.TimeoutExpired:
            raise NativeError(operation, None, "The command took too long.") from None
        if code not in allowed:
            raise NativeError(operation, code, command_reason(argv, code, b"".join(errors + chunks)))
        return b"".join(chunks)
    finally:
        selector.close()
        if child.poll() is None:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()
        if child.stdin is not None:
            child.stdin.close()
        child.stdout.close()
        child.stderr.close()


def command(*args, **kwargs):
    return run(DOCKER + list(args), **kwargs)


def exact_id(value):
    if not isinstance(value, str) or len(value) != 64 or any(c not in "0123456789abcdef" for c in value):
        raise InstallError("Expected an immutable full Docker container ID")
    return value


def inspect(container):
    value = json.loads(command("container", "inspect", "--format", "{{json .}}", exact_id(container)))
    if value.get("Id") != container:
        raise InstallError("Docker identity changed")
    return value


def inventory():
    ids = command("container", "ls", "--all", "--quiet", "--no-trunc").decode().split()
    if len(ids) > 128:
        raise InstallError("Docker inventory exceeds the supported bound")
    return {i: inspect(i) for i in ids}


FINGERPRINT_VERSION = 2


def stable_network_attachments(value):
    """Declared endpoints without the daemon-issued default-bridge identity.

    Docker assigns a new ID to the default ``bridge`` network when it recreates
    it (observed after an unclean shutdown and daemon restart). The configured
    identity is the network name plus its aliases and gateway priority; keeping
    that volatile ID in a launch fingerprint reports false drift and permanently
    blocks automatic renewal. Custom networks keep their IDs so a recreated
    dependency is still reported.
    """
    result = network_attachments(value)
    if "bridge" in result:
        bridge = dict(result["bridge"])
        bridge["NetworkID"] = ""
        result["bridge"] = bridge
    return result


def fingerprint(value, version=FINGERPRINT_VERSION):
    fields = {k: value[k] for k in ("Id", "Name", "Image", "Config", "HostConfig", "Mounts")}
    fields["Mounts"] = sorted(fields["Mounts"], key=lambda mount: (mount.get("Destination", ""), mount.get("Type", ""), mount.get("Source", "")))
    if value["HostConfig"].get("NetworkMode") == "bridge" and not (value["Config"].get("Labels") or {}).get("io.hton.minimum"):
        fields["Networks"] = stable_network_attachments(value) if version >= 2 else network_attachments(value)
    return digest(json.dumps(fields, sort_keys=True).encode())


def native_config(container, *, timeout=30):
    archive = command("container", "cp", exact_id(container) + ":/opt/amnezia/xray/server.json", "-", limit=2 * 1024 * 1024, timeout=timeout)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        entries = tar.getmembers()
        if len(entries) != 1 or not entries[0].isfile() or entries[0].size > 1024 * 1024:
            raise InstallError("Native config must be one regular bounded file")
        data = tar.extractfile(entries[0]).read()
    return data


def bridge_gateway():
    gateway = command("network", "inspect", "bridge", "--format",
        "{{(index .IPAM.Config 0).Gateway}}", limit=4096).decode().strip()
    try:
        address = ipaddress.ip_address(gateway)
    except (TypeError, ValueError):
        raise InstallError("Docker bridge gateway is not a valid address")
    if address.version != 4 or address.is_loopback or address.is_unspecified:
        raise InstallError("Docker bridge gateway must be a non-loopback IPv4 address")
    return str(address)


def native_bridge_gateway(value):
    """Return the native default-bridge IPv4 gateway used for internal fallback."""
    networks = value.get("NetworkSettings", {}).get("Networks") or {}
    endpoint = networks.get("bridge")
    if not isinstance(endpoint, dict):
        raise InstallError("Native self-steal requires the default bridge network")
    gateway = endpoint.get("Gateway")
    try:
        address = ipaddress.ip_address(gateway)
    except (TypeError, ValueError):
        raise InstallError("Native bridge gateway is not a valid address")
    if address.version != 4 or address.is_loopback or address.is_unspecified:
        raise InstallError("Native bridge gateway must be a non-loopback IPv4 address")
    return str(address)


def native_structure_digest(data):
    """Digest the native REALITY structure while ignoring the client list.

    Amnezia appends/removes clients in the native JSON; keys, names, dest,
    listener and stream settings must remain unchanged for requalification.
    """
    try:
        config = json.loads(data)
    except (TypeError, ValueError):
        raise InstallError("Native configuration is not valid JSON")
    if not isinstance(config, dict):
        raise InstallError("Native configuration root is invalid")
    config.pop("log", None)
    for inbound in config.get("inbounds") or []:
        if isinstance(inbound, dict) and isinstance(inbound.get("settings"), dict):
            inbound["settings"].pop("clients", None)
    return digest(json.dumps(config, sort_keys=True).encode())


def replace_native_config(container, source):
    """Copy an exact protected config file into the admitted native container."""
    path = Path(source)
    info = path.lstat()
    if not storage.safe_file(info, os.geteuid()):
        raise InstallError("Native config replacement source ownership or mode differs")
    if info.st_size > 1024 * 1024:
        raise InstallError("Native config replacement exceeds its bound")
    command("container", "cp", str(path), exact_id(container) + ":/opt/amnezia/xray/server.json")


def network_attachments(value, *, required=False):
    """Declared bridge endpoints; exclude dynamically assigned IP/MAC/DNS names."""
    networks = value.get("NetworkSettings", {}).get("Networks") or {}
    if required and ("bridge" not in networks or not 1 <= len(networks) <= 8):
        raise InstallError("Native networking requires default bridge and at most seven local bridge dependencies")
    result = {}
    for name, endpoint in networks.items():
        if not isinstance(name, str) or not name or len(name) > 128 or any(c not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_." for c in name):
            raise InstallError("Native network name is unsupported")
        ipam = endpoint.get("IPAMConfig")
        if ipam is None:
            ipam = {}
        if (not isinstance(ipam, dict)
                or set(ipam) - {"IPv4Address", "IPv6Address", "LinkLocalIPs"}
                or any(ipam.values()) or endpoint.get("Links") or endpoint.get("DriverOpts")):
            raise InstallError("Native network has fixed IPAM, endpoint links or driver options")
        aliases = endpoint.get("Aliases") or []
        if not isinstance(aliases, list) or len(aliases) > 32 or any(not isinstance(a, str) or not a or len(a) > 253 or any(c not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_." for c in a) for a in aliases):
            raise InstallError("Native network aliases are unsupported")
        priority = endpoint.get("GwPriority", 0)
        if type(priority) is not int or abs(priority) > 1000000:
            raise InstallError("Native network gateway priority is unsupported")
        identifier = endpoint.get("NetworkID") or ""
        if identifier:
            exact_id(identifier)
        result[name] = {"NetworkID": identifier, "Aliases": sorted(aliases), "GwPriority": priority}
    return result


def verify_networks(value):
    """Read dependencies only; never change a native network or peer endpoint."""
    attachments = network_attachments(value, required=True)
    for name, endpoint in attachments.items():
        if name == "bridge":
            # Docker recreates its default bridge with a new ID after an unclean
            # shutdown. The name and driver are fixed; re-reading a stale recorded
            # ID would reject a healthy installation. Custom dependencies stay strict.
            continue
        identifier = exact_id(endpoint["NetworkID"])
        observed = json.loads(command("network", "inspect", "--format", "{{json .}}", identifier))
        if observed.get("Id") != identifier or observed.get("Name") != name:
            raise InstallError("Native network identity changed; original retained")
        if observed.get("Driver") != "bridge" or observed.get("Scope") != "local":
            raise InstallError("Native network dependency must be a local bridge")
        if any(k.startswith(("com.docker.compose.", "com.docker.swarm.")) for k in (observed.get("Labels") or {})):
            raise InstallError("An external controller owns a native network dependency")
    return attachments


def same_networks(original, current):
    expected = stable_network_attachments(original)
    actual = stable_network_attachments(current)
    if set(expected) != set(actual):
        return False
    for name in expected:
        observed = dict(actual[name])
        if not observed["NetworkID"] and not current["State"]["Running"]:
            observed["NetworkID"] = expected[name]["NetworkID"]
        if observed != expected[name]:
            return False
    return True


def qualify(value, expected_port=443, *, allow_image_volumes=False, binary="/usr/bin/xray"):
    mounts = value.get("Mounts") or []
    if allow_image_volumes:
        allowed_mounts = {"/usr/local/etc/xray", "/var/log/xray"}
        if any(mount.get("Type") != "volume"
               or mount.get("Destination") not in allowed_mounts
               or not str(mount.get("Source", "")).startswith("/var/lib/docker/volumes/")
               for mount in mounts):
            raise InstallError("Managed Xray volume declaration is unsupported")
    elif mounts or value["Config"].get("Volumes"):
        raise InstallError("Minimum remap supports only no-mount bridge Amnezia")
    if value["HostConfig"].get("NetworkMode") != "bridge":
        raise InstallError("Minimum remap supports only no-mount bridge Amnezia")
    host = value["HostConfig"]
    if any(host.get(key) for key in ("Binds", "Mounts", "Tmpfs", "Links", "AutoRemove", "VolumesFrom")):
        raise InstallError("Unsupported foreign data or launch ownership")
    if host.get("ContainerIDFile"):
        raise InstallError("Native container uses an external identity file")
    network_attachments(value, required=True)
    if any(key.startswith(("com.docker.compose.", "com.docker.swarm.")) for key in (value["Config"].get("Labels") or {})):
        raise InstallError("An external controller owns this container")
    if set(host.get("PortBindings") or {}) != {"443/tcp", "443/udp"}:
        raise InstallError("Minimum remap requires exactly the native TCP/UDP 443 publications")
    for bindings in host["PortBindings"].values():
        if not isinstance(bindings, list) or len(bindings) != 1 or bindings[0].get("HostPort") != str(expected_port):
            raise InstallError("Native publication differs from its exact expected port")
        expected_addresses = ("", "0.0.0.0") if expected_port == 443 else ("127.0.0.1",)
        if bindings[0].get("HostIp") not in expected_addresses:
            raise InstallError("Native publication address differs from the supported scope")
    data = native_config(value["Id"])
    verify_networks(value)
    native = json.loads(data)
    inbounds = native.get("inbounds", [])
    if len(inbounds) != 1:
        raise InstallError("Unsupported native inbound composition")
    inbound = inbounds[0]
    if inbound.get("listen") not in (None, "0.0.0.0", "::"):
        raise InstallError("Native listener is not compatible with loopback Docker publication")
    stream = inbound.get("streamSettings", {})
    reality = stream.get("realitySettings", {})
    names = reality.get("serverNames", [])
    if inbound.get("port") != 443 or inbound.get("protocol") != "vless" or stream.get("security") != "reality" or stream.get("network", "tcp") not in ("tcp", "raw"):
        raise InstallError("The selected service is not the supported VLESS REALITY listener")
    if not names or len(names) > 8 or not reality.get("privateKey") or not inbound.get("settings", {}).get("clients"):
        raise InstallError("Native REALITY configuration is incomplete")
    short_ids = reality.get("shortIds")
    if not isinstance(short_ids, list) or not 1 <= len(short_ids) <= 16 or any(
        not isinstance(value, str) or len(value) > 16 or len(value) % 2 or any(c not in "0123456789abcdef" for c in value) for value in short_ids):
        raise InstallError("Native REALITY short IDs are unsupported")
    if any(not valid_domain(name) for name in names):
        raise InstallError("Native REALITY SNI is invalid")
    # Exact process attribution: the server socket must belong to the captured
    # native executable/config, rather than merely existing in its namespace.
    rows = command("container", "top", value["Id"], "-eo", "pid,comm").decode().splitlines()
    if len(rows) > 256:
        raise InstallError("Native process inventory exceeds bounds")
    attributed = []
    for row in rows[1:]:
        fields = row.split()
        if len(fields) != 2 or fields[1] != "xray" or not fields[0].isdigit():
            continue
        pid = fields[0]
        executable = Path("/proc") / pid / "exe"
        info = executable.stat()
        root_executable = (Path("/proc") / pid / "root") / binary.lstrip("/")
        native_info = root_executable.stat()
        if (info.st_dev, info.st_ino) != (native_info.st_dev, native_info.st_ino) or info.st_uid != 0 or info.st_mode & 0o022:
            raise InstallError("Native executable ownership or identity is unsupported")
        init = value["State"]["Pid"]
        for namespace in ("net", "mnt", "pid"):
            if (Path("/proc") / pid / "ns" / namespace).stat().st_ino != (Path("/proc") / str(init) / "ns" / namespace).stat().st_ino:
                raise InstallError("Native process does not belong to the exact container namespaces")
        raw_stat = (Path("/proc") / pid / "stat").read_text()
        start_ticks = raw_stat[raw_stat.rfind(")") + 2:].split()[19]
        argv = (Path("/proc") / pid / "cmdline").read_bytes().split(b"\x00")
        expected_argv = [binary.encode(), b"-config", b"/opt/amnezia/xray/server.json"]
        basename_argv = [PurePosixPath(binary).name.encode(), b"-config", b"/opt/amnezia/xray/server.json"]
        if argv[:-1] not in (expected_argv, basename_argv):
            raise InstallError("Unsupported native Xray launch arguments")
        inodes = set()
        for table in ("tcp", "tcp6"):
            path = Path("/proc") / pid / "net" / table
            with path.open("rb") as stream:
                table_bytes = stream.read(1024 * 1024 + 1)
            if len(table_bytes) > 1024 * 1024:
                raise InstallError("Native socket inventory exceeds bounds")
            for line in table_bytes.decode().splitlines()[1:]:
                fields = line.split()
                if len(fields) >= 10 and fields[3] == "0A" and fields[1].endswith(":01BB"):
                    inodes.add("socket:[" + fields[9] + "]")
        descriptors = list((Path("/proc") / pid / "fd").iterdir())
        if len(descriptors) > 1024:
            raise InstallError("Native descriptor inventory exceeds bounds")
        if any(os.readlink(fd) in inodes for fd in descriptors):
            attributed.append((pid, info.st_dev, info.st_ino, start_ticks))
    if len(attributed) != 1:
        raise InstallError("The native TCP/443 listener has no unique supported Xray process owner")
    pid, device, inode, ticks = attributed[0]
    def executable_digest():
        sha, count = hashlib.sha256(), 0
        with (Path("/proc") / pid / "exe").open("rb") as stream:
            while block := stream.read(1024 * 1024):
                count += len(block)
                if count > 128 * 1024 * 1024:
                    raise InstallError("Native executable exceeds its observation bound")
                sha.update(block)
        return sha.hexdigest()
    binary_digest = executable_digest()
    version = command("container", "exec", "--user", "65534:65534", "--workdir", "/", value["Id"], binary, "version", limit=65536).decode()
    if not re.match(r"^Xray \d+\.\d+\.\d+\b", version):
        raise InstallError("Native Xray version is unsupported or unverified")
    after_binary = ((Path("/proc") / pid / "root") / binary.lstrip("/")).stat()
    after_stat = (Path("/proc") / pid / "stat").read_text()
    if (after_binary.st_dev, after_binary.st_ino) != (device, inode) or after_stat[after_stat.rfind(")") + 2:].split()[19] != ticks or executable_digest() != binary_digest:
        raise InstallError("Native executable changed during qualification")
    repeated = inspect(value["Id"])
    if (repeated["State"]["Pid"], repeated["State"]["StartedAt"]) != (value["State"]["Pid"], value["State"]["StartedAt"]) or fingerprint(repeated) != fingerprint(value) or native_config(value["Id"]) != data:
        raise InstallError("Native container or config changed during preflight")
    return {"names": names, "nativeDigest": digest(data), "target": reality.get("target", reality.get("dest")), "nativeVersion": version.splitlines()[0], "binaryDigest": binary_digest}


def public_holder(original):
    """Unknown OS listeners never inherit authority from a matching port."""
    rows = run(["/usr/bin/ss", "-H", "-ltnp", "sport = :443"], limit=262144).decode()
    addresses = {network.get("IPAddress") for network in original["NetworkSettings"]["Networks"].values()}
    for row in rows.splitlines():
        pids = re.findall(r"pid=(\d+)", row)
        if not pids:
            raise InstallError("Public TCP/443 holder has no exact process evidence")
        for pid in pids:
            exe = Path("/proc") / pid / "exe"
            info = exe.stat()
            if Path(os.readlink(exe)).name != "docker-proxy" or info.st_uid != 0 or info.st_mode & 0o022:
                raise InstallError("An unrelated OS process holds TCP/443")
            data = (Path("/proc") / pid / "cmdline").read_bytes()
            if len(data) > 16384:
                raise InstallError("Public listener arguments exceed bounds")
            args = [part.decode() for part in data.split(b"\0") if part]
            def flag(name):
                return args[args.index(name) + 1] if name in args and args.index(name) + 1 < len(args) else None
            if flag("-host-port") != "443" or flag("-container-port") != "443" or flag("-container-ip") not in addresses or flag("-proto") != "tcp":
                raise InstallError("Public listener is not the exact selected Amnezia mapping")


def valid_domain(value):
    try:
        ipaddress.ip_address(value)
        return False
    except ValueError:
        pass
    return isinstance(value, str) and len(value) <= 253 and "." in value and all(
        part and len(part) <= 63 and part[0] != "-" and part[-1] != "-" and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-" for c in part)
        for part in value.split("."))


RESTORED_EMPTY_LIST_FIELDS = ("Dns",)


def canonical_host_config(host, *, normalize_restored_lists=False):
    """Docker materializes an unset OOM-disable as false at create, then reverts
    the inspect value to null once started. Treat both as the same unspecified
    default so create- and start-time observations compare equal.

    A daemon restart also rewrites a nil ``Dns`` to ``[]`` (Go re-encodes the
    same zero value). Normalize the restored empty form to null so create- and
    restore-time launch observations compare equal."""
    result = copy.deepcopy(host)
    if "OomKillDisable" in result and result["OomKillDisable"] is None:
        result["OomKillDisable"] = False
    if normalize_restored_lists:
        for name in RESTORED_EMPTY_LIST_FIELDS:
            if result.get(name) == []:
                result[name] = None
    return result


def create_payload(original, image, port):
    if type(port) is not int or not 5000 <= port <= 6000:
        raise InstallError("VPN backend port must be in 5000-6000")
    payload = copy.deepcopy(original["Config"])
    payload["Image"] = image
    host = canonical_host_config(original["HostConfig"])
    host["RestartPolicy"] = {"Name": "no", "MaximumRetryCount": 0}
    host["PortBindings"] = {key: [{"HostIp": "127.0.0.1", "HostPort": str(port)}] for key in ("443/tcp", "443/udp")}
    payload["HostConfig"] = host
    attachments = network_attachments(original)
    if attachments:
        payload["NetworkingConfig"] = {"EndpointsConfig": attachments}
    return payload


class Engine(http.client.HTTPConnection):
    def __init__(self):
        super().__init__("localhost", timeout=30)

    def connect(self):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(self.timeout)
        self.sock.connect("/var/run/docker.sock")


def create(name, payload):
    if not name or len(name) > 128 or any(c not in "abcdefghijklmnopqrstuvwxyz0123456789-_." for c in name):
        raise InstallError("Invalid replacement name")
    engine = Engine()
    try:
        engine.request("GET", "/version")
        response = engine.getresponse()
        if response.status != 200:
            raise InstallError("Docker version observation failed")
        version = json.loads(response.read(65537)).get("ApiVersion", "")
        parts = version.split(".")
        if len(parts) != 2 or parts[0] != "1" or not parts[1].isdigit():
            raise InstallError("Unsupported Docker API version")
        data = json.dumps(payload).encode()
        if len(data) > 4 * 1024 * 1024:
            raise InstallError("Replacement launch exceeds bounds")
        engine.request("POST", "/v" + version + "/containers/create?" + urlencode({"name": name}), data, {"Content-Type": "application/json"})
        response = engine.getresponse()
        if response.status != 201:
            response.read(65537)
            raise InstallError("Docker replacement creation failed; original retained")
        return exact_id(json.loads(response.read(65537))["Id"])
    finally:
        engine.close()


def save_image(image, target: Path):
    """Durable bounded archive; never capture a multi-gigabyte image in RAM."""
    fd = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, storage.LEGACY_FILE_MODE)
    gid = storage.group_id()
    if gid is not None:
        try:
            os.fchown(fd, -1, gid)
            os.fchmod(fd, storage.modes()[1])
        except OSError:
            pass
    child = subprocess.Popen(DOCKER + ["image", "save", image], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, start_new_session=True)
    selector = selectors.DefaultSelector()
    selector.register(child.stdout, selectors.EVENT_READ)
    total, deadline = 0, time.monotonic() + 180
    import hashlib
    sha = hashlib.sha256()
    try:
        while selector.get_map():
            if time.monotonic() >= deadline:
                raise InstallError("Snapshot export timed out; originals retained")
            for key, _ in selector.select(0.1):
                block = os.read(key.fd, 65536)
                if not block:
                    selector.unregister(child.stdout)
                    continue
                total += len(block)
                if total > 2 * 1024**3:
                    raise InstallError("Snapshot exceeds 2 GiB; originals retained")
                sha.update(block)
                view = memoryview(block)
                while view:
                    view = view[os.write(fd, view):]
        if child.wait(timeout=max(0.01, deadline - time.monotonic())) != 0:
            raise InstallError("Snapshot export failed; original retained")
        os.fsync(fd)
        return {"bytes": total, "digest": sha.hexdigest()}
    finally:
        if child.poll() is None:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()
        selector.close()
        child.stdout.close()
        os.close(fd)
