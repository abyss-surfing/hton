//! Bounded read-only Linux TCP/UDP reservation evidence.
use crate::{
    Budget, Cancellation, Error, Result,
    docker::{ContainerId, DockerObserver, DockerPublishedBinding, TransportProtocol},
    process::{Runner, Tool},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    os::unix::fs::MetadataExt,
    sync::Arc,
};

const MAX_TABLE_BYTES: u64 = 1_048_576;
const MAX_TABLE_ROWS: usize = 65_536;
const MAX_PROCESS_BYTES: u64 = 16_384;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HostPortReservation {
    pub protocol: TransportProtocol,
    pub port: u16,
    /// Present only for an exact Docker published binding; callers must map it
    /// through exact registry identity before treating it as managed.
    pub container_id: Option<ContainerId>,
    /// Canonical structural fingerprint from the same exact Docker inspect.
    pub container_fingerprint: Option<String>,
}
impl fmt::Debug for HostPortReservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostPortReservation")
            .field("protocol", &self.protocol)
            .field("port", &self.port)
            .field("dockerIdentity", &self.container_id.is_some())
            .field("dockerFingerprint", &self.container_fingerprint.is_some())
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ProcSocket {
    pub(crate) protocol: TransportProtocol,
    pub(crate) address: IpAddr,
    pub(crate) port: u16,
    pub(crate) inode: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SsOwner {
    name: String,
    pid: u32,
    fd: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SsSocket {
    protocol: TransportProtocol,
    address: SsAddress,
    port: u16,
    inode: u64,
    owner: Option<SsOwner>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SsAddress {
    Exact(IpAddr),
    Wildcard,
    Unsupported,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct ProxyArgs {
    protocol: TransportProtocol,
    host_ip: IpAddr,
    host_port: u16,
    container_ip: IpAddr,
    container_port: u16,
}

pub(crate) fn snapshot(
    runner: Arc<Runner>,
    docker: &DockerObserver,
    budget: Budget,
    cancellation: &dyn Cancellation,
) -> Result<Vec<HostPortReservation>> {
    let proc_sockets = linux_socket_reservations()?;
    let docker_bindings = docker.published_port_bindings(budget, cancellation)?;

    // `ss` is an optional identity refinement. Failure, missing privileges, output
    // drift or parse ambiguity leaves procfs rows unknown without losing the full
    // base TCP/UDP reservation set.
    let attributed = attribute_docker_proxy_sockets(
        &runner,
        &proc_sockets,
        &docker_bindings,
        budget,
        cancellation,
    );
    let mut reservations: BTreeSet<HostPortReservation> = docker_bindings
        .iter()
        .map(|binding| HostPortReservation {
            protocol: binding.protocol,
            port: binding.host_port,
            container_id: Some(binding.container_id.clone()),
            container_fingerprint: Some(binding.structural_fingerprint.clone()),
        })
        .collect();
    for socket in proc_sockets {
        reservations.insert(HostPortReservation {
            protocol: socket.protocol,
            port: socket.port,
            container_id: attributed.get(&socket.inode).map(|(id, _)| id.clone()),
            container_fingerprint: attributed
                .get(&socket.inode)
                .map(|(_, fingerprint)| fingerprint.clone()),
        });
    }
    budget.check(cancellation)?;
    Ok(reservations.into_iter().collect())
}

/// Reads fixed kernel socket tables. Missing, truncated or malformed data makes
/// the entire base observation unavailable, never partially complete.
fn linux_socket_reservations() -> Result<Vec<ProcSocket>> {
    read_socket_tables("/proc/net")
}

/// Read one exact process network namespace, bracketing the read with stable
/// process start-time and namespace inode facts. The caller must independently
/// bind this PID to a Docker container and re-inspect that container afterward.
pub(crate) fn container_network_sockets(pid: u32) -> Result<Vec<ProcSocket>> {
    if pid == 0 {
        return Err(Error::InvalidInput);
    }
    let root = format!("/proc/{pid}");
    let before_start = process_start_time(&root).ok_or(Error::Io)?;
    let before_namespace = network_namespace_identity(&root)?;
    let rows = read_socket_tables(&format!("{root}/net"))?;
    let after_namespace = network_namespace_identity(&root)?;
    let after_start = process_start_time(&root).ok_or(Error::Io)?;
    if before_start != after_start || before_namespace != after_namespace {
        return Err(Error::Busy);
    }
    Ok(rows)
}

/// Process attribution for the one pinned native Amnezia Xray launch shape.
/// No process is run and no config/key bytes are returned.
pub(crate) fn container_xray_listener(
    init_pid: u32,
    address: Ipv4Addr,
    port: u16,
    budget: Budget,
    cancellation: &dyn Cancellation,
) -> Result<String> {
    if init_pid == 0 || port == 0 {
        return Err(Error::InvalidInput);
    }
    let before = process_tree(init_pid, budget, cancellation)?;
    let init_root = format!("/proc/{init_pid}");
    let namespaces = process_namespaces(&init_root)?;
    let sockets = read_socket_tables(&format!("{init_root}/net"))?;
    let matching: BTreeSet<_> = sockets
        .iter()
        .filter(|socket| {
            socket.protocol == TransportProtocol::Tcp
                && socket.address == IpAddr::V4(address)
                && socket.port == port
        })
        .map(|socket| socket.inode)
        .collect();
    if matching.len() != 1 || matching.contains(&0) {
        return Err(Error::InvalidResponse);
    }
    let inode = *matching.iter().next().ok_or(Error::InvalidResponse)?;
    let expected_fd = format!("socket:[{inode}]");
    let mut owners = Vec::new();
    for (pid, start) in &before {
        budget.check(cancellation)?;
        let root = format!("/proc/{pid}");
        if process_namespaces(&root)? != namespaces {
            return Err(Error::IdentityMismatch);
        }
        let mut fds = Vec::new();
        for entry in fs::read_dir(format!("{root}/fd")).map_err(|_| Error::Io)? {
            budget.check(cancellation)?;
            let entry = entry.map_err(|_| Error::Io)?;
            if fds.len() >= 4096 {
                return Err(Error::OutputLimit);
            }
            let name = entry.file_name();
            let fd = name
                .to_str()
                .and_then(|name| name.parse::<u32>().ok())
                .ok_or(Error::InvalidResponse)?;
            fds.push(fd);
        }
        for fd in fds {
            budget.check(cancellation)?;
            let fd_path = format!("{root}/fd/{fd}");
            let link = fs::read_link(&fd_path).map_err(|_| Error::Busy)?;
            if link != std::path::Path::new(&expected_fd) {
                continue;
            }
            let argv = process_cmdline(&root).ok_or(Error::InvalidResponse)?;
            let supported = [
                b"xray\0-config\0/opt/amnezia/xray/server.json\0".as_slice(),
                b"/usr/bin/xray\0-config\0/opt/amnezia/xray/server.json\0".as_slice(),
            ];
            if !supported.contains(&argv.as_slice())
                || fs::read_link(format!("{root}/exe")).map_err(|_| Error::Io)?
                    != std::path::Path::new("/usr/bin/xray")
            {
                return Err(Error::IdentityMismatch);
            }
            let digest = executable_digest(&root, budget, cancellation)?;
            if process_cmdline(&root).as_ref() != Some(&argv)
                || fs::read_link(&fd_path).map_err(|_| Error::Busy)? != link
                || process_start_time_bounded(&root)? != *start
                || process_namespaces(&root)? != namespaces
            {
                return Err(Error::Busy);
            }
            owners.push((*pid, fd, digest, argv));
        }
    }
    // Duplicated descriptors in one process are permitted; multiple processes
    // sharing an inherited listener are outside this pinned single-Xray shape.
    let distinct: BTreeSet<_> = owners.iter().map(|(pid, _, _, _)| *pid).collect();
    if distinct.len() != 1 {
        return Err(Error::IdentityMismatch);
    }
    let (pid, fd, digest, argv) = owners.first().ok_or(Error::IdentityMismatch)?;
    let root = format!("/proc/{pid}");
    let after = process_tree(init_pid, budget, cancellation)?;
    let final_sockets = read_socket_tables(&format!("{init_root}/net"))?;
    if before != after
        || process_namespaces(&init_root)? != namespaces
        || !final_sockets.iter().any(|socket| {
            socket.protocol == TransportProtocol::Tcp
                && socket.address == IpAddr::V4(address)
                && socket.port == port
                && socket.inode == inode
        })
        || fs::read_link(format!("{root}/fd/{fd}")).map_err(|_| Error::Busy)?
            != std::path::Path::new(&expected_fd)
        || executable_digest(&root, budget, cancellation)? != *digest
        || process_cmdline(&root).as_ref() != Some(argv)
        || process_namespaces(&root)? != namespaces
        || fs::read_link(format!("{root}/exe")).map_err(|_| Error::Io)?
            != std::path::Path::new("/usr/bin/xray")
        || process_tree(init_pid, budget, cancellation)? != before
    {
        return Err(Error::Busy);
    }
    budget.check(cancellation)?;
    Ok(digest.clone())
}

fn process_namespaces(root: &str) -> Result<Vec<(u64, u64)>> {
    ["net", "pid", "mnt"]
        .into_iter()
        .map(|name| {
            let metadata = fs::metadata(format!("{root}/ns/{name}")).map_err(|_| Error::Io)?;
            Ok((metadata.dev(), metadata.ino()))
        })
        .collect()
}

fn read_proc_bounded(path: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| Error::Io)?
        .take(MAX_PROCESS_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Io)?;
    if bytes.len() as u64 > MAX_PROCESS_BYTES {
        return Err(Error::OutputLimit);
    }
    Ok(bytes)
}

fn process_start_time_bounded(root: &str) -> Result<u64> {
    let bytes = read_proc_bounded(&format!("{root}/stat"))?;
    let stat = std::str::from_utf8(&bytes).map_err(|_| Error::InvalidResponse)?;
    stat.rsplit_once(')')
        .and_then(|(_, tail)| tail.split_ascii_whitespace().nth(19))
        .and_then(|value| value.parse().ok())
        .ok_or(Error::InvalidResponse)
}

fn process_tree(
    init_pid: u32,
    budget: Budget,
    cancellation: &dyn Cancellation,
) -> Result<BTreeMap<u32, u64>> {
    let mut tree = BTreeMap::new();
    let mut pending = vec![init_pid];
    let mut task_count = 0usize;
    while let Some(pid) = pending.pop() {
        budget.check(cancellation)?;
        if tree.contains_key(&pid) {
            continue;
        }
        if tree.len() >= 256 {
            return Err(Error::OutputLimit);
        }
        let root = format!("/proc/{pid}");
        tree.insert(pid, process_start_time_bounded(&root)?);
        for task in fs::read_dir(format!("{root}/task")).map_err(|_| Error::Io)? {
            budget.check(cancellation)?;
            task_count += 1;
            if task_count > 1024 {
                return Err(Error::OutputLimit);
            }
            let task = task.map_err(|_| Error::Io)?;
            let tid = task
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u32>().ok())
                .ok_or(Error::InvalidResponse)?;
            let children = read_proc_bounded(&format!("{root}/task/{tid}/children"))?;
            for child in std::str::from_utf8(&children)
                .map_err(|_| Error::InvalidResponse)?
                .split_ascii_whitespace()
            {
                let child = child.parse::<u32>().map_err(|_| Error::InvalidResponse)?;
                if child == 0 || pending.len() >= 256 {
                    return Err(Error::OutputLimit);
                }
                pending.push(child);
            }
        }
    }
    Ok(tree)
}

fn executable_digest(
    root: &str,
    budget: Budget,
    cancellation: &dyn Cancellation,
) -> Result<String> {
    let mut file = File::open(format!("{root}/exe")).map_err(|_| Error::Io)?;
    let before = file.metadata().map_err(|_| Error::Io)?;
    if !before.is_file()
        || before.uid() != 0
        || before.mode() & 0o022 != 0
        || before.mode() & 0o111 == 0
        || before.len() == 0
        || before.len() > 128 * 1024 * 1024
    {
        return Err(Error::IdentityMismatch);
    }
    let mut hash = Sha256::new();
    let mut size = 0u64;
    let mut bytes = [0u8; 65536];
    loop {
        budget.check(cancellation)?;
        let count = file.read(&mut bytes).map_err(|_| Error::Io)?;
        if count == 0 {
            break;
        }
        size += count as u64;
        if size > before.len() {
            return Err(Error::Busy);
        }
        hash.update(&bytes[..count]);
    }
    let after = file.metadata().map_err(|_| Error::Io)?;
    let current = fs::metadata(format!("{root}/exe")).map_err(|_| Error::Io)?;
    let identity = |metadata: &fs::Metadata| {
        (
            metadata.dev(),
            metadata.ino(),
            metadata.len(),
            metadata.mode(),
            metadata.uid(),
            metadata.ctime(),
            metadata.ctime_nsec(),
            metadata.mtime(),
            metadata.mtime_nsec(),
        )
    };
    if size != before.len()
        || identity(&before) != identity(&after)
        || identity(&after) != identity(&current)
    {
        return Err(Error::Busy);
    }
    Ok(format!("sha256:{:x}", hash.finalize()))
}

fn network_namespace_identity(root: &str) -> Result<(u64, u64)> {
    let metadata = fs::metadata(format!("{root}/ns/net")).map_err(|_| Error::Io)?;
    Ok((metadata.dev(), metadata.ino()))
}

fn read_socket_tables(root: &str) -> Result<Vec<ProcSocket>> {
    let mut reservations = Vec::new();
    for (table, protocol, ipv6, tcp) in [
        ("tcp", TransportProtocol::Tcp, false, true),
        ("tcp6", TransportProtocol::Tcp, true, true),
        ("udp", TransportProtocol::Udp, false, false),
        ("udp6", TransportProtocol::Udp, true, false),
    ] {
        let path = format!("{root}/{table}");
        let file = File::open(path).map_err(|_| Error::Io)?;
        let mut bytes = Vec::new();
        file.take(MAX_TABLE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Io)?;
        if bytes.len() as u64 > MAX_TABLE_BYTES {
            return Err(Error::OutputLimit);
        }
        let reader = BufReader::new(bytes.as_slice());
        let mut lines = reader.lines();
        lines
            .next()
            .ok_or(Error::InvalidResponse)?
            .map_err(|_| Error::InvalidResponse)?;
        let mut rows = 0usize;
        for line in lines {
            rows += 1;
            if rows > MAX_TABLE_ROWS {
                return Err(Error::OutputLimit);
            }
            let line = line.map_err(|_| Error::InvalidResponse)?;
            let columns: Vec<_> = line.split_ascii_whitespace().collect();
            if columns.len() < 11 {
                return Err(Error::InvalidResponse);
            }
            // TCP LISTEN only; every UDP local bind reserves the local port.
            if tcp && columns[3] != "0A" {
                continue;
            }
            let (raw_address, raw_port) =
                columns[1].rsplit_once(':').ok_or(Error::InvalidResponse)?;
            let address = parse_proc_address(raw_address, ipv6)?;
            let port = u16::from_str_radix(raw_port, 16).map_err(|_| Error::InvalidResponse)?;
            let inode = columns[10]
                .parse::<u64>()
                .map_err(|_| Error::InvalidResponse)?;
            if port != 0 {
                reservations.push(ProcSocket {
                    protocol,
                    address,
                    port,
                    inode,
                });
            }
        }
    }
    Ok(reservations)
}

fn parse_proc_address(value: &str, ipv6: bool) -> Result<IpAddr> {
    if ipv6 {
        if value.len() != 32 {
            return Err(Error::InvalidResponse);
        }
        let mut bytes = [0u8; 16];
        for (index, chunk) in value.as_bytes().chunks_exact(8).enumerate() {
            let text = std::str::from_utf8(chunk).map_err(|_| Error::InvalidResponse)?;
            let word = u32::from_str_radix(text, 16).map_err(|_| Error::InvalidResponse)?;
            bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_ne_bytes());
        }
        Ok(IpAddr::V6(Ipv6Addr::from(bytes)))
    } else {
        if value.len() != 8 {
            return Err(Error::InvalidResponse);
        }
        let word = u32::from_str_radix(value, 16).map_err(|_| Error::InvalidResponse)?;
        Ok(IpAddr::V4(Ipv4Addr::from(word.to_ne_bytes())))
    }
}

fn attribute_docker_proxy_sockets(
    runner: &Runner,
    proc_sockets: &[ProcSocket],
    docker_bindings: &[DockerPublishedBinding],
    budget: Budget,
    cancellation: &dyn Cancellation,
) -> BTreeMap<u64, (ContainerId, String)> {
    if !proc_sockets.iter().any(|socket| {
        docker_bindings
            .iter()
            .any(|binding| binding.protocol == socket.protocol && binding.host_port == socket.port)
    }) {
        return BTreeMap::new();
    }
    let first = match ss_snapshot(runner, budget, cancellation) {
        Some(snapshot) => snapshot,
        None => return BTreeMap::new(),
    };
    let second = match ss_snapshot(runner, budget, cancellation) {
        Some(snapshot) => snapshot,
        None => return BTreeMap::new(),
    };
    let mut output = BTreeMap::new();
    let mut ambiguous_inodes = BTreeSet::new();
    fn matching_rows<'a>(snapshot: &'a [SsSocket], socket: &ProcSocket) -> Vec<&'a SsSocket> {
        snapshot
            .iter()
            .filter(|row| {
                row.protocol == socket.protocol
                    && row.port == socket.port
                    && (row.address == SsAddress::Wildcard
                        || row.address == SsAddress::Exact(socket.address))
                    && row.inode == socket.inode
            })
            .collect()
    }
    for socket in proc_sockets {
        // Only a procfs/Docker duplicate needs process attribution.
        if !docker_bindings
            .iter()
            .any(|binding| binding.protocol == socket.protocol && binding.host_port == socket.port)
        {
            continue;
        }
        let first_rows = matching_rows(&first, socket);
        let second_rows = matching_rows(&second, socket);
        if first_rows.len() != 1 || second_rows.len() != 1 || first_rows[0] != second_rows[0] {
            continue;
        }
        let Some(owner) = first_rows[0].owner.as_ref() else {
            continue;
        };
        if owner.name != "docker-proxy" {
            continue;
        }
        let Some(args) = read_docker_proxy_args(owner, socket.inode) else {
            continue;
        };
        if args.protocol != socket.protocol
            || args.host_ip != socket.address
            || args.host_port != socket.port
        {
            continue;
        }
        let matching = docker_bindings
            .iter()
            .filter(|binding| {
                binding.protocol == args.protocol
                    && binding.host_ip == args.host_ip
                    && binding.host_port == args.host_port
                    && binding.container_port == args.container_port
                    && binding
                        .container_ips
                        .values()
                        .filter(|ip| **ip == args.container_ip)
                        .count()
                        == 1
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            continue;
        }
        let identity = (
            matching[0].container_id.clone(),
            matching[0].structural_fingerprint.clone(),
        );
        if ambiguous_inodes.contains(&socket.inode) {
            continue;
        }
        if output
            .get(&socket.inode)
            .is_some_and(|previous| previous != &identity)
        {
            output.remove(&socket.inode);
            ambiguous_inodes.insert(socket.inode);
        } else {
            output.insert(socket.inode, identity);
        }
    }
    output
}

