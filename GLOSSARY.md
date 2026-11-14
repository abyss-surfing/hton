# Glossary

This file defines project vocabulary only; implementation contracts belong in the area SDDs.

| Term | Definition |
|---|---|
| HTON Core | The required project component used directly by console commands to interpret configuration and perform HTON operations. |
| Management API | An optional, self-sufficient service exposing HTON operations to clients such as a future web administration module. |
| Module | An optional, independently configured capability with a defined contract, validation and lifecycle. |
| Installed composition | The set of modules retained as part of the HTON installation. `add` and `remove` change this set. |
| Enabled module | An installed module selected to participate when the saved HTON composition starts. `enable` and `disable` change this selection. |
| External VPN component | A VPN engine or client managed outside HTON, such as an existing Amnezia installation or separately configured uplink client. |
| Uplink | An external VPN connection used by an HTON server to reach another server or region. HTON compatibility does not imply ownership of the external client's configuration. |
| Genuine domain certificate | A valid HTTPS certificate issued for a domain controlled/configured for the HTON deployment. |
| Candidate Docker check | Optional isolated qualification of a proposed configuration using Docker; it is not a mandatory gate for every apply. |
