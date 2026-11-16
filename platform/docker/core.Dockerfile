# Artifact export only. Context is the unpacked, checksum-verified release tree.
# Helpers are compiled externally; no compiler or management image boots the VPN.
FROM scratch AS artifacts
COPY --chmod=0755 libexec/hton-core /opt/hton/libexec/hton-core
COPY --chmod=0755 libexec/hton-runtime /opt/hton/libexec/hton-runtime
# No ENTRYPOINT: this target is exported with BuildKit --output, never started.
