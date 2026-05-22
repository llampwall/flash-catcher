use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Process subsystem — only `Console` subsystem processes can cause visible flashes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Subsystem {
    Console,
    Windows,
    Unknown,
}

/// Integrity level reported by the token of the process.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IntegrityLevel {
    Untrusted,
    Low,
    Medium,
    High,
    System,
    Unknown,
}

/// Classification verdict assigned by `classify.rs`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    /// Known Claude Code subprocess probe (reg.exe MachineGuid, tasklist, Get-CimInstance Win32_Process, ...)
    ClaudeCodeProbe,
    /// chinvex-side spawn (gateway, ingest, sync)
    Chinvex,
    /// AllMind / mercenary / its hooks
    OurCode,
    /// Recognized but uninteresting (build tools, scheduled tasks, etc.)
    KnownBenign,
    /// Unrecognized — surface prominently
    Unknown,
}

/// Stdio handle classification — pipe = silent, console = visible, null = silent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HandleKind {
    Pipe,
    Console,
    File,
    Null,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioHandles {
    pub stdin: HandleKind,
    pub stdout: HandleKind,
    pub stderr: HandleKind,
}

/// Enriched process snapshot captured at (or as close to) spawn time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub exe_path: Option<String>,
    pub command_line: Option<String>,
    pub working_directory: Option<String>,
    pub session_id: u32,
    pub integrity_level: IntegrityLevel,
    pub subsystem: Subsystem,
    pub creation_flags: u32,
    pub stdio: StdioHandles,
}

/// Ancestry walk from the spawned process up to the root.
/// Element 0 is the immediate parent; last element is the root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameChain {
    pub ancestors: Vec<BlameNode>,
    /// Deterministic key for grouping in the UI — exe-name chain joined with `<-`
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameNode {
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<String>,
}

/// A conhost.exe allocation paired with the process that allocated it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConhostPairing {
    pub conhost_pid: u32,
    pub spawned_at: DateTime<Utc>,
    pub exited_at: Option<DateTime<Utc>>,
}

/// The canonical event written to JSONL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashEvent {
    pub event_id: String,
    pub spawned_at: DateTime<Utc>,
    pub exited_at: Option<DateTime<Utc>>,
    pub lifetime_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub process: ProcessInfo,
    pub blame: BlameChain,
    pub conhost: Option<ConhostPairing>,
    pub classification: Classification,
    /// Free-form tag emitted by the classifier (rule name that matched).
    pub classification_rule: Option<String>,
    /// True iff the OS would actually display a visible window for this spawn.
    pub visible_flash: bool,
}

impl FlashEvent {
    pub fn new_id() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let ts = chrono::Utc::now().timestamp_micros() as u64;
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        format!("{:016x}{:08x}", ts, seq)
    }
}
