# runtime/src/lib.rs

Stable ID: `runtime.library` · Phase: `R1` · Kind: source
Implementation wave: I1-C. Public independent runtime composition and typed ports.

Exports manifest/state/actions/guardian/probes plus Clock, Cancellation and typed
runtime error builders. RuntimeExecutor coordinates admitted activation, accepted
startup, durable stopped intent, commit and recovery; all effects remain behind
injected ports. Dependency closure: runtime-contract, storage, logging, serde/JSON, sha2.
No Core or management DTO dependency. Product tests follow I4.

I3 exports concrete host composition independently of Core. Native execution still
passes through admission, exact state epochs and fixed drivers. Explicit stopped
startup is prequalified against prospective state before publishing StartIntent;
boot does not mutate stopped intent. Read-only Observe uses transient logs.
`ExecutorAction::RollbackAccepted` delegates to the guardian's explicit committed
fallback restore and returns partial/recovery-required status rather than a synthetic
success when the restore does not finish.
