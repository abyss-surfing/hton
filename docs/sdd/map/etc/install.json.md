# etc/install.json

Stable ID: config.installation-bootstrap · M0 · configuration

Non-secret source inputs: schemaVersion=1, installation.directory, optional
core.edge.domains and core.certificates.email. Public defaults contain only the default /opt/hton directory; no personal domain
or email is shipped. Instance values remain in protected installed configuration. load_settings reads this
finite JSON mapping; installed config/hton.yaml wins. Missing site values are
prompted and persisted. Source deployment copies this bootstrap input, never live
secrets or mutable canonical state. Contract: OR-16/D110 and minimum-installation.
