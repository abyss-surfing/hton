//! Private mechanics: only fixed typed drivers can construct a command.
use crate::{Budget, Cancellation, Error, Result};
use rustix::{
    fs::{OFlags, fcntl_getfl, fcntl_setfl},
    process::{Pid, Signal, WaitId, WaitIdOptions, kill_process_group, waitid},
};
use std::{
    io::{Read, Write},
    os::{fd::AsFd, unix::process::CommandExt},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

pub(crate) enum Tool {
    Docker,
    Systemctl,
    Age,
    AgeKeygen,
    OpenSsl,
    Runtime,
    Curl,
    Dig,
    Ss,
}
pub(crate) struct CommandOutput {
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) success: bool,
}
pub(crate) struct Runner {
    slots: Arc<AtomicUsize>,
}
struct Permit(Arc<AtomicUsize>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Running {
    child: Option<Child>,
    permit: Option<Permit>,
    reaped: bool,
}
impl Drop for Running {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if !self.reaped {
                if let Some(pid) = Pid::from_raw(child.id() as i32) {
                    let _ = kill_process_group(pid, Signal::KILL);
                }
                let _ = child.kill();
                let permit = self.permit.take();
                // At most eight reapers per shared runner; the permit survives until reap.
                let _ = std::thread::spawn(move || {
                    let _ = child.wait();
                    drop(permit);
                });
            }
        }
    }
}
fn nonblocking(fd: &impl AsFd) -> Result<()> {
    let flags = fcntl_getfl(fd).map_err(|_| Error::Io)?;
    fcntl_setfl(fd, flags | OFlags::NONBLOCK).map_err(|_| Error::Io)
}
fn drain(
    reader: &mut impl Read,
    output: &mut Vec<u8>,
    total: &mut usize,
    cap: usize,
    retain: bool,
) -> Result<bool> {
    let mut buffer = [0u8; 8192];
    // A finite turn prevents a chatty pipe from starving cancellation/deadline checks.
    for _ in 0..4 {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(true),
            Ok(count) => {
                *total = total.checked_add(count).ok_or(Error::OutputLimit)?;
                if *total > cap {
                    return Err(Error::OutputLimit);
                }
                if retain {
                    output.extend_from_slice(&buffer[..count]);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(Error::Io),
        }
    }
    Ok(false)
}
fn drain_stream(
    reader: &mut impl Read,
    total: &mut u64,
    cap: u64,
    sink: &mut dyn FnMut(&[u8]) -> Result<()>,
) -> Result<bool> {
    let mut buffer = [0u8; 8192];
    for _ in 0..4 {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(true),
            Ok(count) => {
                *total = total.checked_add(count as u64).ok_or(Error::OutputLimit)?;
                if *total > cap {
                    return Err(Error::OutputLimit);
                }
                sink(&buffer[..count])?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(Error::Io),
        }
    }
    Ok(false)
}
impl Runner {
    pub(crate) fn new() -> Self {
        Self {
            slots: Arc::new(AtomicUsize::new(0)),
        }
    }
    pub(crate) fn run(
        &self,
        tool: Tool,
        args: &[&str],
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<u8>> {
        self.run_input(tool, args, &[], budget, cancellation)
    }
    pub(crate) fn run_input(
        &self,
        tool: Tool,
        args: &[&str],
        input: &[u8],
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<u8>> {
        let output = self.run_capture(tool, args, input, budget, cancellation)?;
        if output.success {
            Ok(output.stdout)
        } else {
            Err(Error::CommandFailed)
        }
    }
    pub(crate) fn run_capture(
        &self,
        tool: Tool,
        args: &[&str],
        input: &[u8],
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<CommandOutput> {
        self.run_capture_impl(tool, args, input, budget, cancellation, None)
            .map(|(output, _)| output)
    }
    pub(crate) fn run_stream(
        &self,
        tool: Tool,
        args: &[&str],
        budget: Budget,
        cancellation: &dyn Cancellation,
        max_bytes: u64,
        sink: &mut dyn FnMut(&[u8]) -> Result<()>,
    ) -> Result<u64> {
        if max_bytes == 0 || max_bytes > 8 * 1024 * 1024 * 1024 {
            return Err(Error::InvalidInput);
        }
        let (output, bytes) = self.run_capture_impl(
            tool,
            args,
            &[],
            budget,
            cancellation,
            Some((max_bytes, sink)),
        )?;
        if !output.success {
            return Err(Error::CommandFailed);
        }
        Ok(bytes)
    }
    fn run_capture_impl(
        &self,
        tool: Tool,
        args: &[&str],
        input: &[u8],
        budget: Budget,
        cancellation: &dyn Cancellation,
        mut stream: Option<(u64, &mut dyn FnMut(&[u8]) -> Result<()>)>,
    ) -> Result<(CommandOutput, u64)> {
        if input.len() > 16 * 1024 * 1024 {
            return Err(Error::InvalidInput);
        }
        if cancellation.cancelled() {
            return Err(Error::Cancelled);
        }
        if Instant::now() >= budget.deadline {
            return Err(Error::Deadline);
        }
        if args.len() > 512
            || args.iter().map(|arg| arg.len() + 1).sum::<usize>() > 65536
            || args
                .iter()
                .any(|arg| arg.len() > 4096 || arg.contains('\0'))
        {
            return Err(Error::InvalidInput);
        }
        self.slots
            .try_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 8).then_some(count + 1)
            })
            .map_err(|_| Error::Busy)?;
        let permit = Permit(self.slots.clone());
        let executable = match tool {
            Tool::Docker => "/usr/bin/docker",
            Tool::Systemctl => "/usr/bin/systemctl",
            Tool::Age => "/usr/bin/age",
            Tool::AgeKeygen => "/usr/bin/age-keygen",
            Tool::OpenSsl => "/usr/bin/openssl",
            Tool::Runtime => "/opt/hton/libexec/hton-runtime",
            Tool::Curl => "/usr/bin/curl",
            Tool::Dig => "/usr/bin/dig",
            Tool::Ss => "/usr/bin/ss",
        };
        let child = Command::new(executable)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("LANG", "C")
            .env("LC_ALL", "C")
            .env("HOME", "/nonexistent")
            .env("TZ", "UTC")
            .current_dir("/")
            .stdin(if input.is_empty() {
                Stdio::null()
            } else {
                Stdio::piped()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    Error::ToolUnavailable
                } else {
                    Error::Io
                }
            })?;
        let mut running = Running {
            child: Some(child),
            permit: Some(permit),
            reaped: false,
        };
        let child = running.child.as_mut().ok_or(Error::Io)?;
        let mut stdin = child.stdin.take();
        if let Some(stdin) = &stdin {
            nonblocking(stdin)?;
        }
        let mut written = 0usize;
        let mut stdout = child.stdout.take().ok_or(Error::Io)?;
        let mut stderr = child.stderr.take().ok_or(Error::Io)?;
        nonblocking(&stdout)?;
        nonblocking(&stderr)?;
        let mut output = Vec::new();
        let mut discarded = Vec::new();
        let mut total = 0;
        let mut streamed_bytes = 0u64;
        let mut stdout_eof = false;
        let mut stderr_eof = false;
        let pid =
            Pid::from_raw(running.child.as_ref().ok_or(Error::Io)?.id() as i32).ok_or(Error::Io)?;
        loop {
            if cancellation.cancelled() {
                return Err(Error::Cancelled);
            }
            if Instant::now() >= budget.deadline {
                return Err(Error::Deadline);
            }
            if let Some(writer) = stdin.as_mut() {
                let end = input.len().min(written.saturating_add(8192));
                match writer.write(&input[written..end]) {
                    Ok(0) => return Err(Error::CommandFailed),
                    Ok(count) => written += count,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            || error.kind() == std::io::ErrorKind::Interrupted =>
                    {
                        ()
                    }
                    Err(_) => return Err(Error::CommandFailed),
                }
                if written == input.len() {
                    stdin = None;
                }
            }
            if !stdout_eof {
                stdout_eof = if let Some((cap, sink)) = stream.as_mut() {
                    drain_stream(&mut stdout, &mut streamed_bytes, *cap, *sink)?
                } else {
                    drain(
                        &mut stdout,
                        &mut output,
                        &mut total,
                        budget.output_bytes,
                        true,
                    )?
                };
            }
            if !stderr_eof {
                stderr_eof = drain(
                    &mut stderr,
                    &mut discarded,
                    &mut total,
                    budget.output_bytes,
                    true,
                )?;
            }
            // NOWAIT pins the exited leader's PID until pipe cleanup, preventing
            // a timeout cleanup signal from targeting a recycled process group.
            let exited = waitid(
                WaitId::Pid(pid),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            )
            .map_err(|_| Error::Io)?
            .is_some();
            if exited && stdout_eof && stderr_eof {
                let _ = kill_process_group(pid, Signal::KILL);
                let status = running
                    .child
                    .as_mut()
                    .ok_or(Error::Io)?
                    .try_wait()
                    .map_err(|_| Error::Io)?
                    .ok_or(Error::Io)?;
                running.reaped = true;
                if cancellation.cancelled() {
                    return Err(Error::Cancelled);
                }
                if Instant::now() >= budget.deadline {
                    return Err(Error::Deadline);
                }
                return if written == input.len() {
                    Ok((
                        CommandOutput {
                            stdout: output,
                            stderr: discarded,
                            success: status.success(),
                        },
                        streamed_bytes,
                    ))
                } else {
                    Err(Error::CommandFailed)
                };
            }
            std::thread::sleep(
                Duration::from_millis(5)
                    .min(budget.deadline.saturating_duration_since(Instant::now())),
            );
        }
    }
}
