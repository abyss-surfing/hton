# `scripts/hton/styx-receiver`

Compile an opt-in Styx HTTPS CONNECT receiver configuration for sing-box. The
command starts nothing and changes neither the current ingress nor Dokimasia.

```sh
./scripts/hton/styx-receiver \
  --spec .secrets/styx/receiver.json \
  --output .secrets/styx/receiver-sing-box.json
```

The Tartarus launcher execs the project script [`scripts/styx-receiver`](../../scripts/styx-receiver);
paths are relative to HTON.

The specification and generated configuration contain credentials. Keep them
in HTON's secret repository. Output is replaced atomically with mode `0600`;
invalid input leaves an existing output intact. Errors omit supplied values.
The input must differ from the output and must not exceed 1 MiB.

Specification shape (all credentials below are placeholders):

```json
{
  "version": 1,
  "transport": "https-connect",
  "listen": "127.0.0.1",
  "listen_port": 8443,
  "tls": {
    "certificate_path": "/opt/hton/.secrets/styx/cert.pem",
    "key_path": "/opt/hton/.secrets/styx/key.pem"
  },
  "users": [
    {
      "username": "d6",
      "password": "replace-with-random-private-password",
      "backends": ["node-a"]
    }
  ],
  "backends": [
    {"id": "node-a", "protocol": "vless", "address": "127.0.0.1", "port": 10001}
  ]
}
```

Every field shown is required. Unknown fields are rejected. Backend IDs and
usernames are lowercase DNS labels, at most 63 characters. Passwords are unique
per user, printable ASCII and 16–1024 characters; generate strong random values.
Each user has a nonempty backend allowlist. Backend addresses must be literal
unicast IP addresses, never DNS names or wildcard addresses. TLS paths must be
absolute; certificate provisioning and renewal remain outside this compiler.

D6ton authenticates to the receiver, then requests
`CONNECT vless.node-a.styx.invalid:443` inside TLS. The protocol is derived
from the D6ton outbound type. This selector is not resolved in
public DNS. The receiver matches the authenticated username and exact
protocol-qualified TCP destination, forwards to the registered backend, and
rejects all other requests.
Different independent backends can share this receiver and its outer SNI.
The VPN stream, inner key and inner TLS handshake remain unchanged.

Validate with the selected engine before starting it:

```sh
sing-box check -c .secrets/styx/receiver-sing-box.json
```

The generated configuration uses the existing HTTP inbound, TLS, `auth_user`
route matching and destination overrides (sing-box 1.13-compatible fields).
The current protocol-qualified selector passed local paired acceptance with
sing-box `1.14.4` on 2026-11-14 (7 functional tests, 2 subtests); the receiver
and D6ton unit suites passed 23 tests and 47 subtests. An earlier
Dokimasia run used Xray/REALITY; this completion pass did not start or rewire the
stand. Record the engine version when repeating acceptance on another build or
platform.
See the [file contract](../sdd/map/python/styx_receiver.py.md),
[HTTP inbound](https://sing-box.sagernet.org/configuration/inbound/http/) and
[route actions](https://sing-box.sagernet.org/configuration/route/rule_action/).

This first slice carries TCP only; native UDP AWG/Hysteria wrapping is not
implemented. Per-user counters, dynamic control messages and an ordinary-site
fallback are not implemented. A common SNI is not evidence of resistance to
traffic classification or IP blocking. Triodos publication, certificate handoff,
sharing TCP/443 with the ordinary site, deployment and lifecycle integration are
separate work. Existing official-client paths remain unchanged.
