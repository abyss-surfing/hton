//! Typed bounded host observations and explicitly scoped effect drivers; no shell interface.
#![forbid(unsafe_code)]
#[cfg(not(target_os = "linux"))]
compile_error!("HTON host bindings require Linux");
pub mod crypto;
pub mod docker;
pub mod docker_remap;
pub mod docker_snapshot;
pub mod network;
pub mod ports;
pub mod probe;
mod process;
pub mod systemd;
pub mod validation;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    ToolUnavailable,
    Busy,
    Deadline,
    Cancelled,
    OutputLimit,
    Io,
    CommandFailed,
    InvalidResponse,
    IdentityMismatch,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
pub trait Cancellation: Send + Sync {
    fn cancelled(&self) -> bool;
}
impl Cancellation for AtomicBool {
    fn cancelled(&self) -> bool {
        self.load(Ordering::Acquire)
    }
}
/// Absolute monotonic deadline and combined stdout/stderr cap, never widened by a driver.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    deadline: Instant,
    output_bytes: usize,
}
impl Budget {
    pub(crate) fn check(&self, cancellation: &dyn Cancellation) -> Result<()> {
        if cancellation.cancelled() {
            return Err(Error::Cancelled);
        }
        if Instant::now() >= self.deadline {
            return Err(Error::Deadline);
        }
        Ok(())
    }
    /// Cryptographic payloads are bounded separately from inspection metadata.
    pub fn crypto(deadline: Instant, output_bytes: usize) -> Result<Self> {
        if output_bytes == 0
            || output_bytes > 16 * 1024 * 1024
            || deadline.saturating_duration_since(Instant::now())
                > std::time::Duration::from_secs(300)
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            deadline,
            output_bytes,
        })
    }
    pub fn new(deadline: Instant, output_bytes: usize) -> Result<Self> {
        if output_bytes == 0
            || output_bytes > 4 * 1024 * 1024
            || deadline.saturating_duration_since(Instant::now())
                > std::time::Duration::from_secs(300)
        {
            return Err(Error::InvalidInput);
        }
        Ok(Self {
            deadline,
            output_bytes,
        })
    }
}
#[derive(Clone)]
pub struct HostObserver {
    runner: Arc<process::Runner>,
}
impl Default for HostObserver {
    fn default() -> Self {
        Self::new()
    }
}
impl HostObserver {
    pub fn new() -> Self {
        Self {
            runner: Arc::new(process::Runner::new()),
        }
    }
    pub fn probes(&self) -> probe::ProbeTools {
        probe::ProbeTools::new(self.runner.clone())
    }
    pub fn network(&self) -> network::NetworkObserver {
        network::NetworkObserver::new(self.runner.clone())
    }
    /// Complete collision-only host TCP/UDP reservations from Linux sockets and Docker.
    pub fn port_reservations(
        &self,
        budget: Budget,
        cancellation: &dyn Cancellation,
    ) -> Result<Vec<ports::HostPortReservation>> {
        ports::snapshot(self.runner.clone(), &self.docker(), budget, cancellation)
    }
    pub fn docker(&self) -> docker::DockerObserver {
        docker::DockerObserver::new(self.runner.clone())
    }
    pub fn runtime_client(&self) -> RuntimeClient {
        RuntimeClient {
            runner: self.runner.clone(),
        }
    }
    pub fn crypto(&self) -> crypto::CryptoTools {
        crypto::CryptoTools::new(self.runner.clone())
    }
    pub fn docker_control(&self) -> docker::DockerControl {
        docker::DockerControl::new(self.runner.clone())
    }
    pub fn systemd_control(&self) -> systemd::SystemdControl {
        systemd::SystemdControl::new(self.runner.clone())
    }
    pub fn systemd(&self) -> systemd::SystemdObserver {
        systemd::SystemdObserver::new(self.runner.clone())
    }
}

/// Fixed independent runtime subprocess transport; callers supply versioned typed JSON.
#[derive(Clone)]
pub struct RuntimeClient {
    runner: Arc<process::Runner>,
}
impl RuntimeClient {
    pub fn execute(
        &self,
        input: &[u8],
        budget: Budget,
        cancel: &dyn Cancellation,
    ) -> Result<Vec<u8>> {
        if input.len() > 4 * 1024 * 1024 {
            return Err(Error::InvalidInput);
        }
        let output = self.runner.run_capture(
            process::Tool::Runtime,
            &[
                "execute",
                "--state-root",
                "/opt/hton/work/runtime",
                "--bundle-root",
                "/opt/hton/work/bundles",
                "--authority-root",
                "/opt/hton/work/runtime/authorities",
            ],
            input,
            budget,
            cancel,
        )?;
        if output.stdout.is_empty() {
            return Err(if output.success {
                Error::InvalidResponse
            } else {
                Error::CommandFailed
            });
        }
        Ok(output.stdout)
    }
}
