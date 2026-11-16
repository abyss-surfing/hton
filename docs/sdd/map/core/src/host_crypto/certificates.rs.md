# core/src/host_crypto/certificates.rs

R1 I3-B concrete OpenSslVerifier, AcmeContainerIssuer, FileCertificateInventory.
OpenSSL verifies configured production/staging trust at the trusted clock, extracts
actual SAN/time/public key and compares leaf/key SPKI. Sealed anonymous public-chain
memory avoids disk writes during read-only verification; private keys use bounded
stdin. Staging trust must be explicitly provisioned, never system-trust fallback.

ACME work is private, request/transaction bound and single-owner serialized. DNS
credentials enter a private Docker envfile only; pinned short-lived acme.sh has no
Docker socket, public binds, arbitrary hook or upgrade path. Uncertain native jobs
retain durable begin evidence and cannot be reissued blindly. Candidate files use
protected immutable bytes and exact transaction aliases; reads reverify fingerprints
and actual certificate qualification. Policy callbacks refresh existing desired/runtime
owners; they are not alternative domain rules or synthetic-success drivers.
