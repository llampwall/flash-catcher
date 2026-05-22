use crate::event::ProcessInfo;
use anyhow::Result;
use tokio::sync::mpsc;

/// Raw event emitted by the ETW kernel logger before enrichment.
#[derive(Debug, Clone)]
pub enum RawEvent {
    ProcessStart {
        pid: u32,
        ppid: u32,
        image_file_name: String,
        command_line: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ProcessExit {
        pid: u32,
        exit_code: i32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

/// Spawn the ETW kernel session that captures process start/exit events.
/// Requires admin. Returns a receiver channel that streams `RawEvent`s.
///
/// Implementation notes for the executing agent:
/// - Uses `ferrisetw::trace::KernelTrace` with the `Process` provider mask
/// - Session must be named (e.g. "flash-watcher-kernel") and stopped on Drop
/// - Multiple instances cannot coexist on Windows — start aborts if the
///   named session already exists; provide a force-stop helper.
pub fn start_kernel_session() -> Result<mpsc::Receiver<RawEvent>> {
    unimplemented!("create EVENT_TRACE_PROPERTIES, start session, register Process callback, forward to mpsc")
}

/// Resolve the rest of `ProcessInfo` for a freshly observed pid.
/// ETW provides image name + cmdline; everything else (exe path, working dir,
/// integrity level, subsystem, stdio handle kinds, creation flags) must be
/// queried out-of-band via `process::enrich`.
pub fn enrich_raw(_raw: &RawEvent) -> Result<Option<ProcessInfo>> {
    unimplemented!("called by collector loop, delegates to process::enrich")
}

/// Stop the named ETW session if it exists.
/// Useful for cleanup on Ctrl-C and for forcing recovery if a prior run leaked.
pub fn stop_session(_session_name: &str) -> Result<()> {
    unimplemented!("ControlTrace with EVENT_TRACE_CONTROL_STOP")
}
