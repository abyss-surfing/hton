# SDD lifecycle — composition, setup and whole-server run intent

Authority: owner Q12–Q16 and Q26–Q31. Planned behavior only.

## State model

Keep independent dimensions: installed/associated membership; setup complete or
incomplete; desired enabled boolean; server run intent running/stopped; observed
running/stopped/failed/unknown. Availability/qualification is separate evidence.
The shipped catalogue is not the installed registry. The registry under `work/`
uses stable IDs and explicit owned/associated resource identities, approved scope,
original restart policies, setup checkpoints and image versions. Discovery alone
cannot register a foreign container. Replaced container IDs require reconciliation.

## Module operations

| Operation | Durable effect and failure behavior |
|---|---|
| add | Obtain declared resources, seed missing config, request missing inputs; new member stays disabled; interruption retains incomplete state; repeated add resumes |
| enable | Require installed complete compatible module and dependencies; validate batch, preview interruption and apply by default |
| disable | Remove participation and owned effects; retain settings/data/credentials; shared engines remain for other consumers |
| remove | Stop/exclude registered module, retain config/data and recovery records; reject unresolved dependents; failed cleanup remains visible |
| purge | Separate explicit deletion of retained module-owned data; refuse shared/unrelated resources or needed recovery evidence |
| update | Separate module-owned image/resource update with pinned prior version and declared compatibility/recovery; never implicit in up |

### Stable VPN port state in lifecycle plans (D91–D93)

For `add` and `enable`, lifecycle preparation derives the prospective selected
service graph from the exact selection candidate (or the already-saved desired
state). When that graph contains descriptor-declared external VPN backend
bindings, host preparation obtains complete fresh TCP/UDP reservation evidence
and explicit OS CSPRNG entropy, then delegates to `core.port-allocation.planGraph`.
The returned immutable `PortAllocationCandidate` is included in `LifecyclePlan`
and the materialized claims from `catalog.bindAllocation` are the same claims used
for conflict and effect planning. No caller traverses descriptor setting paths.
Existing valid service settings are retained; only missing allocations get a
random value. The allocator also considers retained installed/disabled services
for uniqueness. A service's number is stored in its existing module settings and
is reused by later `scripts/install`, add and enable operations.

`LifecycleService.execute` re-reads and re-prepares against the submitted plan.
For allocation-bearing plans, preparation validates that exact candidate against
current catalogue/descriptor revisions, the coherent config read set (including
known absence), exact registry revision/resource identities, selected service set,
reservation-evidence digest and expiry. Any changed dependency returns
`planStale`; execution does not call the planner with new entropy. The lifecycle
intent serializes the exact candidate before settings writes. The operation then
delegates persistence to `PortSettingsWriter`, which journals protected originals
and performs only descriptor-allowlisted module-settings CAS. Its durable intent
is correlated with the lifecycle operation ID and candidate digest; interrupted
execution reconciles that exact assignment before continuing. After allocation
commit, selection commit and activation/apply may proceed. If later selection or
activation is interrupted, the saved port remains the service's stable value and
retry reuses it; a port is not rolled again because a later step failed.

The allocation review is separate from generic lifecycle/effect approval. After
all blockers and fresh evidence are reviewed, a changed or newly populated saved
port field requires `host.approvals.authorizeAllocation` bound to the full exact
candidate, actor, instance and operation. `--yes`, selection consent, adoption
approval and remap approval do not create this capability. A true no-write
candidate whose saved fields already equal the proposed assignments needs no
allocation grant. If review is unavailable or stale, no port setting, selection,
setup or runtime effect is written. Candidate persistence can precede the root
selection CAS so an enabled selection is never published without its exact port;
a later root CAS conflict leaves only a retained, safely reusable service setting.

Foreign association remains a separate transition. Discovery or selection alone
cannot put a container in the allocation graph. Adoption must first complete with
fresh fingerprint-bound qualification and its own approval/journal. Any existing
foreign host-port change additionally requires a protected exact snapshot and a
distinct remap approval; `AllocationApproval` never authorizes container recreation
or remap. A port candidate can be planned only for the service graph that has
passed these membership and qualification gates.

The internal `PreparedOperation` carries the optional candidate produced by the
host allocation preparation owner. Pure `core.lifecycle::plan` copies that exact
value into `LifecyclePlan`; it does not generate entropy or inspect settings paths.
During execute, the preparation owner receives the submitted plan as its baseline,
revalidates that candidate against the exact current inputs, and returns it unchanged
or fails stale. The planner's full structural equality check then binds the accepted
candidate to the submitted operation before lifecycle intent is written.

The current `LifecyclePlan` contract and host preparation binding do not yet carry
this candidate, and host production preparation is not wired. These requirements
are a target contract, not evidence that lifecycle allocation works today.

### Membership operation scenarios and host binding status

