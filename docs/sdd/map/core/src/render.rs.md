# core/src/render.rs

Stable ID: `core.render` · Phase: `R1` · Kind: source

Planned file binding; no implemented functionality is claimed. Purpose: Compile immutable scoped native bundles

Implementation wave: `I2-A` (planned ownership, not completion).

## Features and responsibility

- compiler

## Methods, arguments and result

- `compile` — arguments: NormalizedSnapshot, DependencyGraph, secretVersions, validatedDockerInputs; result: CandidateBundle in protected staging.

## Structures and injected dependencies

CandidateBundle/NativeArtifacts; deterministic serializers

Dependencies: trusted engine render ports, versioned templates, scoped secret grants, protected staging writer.

## Logic

- Render catalogue settings plus validated editable Docker/native inputs; preserve source customization.
- No writes to active service paths.
- Generate final Compose/image/config manifest for independent testing.
- Consume the exact normalized graph/version bound by the plan; any re-resolution must match its fingerprint.
- Compile shared engines once with all enabled consumers; preserve mandatory Caddy/default-site consumers. No implicit native defaults for declared settings.
- Accepted configs/Compose are immutable outputs; never mount directly editable config/docker into active services.
- Hash customization into admission proof; reject undeclared listeners/mounts/privileges before effect.

## Calls / reads

- [images.lock.json](../../images.lock.json.md) — `compile` -> declarative dependency.
- [compose/dataplane.yaml](../../compose/dataplane.yaml.md) — `compile` -> declarative dependency.
- [etc/haproxy/haproxy.cfg.template](../../etc/haproxy/haproxy.cfg.template.md) — `compile` -> declarative dependency.
- [etc/caddy/Caddyfile.template](../../etc/caddy/Caddyfile.template.md) — `compile` -> declarative dependency.
- [etc/xray/config.json.template](../../etc/xray/config.json.template.md) — `compile` -> declarative dependency.
- [services/onomat/runtime/dns/etc/unbound.conf.template](../../services/onomat/runtime/dns/etc/unbound.conf.template.md) — `compile` -> declarative dependency.
- [etc/mtg/config.toml.template](../../etc/mtg/config.toml.template.md) — `compile` -> declarative dependency.
- [etc/hysteria/config.yaml.template](../../etc/hysteria/config.yaml.template.md) — `compile` -> declarative dependency.
- [adapters/engines/xray.rs](../../adapters/engines/xray.rs.md) — `compile` -> `render`.
- [adapters/engines/caddy.rs](../../adapters/engines/caddy.rs.md) — `compile` -> `render`.
- [adapters/engines/haproxy.rs](../../adapters/engines/haproxy.rs.md) — `compile` -> `render`.
- [services/onomat/runtime/dns/rust/unbound.rs](../../services/onomat/runtime/dns/rust/unbound.rs.md) — `preview` -> `render`.
- [runtime-contract/src/lib.rs](../../runtime-contract/src/lib.rs.md) — `compile` -> `candidate_digest`.

## Called / read by

- [core/src/apply.rs](apply.rs.md) — `apply` -> `compile`.

## Errors and completion

- Use structured redacted errors and explicit state/effects from [shared interfaces](../../../../INTERFACES.md).
- Keep this file within its phase/assigned ownership; implementation and testing run in separate final waves.
- Contract existence is not implementation, compatibility evidence or production acceptance.

Language and extension are replaceable bindings; preserve logical ID and interface.

## I2 concrete binding (2026-10-31)

`EngineRenderer` is an injected trusted pure port: service_id, engine and
render(EngineContext)->Vec<RenderedArtifact>. Protected artifact bytes and scoped
secret handles have redacted Debug and no Serialize implementation. Pure
`preview(RenderInputs,renderers)` renders every shared engine once and returns
protected artifacts, service metadata and canonical per-service artifact digests.
`CompileInputs` additionally binds EffectPlan, journal transaction, bundle/instance
IDs and credential fence. `compile` verifies protected journal Effects intent and
plan-bound target/source/secret/image/resource versions before writing immutable
staging under the anchored bundles directory. Artifacts publish before candidate
metadata; retry may reuse exact bytes but never overwrite a different generation.

