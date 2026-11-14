#!/usr/bin/env python3
"""Read non-secret installation inputs and resolve the protected deployment root."""
from __future__ import annotations
import json
import math
from pathlib import Path
import re
import sys

class ConfigError(Exception):
    """A redacted bootstrap configuration error."""


def load_settings(source: Path):
    path = source / "platform/etc/install.json"
    try:
        with path.open("rb") as stream:
            data = stream.read(65537)
    except FileNotFoundError:
        return {}
    if len(data) > 65536:
        raise ConfigError("Installation configuration exceeds 64 KiB")
    def unique(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("Duplicate key")
            result[key] = value
        return result
    def invalid(value):
        raise ValueError("Non-finite number")
    def finite(value):
        parsed = float(value)
        if not math.isfinite(parsed):
            raise ValueError("Non-finite number")
        return parsed
    try:
        value = json.loads(data, object_pairs_hook=unique, parse_constant=invalid, parse_float=finite)
    except (ValueError, UnicodeError) as error:
        raise ConfigError("Invalid installation JSON; duplicate keys and non-finite values are forbidden") from error
    if not isinstance(value, dict) or type(value.get("schemaVersion", 1)) is not int or value.get("schemaVersion", 1) != 1:
        raise ConfigError("Installation configuration must be a version 1 mapping")
    if not isinstance(value.get("installation", {}), dict) or not isinstance(value.get("core", {}), dict):
        raise ConfigError("Invalid installation configuration sections")
    return value


def target_directory(value):
    raw = value.get("installation", {}).get("directory", "/opt/hton")
    if not isinstance(raw, str) or not re.fullmatch(r"/opt/[A-Za-z0-9][A-Za-z0-9_.-]{0,95}", raw):
        raise ConfigError("installation.directory must be an absolute direct child of /opt with a safe name")
    path = Path(raw)
    if path.resolve() != path:
        raise ConfigError("installation.directory contains a symlink")
    return path


def deployment_root(source: Path):
    return target_directory(load_settings(source.resolve()))


if __name__ == "__main__":
    try:
        if len(sys.argv) != 3 or sys.argv[1] != "root":
            raise ConfigError("usage: install_config.py root SOURCE_DIRECTORY")
        print(deployment_root(Path(sys.argv[2])))
    except (ConfigError, OSError) as error:
        message = str(error) if isinstance(error, ConfigError) else "Installation configuration is unavailable"
        print("ERROR: " + message, file=sys.stderr)
        raise SystemExit(1)
