# SDD whoami — explicit host/DNS identity introspection

## Purpose

One explicit read-only command for owner-facing server identity evidence. It may
make bounded IPv4/DNS-A requests because the owner invoked `identity.whoami`;
other diagnostics, startup and background work do not make public network requests.
Evidence the current adapter cannot collect remains explicitly unverified.

## Requirements

- WHM-1 Output (default human report, `--json` structured): external IPv4 and
  configured-domain A records when available. IPv6, reverse DNS (PTR), WHOIS,
  other DNS record types, local interface/listener details and external port
  reachability remain explicitly unverified until their bounded adapters exist.
- WHM-2 Read domains only from the coherent normalized root configuration; cap
  configured-domain observations at 8 and expose only public DNS metadata.
- WHM-3 Read-only: no writes to config/secrets/journal; network probes occur only
  in the explicit `identity.whoami` operation; no systemctl/docker mutations.
- WHM-4 Privacy: deliberate identity output contains no credentials, private
  configuration, native resource locators or backend process output. Structured
  issue output uses closed error codes.
- WHM-5 Missing external evidence/tools is reported as unverified; no fake local
  fallback is presented as public identity evidence.

## R1 binding and limits

- `scripts/whoami` dispatches to Core `identity.whoami`. The Core observation port
  uses fixed HTTPS services for external IPv4 and configured-domain A records,
  with deadline and response-byte caps.
- `--json` returns structured redacted metadata; default output is a small human
  report. The operation does not expose credentials, native locators or backend
  output.
- Current unprobed fields are IPv6, PTR, WHOIS, AAAA/NS/MX/TXT/DS, NIC addresses,
  local listeners and external port reachability. They remain labelled
  `unprobed`; the R1 adapter does not claim exhaustive coverage.
- `diag.observe` is local-only. Doctor integration may consume the structured
  report later, without adding probes or mutation to this operation.

## Acceptance

- Explicit CLI invocation may perform only the bounded IPv4 and configured-domain
  A observations above. Missing/failing probes remain unverified. Both output
  forms omit secrets and leave config, secrets and journal unchanged.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).
