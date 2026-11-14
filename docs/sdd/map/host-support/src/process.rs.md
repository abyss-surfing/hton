# host-support/src/process.rs

Stable ID: `infra.host-process` · Phase: `R1` · Kind: source · Wave: I3-B

Private bounded fixed-command Linux subprocess transport. Source written; no compilation/product acceptance claimed (D81).

## Methods and responsibility

- `run` — fixed tool/driver argv, monotonic Budget, Cancellation → protected stdout or closed error.

Authority and limits: [INTERFACES](../../../../INTERFACES.md), I3 bounded native observation transport / D81.

No Core/runtime contracts, shell, mutation APIs, secret serialization or ownership discovery. Driver errors are closed/redacted; unavailable facts cannot imply success. Consumers must qualify ownership/admission/fingerprints independently.

Absolute fixed executables, env_clear, no stdin, private process group; combined output cap, cancellation, monotonic deadline and eight shared slots. Nonblocking finite drains avoid deadlock/starvation; waitid NOWAIT retains leader PID through group cleanup. Failed children are killed and reaped while retaining a slot. No public generic runner. `ss` is a fixed optional read-only driver used only for exact Docker-proxy socket attribution; its output is bounded and an unavailable/unknown format leaves procfs reservations unowned.

R1 composition adds private run_input with bounded nonblocking stdin/stdout/stderr
multiplexing, mandatory EOF after exact input and no truncated success. Age/OpenSsl
are fixed executable variants for the separately owned crypto adapter. Crypto Budget
permits at most16MiB while observation Budget retains4MiB. No shell/input logging.

`run_stream` is a private fixed-driver variant for bounded binary snapshot output.
It sends finite nonblocking stdout chunks to an injected protected sink, without
retaining image/rootfs bytes in memory. The caller sets a separate maximum
stream size <=8 GiB; stderr retains the ordinary metadata ceiling. Cancellation,
deadline, concurrency, process-group cleanup and reaping use the same owner as
`run_capture`, and every sink failure aborts/reaps the command. Completion returns
only the streamed byte count; callers publish the protected file only after
successful exit and their identity/digest checks. No caller argv/path is exposed.
