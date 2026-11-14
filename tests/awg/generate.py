#!/usr/bin/env python3
"""Generate a throwaway server/client AmneziaWG2 pair through the hton module."""
from __future__ import annotations

import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve()
HTON = HERE.parents[2]
sys.path.insert(0, str(HTON / "python/minimum"))
sys.path.insert(0, str(HTON / "lib"))

import protocols  # noqa: E402
from storage import Store  # noqa: E402


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: generate.py OUTPUT_DIR PORT", file=sys.stderr)
        return 2
    output = Path(sys.argv[1]).resolve()
    try:
        port = int(sys.argv[2])
    except ValueError:
        port = 0
    if not 1 <= port <= 65535:
        print("ERROR: port is invalid", file=sys.stderr)
        return 2
    try:
        output.mkdir(mode=0o700, parents=True)
    except FileExistsError:
        print("ERROR: output directory already exists", file=sys.stderr)
        return 2
    work = Store(output / "work", create=True)
    try:
        native = {"inbounds": [{"settings": {"clients": [
            {"id": "11111111-1111-1111-1111-111111111111", "email": "d6"}]}}]}
        material = protocols._awg_material(work, protocols._awg_names(native))
        server = protocols.awg_server_config(material)
        client = protocols.awg_client_config(material, material["peers"][0],
            "host.docker.internal", port)
        (output / "server.conf").write_bytes(server)
        (output / "client.conf").write_bytes(client)
        (output / "meta.json").write_text(json.dumps({
            "port": port,
            "server": str(output / "server.conf"),
            "client": str(output / "client.conf"),
        }, indent=2) + "\n")
    finally:
        work.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
