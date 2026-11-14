# systemd/hton-native-log@.service

Stable ID: `systemd.native-log` · Phase: R1 · Wave: I3-B

Independent exact-container bounded native log collector template. The trusted
runtime starts hton-native-log@<full64hexID>.service before starting/recreating a
container and verifies protected acknowledgement plus systemd MainPID. Container
ID and immutable admitted launch are independently checked by the collector. No
PartOf/Requires binding to data/control; no user-provided arbitrary unit/command.
Raw payload never reaches journald/stdout: both unit standard outputs are null;
worker consumes/redacts/discards through bounded LogSink contract. Files go to the
configured projected runtime logging policy. Queue pressure drops rather than
blocking VPN. Root code read-only; protected work acknowledgements writable.
No unit was installed or started during source implementation; D81 defers build
and final U0/F0 acceptance.
