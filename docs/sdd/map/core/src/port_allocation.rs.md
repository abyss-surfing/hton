# core/src/port_allocation.rs

Stable ID: `core.port-allocation` · Phase: `R1` · Kind: source

Planner, graph composition helper, narrow settings writer, recovery source and
exact saved-candidate verifier are implemented; they have not been compiled or
accepted. Host reservation observation is implemented behind
`core.host.observations`; install integration is composed through Core reconcile.
Purpose: single policy owner for random, persistent and unique VPN backend port
allocation.

Implementation wave: `I3-C` (pure planner, graph composition and settings persistence;
coordinator integration remains).

## Features and responsibility

- config
- install
- lifecycle
- safety

## Methods, arguments and result

- `plan_graph` — trusted `ModuleCatalogue`, selected `DependencyGraph`, validated
  `NormalizedSnapshot`, exact diagnostic-free `RegistrySnapshot`, fresh reservation evidence,
  explicit entropy/time/budget → immutable candidate plus graph with exact dynamic
  host listener claims; composes selected and retained installed-service bindings
  without I/O; rejects graph conflicts and registry diagnostics.
- `plan` — `AllocationRequest`, fresh `ReservationEvidence`, explicit
  `EntropyBytes`, `Budget` → immutable `PortAllocationCandidate` or typed blocker;
  deterministic and side-effect free for identical inputs.
- `PortSettingsWriter::commit` — exact candidate, expected coherent read set,
  optional opaque crate-private `AllocationApproval`, journal, typed catalogue and
  module settings directory → receipt; checks exact revisions and writes only
  descriptor-allowlisted leaves through per-document CAS. All already-saved exact
  assignments return a no-write receipt without approval or journal. First
  assignments into fields with no saved port are permitted by the explicit OR-10
  install instruction after exact read-set/reservation validation; changing a
  previously saved port still requires the opaque exact TTY approval. A repeated
  same-operation/same-candidate commit finds and reconciles its exact journal intent;
  a different unfinished allocation digest for that operation conflicts.
- `PortSettingsWriter::recover` — transaction ID and `OperationContext` →
  recovered/partial/recovery-required receipt; accepts only
  expected-old or exact-new documents, never rerolls and refuses drift. Normal
  operation retries remain ID-bound; installer recovery requires OS euid 0 plus
  `ActorSource::Local`/`os.uid.0`, and
  publishes terminal metadata under the protected transaction's original
  operation ID even when invoked from a later CLI operation.
- `candidate_is_saved` — exact candidate and read-only `OperationContext` → bool;
  checks exact descriptor allowlisted settings under the module inventory lock.
  Rootless selection may trust a module document only when bounded journal history
  has one committed candidate for the same `instance_id` and this verifier confirms
  every current settings byte/value matches it.

For lifecycle add/enable, the caller first writes a parent `Intent::Lifecycle`
containing the candidate and then invokes this writer using the same operation ID.
Commit/recovery correlation is exact on operation ID plus candidate digest: a
matching unfinished child intent reconciles its original transaction; a matching
completed child returns its persisted receipt; a different digest conflicts. This
closes the crash window before a parent phase receipt without changing port policy
or allowing replacement entropy.

## Structures and injected dependencies

`AllocationRequest`, `ServicePortBinding`, `ReservationEvidence`,
`PortAllocationCandidate`, opaque host-issued `AllocationApproval`,
`PortAllocationReceipt`.

Dependencies: validated config snapshot/revisions; catalogue-declared stable service
IDs and exact field mappings; fresh host TCP/UDP reservation evidence from
`core.host.observations.reservation_evidence`; exact registry revision; explicit
OS-CSPRNG entropy bytes; opaque host approval; protected `SecureDir` module inventory;
`JournalStore`. No ambient filesystem paths, socket, Docker, shell or RNG access.

## Logic

- Assign one unique port per selected managed VPN backend/service ID from inclusive
  5000–6000. TCP and UDP of the same service share the same number.
