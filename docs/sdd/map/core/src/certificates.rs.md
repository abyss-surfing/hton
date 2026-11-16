# core/src/certificates.rs

Stable ID: `core.certificates` · Phase: `R1` · Kind: source

I2 coordinator implementation binding; native host drivers and final acceptance remain pending. Purpose: One issuer and candidate certificate qualification

Implementation wave: `I2-C` (coordinator source present; final acceptance pending).

## Features and responsibility

- HTTPS

## Methods, arguments and result

- `issue` — arguments: names, staging, force; result: candidate material.
- `status` — arguments: inventory; result: expiry/coverage evidence.

## Structures and injected dependencies

CertificateInventory/CertificateCandidate/ServedEvidence; own certificate lifecycle

Dependencies: short-lived issuer port, configured DNS grants, candidate verifier, journal/apply, clock.

## Logic

- Short-lived acme.sh; explicit LE DNS-01.
- Validate SAN/chain/key/expiry; native consumers only through admitted bundle.
- Retry delivery separately from issuance.
- Amnezia retains native certificate ownership; no HTON certificate manager for it without owner decision.
- No-domain install can remain unpublished; public readiness requires valid matching certificates.

## Calls / reads

- [core/src/secrets.rs](secrets.rs.md) — `issue` -> `resolve`.
- [logging/src/lib.rs](../../logging/src/lib.rs.md) — `issue` -> `emit`.

## Called / read by

- [scripts/certs](../../scripts/certs.md) — `run` -> `issue`.
- [scripts/certs](../../scripts/certs.md) — `run` -> `status`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.


## I2 concrete binding

Typed coordinators own sequencing and validate invariants; trusted injected ports
own bounded native processes/transport and protected logical inventories. Ports
must fail unavailable/unsupported explicitly and never return synthetic success.
Mutation requires OperationContext Mutating, live deadline/cancellation and durable
journal intent before any secret, certificate, archive or runtime effect. Secret
payload wrappers redact Debug and are never cached with ordinary operation status.
Product tests remain deferred until I4; compilation is integration, not acceptance.

CertificateCoordinator.issue(CertificateIssueRequest,ctx) prepares only own certificate
candidates; status(VerifiedCertificate) reports Healthy/RenewalDue/Expiring/Expired/
NotYetValid/Untrusted with 30/14-day thresholds. CertificateName owns normalized
ASCII DNS and single-label wildcard SAN coverage, reused by checks/exports. Issuer
selection is closed to Let's Encrypt production/staging and DNS-01 InternetBS.
Scoped credentials are resolved only after Certificates journal intent. Existing
material is cryptographically reverified and reused when outside renewal window;
forced issue needs exact locally verified approval. The X509Verifier port validates
actual SANs, key/leaf match, configured trust chain and validity for the selected CA.
Neither command starts/reloads consumers nor installs Amnezia certificates (D80).

CertificateInventory.saved authenticates previously saved qualified bytes/metadata;
a pending uncertain issuer with no saved candidate requires explicit reconciliation,
not another issuance. Completed retries return the same candidate metadata; private
keys and DNS credentials use non-serializable redacted wrappers. Publication/delivery
uses normal plan/apply separately, so failed delivery cannot force another issuance.
AcmeIssuer serializes account/transaction work and records interruption checkpoints
before its native process. No Docker socket or executable hook is granted.

`scripts/install` calls this same own-site DNS-01 owner from stored CertificateNames and
protected credential grants. Valid matching material is reused; candidate issuance
does not itself activate a listener. Candidate verification and public delivery
remain separate. Install must not hardcode `example.com`, pass credentials in argv,
or use HTTP-01/certbot as an implicit fallback; public DNS-A remains unverified
until explicit `identity.whoami`.
