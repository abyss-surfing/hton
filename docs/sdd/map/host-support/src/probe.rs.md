# host-support/src/probe.rs

Stable ID adapter.host-probe · R1 · I3

Fixed bounded native clients: curl requests only root path on exact supplied IPv4
endpoint with verified TLS hostname/optional private SOCKS proxy; no arbitrary URL,
headers/redirects/proxy environment or credential argv. Reply bodies remain protected.
DNSSEC check uses fixed dig query root SOA with DNSSEC/AD validation, not open-socket
success. Native Xray test client uses the exact cached pinned image, no pull, private
namespace/no public publication and a protected fixed client.json input; runtime owns
exact ID cleanup and credentials never enter stdout/logs. These methods do not grant
ownership, release acceptance or caller-selected subprocesses.