- Reuse valid durably saved allocations, including retained disabled installed
  services for uniqueness, across process and host restarts. Per-service settings
  are the source of truth; memory or journal state alone is insufficient. Never
  reroll an occupied/out-of-range/duplicate saved value silently. OR-10 authorizes
  first assignment after collision checks; only changing a saved value requires
  distinct exact approval. Recovery validates originals for initial assignments
  and replacement of a saved value.
- Deterministically consume explicit entropy with unbiased sampling; discard ports
  occupied by fresh complete evidence; block if the full requested set cannot fit.
- Candidate carries exact prior fields, proposed allocations, evidence digest,
  trusted descriptor revision, coherent config read set (including exact absence for
  known first-install module settings), exact registry revision and expiry. The
  planner is pure;
- Only unregistered first-install services may use descriptor defaults for an absent
  document; missing settings for a registered/retained service block as incomplete.
  `PortSettingsWriter` edits only
  descriptor-allowlisted settings leaves and retains all unrelated YAML values.
- Settings CAS is preceded by `Intent::PortAllocation` with the exact candidate,
  protected original bytes and original presence. An absent first-install document
  is created atomically from trusted descriptor defaults plus its allocation.
Recovery writes only expected-old/expected-absent→exact-new or durably recognizes
exact-new; descriptor revision drift blocks recreation from changed defaults and
every other state remains recovery-required. Normal retries retain same-operation
correlation; a local-root install recovery may use a new CLI operation ID while the
writer republishes the terminal record under the protected transaction's original
operation ID.
- HAProxy TCP/443 and protocol paths multiplexed there are outside allocation.
- Discovery alone never associates or authorizes foreign resources. Registered
  foreign mutation still requires adapter qualification, exact fingerprint,
  protected snapshot, exact consent and recovery.
- Keep `core.plan` pure; it consumes the candidate but does not generate randomness
  or probe listeners. Config validates but does not allocate.

## Calls / reads

- [core/src/config.rs](config.rs.md) — `plan` → validated snapshot/read set.
- [core/src/journal.rs](journal.rs.md) — `commit/recover` → durable intent/receipt.
- [core/src/catalog.rs](catalog.rs.md) — `plan` → descriptor service/settings mapping.
- [core/src/host/approvals.rs](host/approvals.rs.md) — `commit` → opaque exact candidate capability.
- [core/src/lib.rs](lib.rs.md) — public Core domain API reexport.
- [core/src/host/observations.rs](host/observations.rs.md) — fresh reservation
  evidence maps Docker identity only through exact registered ID and fingerprint.
- [core/src/host/installation.rs](host/installation.rs.md) — install coordinator
  integration remains incomplete.
- [docs/INTERFACES.md](../../../../INTERFACES.md) — allocation candidate and methods.
- [docs/specs/sdd-port-allocation.md](../../../../specs/sdd-port-allocation.md) —
  canonical policy contract.

## Called / read by

- [core/src/host/installation.rs](host/installation.rs.md) — preflight/execute →
  plan/commit for the selected install graph.
- [core/src/lifecycle.rs](lifecycle.rs.md) — add/enable → plan/commit before activation.
- [core/src/plan.rs](plan.rs.md) — consumes a reviewed candidate as an immutable input.
- [core/src/lib.rs](lib.rs.md) — module export binding.

## Errors and completion

Typed blockers include invalid descriptor/saved values, duplicate saved assignment,
reservation evidence missing/stale, entropy unavailable, no free port, exact review
required, candidate expired, read conflict, unauthorized commit, storage failure and
recovery required. Error fields are bounded and redacted.

Planner, graph composition, settings writer/recovery and bounded host reservation
observation source are implemented. Install and direct `modules.enable` use the
candidate writer; full lifecycle port correlation and safe foreign Amnezia remap
remain separate blockers. Build and acceptance remain deferred.
`PortSettingsWriter::recover` now accepts a new local-root CLI operation context for
an existing protected allocation transaction: it preserves the original journal
operation ID solely when publishing the exact recovery terminal. The writer still
checks the transaction kind, candidate digest, protected originals and exact
before/after bytes; it does not reauthorize a different candidate.
