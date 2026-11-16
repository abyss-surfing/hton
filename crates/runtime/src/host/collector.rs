//! Independent Docker attach transport. Raw native payload is never persisted/logged.
use super::*;
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CollectorAck {
    container_id: String,
    service_id: ServiceId,
    pid: u32,
    timestamp_ms: u64,
}
impl HostRuntime {
    pub(super) fn ensure_collector(
        &self,
        id: &ContainerId,
        ctx: &OperationContext,
    ) -> Result<(), RuntimeError> {
        let unit = UnitName::new(format!("hton-native-log@{}.service", id.as_str()))
            .map_err(|e| host_error(e, ctx))?;
        let started = self.clock.now_ms();
        self.observer
            .systemd_control()
            .apply(&unit, UnitAction::Restart, self.budget(ctx)?, &HostCancel)
            .map_err(|e| host_error(e, ctx))?;
        let directory = self
            .state
            .child("collectors")
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let name = format!("{}.json", id.as_str());
        loop {
            if self.clock.now_ms() >= ctx.budget.deadline_ms {
                return Err(error(ErrorCode::DeadlineExceeded, ctx));
            }
            if let Some(bytes) = directory
                .read(&name, 64 * 1024)
                .map_err(|_| error(ErrorCode::Io, ctx))?
            {
                if let Ok(ack) = decode::<CollectorAck>(&bytes) {
                    let status = self
                        .observer
                        .systemd()
                        .inspect(&unit, self.budget(ctx)?, &HostCancel)
                        .map_err(|e| host_error(e, ctx))?;
                    if ack.container_id == id.as_str()
                        && ack.timestamp_ms >= started
                        && ack.timestamp_ms <= self.clock.now_ms()
                        && status.main_pid == Some(ack.pid)
                        && ack.pid > 1
                        && status.active_state == "active"
                    {
                        return Ok(());
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    pub fn collect(
        &self,
        container: &str,
        ctx: &OperationContext,
        logs: &hton_logging::LogSink,
    ) -> Result<(), RuntimeError> {
        let id = ContainerId::new(container).map_err(|e| host_error(e, ctx))?;
        let shutdown = self.shutdown(ctx)?;
        let mut resource = None;
        for entry in shutdown.resources.as_slice() {
            let resolved = self.authorized(&entry.identity.resource_id, ctx)?;
            if matches!(&resolved.locator,NativeLocator::Container{container_id}if container_id==container)
            {
                resource = Some(resolved);
                break;
            }
        }
        let resource = resource.ok_or_else(|| error(ErrorCode::ApprovalRequired, ctx))?;
        self.observer
            .docker()
            .inspect_container(&id, self.budget(ctx)?, &HostCancel)
            .map_err(|e| host_error(e, ctx))?;
        let mut stream =
            UnixStream::connect("/var/run/docker.sock").map_err(|_| error(ErrorCode::Io, ctx))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let request = format!(
            "POST /containers/{container}/attach?stream=1&stdout=1&stderr=1&stdin=0&logs=0 HTTP/1.1\r\nHost: docker\r\nConnection: Upgrade\r\nUpgrade: tcp\r\nContent-Length: 0\r\n\r\n"
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            if header.len() >= 16384 {
                return Err(error(ErrorCode::InvalidManifest, ctx));
            }
            let mut byte = [0];
            stream
                .read_exact(&mut byte)
                .map_err(|_| error(ErrorCode::Io, ctx))?;
            header.push(byte[0]);
        }
        let text =
            std::str::from_utf8(&header).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        if !text.starts_with("HTTP/1.1 101 ") && !text.starts_with("HTTP/1.1 200 ") {
            return Err(error(ErrorCode::Io, ctx));
        }
        let ack = CollectorAck {
            container_id: container.to_owned(),
            service_id: resource.service_id.clone(),
            pid: std::process::id(),
            timestamp_ms: self.clock.now_ms(),
        };
        let directory = self
            .state
            .child("collectors")
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let name = format!("{container}.json");
        let bytes = serde_json::to_vec(&ack).map_err(|_| error(ErrorCode::InvalidManifest, ctx))?;
        let locked = directory
            .lock(Instant::now() + Duration::from_secs(1))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let old = locked
            .read(&name, 64 * 1024)
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        locked
            .compare_exchange(&name, old.as_deref(), &bytes)
            .map_err(|_| error(ErrorCode::RecoveryRequired, ctx))?;
        drop(locked);
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .map_err(|_| error(ErrorCode::Io, ctx))?;
        let mut count = [0u64; 2];
        let mut last = self.clock.now_ms();
        let mut pending_header = [0u8; 8];
        let mut header_read = 0usize;
        let mut remaining = 0usize;
        let mut channel = 0usize;
        let mut buffer = [0u8; 8192];
        loop {
            let read = if remaining == 0 {
                stream.read(&mut pending_header[header_read..])
            } else {
                stream.read(&mut buffer[..remaining.min(8192)])
            };
            match read {
                Ok(0) => break,
                Ok(length) if remaining == 0 => {
                    header_read += length;
                    if header_read == 8 {
                        if !matches!(pending_header[0], 1 | 2) || pending_header[1..4] != [0, 0, 0]
                        {
                            return Err(error(ErrorCode::InvalidManifest, ctx));
                        }
                        channel = (pending_header[0] - 1) as usize;
                        remaining = u32::from_be_bytes(
                            pending_header[4..8].try_into().expect("fixed header"),
                        ) as usize;
                        if remaining > 16 * 1024 * 1024 {
                            return Err(error(ErrorCode::InvalidManifest, ctx));
                        }
                        header_read = 0;
                    }
                }
                Ok(length) => {
                    remaining -= length;
                    count[channel] = count[channel].saturating_add(length as u64);
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    ()
                }
                Err(_) => return Err(error(ErrorCode::Io, ctx)),
            }
            if self.clock.now_ms().saturating_sub(last) >= 1000 {
                emit_counts(
                    logs,
                    &resource.service_id,
                    &resource.identity.resource_id,
                    &mut count,
                );
                last = self.clock.now_ms();
            }
        }
        emit_counts(
            logs,
            &resource.service_id,
            &resource.identity.resource_id,
            &mut count,
        );
        if header_read != 0 || remaining != 0 {
            return Err(error(ErrorCode::Io, ctx));
        }
        Ok(())
    }
}
fn emit_counts(
    logs: &hton_logging::LogSink,
    service: &ServiceId,
    resource: &ResourceId,
    count: &mut [u64; 2],
) {
    for (channel, bytes) in count.iter_mut().enumerate() {
        if *bytes == 0 {
            continue;
        }
        let _ = logs.emit(
            hton_logging::Component::Runtime,
            hton_logging::EventContext {
                service_id: hton_logging::CorrelationId::new(service.as_str()).ok(),
                resource_id: hton_logging::CorrelationId::new(resource.as_str()).ok(),
                ..Default::default()
            },
            if channel == 1 {
                hton_logging::Severity::Warning
            } else {
                hton_logging::Severity::Debug
            },
            hton_logging::EventCode::NativeOutput,
            hton_logging::SafeFields {
                count: Some(*bytes),
                outcome: Some(hton_logging::Outcome::Unknown),
                ..Default::default()
            },
        );
        *bytes = 0;
    }
}
