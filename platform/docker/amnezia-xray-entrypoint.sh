#!/bin/sh
# Fixed entrypoint: the managed container runs only the pinned Xray binary.
set -eu
exec /usr/local/bin/xray -config /opt/amnezia/xray/server.json
