# core/src/host_crypto/identities.rs

R1 I3-B FileIdentityInventory owns immutable logical metadata/CAS state in protected
work/identities. bindings/revision expose only scoped SecretRefs, no material.
Pending replacement retains previous accepted binding for export until fresh current
accepted native proof authorizes the new binding. No engine file is edited here.

Fresh accepted verification invokes independent runtime Observe, then checks the
hash-bound accepted Xray artifact and scoped manifest secret version for exact UUID.
Revocation additionally checks current and previous admissibility/fence exclusion.
Export reads only current accepted endpoint/transport from accepted bundle metadata,
not edited desired settings. Native client qualification is exact, no fallback to
unsupported client/transport. Plan callbacks use host effects owner; no duplicated
planner, secret file writer, or plaintext response cache exists.
