# HTON first run

This is the first server installation path for HTON. It installs the minimum supported profile; the full R1 management platform is still incomplete.

1. Prepare a Linux/systemd server with Docker, Python 3 and PyYAML 6.x, curl, OpenSSL and the system CA trust bundle.
2. Configure the site's domain and contact email in `platform/etc/install.json`, or let the installer collect missing values locally. Ensure DNS points to the server and public TCP/80 and TCP/443 can reach it.
3. From the HTON checkout, run `scripts/install`. The command requests required privileges itself; do not prefix it with `sudo`.
4. Review local prompts carefully. If an existing Amnezia container is found, accept remapping only after reviewing the displayed target and confirmation.
5. Read the final ready/degraded/unverified result and run `scripts/diag` for diagnostics. Save any generated client share data using the protected server-side path reported by the command.

The installer can resume from saved state. Do not place real domains, account details or DNS credentials in public source configuration; use the server's protected config and secret store.

More detail: [Set up HTON](setup.md), [minimum installation](MINIMUM-INSTALL.md), and [Russian first-run guide](_lang/ru/1st-run.md).
