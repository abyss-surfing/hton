# Set up HTON

HTON setup prepares a server installation. It is separate from Tartarus setup: Tartarus configures the development checkout, while HTON runs on the target Linux server.

## Before installing

- Use a Linux server with systemd, Docker enabled at boot, Python 3, PyYAML 6.x, curl, OpenSSL and the system CA bundle.
- Point the chosen site domain at the server and allow public TCP/80 and TCP/443.
- Keep DNS or issuer credentials in the protected HTON secret store. Never put them in the source tree or send them in chat.
- Review the instance-specific site domain, contact email and installation directory in `platform/etc/install.json`.

## First installation

From the HTON checkout, run:

```sh
scripts/install
```

The installer requests required system privileges itself, collects missing site settings locally and prepares the minimum trusted website and supported Amnezia integration. Read each confirmation before accepting it. It can finish with publication unverified if DNS or public reachability is not ready; check the reported status and follow the diagnostics rather than treating that as a verified public service.

Run `scripts/diag` after installation to inspect the result. The installation is resumable; rerunning `scripts/install` verifies or resumes saved state.

See [Minimum installation](MINIMUM-INSTALL.md), [the setup command reference](scripts/setup.md), and the Russian guide [Настроить HTON](_lang/ru/setup.md). For the relationship between checkout setup and server installation, see [First run](1st-run.md).
