# core/src/host/approvals.rs

Shared `confirm_local_scope(app,ctx,prompt,expected)` owns bounded exact local-root
TTY input mechanics only. It cannot mint an approval or interpret a domain plan.
Callers derive and validate their own immutable scope before requesting it, then
persist the appropriate typed intent/grant. All approval families reuse the same
deadline/cancellation/line-bound reader. `--yes` never enters this capability.

Stable ID host.approvals · R1 · I3-A

Protected ordinary-owned policy is seeded during journalled installation. authorize
refreshes exact owned resource inventory and returns typed approvals plus trusted
actor refs; it never accepts caller assertion or grants foreign/host-network policy.
Public listener grants require separate exact TTY challenge (not --yes), store a
bounded expiring consent bound to graph listener/resources/instance and preserve
old bytes via CAS. Read-only plan never prompts or writes, only reports missing
consent. Effects refresh and use returned context before journal/stage/apply.

`authorize_allocation(app,candidate,ctx)` separately issues an opaque
`AllocationApproval` only for an exact current candidate after local-root/TTY
confirmation. The displayed service/port/path/transport set and the complete
candidate (including candidate digest, coherent read-set revisions, prior values,
reservation digest and expiry) are hashed with the instance, local actor and
operation ID. A protected record and `Intent::Approval` preserve the grant; expiry,
candidate, actor, instance or operation mismatch refuses reuse. `AllocationApproval`
has private fields and is crate-private, not a serializable wire value; a bare
`ApprovalId`, caller-supplied `AllocationReview` or `--yes` cannot mint the capability.

`authorize_first_install_allocation(instance_id,candidate,ctx,control)` provides
the equivalent grant before protected approval storage exists. It performs the
exact local TTY challenge and returns an opaque in-memory `AllocationApproval`
bound to the pre-generated instance, actor, operation, complete candidate and
deadline. It writes no approval record or journal and must run before bootstrap
effects; `--yes` cannot satisfy it. The caller may use the capability only during
that operation after creating the protected stores.

Called by: `core.port-allocation.commit` verifies its `covers(candidate,instance,ctx,
now)` capability before settings changes. `core.host` reexports the issuance method
and opaque type only within the crate.

`authorize_install_input(app,candidate,ctx)` uses a separate domain-separated
`install-input-approval-v1` scope. It displays exact previous→proposed site domain,
public TCP/443 endpoint and non-secret certificate account email, requires the
matching bounded TTY challenge, and
persists a protected record plus `Intent::Approval`. The grant expires within five
minutes and no later than the operation deadline. It is not interchangeable with
VPN allocation grants or public-listener/foreign consent.

`verify_install_input(app,candidate,review,ctx)` reloads that protected record and
checks the candidate digest and full candidate scope, instance, local actor,
operation and expiry before returning an opaque crate-private `InstallInputApproval`.
The serializable `InstallInputReview` is only a lookup reference; it cannot authorize
the writer by itself, and `--yes` cannot mint or broaden the capability.

Called by: `core.host-installation.commitInputs` obtains the review after full
read-only preflight and verifies the opaque capability before its allowlisted config
CAS. Candidate changes are limited to three unique typed fields and validated
certificate emails; approval text renders only those exact non-secret before/after
values. Credential bytes and SecretStore material never enter the candidate.

First install has no persisted instance/approval inventory yet. Its TTY decision
must use a separate opaque in-memory review session bound to the pre-generated
instance, actor, operation, exact candidates and deadline; only after consent may
bootstrap create durable records. Do not call the persisted authorization path by
seeding metadata before full preflight.

`authorize_module_purge(app,module,artifact,scope,ctx)` mints an opaque in-memory
PurgeApproval only after local root TTY confirmation of the exact scope digest.
The host membership owner binds that scope to immutable registry/config evidence
and durably journals it before deletion. Ordinary --yes and serialized input cannot
mint this capability. No secret config bytes are printed.
