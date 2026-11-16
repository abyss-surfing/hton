# host-support/src/validation.rs

Stable ID: `infra.validation-docker` · R1 · I3-B

Bounded Docker sandbox driver, used only by explicit `bundle.validate`.
`ValidationDocker` accepts a digest-derived operation scope, pinned images, fixed
engine enum and fixed HTTPS site scenarios. It never accepts caller commands,
Compose, host port publications, foreign paths or privileged capabilities.

Resources have deterministic names and exact operation labels. A durable Core
intent precedes creation; returned immutable IDs are journaled before start.
Recovery enumerates the exact operation label, then independently verifies each
resource label/name before removal. Containers are removed before the internal
network. An uncertain create/start/cleanup cannot yield a successful attestation.

Native parsers run with network none. Servers and clients use one internal bridge
without public binds. Copies of exact hash-verified final artifacts are mounted
read-only from `/opt/hton/work/validation/<scope>/<service>`; deterministic private
address overrides are separately bound into the scenario digest. Fixed pinned
ACME-image `/usr/bin/curl` clients verify trusted HTTPS and exact body through Caddy and
HAProxy. Absence of a required client utility is a failed scenario, never a skip.
Pinned Xray provides the VLESS WS/XHTTP client through a fixed SOCKS listener;
curl requests an exact private HTTP fixture through that client. Fixed dig clients
exercise Unbound UDP/TCP with a private local zone. Native parser success cannot
replace these actual client round trips. Unsupported protocols fail explicitly.

Dependencies: shared bounded Runner, Docker image/container inspector. Product
checks run only when explicitly invoked; implementing this module runs no tests.
