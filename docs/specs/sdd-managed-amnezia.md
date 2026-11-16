# SDD managed Amnezia — provisioning, host access and native clients

Status: implementation design frozen for M1; no product acceptance. Owner sources:
OR-36–OR-44. Engineering decisions: D143–D145. Scope and exact files are in the
[implementation plan](../plans/INSTALLATION-IMPLEMENTATION.md) and
[file graph](../sdd/INSTALLATION-FILE-GRAPH.md). Associated resources retain
[sdd-foreign-adapters](sdd-foreign-adapters.md); shared storage/config/DNS/proxy
rules have their existing single owners. No parallel Rust controller in M1.

## Modes and resources

`foreign-amnezia.settings.mode` is `associate` or `provision`. Absent mode in an
existing installed document means associate; an absent document on a demonstrably
fresh host offers provision and persists the reviewed mode. Existing same-name
containers never become managed by discovery. `--amnezia-container` selects
association and conflicts with explicit provision. No associated mode creates a
host account or private admin key. Managed resources have immutable ID/launch/image
receipts, not ownership inferred from the name `amnezia-xray`.

Provision creates the pinned Xray bridge container with private host TCP/UDP
bindings at `127.0.0.1:<saved 5000–6000>` from its first creation; native TCP/443 is
the REALITY inbound. Keep UDP mapping only for the accepted native compatibility
shape; it is private and is not a public QUIC endpoint. Do not open public native
443 just to satisfy the old association qualifier. Managed qualification is an
explicit typed mode of the existing exact observer, not weaker name-based adoption.
No mounts/volume declarations/Compose controller labels in this baseline.

One existing Caddy serves the admitted website content and REALITY-host 403. Native
`dest` is its existing private bridge-reachable TLS endpoint. A container's own
127.0.0.1 cannot stand for host loopback; the approved site binding is restricted
to the bridge/internal scope. No public-domain loop and no MTProto hop. Own names,
legacy names and exact routes are resolved by the config owner; no hostname literals.

## Pinned image and application compatibility

Source baseline is AmneziaVPN 5.0.3.0 commit
`de93650a90739b87bb47a632872ea9d0adc9412f`; its researched Xray recipe selects
`v26.7.28`. Future builder pins authenticated upstream archive checksums for each
architecture, base image digest and build recipe. Target architectures are amd64
and arm64; unavailable upstream artifacts are a precise build blocker, not a
fallback to latest. Planned registry namespace is
`ghcr.io/d6/hton-amnezia-xray`; actual publication needs
separate authorization. CI can first emit a local OCI artifact. Record the built
multi-arch digest/provenance in images.lock; never invent a digest now.

Use `/usr/bin/xray -config /opt/amnezia/xray/server.json` as the native process.
Include the bounded shell/file tools actually required for the pinned application's
SSH file read/upload path; do not blindly copy upstream privileged startup/iptables
or global tuning. Cap-drop/no-new-privileges; native container starts without
privileged/NET_ADMIN. Baseline native UID is 0 inside its unprivileged bridge container with all
capabilities dropped and no-new-privileges; this preserves application file-copy
ownership compatibility without host mounts. Do not describe it as rootless or
a host security sandbox. A later non-root profile requires actual app upload/
restart evidence rather than a speculative chmod workaround; app Docker operations
run under the separately approved host sudo identity.

Provision consistent protected regular files in `/opt/amnezia/xray/`:

| File | Contents / use |
|---|---|
| server.json | Accepted native REALITY config, current clients, DNS policy |
| xray_private.key | Same private key as native JSON; protected secret |
| xray_public.key | Derived matching public key, used by app |
| xray_short_id.key | Same short ID as JSON, used by app |
| xray_uuid.key | Initial client UUID; never substitute it for the live full client set |

Key files are produced once and reused; keep protected authoritative custody and
current native copies consistent. A mismatch blocks, rather than silently rerolls.
Build/run qualification verifies actual tools/permissions, not Dockerfile intent.
App add/revoke/export must preserve native names/dest/port/DNS and all other clients.
Native listen and exported public endpoint both use port 443; private host mapping
is never exported as the public VPN port. SSH port is separate (`admin.sshPort`).

