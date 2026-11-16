# M1 target configuration example

Planning artifact only. These target fields are not yet supported by the current
M0 runtime. M1-A must synchronize root/module schema, parser and main YAML example
with this [config-owned field contract](../specs/sdd-config.md). No actual keys,
secret path or personal production domain is a shipped default.

## Main example (target hton.example.yaml)

```yaml
schemaVersion: 1
instance:
  name: hton
installation:
  directory: /opt/hton
  systemd: true
  pathShim: false
core:
  edge:
    listen:
      transport: tcp
      port: 443
      addressFamilies: [ipv4]
    domains: [example.com]
    hostPolicy:
      wildcardZone: example.com
      redirectTarget: https://example.com/
      unknown: reject
  certificates:
    ca: letsencrypt
    challenge: dns-01
    provider: internetbs
    email: operator@example.com
    wildcardDomain: example.com
    credentials:
      apiKey: {secretRef: dns/internetbs/api-key}
      apiPassword: {secretRef: dns/internetbs/api-password}
  dns:
    upstreamMode: tls-forward
    upstreams:
      primary:
        addresses: [1.1.1.1, 1.0.0.1]
        port: 853
        tlsName: one.one.one.one
    upstreamOrder: [primary]
    fallback: fail-closed
  clientAccessPage:
    # Omit path to publish guest credentials at a generated secret URL on first
    # install. An explicit path: "" deliberately disables the page.
    names: {}
    refreshSeconds: 60
features:
  foreign-amnezia: {enabled: true}
  mtproto: {enabled: true}
  outline-wss: {enabled: true}
  hysteria: {enabled: true}
```

The main default example omits `core.clientAccessPage.path`, so a fresh install
generates a CSPRNG bearer path; an explicit `path: ""` opts out. The owner's supplied
instance uses base domain example.com with relative path client-access; do not copy that
bearer path into a distributed default. Example DNS upstream is an explicit
editable engineering default, not private DNS ownership: the upstream still sees
queries. [Cloudflare's official DoT setup](https://developers.cloudflare.com/1.1.1.1/setup/linux/)
documents these endpoints/TLS name. Other upstreams are configured in the same
mapping; no silent system DNS fallback. No live external query is made by examples.

## Module examples (target config/modules documents)

```yaml
schemaVersion: 1
moduleId: foreign-amnezia
settings:
  mode: provision
  container: amnezia-xray
  serverNames: [static.example.com]
  compatibility:
    legacyServerNames: []
  hostPorts:
    tcp: 5714
    udp: 5714
  provision:
    clientName: guest
  admin:
    access: generate
    user: hton-amnezia
    sshPort: 22
```

5714 is illustrative only; allocator generates/reuses an available private backend
value, not this literal on every host. Additional native clients are read from the
accepted native state, not regenerated from provision.clientName.

```yaml
schemaVersion: 1
moduleId: mtproto
settings:
  backend: mtg
  camouflageName: media.example.com
```

```yaml
schemaVersion: 1
moduleId: outline-wss
settings:
  serverName: assets.example.com
  streamPath: /assets/stream
  packetPath: /assets/session
```

These configured paths are not passwords; each Outline user's cryptographic key
remains independent. Existing random paths need a compatibility transition.

```yaml
schemaVersion: 1
moduleId: hysteria
settings:
  hostname: example.com
  listen:
    transport: udp
    port: 443
    scope: public
    addressFamilies: [ipv4]
```

The native Hysteria renderer selects standard HTTP3/file masquerade and the shared
DNS/certificate/site artifact contracts; no obfs password or guessed bandwidth
parameter is added to this default profile. Public UDP/443 has one owner only.
These examples are checked for Markdown/YAML structure and contract invariants,
not claimed to validate against the unchanged M0 schemas or deployed services.