fn ss_snapshot(
    runner: &Runner,
    budget: Budget,
    cancellation: &dyn Cancellation,
) -> Option<Vec<SsSocket>> {
    let bytes = runner
        .run(
            Tool::Ss,
            &[
                "--no-header",
                "--listening",
                "--numeric",
                "--processes",
                "--tcp",
                "--udp",
                "--extended",
            ],
            budget,
            cancellation,
        )
        .ok()?;
    let text = std::str::from_utf8(&bytes).ok()?;
    let mut rows = BTreeSet::new();
    for line in text.lines() {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.len() < 7 {
            return None;
        }
        let protocol = match fields[0] {
            "tcp" if fields[1] == "LISTEN" => TransportProtocol::Tcp,
            "udp" if fields[1] == "UNCONN" => TransportProtocol::Udp,
            _ => return None,
        };
        let (address, port) = parse_ss_endpoint(fields[4])?;
        let inode = fields
            .iter()
            .filter_map(|field| field.strip_prefix("ino:"))
            .collect::<Vec<_>>();
        if inode.len() != 1 {
            return None;
        }
        let inode = inode[0].parse::<u64>().ok()?;
        let owners = fields
            .iter()
            .filter(|field| field.starts_with("users:"))
            .collect::<Vec<_>>();
        if owners.len() > 1 {
            return None;
        }
        let owner = if matches!(address, SsAddress::Unsupported) {
            None
        } else {
            owners.first().and_then(|field| parse_ss_owner(field))
        };
        rows.insert(SsSocket {
            protocol,
            address,
            port,
            inode,
            owner,
        });
    }
    Some(rows.into_iter().collect())
}

