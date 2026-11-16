//! Independent, bounded, closed-field logging. Disk I/O never runs on emit callers.
use hton_storage::SecureDir;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{SyncSender, TrySendError, sync_channel},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
    Information,
    Debug,
    Trace,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Component {
    Cli,
    Core,
    Registry,
    Journal,
    Setup,
    Runtime,
    Guardian,
    Certificates,
    Backup,
    Adapter,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventCode {
    OperationStarted,
    OperationFinished,
    IntentDurable,
    StepRecorded,
    StatePublished,
    RecoveryRequired,
    ResourceDrift,
    ApprovalRequired,
    ValidationFailed,
    DeadlineExceeded,
    Cancelled,
    StorageUnavailable,
    LoggingPressure,
    NativeOutput,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CorrelationId(String);
impl CorrelationId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidPolicy> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        {
            return Err(InvalidPolicy);
        }
        Ok(Self(value))
    }
}
impl TryFrom<String> for CorrelationId {
    type Error = InvalidPolicy;
    fn try_from(v: String) -> Result<Self, Self::Error> {
        Self::new(v)
    }
}
impl From<CorrelationId> for String {
    fn from(v: CorrelationId) -> Self {
        v.0
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EventContext {
    pub operation_id: Option<CorrelationId>,
    pub transaction_id: Option<CorrelationId>,
    pub module_id: Option<CorrelationId>,
    pub service_id: Option<CorrelationId>,
    pub resource_id: Option<CorrelationId>,
    pub object_id: Option<CorrelationId>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FailureReason {
    ConfigSyntax,
    UnsupportedFormat,
    SemanticConflict,
    StaleInput,
    IncompleteSetup,
    MissingArtifact,
    MissingSecret,
    NativeValidation,
    CertificateInvalid,
    ApprovalRequired,
    ResourceDrift,
    Stopped,
    DeadlineExceeded,
    Cancelled,
    StorageUnavailable,
    RecoveryRequired,
    PermissionDenied,
    Partial,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Succeeded,
    Failed,
    Partial,
    Unknown,
    Skipped,
    Stopped,
}
/// No free-form message, credential URL, command output or secret slot exists.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SafeFields {
    pub count: Option<u64>,
    pub duration_ms: Option<u64>,
    pub outcome: Option<Outcome>,
    pub retryable: Option<bool>,
    pub reason: Option<FailureReason>,
}
#[derive(Clone, Debug)]
pub struct LoggingPolicy {
    pub max_level: Severity,
    pub max_file_bytes: usize,
    pub max_files: u8,
    pub max_age: Duration,
    pub queue_capacity: usize,
}
impl Default for LoggingPolicy {
    fn default() -> Self {
        Self {
            max_level: Severity::Information,
            max_file_bytes: 65_536,
            max_files: 8,
            max_age: Duration::from_secs(7 * 24 * 3600),
            queue_capacity: 128,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct InvalidPolicy;
impl std::fmt::Display for InvalidPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid bounded logging policy or correlation identifier")
    }
}
impl std::error::Error for InvalidPolicy {}
impl LoggingPolicy {
    /// A requested total is a ceiling; fixed bounded ring capacity may be lower.
    pub fn bounded(
        max_level: Severity,
        max_bytes: u64,
        retention_days: u64,
    ) -> Result<Self, InvalidPolicy> {
        if !(4096..=1_073_741_824).contains(&max_bytes) || !(1..=7).contains(&retention_days) {
            return Err(InvalidPolicy);
        }
        let max_file_bytes = max_bytes.min(1_048_576) as usize;
        let max_files = (max_bytes / max_file_bytes as u64).clamp(1, 16) as u8;
        let policy = Self {
            max_level,
            max_file_bytes,
            max_files,
            max_age: Duration::from_secs(retention_days * 86400),
            queue_capacity: 128,
        };
        policy.validate()?;
        Ok(policy)
    }
    fn validate(&self) -> Result<(), InvalidPolicy> {
        if !(4096..=1_048_576).contains(&self.max_file_bytes)
            || !(1..=16).contains(&self.max_files)
            || self.max_age < Duration::from_secs(60)
            || self.max_age > Duration::from_secs(365 * 24 * 3600)
            || !(1..=1024).contains(&self.queue_capacity)
        {
            Err(InvalidPolicy)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogWriteStatus {
    Queued,
    Transient,
    Filtered,
    Dropped,
}
#[derive(Default)]
struct Counters {
    written: AtomicU64,
    dropped: AtomicU64,
}
#[derive(Clone, Copy, Debug)]
pub struct LogStatistics {
    pub written: u64,
    pub dropped: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    timestamp_ms: u64,
    component: Component,
    context: EventContext,
    severity: Severity,
    code: EventCode,
    fields: SafeFields,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Cursor {
    schema_version: u8,
    index: u8,
    started_at_ms: u64,
}
pub struct LogSink {
    sender: Option<SyncSender<Vec<u8>>>,
    transient: Option<Mutex<VecDeque<Vec<u8>>>>,
    policy: LoggingPolicy,
    counters: Arc<Counters>,
}
impl LogSink {
    /// Caller has already selected/opened the installer-approved protected directory.
    pub fn configure(policy: LoggingPolicy, directory: SecureDir) -> Result<Self, InvalidPolicy> {
        policy.validate()?;
        let (sender, receiver) = sync_channel::<Vec<u8>>(policy.queue_capacity);
        let counters = Arc::new(Counters::default());
        let worker_counters = counters.clone();
        let worker_policy = policy.clone();
        let worker = thread::Builder::new()
            .name("hton-log-writer".into())
            .spawn(move || {
                while let Ok(line) = receiver.recv() {
                    if persist(&directory, &worker_policy, &line).is_ok() {
                        worker_counters.written.fetch_add(1, Ordering::Relaxed);
                    } else {
                        worker_counters.dropped.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });
        if worker.is_err() {
            return Err(InvalidPolicy);
        }
        Ok(Self {
            sender: Some(sender),
            transient: None,
            policy,
            counters,
        })
    }
    /// Zero-write mode: a bounded memory queue; no directory or worker is created.
    pub fn transient(policy: LoggingPolicy) -> Result<Self, InvalidPolicy> {
        policy.validate()?;
        Ok(Self {
            sender: None,
            transient: Some(Mutex::new(VecDeque::with_capacity(policy.queue_capacity))),
            policy,
            counters: Arc::new(Counters::default()),
        })
    }
    pub fn emit(
        &self,
        component: Component,
        context: EventContext,
        severity: Severity,
        code: EventCode,
        fields: SafeFields,
    ) -> LogWriteStatus {
        if severity > self.policy.max_level {
            return LogWriteStatus::Filtered;
        }
        let event = Event {
            timestamp_ms: now_ms(),
            component,
            context,
            severity,
            code,
            fields,
        };
        let Ok(mut bytes) = serde_json::to_vec(&event) else {
            self.counters.dropped.fetch_add(1, Ordering::Relaxed);
            return LogWriteStatus::Dropped;
        };
        if bytes.len() > 2048 {
            self.counters.dropped.fetch_add(1, Ordering::Relaxed);
            return LogWriteStatus::Dropped;
        }
        bytes.push(b'\n');
        let Some(sender) = &self.sender else {
            if let Some(queue) = &self.transient {
                if let Ok(mut queue) = queue.try_lock() {
                    if queue.len() < self.policy.queue_capacity {
                        queue.push_back(bytes);
                        self.counters.written.fetch_add(1, Ordering::Relaxed);
                        return LogWriteStatus::Transient;
                    }
                }
            }
            self.counters.dropped.fetch_add(1, Ordering::Relaxed);
            return LogWriteStatus::Dropped;
        };
        match sender.try_send(bytes) {
            Ok(()) => LogWriteStatus::Queued,
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                self.counters.dropped.fetch_add(1, Ordering::Relaxed);
                LogWriteStatus::Dropped
            }
        }
    }
    pub fn statistics(&self) -> LogStatistics {
        LogStatistics {
            written: self.counters.written.load(Ordering::Relaxed),
            dropped: self.counters.dropped.load(Ordering::Relaxed),
        }
    }
    /// Returns closed-format metadata events for a read-only CLI diagnostic view.
    /// A contended queue yields no data; it never blocks the operation emitting logs.
    pub fn drain_transient(&self) -> Vec<Vec<u8>> {
        self.transient
            .as_ref()
            .and_then(|queue| queue.try_lock().ok())
            .map(|mut queue| queue.drain(..).collect())
            .unwrap_or_default()
    }
}
fn persist(directory: &SecureDir, policy: &LoggingPolicy, line: &[u8]) -> Result<(), ()> {
    let lock = directory
        .lock(Instant::now() + Duration::from_millis(10))
        .map_err(|_| ())?;
    let meta = lock.read("cursor.json", 1024).map_err(|_| ())?;
    let now = now_ms();
    let mut cursor = match &meta {
        Some(bytes) => serde_json::from_slice::<Cursor>(bytes).map_err(|_| ())?,
        None => Cursor {
            schema_version: 1,
            index: 0,
            started_at_ms: now,
        },
    };
    if cursor.schema_version != 1 || cursor.index >= 16 {
        return Err(());
    }
    if cursor.index >= policy.max_files {
        cursor.index = 0;
        cursor.started_at_ms = 0;
    }
    let mut name = format!("events-{}.jsonl", cursor.index);
    let mut previous = lock.read(&name, 1_048_576).map_err(|_| ())?;
    let age_limit = policy.max_age.as_millis().min(u64::MAX as u128) as u64;
    let rotate = now.saturating_sub(cursor.started_at_ms) >= age_limit
        || previous.as_ref().map_or(0, Vec::len) + line.len() > policy.max_file_bytes;
    if rotate {
        cursor.index = (cursor.index + 1) % policy.max_files;
        cursor.started_at_ms = now;
        name = format!("events-{}.jsonl", cursor.index);
        previous = lock.read(&name, 1_048_576).map_err(|_| ())?;
    }
    let mut bytes = if rotate {
        Vec::new()
    } else {
        previous.clone().unwrap_or_default()
    };
    bytes.extend_from_slice(line);
    if bytes.len() > policy.max_file_bytes {
        return Err(());
    }
    lock.compare_exchange(&name, previous.as_deref(), &bytes)
        .map_err(|_| ())?;
    let encoded = serde_json::to_vec(&cursor).map_err(|_| ())?;
    lock.compare_exchange("cursor.json", meta.as_deref(), &encoded)
        .map_err(|_| ())?;
    // Remove stale slots by the timestamp of their first closed-format event.
    for index in 0..policy.max_files {
        if index == cursor.index {
            continue;
        }
        let old_name = format!("events-{index}.jsonl");
        if let Some(old) = lock.read(&old_name, 1_048_576).map_err(|_| ())? {
            let first = old.split(|b| *b == b'\n').next().unwrap_or_default();
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(first) {
                if value
                    .get("timestampMs")
                    .and_then(|v| v.as_u64())
                    .is_some_and(|t| now.saturating_sub(t) >= age_limit)
                {
                    lock.remove(&old_name, &old).map_err(|_| ())?;
                }
            }
        }
    }
    for index in policy.max_files..16 {
        let unused = format!("events-{index}.jsonl");
        if let Some(bytes) = lock.read(&unused, 1_048_576).map_err(|_| ())? {
            lock.remove(&unused, &bytes).map_err(|_| ())?;
        }
    }
    Ok(())
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}
