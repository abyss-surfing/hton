# core/src/host/effects.rs

Stable ID: `host.effects` · Phase: R1 · Wave: I3/I4

Concrete Core preparation/apply composition. `prepare(app,ctx)` creates a read-only
EffectPlan; `prepare_identity(app,binding,include,ctx)` renders a prospective exact
credential grant selection without editing inventory. `stage` persists only through
JournalStore, and `apply/apply_staged` use existing ApplyCoordinator. Pure planning,
rendering, native validation, journal and registry policy each retain their owner.

`prepare_publication(app,requestedPort,ctx)` returns the ordinary exact EffectPlan
and single resolved public HAProxy TCP port; optional port must match saved desired
state. All preparation uses the shared `observations.listener_conflicts` policy and
complete fresh host TCP/UDP reservations before public consent writes. Unknown
holders block without foreign mutation. Existing apply/compensation retains sole
activation authority.

Assemble coherent config/catalogue/registry/runtime/identity revisions, scoped
secrets, verified cert metadata, the exact bounded read-only `SiteIndex` body/digest,
bounded fresh native facts, protected local approval policy and deterministic private
network. `with_render` lends only the captured body/digest through `RenderInputs`;
it performs no source writes or network requests. Preparation never performs public DNS-A
or public IPv4 probes; only explicit `identity.whoami` owns those external checks.
Local HTTPS activation binds the verified native certificate to the current root
revision. Native before/after fingerprints are
computed by the shared native owner from actual cached image defaults and final
Compose launch specs, before review. No artifact-hash-as-native-fingerprint shortcut.
Qualified native validators must actually run against exact staged artifacts/images;
only then publish_qualified and independent RuntimeAuthority. Staging remains explicit.

EffectPlan.secret_versions is the prospective credential grant mask for R1 VLESS;
full state.identities revision is independently bound, unknown/duplicate refs rejected,
and configured path/cert refs cannot be suppressed. Identity revoke removes exact
binding in preview and raises candidate fence; successful native acceptance precedes
AdvanceFence, so fallback cannot retain a now-admissible revoked credential.

Apply refresh reproduces the same read set/actions/grants under fixed original expiry,
then shared coordinator journals, compiles/qualifies, activates independently, reconciles
actual owned recreated locators through RegistryStore.RebindOwned and stores durable
result. Unknown/interrupted effects remain recovery-required; no blind replay. A
stopped server gets a pending bundle and no native activation. No compilation or
product execution during source implementation (D81).
Preparation and recovery treat `RollingBack` as an active runtime lease and refuse
candidate work until the independent rollback reaches a terminal state.

## Prospective membership preparation

`prepare_selection(app, opaque SelectionCandidate, context)` rechecks the candidate's
exact root baseline and every non-root config dependency against current protected
configuration. It then feeds its prospective DesiredReadResult into the same assemble,
module/global validation, native rendering and calculate owners as ordinary prepare.
Returns the exact effect, validation report and target graph; it never writes the
selection. After the selection CAS, normal apply must reproduce the same effect.
