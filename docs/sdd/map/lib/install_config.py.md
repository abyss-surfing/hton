# lib/install_config.py

Stable ID: shell.install-config · M0 · source

Owns finite non-secret source bootstrap JSON, duplicate rejection and /opt target
normalization. load_settings(source) returns a mapping; deployment_root(source)
returns the configured root or protected snapshot ancestor; errors expose no
config values. CLI root/target commands emit only a validated absolute directory.
Called by scripts/deploy, lib/hton and minimum Core. No writes, sudo, subprocess,
network or config execution. Contract: sdd-minimum-installation, OR-16/D110.