fn parse_ss_endpoint(value: &str) -> Option<(SsAddress, u16)> {
    let (raw_address, raw_port) = value.rsplit_once(':')?;
    let port = raw_port.parse().ok()?;
    if raw_address == "*" {
        return Some((SsAddress::Wildcard, port));
    }
    let address = raw_address
        .strip_prefix('[')
        .and_then(|address| address.strip_suffix(']'))
        .unwrap_or(raw_address);
    if address.contains('%') {
        return Some((SsAddress::Unsupported, port));
    }
    Some((SsAddress::Exact(address.parse().ok()?), port))
}

fn parse_ss_owner(field: &str) -> Option<SsOwner> {
    let contents = field.strip_prefix("users:((")?.strip_suffix("))")?;
    if contents.contains("),(") {
        return None;
    }
    let (name, identity) = contents.strip_prefix('"')?.split_once("\",pid=")?;
    let (pid, fd) = identity.split_once(",fd=")?;
    if pid.contains(',') || fd.contains(',') || !fd.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(SsOwner {
        name: name.to_owned(),
        pid: pid.parse().ok()?,
        fd: fd.parse().ok()?,
    })
}

fn read_docker_proxy_args(owner: &SsOwner, inode: u64) -> Option<ProxyArgs> {
    let root = format!("/proc/{}", owner.pid);
    let before = process_start_time(&root)?;
    let exe = fs::read_link(format!("{root}/exe")).ok()?;
    if exe.to_str()? != "/usr/bin/docker-proxy" {
        return None;
    }
    let metadata = fs::metadata(format!("{root}/exe")).ok()?;
    if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
        return None;
    }
    let fd_path = format!("{root}/fd/{}", owner.fd);
    let socket = fs::read_link(&fd_path).ok()?;
    if socket != format!("socket:[{inode}]") {
        return None;
    }
    let cmdline = process_cmdline(&root)?;
    if fs::read_link(format!("{root}/exe")).ok()? != exe
        || fs::read_link(&fd_path).ok()? != socket
        || process_cmdline(&root)? != cmdline
        || process_start_time(&root)? != before
    {
        return None;
    }
    let args = cmdline
        .split(|byte| *byte == 0)
        .filter(|arg| !arg.is_empty())
        .map(std::str::from_utf8)
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    if args.len() < 11 || args[0] != "/usr/bin/docker-proxy" {
        return None;
    }
    parse_proxy_args(&args[1..])
}

