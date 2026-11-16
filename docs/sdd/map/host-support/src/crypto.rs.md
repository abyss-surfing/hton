# host-support/src/crypto.rs

Stable ID: `adapter.host-crypto` · R1 · I3-B. Fixed cryptographic tool transport.

`CryptoTools.age_encrypt/age_decrypt` use the private bounded run_input transport,
X25519 age recipients and an external private identity file. ProtectedBytes has no
Serialize and redacted Debug. `certificate_der/certificate_public_key/private_public_key/
certificate_metadata/verify_chain` expose fixed OpenSSL operations only, never raw
executable/options or arbitrary shell. PrivateDirectory/PrivateFile reuse hton-storage
for absolute euid-owned private directory and single-link bounded file validation.
`issue_acme` runs the pinned acme.sh image with no Docker socket/public ports, fixed
DNS-01/LE endpoint, private mounted work and credential envfile; no hooks/upgrades.

Inputs remain protected, tool stderr is never surfaced. Deadline/cancellation/output
budgets are enforced before and after native calls. Native execution and build are
not performed during this source-only implementation phase (D81).

Primary sources: [age](https://github.com/FiloSottile/age), [OpenSSL x509](https://docs.openssl.org/3.0/man1/openssl-x509/),
[pinned DNS adapter](https://raw.githubusercontent.com/acmesh-official/acme.sh/3.1.6/dnsapi/dns_internetbs.sh),
[acme image source](https://raw.githubusercontent.com/acmesh-official/acme.sh/3.1.6/Dockerfile).
