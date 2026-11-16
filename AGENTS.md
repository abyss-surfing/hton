# Project operating rules

- This project lives at `hton`; root launchers are
  `scripts/hton/<name>` and only exec `hton/scripts/<name>`.
- User scripts are Linux/macOS shell only. Do not add PowerShell entry points
  to this project.
- Documentation: English by default; localized docs only under
  `docs/_lang/ru/` and `docs/_lang/cn/`. Every root launcher has a page in
  `docs/scripts/<name>.md` and `docs/_lang/ru/scripts/<name>.md`.
- README files live only at the project root or under `docs/`; the secret
  repository `hton/.secrets/` is the only exception.
- Anonymity: never put real names, emails, domains, IPs, keys or server
  identifiers in content, file/dir names, Git metadata, commit messages or
  remotes. Use `example.com` and placeholders.
- Real server credentials belong only in `hton/.secrets/`; never
  commit them. Git history must not carry author identifiers.
- Git/push through `scripts/hetaira/*`, deploy through `scripts/hephaestus/deploy`;
  never `gh`, never raw git
  against the source tree.


Treat every substantive, detailed owner statement as a binding architectural requirement, including statements made while answering questions. Preserve its scope and rationale in docs/OWNER-REQUIREMENTS.md and trace it from docs/DECISIONS.md. Brief assent such as “yes to everything” or “at your discretion” accepts the stated proposal; it does not create additional unstated requirements.

You may challenge a requirement, explain a contradiction or propose a replacement. Change the decision only after the owner explicitly accepts that replacement. Keep owner requirements distinct from agent recommendations and unresolved questions.

When designing, reviewing or implementing installation, management, routing, validation or release scope, read docs/OWNER-REQUIREMENTS.md first. Its current owner clarifications override older derived specifications, schemas and file maps. Reconcile affected artifacts before claiming implementation readiness; report remaining inconsistencies explicitly.

# Architecture before implementation

Current phase: the owner-authorized M1 slices are in implementation (2026-11-07).
Update Markdown architecture, structures and contracts alongside them; full-R1
implementation still requires an explicit owner phase change. Prepare the future
multi-agent run from docs/IMPLEMENTATION-ORCHESTRATOR.md; preparing that document
does not authorize launching implementation agents. Deployment to an external
server, service startup and image publication remain separately unauthorized.

Before assigning or implementing a file, read its area SDD and mirrored contract
in docs/sdd/map/, and the shared interfaces in docs/INTERFACES.md. Resolve affected
contradictions in the plan first; synchronize architecture, methods/arguments,
file ownership and graph. Record reasonable decisions in the owning Markdown
source, keeping engineering choices distinct from owner requirements.

Design production decomposition using SDD, SOLID, GRASP and DRY. Give each module
one coherent responsibility, explicit dependencies, bounded authority, typed
results/errors and declared state/effects. Put shared rules in one owner; callers
use that owner's interface. Prefer resolving routine design choices independently.
Escalate only consequential unresolved owner choices with context, options and a
recommended answer; preserve already accepted requirements.

Complete the planned product implementation before executing testing/acceptance.
The concluding order is unit tests, then functional tests; decide further checks
after those results. During planning describe contracts/scenarios and check document
consistency. Runtime configuration validation remains required product behavior.
Do not substitute early test gates for the owner's development sequence.

Aim for one coherent implementation-and-acceptance pass by settling interfaces and
file ownership in the plan. Correct defects found at final acceptance through the
owning module and its contract; preserve honest status and avoid compatibility
patches that introduce duplicate business logic.
