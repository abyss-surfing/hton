# Deferred SDD identity — accounts, protocol profiles and one-time credentials

## Phase and authority

Owner request, 2026-10-31: unified creation with protocol selection, configured
protocol defaults (currently Amnezia), one-time secret display, personal accounts,
admin visibility and login -> protocols -> named tokens. Explicitly POST-RELEASE.
This is phase 2, not an additional release gate or a claim of implementation.
Release admin/bootstrap/module toggles stay in sdd-config.md. The phase-2 model
supersedes reusable share/keys --show behavior for managed user credentials only
when this migration is delivered; it must not silently break existing clients.

## Data and permissions

```text
User: immutable userId, editable display name, unique normalized login, role
  -> ProtocolProfile: immutable profileId, protocol/transport, allowed capabilities
      -> Credential: immutable credentialId, editable name, status, createdAt,
         optional expiresAt, backend binding, issuanceBatchId, delivery state
```

- Account login/password is separate from network-access tokens. Passwords use
  hashes, sessions use scoped expiring cookies; no public registration by default.
- Each user sees only their own profile/credential metadata and may create/delete
  allowed credentials. Admin sees every user, profile and token's metadata and
  can revoke or disable them. Neither role can reveal an already delivered secret.
- Authorization checks ownership for every list, mutation and issuance endpoint;
  hiding rows in React is not authorization. User actions cannot change global
  feature config, protocol grants, listeners or other users' credentials.
- ProtocolProfile identifies an actual configured server/transport/adapter, not
  just a brand name. Amnezia is the initial configured default preset, resolved to
  a detected supported Amnezia profile; do not assume it means any protocol or
  create an AmneziaWG exception. Unavailable defaults are visibly blocked, never
  silently substituted or auto-installed from a user's credential wizard.
- Required hierarchy is login -> protocols -> named tokens. Multi-protocol creation
  emits one independently revocable credential per selected profile, grouped by
  an issuance batch. Reuse the entered name across the group if desired; never
  share one bearer secret across unrelated protocols or users.
- Default token name is the creation date in configured display timezone. Store
  timestamp in UTC and immutable ID separately; same-date duplicate display names
  are allowed and disambiguated by ID/time. Rename changes metadata, not the secret.
- Protocol grant/default changes affect future issuance. They do not silently
  migrate or revoke existing credentials; use an explicit managed change plan.

## Unified creation and one-time display

1. Admin chooses a user, or a signed-in user opens their own account.
2. Select permitted protocol profiles, preselected from configured defaults.
   Optional name defaults to today's date and remains editable later.
3. Preview selected protocols and create one batch through shared apply. Backend
   configuration/credential activation must succeed before exposing material.
4. Return each newly generated secret/client config/QR in the creation response
   only. Allow copying/downloading during that creation view; no separate reusable
   credential-retrieval URL. Clearly ask the user to save it before closing.
5. After closing, navigation, refresh, logout or a later session, show metadata
   only. Lost credential -> delete/revoke it and create a new one. No reveal button,
   reusable share link, historical QR, privileged admin reveal or support backdoor.

Mark the issuance response consumed durably before sending secret bytes. A lost
response, crash or disconnect does not authorize another disclosure. Retrying the
same requestId returns IDs/status only, never the previous secret; then revoke
and recreate when needed. Do not persist response bodies in idempotency tables,
queues, browser storage, service-worker caches, session replay or observability.
Return Cache-Control: no-store, redact request/response logging and discard UI
in-memory material on exit. No secret-bearing history route or notifications.
The system cannot erase a user's saved file, clipboard or screenshot; the promise
is no subsequent disclosure through managed interfaces.

The logical token is immutable secret material after issuance. Changing secret
means revoke old + create new ID; renewal/rotate actions follow that same one-time
creation flow rather than redisplaying a stored value. Rename is independent.
Imported credentials are marked already delivered and cannot be revealed.

## IPv4/IPv6 exit selector — far future

