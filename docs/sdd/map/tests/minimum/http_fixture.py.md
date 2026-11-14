# tests/minimum/http_fixture.py

Stable ID: tests.minimum.http-fixture · M0 · source

Disposable local ACME test authority for native pinned acme.sh HTTP-01 acceptance.
Serves directory/nonces/account/order/authz/finalize/certificate on loopback high
ports, verifies the exact token/JWK thumbprint over the real standalone listener,
and signs a test CSR with an ephemeral private CA. No public DNS or CA requests.
Production trust must reject the private test CA. Uses the production issuer argv
factory, replacing only test URI/port/UID and adding the per-run cleanup label.
No test endpoint is available through product CLI. Caller cleans exact fixture IDs.
