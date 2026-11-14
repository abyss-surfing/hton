# core/src/host/setup.rs

Stable ID: `host.setup` · Phase: R1 · Wave: I3-B

Concrete local setup provider. `complete_core(app,ctx)` and `add_module(app,id,ctx)`
compose the existing SetupCoordinator, RegistryStore, JournalStore and scoped
configuration/storage capabilities. No parallel implementation of setup sequencing,
checkpoint/terminal or registry mutation policy is introduced.

Core recipe obtains immutable cached HAProxy/Caddy/Xray/Unbound/ACME images; only
four physical engines are created, parked stopped with no restart/listeners. Each
step is declared in SetupPlan, journaled before effects, then independently observed.
Resource IDs engine.<service> and exact locator/structural/state fingerprints are
recorded by shared registry owners. Existing installed resources are observed by
exact recorded identity, never discovered/adopted; running accepted members are not
stopped/recreated on repeated add. A crashed unrecorded create is recovered only
against a prior durable identical setup step and exact parked launch qualification.
Unknown native errors are not absence and cannot authorize replay.

vless seeds missing module YAML only; shared physical engines remain core-owned.
A newly registered module's participation is saved false through SelectionWriter,
not direct YAML edits; existing installed enabled members keep their selection.
Traffic credentials remain explicit keys/identity input before Core allows enable.
New config is protected; existing bytes are read/validated and never overwritten.

No compile/tests/product execution before D81 completion. Source existence is not
native qualification, production acceptance or evidence of actual installed VPS.

`preview_module(app,id,ctx)` returns the fixed SetupPlan through the coordinator
without selection/config/native acquisition. Core bootstrap preview still requires
its explicit local installation scope; optional-module preview grants no mutation.
