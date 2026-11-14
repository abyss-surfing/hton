# SDD configurator — deferred standalone configuration maintenance

Status: future module/interface design, not R1 implementation. Owner Q21–Q25,
Q30–Q34 govern preservation and atomic edits. Engineering protocol details below
remain design choices; no change to owner decisions is implied.

## Interface and independence

The module runs without web administration. CLI can invoke it directly; the future
management API exposes its typed operations with tokens. `read/diagnose` must work
when managed YAML is malformed or its format unsupported; target configuration is
input data, never a prerequisite for the configurator's own startup/authentication.
It reuses Core parsing/validation and runtime apply, with no second renderer.

`prepareChanges(objectIds, changes, preconditions)` creates a protected durable draft.
A change addresses a known object and stable nested key and declares set/remove;
absence differs from null. Baselines include touched values and semantic dependencies.
Lists are replaced/edited only using the owning module's explicit list strategy.
No arbitrary filesystem paths, whole-file replacement endpoint or executable hooks.

`commitChanges(draftId)` rereads current documents, preserves independent edits,
compares touched values and operation conditions, validates the combined candidate,
then saves the whole batch or none. Same-value updates may be idempotent. A semantic
conflict returns paths/codes with secret values redacted and retains the draft for
retry. A mere file revision change is not itself a semantic conflict. Replanning
against the merged snapshot is required before runtime effects. Save and runtime
activation have separate results: saving desired state is not proof of activation.

## Atomic save and external writers

Proposed storage protocol: stage all changed files and protected originals, fsync
transaction intent, recheck current inputs, publish a manifest-marked generation of
the config set under the writer lock, then finish/recover before serving another
managed read. Readers use the manifest/coherent snapshot protocol, not independent
uncoordinated file reads. No claim of filesystem-wide atomic rename is made.

A direct editor does not necessarily honor the lock. Before implementation, choose
and prove editor coordination (for example an explicit edit lease) or an equivalent
conditional publication mechanism. Hash rechecking alone leaves a check/rename race
and cannot promise that arbitrary writes are never lost. Until proven, concurrent
manual editing during publication is an explicit unresolved guarantee; an API
implementation cannot advertise Q34 safety merely because it uses flock. Interrupted
multi-file commit recovers originals or completes its whole set before activation.

## Migration, deprecated values and upgrade

Read platform/module format versions and execute declared sequential transformations
for every skipped format. No format change means no required migrator; missing or
empty scripts alone are not a failure. A required unsupported conversion blocks
activation with preserved source, rather than guessing a rename/reset strategy.

Each transformation declares source/target paths, handling of null/absence, defaults,
rename/reset/removal and preserved original. Move unused/unsupported values to the
corresponding `*.deprecated.yml` with original path, source version and transaction
provenance. Repeated moves preserve earlier values without overwriting collisions.
Treat originals as possible secrets; archive them, redact diagnostics, never execute
them. Preserve comments where feasible, values always. Editable Docker/native
customization participates in migrations and compatibility checks; fresh templates
are not an update strategy.

`upgrade` later coordinates a protected code update (`git pull` in owner terms),
configuration maintenance and each module's own update action. Code/image/format
versions and data migrations have distinct recovery requirements. Refuse dirty or
incompatible source updates; record prior code revision, config originals and pinned
images. Do not promise rollback across irreversible backend data changes. Full
upgrade implementation is deferred and is not an implicit R1 requirement.

## Acceptance and unresolved decisions

Independent concurrent edits merge; touched-value/dependency conflicts retain draft;
a multi-file batch saves all or none through restart recovery; manual-write race
is demonstrated under the chosen coordination contract; unsupported values/secrets
survive chained migration, rename/reset/removal and repeated retries. Invalid managed
YAML never prevents diagnostics/repair. Restore does not start services.

Still to decide before implementation: exact manual-editor coordination, crash-safe
publication format, public token scopes/revocation, repair of syntactically malformed
text through typed operations, and platform/module upgrade compatibility policy.

## Development sequence

This SDD plans product behavior and final evidence. Under the latest owner
instruction, complete the selected implementation before executing product tests:
unit tests first, functional tests next, further acceptance determined afterward.
Current planning checks document consistency only. Runtime input validation remains
required product behavior. See [orchestrator](../IMPLEMENTATION-ORCHESTRATOR.md).

## Supported repair path

If syntax is too malformed for typed parameter operations, Diagnose still returns
object identity and redacted syntax location. A local CLI repair session can open
a protected copy under an explicit edit lease, validate the repaired candidate and
commit through the supported coordination protocol. API parameter edits do not
turn into raw whole-file uploads to work around this error. Until the later repair
session is implemented, manual local repair remains available. This preserves tool
availability without promising arbitrary automatic reconstruction of broken text.

Standalone CLI diagnostics use OS-authorized ToolContext, not target settings.
A future resident API/configurator starts from its accepted bootstrap artifact;
it does not parse the currently edited managed tree as a startup prerequisite.
Invalid edits to that tree therefore remain diagnosable. No valid accepted first
configuration means local CLI guidance, not an unauthenticated public fallback.