`remove` operates on a private prospective selection candidate. Before selection is
committed, preparation must validate the resulting graph and produce the exact
effect plan; apply rechecks its read set and journals both the membership transition
and bounded effects. The operation retains settings, config, data, credentials and
recovery records. It stops only the target's exact owned runtime resources and
removes its participation; shared Xray/Unbound or other services stay while any
consumer remains. Unresolved dependents, stale candidate evidence, or an unknown
resource owner fail before effects. A stop failure records the surviving identity
and `CleanupRequired`; it never erases registry ownership. The host now binds opaque
prospective-selection preparation to the ordinary effect owner. R1 VLESS is a
resource-free Owned feature: its physical engines belong to Core, so journalled
removal applies the prospective composition and retains its configuration without
stopping shared engines. A resource-bearing or Associated entry still requires a
qualified exact runtime stop adapter and is rejected before writes. Interrupted
ambiguous removal remains a recovery blocker rather than being blindly replayed.

`purge` requires a verified removed/stopped member and an explicit approval bound to
the exact typed deletion set. The set consists of logical config/artifact/secret and
owned-resource IDs plus revision/before-digest, each routed to its owning deletion
port. Planning proves there are no active, shared, foreign or recovery references;
unknown reference state refuses deletion. Secret versions are deleted only by the
secret owner, files through descriptor-anchored storage, and native resources through
Runtime. Durable intent precedes deletion and each receipt records exact completion;
after interruption, reconcile by exact identity/digest/absence before retry. There
is no raw path, recursive delete, shell or generic Docker delete input. The R1
binding supports the removed resource-free Owned VLESS feature's exact
`config.vless.v1` settings artifact. Local root confirms a digest-bound TTY challenge;
`ModulePurge` retains protected exact bytes before descriptor-anchored CAS deletion
and registry purge. Resume admits only exact-original/absent configuration and
unchanged removed entry; re-add, drift, active participation or recovery references
block. Nested registry intents reconcile exact-before/exact-after state. Shared
credentials and engines remain with their owners; unsupported deletion sets are
rejected before effects.

`update` requires a module-owned installed artifact, trusted pinned target digest,
compatibility and config/data-format evidence, retained prior version, exact action
set and approval. It updates only the named module's resources; shared engines use
their own platform lifecycle. Missing capability or owned target fails as unsupported
before effects. Stale installed digest, incompatible data format, missing prior
version/recovery evidence or target substitution refuses execution. `up` performs no
update, migration or image pull. Current Core has no module update planner/adapter,
and VLESS owns no image artifact; its former `xray.update` binding incorrectly
implied authority over a shared engine and is removed pending a module-specific
updater.

An explicit save-only mode defers activation for a batch. While server run intent
is stopped, participation changes may be saved/validated but cannot start services;
report deferred activation. Adding resources is distinct from enabling listeners.
Re-add of retained data validates versions/ownership and never silently overwrites it.

## Whole-server transitions

`up` validates desired state before effects. Invalid configuration returns an error
and offers a separate explicit accepted-bundle start, without rewriting desired YAML.
For valid config, persist running intent and reconcile missing enabled members;
healthy members stay running. Report partial failure per module and return error.
Repeated up resumes. No installs, image pulls, updates or migrations during startup.

`down` uses durable registry/runtime records, so broken editable YAML cannot prevent
stopping known associated resources. Persist stopped intent first; fence active
trials/guardians/watchers and disable systemd/Docker/associated autostart owners.
Attempt every stop in dependency order with bounded timeouts, even after failures.
Report survivors and unknown observations; off means all members verified stopped.
Preserve data and the boot prohibition on failure and across reboot.

Boot checks durable intent before starting accepted artifacts. Stopped intent wins
over a guardian's old rollback lease; compensation may preserve artifacts but cannot
restart the stack. A power loss during down leaves stopped intent for next boot.
External components need verified restart-policy coordination; registration is blocked
if another independent owner would restart them in violation of this contract.
Renewal may prepare cert material while stopped, never start traffic services.

`runtime.rollback [BUNDLE_ID]` is a journaled explicit restore of the exact
`previousCompatible` bundle retained by the committed activation lease. An optional
ID must equal that target; arbitrary retained bundles are not selectable. The
independent runtime checks the lease's protected digest/fallback authority, exact
current resource bindings, credential fence and certificate validity before effects
and again before accepting the restored bundle. It applies only declared inverse
actions under the current run epoch. If `down` advances the epoch, rollback cannot
restart services and reports stopped/recovery-required. An expired or incompatible
target is never accepted. A successful explicit rollback consumes this one-step
fallback; a later rollback target requires another qualified commit.

