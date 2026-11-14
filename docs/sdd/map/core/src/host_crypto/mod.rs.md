# core/src/host_crypto/mod.rs

R1 I3-B. Exports concrete archive/certificate/identity/inventory/restore/application
owners. No second planner, renderer, runtime state writer, subprocess mechanism or
plain secret metadata cache. Composition callbacks go through existing Core effect
owner; cryptographic tools use hton-host fixed bounded private-byte capabilities.
