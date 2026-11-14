# Explicit final-bundle qualification

Run `platform/validation/run-bundle BUNDLE_ID --json` against an installed protected bundle.
The wrapper delegates to the same `bundle.validate` Core operation as `scripts/validate`.
It does not build, pull images or apply the candidate to production.

Core verifies the exact candidate/artifact hashes and cached immutable server and
client image pins. It journals sandbox ownership, runs fixed native parsers, then
starts an internal Docker topology without published ports or foreign mounts.
Supported scenarios are trusted HTTPS direct/through HAProxy, unknown-SNI rejection,
selected VLESS WS/XHTTP through a pinned Xray client and selected Unbound UDP/TCP.
The VLESS target and DNS local zone are explicit private test fixtures, with mapping
and generated client hashes bound into evidence. Public recursion, external
reachability, real devices and Amnezia REALITY remain unverified. Unsupported
selected protocols fail explicitly.

The ACME image is reused solely for fixed `/usr/bin/curl` and `/usr/bin/dig` clients;
its ACME entrypoint never runs. The pinned image source installs those tools:
[acme.sh Dockerfile](https://raw.githubusercontent.com/acmesh-official/acme.sh/3.1.6/Dockerfile).
TLS clients use system trust; the negative unknown-SNI check alone permits an
untrusted certificate to distinguish routing rejection from certificate-name failure.
Xray client schemas follow the upstream [VLESS outbound](https://xtls.github.io/en/config/outbounds/vless.html),
[TLS transport](https://xtls.github.io/en/config/transport.html),
[WebSocket](https://xtls.github.io/en/config/transports/websocket.html) and
[XHTTP](https://xtls.github.io/en/config/transports/xhttp.html) contracts. This source
binding is not evidence those scenarios have passed on the selected pinned images.

Returned IDs are journaled before start. After a crash, another explicit validation
cleans only exact operation-labelled sandbox resources before proceeding. Containers
are removed before their internal network, then protected copied artifacts are
removed. Uncertain cleanup prevents a passing attestation. Successful attestations
are protected under `work/bundles/BUNDLE_ID/validation/` and expire after five minutes.