`services.restart [SERVICE_ID...] [--wait N]` is a journaled bounded operation over
the saved enabled optional/data-service closure and current accepted bundle. Omitted
IDs exclude core ingress; explicit IDs must also be enabled, installed, Owned and
represented by a current accepted receipt. Shared services follow enabled consumers.
The intent binds exact bundle ID, run epoch, selected IDs and wait deadline. Runtime
rejects stale, Associated or missing resources and active trial/maintenance. It
restarts only exact admitted identities; launch drift cannot turn into Recreate or
artifact rewrite. Post-restart probes cover selected services only. Unknown or
unsupported evidence returns Partial with successful receipts/survivors retained.

## Foreign Amnezia

Foreign candidate discovery is read-only and does not add membership. Association
requires the operator to select one exact immutable resource ID, successful fresh
qualification bound to its structural fingerprint, then a separate local approval
and journalled `RegistryChange::Adopt`. The associated entry records exact resource
identities, minimal lifecycle scope, qualification digest, original restart owner and
policy. A changed fingerprint invalidates the qualification and blocks adoption.
Preserve configuration/certificate ownership and prior restart behavior for
detach/undo. Associated resources participate in up/down; unrelated discovered
stacks do not. Do not extend this scope to the separately managed outbound VPN
client.

Association, port allocation and foreign remap are separate authorities. Adoption
grants only the recorded lifecycle scope. After association, allocation reads the
selected service's saved host-port settings and reuses them on repeated install;
new values are generated only for an absent setting and committed through the
allocation owner. If applying a new host binding is necessary, a separate remap
approval must bind the exact source/destination TCP/UDP mappings and protected
snapshot digest. Neither adoption nor allocation approval grants permission to
recreate the foreign resource. Snapshot/recovery must cover writable-layer state,
create parameters, mount data ownership and prior runtime/restart state; unsupported
or incompletely captured state blocks remap and preserves recovery data.

D80 accepts native REALITY passthrough with genuine own-domain HTTPS on the ordinary
site. Qualification must establish exact installed identity, SNI/target, route-loop
absence and restart semantics; unsupported or unavailable facts remain a blocker.
No extra certificate manager is permitted. Detailed foreign discovery,
qualification, adoption and snapshot requirements are owned by
[sdd-foreign-adapters](sdd-foreign-adapters.md).

The candidate field is additive and optional in serialized `LifecyclePlan` so
older records without an allocation remain readable. If an older add/enable plan
is resumed where the current trusted graph now requires a candidate, it becomes
stale/recovery-required; the coordinator must not create a new random assignment
while replaying that old plan.

## Acceptance

Interrupted add resumes disabled; incomplete module cannot enable. Disable/re-enable
preserves data/keys; remove differs from purge; update differs from up. Stop failure
continues other stops and lists survivors; reboot after down remains off. Partial up
and repeated up preserve healthy connections. Invalid desired YAML refuses ordinary
up but permits explicit accepted start; down works with malformed desired files.
Race down against trial, lease expiry, boot, renewal and concurrent lifecycle calls.
No rollback or scheduler may undo the later durable stop intent.

## Background jobs, restore and partial dependency failure

Whole-server down also stops optional associated management/web and suspends HTON
periodic jobs, including automatic renewal/watch tasks; no background job revives
the server. Explicit manual certificate preparation while stopped is allowed and
does not start services. Up restores declared schedules with the enabled composition.
The mandatory HTON daemon is runtime supervision, not a mandatory management API.

Restore always records stopped intent and keeps restored autostart inactive, even
if archived RuntimeState said running. Inspect versions/secrets/module availability;
unsupported migration is reported without startup or value loss. A later explicit
up/accepted-start authorizes running again.

Startup follows dependency order: failed dependencies block only their dependent
members; unrelated healthy members remain up. Shared services stop after their
last consumer. Removal with failed stop remains cleanup-required in the registry;
never erase the record of a surviving process. Cancelled/uncertain update/add
steps retain checkpoints and their exact receipts before a retry.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

Future API down is a durable operation independent of its requesting HTTP process:
acknowledge its ID/intent before stopping optional management, then finish through
runtime even if the connection closes. Local CLI reads the terminal result. Purge
requires removed/stopped state and no shared consumers or needed recovery originals.

Membership acceptance additionally covers: removing an active VLESS route while
retaining config/data/secrets; failed exact stop with survivor and cleanup-required
record; removing one of multiple shared-service consumers without stopping the
shared service; purge refusal for active/shared/recovery references or unknown
ownership; interrupted purge reconciliation before retry; update refusal for stale
installed/target digests or incompatible config/data; and proof that `up` issues no
image pull. These remain planned scenarios, not implementation evidence.

## M1 managed access and client-page resources

For managed Amnezia use sdd-managed-amnezia: remove/uninstall deactivate native
service and generated SSH admission/sudo grant but retain keys/data/home/receipts;
only explicit purge deletes retained exact owned resources. Disable/down preserve
credentials and stop the associated page-serving/refresh scope, without touching
unassociated accounts/containers. The page timer cannot clear durable stopped
intent or start Caddy/VPN. Remove the page route before deleting its active artifact;
shared Caddy remains when needed by the site or other profiles.
