#!/bin/sh
# Local Tartarus check: install the hton-generated profile through D6ton.
set -eu

ROOT="${D6TON_ROOT:-/opt/d6ton}"
SOURCE="${AWG_CLIENT_CONF:-/run/hton/client.conf}"
TARGET="$ROOT/vpn/hton.conf"

mkdir -p "$ROOT/vpn" "$ROOT/.work/tmp" /etc/amnezia/amneziawg
cp "$SOURCE" "$TARGET"
chmod 600 "$TARGET"

cat >"$ROOT/.env" <<EOF
PYTHON=global
VPN_BACKEND=amneziawg
VPN_PROTOCOLS=awg2
EOF
export PYTHON=global
export VPN_BACKEND=amneziawg
export VPN_PROTOCOLS=awg2

cd "$ROOT"
/opt/d6ton/backend/amneziawg/linux/up hton
touch /tmp/awg-ready
exec tail -f /dev/null