fn process_cmdline(root: &str) -> Option<Vec<u8>> {
    let mut cmdline = Vec::new();
    File::open(format!("{root}/cmdline"))
        .ok()?
        .take(MAX_PROCESS_BYTES + 1)
        .read_to_end(&mut cmdline)
        .ok()?;
    if cmdline.len() as u64 > MAX_PROCESS_BYTES || cmdline.last() != Some(&0) {
        return None;
    }
    Some(cmdline)
}

fn process_start_time(root: &str) -> Option<u64> {
    let stat = fs::read_to_string(format!("{root}/stat")).ok()?;
    let tail = stat.rsplit_once(')')?.1;
    tail.split_ascii_whitespace().nth(19)?.parse().ok()
}

fn parse_proxy_args(args: &[&str]) -> Option<ProxyArgs> {
    let mut values = std::collections::BTreeMap::new();
    let mut use_listen_fd = false;
    let mut index = 0;
    while index < args.len() {
        if args[index] == "-use-listen-fd" {
            if use_listen_fd {
                return None;
            }
            use_listen_fd = true;
            index += 1;
            continue;
        }
        let key = match args[index] {
            "-proto" => "proto",
            "-host-ip" => "host-ip",
            "-host-port" => "host-port",
            "-container-ip" => "container-ip",
            "-container-port" => "container-port",
            _ => return None,
        };
        let value = *args.get(index + 1)?;
        if value.is_empty() || value.starts_with('-') || values.insert(key, value).is_some() {
            return None;
        }
        index += 2;
    }
    if values.len() != 5 {
        return None;
    }
    Some(ProxyArgs {
        protocol: match *values.get("proto")? {
            "tcp" => TransportProtocol::Tcp,
            "udp" => TransportProtocol::Udp,
            _ => return None,
        },
        host_ip: values.get("host-ip")?.parse().ok()?,
        host_port: values.get("host-port")?.parse().ok()?,
        container_ip: values.get("container-ip")?.parse().ok()?,
        container_port: values.get("container-port")?.parse().ok()?,
    })
}
