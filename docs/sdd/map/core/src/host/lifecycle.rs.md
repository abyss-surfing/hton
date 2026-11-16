# core/src/host/lifecycle.rs

Stable ID host.lifecycle · R1 · I3-A

Config-independent host bridge for whole-server up/down and explicit accepted
startup, using the Core lifecycle owner execute_runtime_request and independent
runtime typed IPC. `lifecycle.up` and `runtime.start` share the accepted-start path:
they start the saved accepted composition and never compile desired YAML or pull
images. Candidate changes are prepared and activated through config apply.
It reads protected registry/runtime only, never desired YAML, maps verified results
with per-member metadata/survivors and leaves runtime epoch/autostart changes to
independent runtime. Dry-run produces the same pure lifecycle plan without writes.

`runtime.rollback [BUNDLE_ID]` uses the same journaled request path and accepts only
the exact previous-compatible ID in runtime state and the current committed lease.
The installed target digest and inverse actions remain runtime-owned; Core supplies
only that checked ID. A read-only invocation returns the pure lifecycle preview.

`services.restart [SERVICE_ID...] [--wait N]` enters the same Core lifecycle plan
and durable intent path. Core resolves selectors from the saved enabled service
closure, exact installed Owned resources and current accepted receipts; shared Xray
is selected whenever an enabled consumer requires it, and core ingress is excluded.
Bundle ID, planned epoch, selected IDs and wait bound are journaled and carried into
the typed Runtime action. `--wait` is validated within 1..300 (default 10) and narrows
the Runtime deadline. Post-restart process/launch checks and selected-service probes
determine completion; unsupported probes yield journaled Partial plus persisted
Runtime receipts/survivors.