Primary pinned sources: [Dockerfile](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/xray/Dockerfile),
[configurator](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/core/configurators/xrayConfigurator.cpp),
[key-file generation](https://github.com/amnezia-vpn/amnezia-client/blob/de93650a90739b87bb47a632872ea9d0adc9412f/client/server_scripts/xray/configure_container.sh).
Final application acceptance remains mandatory; these references prove source
expectations, not compatibility of the not-yet-built image.

## Operations and authority

| Owner / method | Inputs | Result / errors | Effects |
|---|---|---|---|
| managed_amnezia.plan | ConfigSnapshot, pins, exact resource/account observations, prior receipts | ManagedAmneziaPlan / conflict, unsupported, inputRequired | Pure; no key generation |
| managed_amnezia.prepare | Accepted plan, operation/journal, scoped secret store | NativeReceipt / partial, recoveryRequired | Generate once, stage keys/config, create private managed container via Docker owner |
| managed_amnezia.reconcile | Plan + receipt + current observed config | ManagedServiceReceipt / drift, policyConflict | Bounded lifecycle/update; coordinator owns public cutover |
| host_access.plan | Configured user/access/port, account/sshd facts, prior ownership | HostAccessPlan / accountConflict, policyBlocked | Read-only |
| host_access.reconcile | Approved plan, protected key handle, journal context | HostAccessReceipt / drift, partial | Account, authorized key and sudoers only |
| host_access.deactivate | Exact receipt + current facts | DeactivatedReceipt / drift | Remove only managed privilege/key, disable admission; preserve data |
| host_access.purge | Exact retained receipt + explicit purge consent | PurgeReceipt / recoveryRequired | Delete exact retained managed resources only |
| access_links.encode | Accepted native/profile settings, key handle, host receipt, DNS coverage | Protected artifact bytes / incompatibleExport | Pure encoding; no Docker/account effects |

Every effect is journaled before execution; returned records expose secretRefs,
public key digests and owned identities/paths, never private bytes. Typed errors
are converted once to redacted InstallError metadata. Docker transport owns argv
and process execution; host access has a separate fixed host-command boundary,
never DockerLifecycle or ManageSystemd as a substitute for account authority.

## Account contract

When `admin.access: generate`, require `admin.user` from config: ASCII lowercase
letter followed by lowercase letters/digits/_/-, total 1–32, never root/UID0.
Reject unmanaged collisions. Create a dedicated system account, protected home
under `/opt/hton/host-access/<receipt-id>/home`, valid login shell and Ed25519
key. Put it in the distro's sudo group (Debian/Ubuntu: sudo; separately qualified
platforms may use wheel) because the pinned app checks group membership. Also
use the existing hton group for traversal of the protected installation root; this
account is already an approved console/root-equivalent administrator. Do not
add docker group: sudo already supplies the approved power. Use
`/etc/sudoers.d/hton-amnezia-<receipt-id>` with the validated username and
NOPASSWD: ALL, 0440/root, validate before atomic installation. ~/.ssh is 0700,
authorized_keys 0600; private key is `config/secrets/amnezia/admin-ssh-key` root-only.

No password or root-password storage. Validate effective account/sshd/PAM admission:
blindly locking shadow can also reject public keys on some platforms. A supported
key-only profile must deny password login while admitting this key; otherwise
report policyBlocked. Do not relax global sshd/root/password settings or replace
unmanaged keys. Normal CLI hton group is not this privileged app account.

Account name/UID/home/sudoers/key digest/created flags are receipt-bound. A username
change is explicit deactivation/migration with retained originals, never a second
silent privileged account. Full access remains root-equivalent, not least privilege.
`admin.access: none` is explicit guest-only operation, with disabledByOperator and
no private admin artifact; a requested generation failure is blocked/degraded.

## Amnezia and HTON write ownership

