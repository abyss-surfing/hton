# Managed Amnezia Xray image, built outside the weak VPS.
# Source baseline: AmneziaVPN 5.0.3.0 commit de93650a90739b87bb47a632872ea9d0adc9412f,
# whose Xray recipe selects v26.7.28. Every digest/checksum is supplied at build
# time by the builder; this file never falls back to a floating tag.
ARG BASE_IMAGE
ARG BASE_IMAGE_DIGEST
FROM ${BASE_IMAGE}@${BASE_IMAGE_DIGEST}

ARG TARGETARCH
ARG XRAY_VERSION
ARG XRAY_SHA256_AMD64
ARG XRAY_SHA256_ARM64

RUN set -eu; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates curl unzip ssh openssl; \
    rm -rf /var/lib/apt/lists/*; \
    case "${TARGETARCH}" in \
      amd64) sha="${XRAY_SHA256_AMD64}"; arch="64" ;; \
      arm64) sha="${XRAY_SHA256_ARM64}"; arch="arm64-v8a" ;; \
      *) echo "unsupported architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    [ -n "${sha}" ] || { echo "missing pinned Xray checksum for ${TARGETARCH}" >&2; exit 1; }; \
    curl -fsSLo /tmp/xray.zip "https://github.com/XTLS/Xray-core/releases/download/v${XRAY_VERSION}/Xray-linux-${arch}.zip"; \
    echo "${sha}  /tmp/xray.zip" | sha256sum -c -; \
    unzip -q /tmp/xray.zip -d /usr/local/bin xray; \
    chmod 0755 /usr/local/bin/xray; \
    rm -f /tmp/xray.zip; \
    mkdir -p /opt/amnezia/xray

COPY amnezia-xray-entrypoint.sh /usr/local/bin/amnezia-xray-entrypoint
RUN chmod 0755 /usr/local/bin/amnezia-xray-entrypoint

USER 0:0
ENTRYPOINT ["/usr/local/bin/amnezia-xray-entrypoint"]