Far-future creation adds egressFamily: ipv4 | ipv6, seeded from
accounts.defaults.egressFamily (initial ipv4). It selects payload Internet egress
on the VPS, not client-to-VPS transport or dedicated public IP assignment. A client
with only IPv4 can select IPv6 when the selected engine has verified IPv6 egress
and per-credential enforcement. Unsupported adapters, including foreign ones without the required evidence, block selection;
no global outbound switch, wrong-family fallback or automatic module enablement.
Strict IPv6 cannot reach IPv4-only destinations by pretending NAT64 solves the
reverse problem. Family is immutable token policy; change through revoke/create.
Smart failover preserves it. See sdd-client-ipv6.md for TCP/UDP/DNS/literal tests.

## Backend requirements and honest limits

Store a verifier/hash when the protocol can authenticate that way. Where runtime
configuration requires the actual secret/UUID/private material, retain protected
backend-only secret references and generated service copies. Exclude these from
all admin/user read/export APIs, including raw config download, logs and database
views. Do not claim the infrastructure root operator cannot read necessary runtime
secrets. Backups still need encrypted backend material for existing clients.

The one-time rule covers complete reusable client configs and QR images as well
as token strings: hiding the password but allowing a later vless:// export breaks
it. Legacy release share/keys --show need an explicit phase-2 compatibility change;
there must be no web/TG/CLI path that silently bypasses the new managed-credential
policy. Service-owned ACME secrets and login password reset are separate workflows,
not recoverable user-token disclosure APIs.

Adapters must support multiple independently revocable credentials before being
listed as per-user profiles. Do not relabel one shared Naive password or one global
MTProto secret as independent users. Unsupported backend versions are marked
unsupported until an adapter or isolated instance provides real separation;
measure instance cost against the weak-VPS budget.

## Future configuration and migration

Phase 2 extends the schema through a versioned migration with an accounts section:
accounts.defaults.protocolProfiles selects the configured Amnezia preset initially; profile grants
are explicit; accounts.registration is closed. UI offers only enabled, granted and
qualified profiles. Defaults never bypass policy blocks or install optional modules.
This is not an unimplemented new key silently added to the release YAML example.

Migrate release owner to admin; preserve active legacy network credentials under
an explicit owner/import profile, without treating them as newly revealable.
Map users.json/device records to stable IDs, ownership and credential metadata;
retain secret references solely for backend rendering. Journal migration and
recovery; keep existing clients working until explicit revoke. No migration opens
self-registration, enables a new protocol or exposes credentials again.

## Deferred work and acceptance

- User accounts/login, authorization and personal dashboards; admin overview.
- Profile grants and configurable Amnezia default resolution; date-based naming.
- Multi-protocol batch issuance, per-profile revocation and backend capability checks.
- Single-disclosure response semantics across HTTP, CLI, Telegram, QR, exports,
  retries, crashes, logs and browser caches. No implementation before release.
- Cross-user ID tampering denied; admin sees metadata but cannot reveal secrets.
- Refresh/back/close/lost response cannot redisclose; revoke/recreate works.
- Rename keeps connectivity; revoke rejects the old credential after verified apply.
- Partial batch activation failure rolls back/invalidates unpublished credentials
  and never reveals a partly failed set as successful.
- Clean restore preserves ownership, active credentials and consumed delivery state.
- Rollback/failover never resurrects revoked tokens or downgrades egressFamily; active and fallback bundles honor the current policy fence (sdd-runtime-safety.md).

See [ROADMAP phase 2](../ROADMAP.md), [core config](sdd-config.md) and
[controller](sdd-control.md). This file owns phase-2 identity policy.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

## M1 client-key page exception — OR-45

The current passwordless shared client-page requirement deliberately precedes
R2 personal accounts/one-time delivery. Its typed current client inventory may
re-display active client credentials at the explicitly configured bearer URL;
admin credentials remain forbidden. This does not implement a user account,
admin console, R2 sessions or one-time token policy. Future migration must preserve
existing clients and explicitly settle page compatibility rather than silently
remove owner-approved delivery. See sdd-client-access-page.
