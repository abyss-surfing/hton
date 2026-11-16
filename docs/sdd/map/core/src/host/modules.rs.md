# core/src/host/modules.rs

Stable ID: `host.modules` · R1 · I3-A/I4

Narrow installed module membership composition. It delegates policy, selection,
immutable effects and registry transitions to their existing owners. `remove`
prepares an exact private disabled selection and validates/render-plans its
prospective composition before committing any desired state. Only an Owned module
with no physical resources can use the resource-free stop path; this is the R1
VLESS feature whose engines belong to Core. Any nonempty or Associated resource
set requires a separately qualified exact runtime stop adapter and fails before
writes. The feature's configuration, secrets and recovery records are retained.

`operation(app, operation, args, context)` accepts one catalogue module ID.
Dry-run is read-only. `update` checks the module descriptor's update capability and
ownership; R1 VLESS and Amnezia descriptors have no module-owned updater, so return
`module.unsupported` rather than touching shared engine images. Purge requires an
exact approved retained-artifact deletion owner: the supported R1 target is the
removed Owned VLESS config described below. Other deletion sets are unsupported,
never an empty successful purge.

`effects.prepare_selection` accepts the opaque SelectionCandidate, rechecks its
root baseline and non-root read set, and invokes the same normalization, native
validation, rendering and effect planner as ordinary apply. `lifecycle` owns the
journalled resource-free removal sequence and terminal result. No shell, direct
Docker calls, StopAll fallback or duplicate lifecycle decisions exist here.

Unfinished removal is a recovery blocker. Desired selection already committed by
an interrupted apply remains saved; ownership is not erased without confirmed
successful effects. Final unit/functional scenarios follow I4 under D81.

## Exact VLESS retained-config purge

`modules.purge vless` supports only a removed Owned, resource-free VLESS entry
whose complete retained set is `config.vless.v1`. Shared engines and traffic
credentials remain owned by their independent owners. The pure lifecycle owner
checks disabled participation, no dependent/retained/recovery references and exact
module state. A local root TTY confirmation binds the installation, actor,
registry entry digest, config content revision and original registry revision.
`--yes` cannot manufacture this approval. Dry-run displays logical IDs/revisions
without reading user consent or making writes.

A typed ModulePurge intent durably retains protected config originals before a
descriptor-anchored exact-byte deletion. Registry purge follows deletion. Resume
checks the original revision/digest and exact removed registry entry, rejects
re-add/drift/foreign ownership, and reconciles only exact-original or absent state.
No raw path, recursive removal, secret deletion or image removal is accepted.

Depends on `HostApplication`, the lifecycle policy owner, private SelectionCandidate,
existing immutable effect owner, registry reference inventory, bounded root TTY
approval and descriptor-anchored config storage. `operation` is the fixed dispatch
adapter for the planned remove/purge/update operations; it does not expose raw paths
or native commands. Typed ModulePurge intent and OperationResult retain completion
and recovery state. Native-resource/Associated removal requires the still-unbound
exact module-stop port; null module update capabilities remain unsupported.