`RenderInputs` includes normalized snapshot/graph/registry, trusted catalogue,
exact `RuntimeServiceBinding` resource/image/restart-owner records, private
`NetworkLayout`, scoped resolved `ScopedSecret` handles, per-credential secret refs,
a prepared qualified own-domain certificate and a prepared DNSSEC root anchor.
Network layout uses explicit IPv4 subnet and endpoint addresses, not hidden DNS or
native defaults. Foreign route targets include exact resource/qualification/native
target observations and cannot loop back to the public edge or project listeners.
Certificate preparation/qualification and DNSSEC-anchor provisioning belong to
separate owners; renderer verifies bindings/coverage/digests, never issues material.

Docker customization is native Compose input with a closed supported service field
set (bounded mem_limit, cpus, pids_limit, labels and logging options). Unsupported
images, ports, privileges, commands, networks or mounts are rejected without source
loss. Output is complete generated immutable Compose, never a live editable mount.
Service config mounts remain service-scoped stable trees with atomic current
references. `RenderInputs.site_body` and `site_index_digest` bind the exact bounded inspected
host index bytes into Caddy's generated immutable bundle as `site-index.html`. The
Caddy route rewrites every non-VLESS request to that fixed artifact and serves only
it; no host webroot or arbitrary source directory is mounted into the container. Install creates the bundled source
placeholder only when `/var/www/index.html` is absent; the webroot owner rejects
symlinks, hardlinks, non-regular, empty and oversized inputs. Local readiness
rechecks source identity, digest and bytes around the HTTPS/SNI probe. API/admin
routes are absent.

`CompiledBundle.candidate` intentionally has no native pass evidence. Independent
native qualification attaches exact-hash evidence; `publish_qualified` permits only
that metadata extension with unchanged canonical candidate digest and writes final
manifest.json once. Runtime alone admits and activates the final manifest. All
native transition bodies and inverses come from approved EffectPlan unchanged.

I2 integration corrections: native logs use stdout/stderr and Docker `logging:
{driver: none}`; json-file/global `/var/lib/docker` log storage is forbidden here.
The I3 runtime composition must supervise an independent asynchronous attach/drain
collector for exact admitted container IDs, redact before persistence into the
installer-selected LogSink directory and drop bounded queued events under pressure
without blocking VPN. No native log bind/mount or API dependency is required.
Until that collector exists, detailed component logging is not complete.

Caddy native reload uses an unexposed container Unix admin socket, not `admin off`:
`caddy reload --config /etc/hton/current/Caddyfile --adapter caddyfile --address
unix//run/caddy-admin.sock`. This is the native engine control mechanism, not an
HTON management API. It remains independent of web/API modules.

`CandidateBundle.retiredServices` contains only exact owned targets of reviewed
Stop/DisableAutostart effects outside the desired startup closure. No disabled
settings are rendered and no retired images are pulled/validated. Inverse authority
comes from the reviewed exact transition plus reserved qualified fallback, not a
retiree silently reintroduced into accepted boot. Native config replacement/reload
must be performed by fixed runtime actions over admitted service artifact handles;
compiler does not write service current references or signal live engines.


RuntimeServiceBinding.structural_fingerprint binds the installed Core registry;
its resource.fingerprint is the independently observed mutable runtime hash.
Compiler checks both roles separately, including native first-action precondition.
QualifiedCertificate.covers delegates SAN/wildcard matching to the certificate
owner; no second wildcard implementation is introduced.


The selected Unbound image is pinned to madnuttah/unbound 1.26.1-1, whose inspected
PATH and fixed source place binaries under /usr/local/unbound/unbound.d/sbin/.
Native execution uses that absolute unbound path rather than assuming another
image's layout. Native qualification remains outstanding, and no generic image
can be substituted while retaining these fixed commands.