Amnezia app may list/add/revoke clients and restart this native container. HTON
owns image, topology, SNI/dest, DNS and platform cutover; normal reinstall reads and
preserves the actual current native client array. Normal accepted app edits update
client-set evidence without pretending the full config digest is unchanged.
Unexpected topology/key/DNS changes are drift, not automatically repaired.
No automatic synchronization to old copied listeners: legacy associate mode may
retain a copied listener and must mark client-set synchronization unverified.
New provision uses one native listener so app client changes apply to that listener.

An HTON lock cannot exclude the root-equivalent app. Topology/image update requires
an explicit exclusive maintenance window (app/admin writes stopped), coherent
native snapshot and recheck before/after effects. Do not claim compare-and-swap
against an uncooperating external editor. If exclusivity or current native state
cannot be established, refuse update; leave VPN running and report conflict.
No unattended image update, recreation or SNI migration during renewal.
Changing server settings/reinstalling protocols through the Amnezia app is outside
the qualified normal-management flow and can break shared ingress; detect/report it.
Full-access credentials technically permit that action; this is not an enforced
security sandbox. Final F0 must verify profile export, add, revoke and restart;
if any changes shared topology, fix the owning adapter or report incompatibility.

## Transaction, removal and recovery

Coordinator sequence: coherent read/preflight/conflict review -> journal -> narrow
config commit -> valid wildcard cert/private shared DNS/site -> managed native
prepare/qualification -> host-access prepare if requested -> accepted ingress
cutover -> local route/auth verification -> artifact write -> terminal receipt.
No missing native fallback/DNS dependency is deferred until after public admission.
Failure keeps prior accepted VPN/site or returns degraded with durable created
receipts; do not delete already accepted access on an unrelated later retry failure.

Recreation (image, launch or `dest` change) reuses the live native REALITY key pair
and the complete current client array, falling back to protected `amnezia/server.json`
only when the live bytes are unreadable; unreadable saved material is
`recoveryRequired`, never a silent reroll. Only a definitively absent container or a
missing native file authorizes recreation; any ambiguous Docker error aborts with the
container intact.

Remove/uninstall stop/exclude managed services and deactivate managed SSH access;
retain protected keys, client config, home, data and recovery receipts. Purge is a
separate explicitly consented destructive action and refuses needed recovery/shared
or unrelated resources. Associated originals are restored only through their exact
foreign recovery contract. Managed containers have managed before-images, not a
fictional foreign original. Never prune the only current client-set snapshot, restore
revoked clients, or delete a container/account merely because its name matches.

## Final evidence after completed implementation

U0: mode/legacy classification, typed key/config/export vectors, account collision,
permission/sshd failure, ownership/drift, client preservation, remove-vs-purge,
secret redaction and crash recovery. F0: fresh VM app import, guest connection,
app add/revoke/restart with current public endpoint/DNS, private-first creation,
reinstall/update/renew/reboot/removal, legacy associate regression and rollback.
Only later A0 assesses real-network DNS/DPI/exposure results; no anonymity claim.

`managed_amnezia.client_inventory` is the pure native-client inventory port; it
uses the existing access-links codec with an explicit Guest request. Optional
protocols expose their own client_inventory port. The coordinator combines typed
client entries and applies configured display-name overrides; page rendering never
reads a mixed client/admin artifact directory. encode accepts a discriminated
Guest or Admin request so guest inventory has no access to SSH key material.

The SSH home is a distinct retained `host-access/` top-level tree, not beneath
work/config: those runtime stores are `root:hton` 0750/0640 (OR-46) and are not
the storage boundary for a per-account home. Installation root remains root:hton
0750; root-owned host-access parent is traversable (0711, no listing),
per-receipt/home is account-owned 0700.
Deploy explicitly preserves and never source-copies this new mutable tree; code
pruning cannot remove it. Private keys/config/journal remain under their existing
protected stores, not the user's home. Rerun detects home/key drift, not chmods
unrelated account files. Receipt-bound purge may remove only the managed subtree.
