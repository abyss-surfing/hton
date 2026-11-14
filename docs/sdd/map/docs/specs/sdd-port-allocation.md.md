# docs/specs/sdd-port-allocation.md

Stable ID: `doc.docs-specs-sdd-port-allocation-md` · Phase: `design` · Kind: documentation

Canonical Core VPN port-allocation policy contract. D91–D93 owner references and
requirements are linked at the top of the SDD; the allocation policy is distinct
from pure `core.plan`. Planner, host evidence, install persistence and recovery are
implemented in source; build and product acceptance are deferred.

## Methods, arguments and result

- `plan` — selected managed VPN service bindings, coherent settings/registry read
  set, fresh reservation evidence, explicit OS-CSPRNG entropy bytes and budget →
  deterministic `PortAllocationCandidate` or typed blocker.
- `commit` — exact candidate, opaque allocation approval, protected journal intent
  and descriptor-scoped settings CAS → receipt or read/review/storage/recovery error;
  same-operation/same-candidate retries reconcile the exact child intent and a
  different unfinished candidate digest conflicts.
- `host.approvals.authorizeAllocation` — exact candidate and local operation
  context → private opaque capability, issued only after exact digest-bound TTY
  confirmation; scope includes candidate, actor, instance and operation ID.
- `host.observations.reservationEvidenceFor` — exact registry snapshot, injected
  operation control and context → complete host TCP/UDP reservations without
  opening/initializing `HostApplication`; enables first-install virtual preflight.
- `recover` — journaled candidate and exact expected revisions → terminal or
  recovery-required receipt; never chooses replacement ports.

## Logic and effects

- One distinct persistent 5000–6000 port per managed VPN backend, same value for
  TCP/UDP on the same service; public HAProxy TCP/443 is excluded.
- The current Amnezia descriptor is the only allocation-bearing module descriptor.
  VLESS uses the shared HAProxy/Caddy TCP/443 edge and has no separate host-published
  listener. Other standalone VPN listener modules must declare their owned settings
  paths/transports/claims before selection or enablement; OR-10 coverage for future
  backends is incomplete and cannot be inferred from the Amnezia mapping alone.
- Explicit entropy/evidence makes planning deterministic and testable without hidden
  probes or ambient randomness. Cryptographic entropy is injected by the trusted OS
  adapter; reservation evidence by the host observation owner. Linux socket rows
  remain unknown-holder; Docker binding rows retain exact container IDs that Core
  must match to an exact registered resource before recognizing a self-reservation.
- Reservation evidence reads fixed `/proc/net` tables and requires two stable Docker
  inventory/binding snapshots. Matching unknown procfs rows are retained and block;
  they are never inferred to belong to the matching Docker binding.
- Optional exact docker-proxy attribution requires socket inode↔PID/FD identity,
  stable process start time, trusted root-owned `/usr/bin/docker-proxy`, bounded exact
  Moby argv and a unique Docker binding/container-address match. If `ss` or any proof
  field is unavailable, base reservations remain complete with unknown holder and
  block reuse; no rerun readiness is promised.
- Saved values are reused. Invalid/duplicate/conflicting values require exact
  review/remediation; no silent reroll. A descriptor-known missing first-install
  settings document is bound as absent in the coherent `ReadSet`, planned from
  trusted defaults, and created with the selected port in one post-preflight CAS.
- Canonical candidateDigest covers service→port/path/transport assignments, prior
  values, trusted descriptor revisions, retained allocations, read-set revisions and
  expected absences, reservation digest and validity bounds. `host.approvals` binds
  the complete candidate and its digest to instance, actor and operation context;
  `--yes` cannot create the opaque capability.
- `Intent::PortAllocation { candidate }` durably retains the exact candidate before
  settings writes; recovery never substitutes new entropy or a new port. Descriptor
  revision drift refuses to recreate absent settings from changed defaults.
- A no-op with every exact allocation already persisted returns without approval,
  journal or settings writes. Changed values preserve all other YAML fields and use
  per-document CAS after protected originals/presence are journaled. Recovery accepts
  expected-old/expected-absence→exact-new or exact-new; every other state is
  recovery-required.
- Foreign discovery does not add a service to scope or authorize mutation.

Lifecycle add/enable first journals its immutable parent plan, then commits this
exact candidate with the same operation ID. `PortSettingsWriter::commit` and
`recover` correlate the child intent by operation ID plus candidate digest: resume
only the exact unfinished transaction, return an exact completed receipt, and block
any digest mismatch. This is the recovery seam for a crash after allocation CAS but
before the lifecycle caller records its phase receipt; it does not add a second
settings writer or permit reroll.

## Calls / reads

- [docs/INTERFACES.md](../../../../INTERFACES.md) — shared method and candidate contract.
- [docs/specs/sdd-installer.md](sdd-installer.md.md) — install caller.
- [docs/specs/sdd-config.md](sdd-config.md.md) — persisted config ownership.
- [docs/specs/sdd-foreign-adapters.md](sdd-foreign-adapters.md.md) — foreign boundary.
- [core/src/port_allocation.rs](../../core/src/port_allocation.rs.md) — planner and narrow settings writer/recovery owner.

## Called / read by

- [docs/specs/sdd-installer.md](sdd-installer.md.md) — install graph allocation.
- [core/src/host/installation.rs](../../core/src/host/installation.rs.md) — coordinator.
- [core/src/lifecycle.rs](../../core/src/lifecycle.rs.md) — module add/enable reuse.
- [core/src/plan.rs](../../core/src/plan.rs.md) — immutable candidate consumer.

Planner, descriptor validation, host reservation observation, install preflight and
durable allocation persistence are implemented in source. Build/product acceptance
is deferred; foreign remap and other standalone listener descriptors remain
incomplete.
