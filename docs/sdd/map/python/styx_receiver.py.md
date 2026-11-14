# Styx receiver configuration compiler

Implementation: `python/styx_receiver.py`. Entry: `scripts/styx-receiver`.
This opt-in transport slice is separate from the existing minimum ingress;
it does not migrate Triodos or change Dokimasia's composition.

## Interface and ownership

`compile_receiver(spec: dict) -> dict` is a pure transformation. It validates
the `version: 1`, `transport: "https-connect"` contract and returns a complete
sing-box configuration. Invalid input raises `ValueError` with a static message
that contains no supplied credentials or identifiers. It does not read files,
issue certificates, open sockets, launch processes or mutate its argument.

Required root fields: `version`, `transport`, `listen` (literal IP),
`listen_port`, `tls`, `users`, `backends`. TLS contains absolute
`certificate_path` and `key_path`; the runtime reads them. Each backend contains
`id` (lowercase DNS label), `protocol` (`vless`, `vmess`, or `trojan`), literal
destination `address`, and `port`.
Each user contains a unique `username`, unique nonempty `password`, and a
nonempty list of authorized backend IDs. Unknown fields, invalid versions,
duplicates, empty grants, absent destinations and nonliteral backend addresses
are rejected. Backend IDs and usernames use `[a-z0-9]` followed by at most 62
lowercase alphanumerics/hyphens, ending in an alphanumeric. Passwords must be
printable ASCII, 16–1024 characters; entropy remains the credential issuer's
responsibility. Ports are integers 1–65535, never booleans.

## Runtime contract

The HTTP inbound `styx-in` requires TLS and HTTP authentication. D6ton CONNECTs
to `<protocolId>.<backendId>.styx.invalid:443` inside TLS; the protocol ID is
derived from the validated inner outbound. That address is a logical selector,
not a public DNS record. Rules jointly match inbound, authenticated username,
exact protocol-qualified selector, selector port and TCP, then override the
destination with its registered IP/port. A final reject rule denies everything else. There is no
sniffing, DNS lookup, arbitrary target forwarding or default credential.
Inner VPN bytes and cryptographic identity remain opaque and unchanged.

This version carries TCP connections only. It does not implement UDP, traffic
accounting, control messages, automatic certificate renewal or camouflage
guarantees. Public SNI/443 binding and a real-site coexistence policy remain
owned by the public edge and require separate integration.

## CLI effects and validation

`main(argv) -> int` accepts `--spec FILE --output FILE`. It reads at most 1 MiB,
rejects duplicate JSON object keys, compiles, then writes an atomic mode-0600
file. Input and output must differ. Failure preserves an existing output.
The CLI reports a redacted error and nonzero status; it never prints the config.
The operator must keep both files under HTON's secret repository. Engine
configuration validation and bounded local integration tests follow compilation;
the CLI neither starts the receiver nor deploys it.

Tests: `tests/test_styx_receiver.py` exercises the public compiler and CLI,
authorization rendering, malformed records, redaction and atomic output effects.
The cross-project Styx transport test exercises actual CONNECT delivery separately.
