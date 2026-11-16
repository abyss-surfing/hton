# core/src/host/runtime.rs

Stable ID core.host-runtime · R1 · I3-A

Core-to-independent-runtime bounded typed transport, exact result mapping and
journal-bound protected authority publication. Runtime actions retain independent
admission and state/fence checks. No Core callback is required at accepted boot.
Methods: execute(app,ExecutorAction,ctx)->RuntimeResult; publish_authority(app,
authority,transaction,ctx)->durable authority record; map_result -> management
OperationResult. Never invent concrete locator from a digest, mutable fingerprint
from an artifact hash or success from exit status. Authority publication binds exact
existing journal plan/bundle/epoch and protected manifest; no generic JSON writer.
`RollbackAccepted` carries only a checked previous-compatible ID; runtime resolves
its digest/proof from committed lease authority and returns stopped/partial/recovery
outcomes without Core-side resource or state mutation.
