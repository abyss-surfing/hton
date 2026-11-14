# scripts/setup

Stable ID: cmd.setup · M0 · source

Target-only setup entry. Same typed install options and common flags. Verify
physical CWD equals configured root before privilege, prompts or effects. Load
active protected source, obtain sudo if needed, then invoke minimum Core setup.
No deploy, Git or independent config/business logic. install always calls this
entry after delivery/CD or directly when already inside target. Core owns missing
parameters, secrets, configuration, certificates and reversible native cutover.
Contract: OR-17/D111, minimum-installation and INTERFACES.
