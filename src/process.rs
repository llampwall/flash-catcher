use crate::event::{HandleKind, IntegrityLevel, ProcessInfo, StdioHandles, Subsystem};
use anyhow::Result;

/// Take a raw (pid, ppid, name, command_line) tuple from ETW and enrich it
/// with everything that requires Win32 API calls against the live process.
///
/// Must be called as soon as possible after spawn — short-lived processes
/// (<50ms) frequently exit before the handle can be opened. When that happens
/// the returned `ProcessInfo` carries whatever ETW provided and `None` for
/// every field that requires an open handle.
pub fn enrich(
    _pid: u32,
    _ppid: u32,
    _image_file_name: &str,
    _command_line: &str,
) -> Result<ProcessInfo> {
    unimplemented!("OpenProcess(QUERY_LIMITED_INFORMATION) + QueryFullProcessImageNameW + token + PEB walk for cwd")
}

/// Determine the PE subsystem of the executable on disk.
/// Reads the PE header IMAGE_OPTIONAL_HEADER.Subsystem field.
/// Only IMAGE_SUBSYSTEM_WINDOWS_CUI (Console) produces visible flashes.
pub fn read_subsystem(_exe_path: &str) -> Subsystem {
    unimplemented!("mmap PE header, read DOS+NT signature, return Subsystem")
}

/// Read the integrity level from the process token.
pub fn read_integrity_level(_pid: u32) -> IntegrityLevel {
    unimplemented!("OpenProcessToken + GetTokenInformation(TokenIntegrityLevel)")
}

/// Classify stdin/stdout/stderr handle types for a given process.
/// A CONSOLE subsystem process whose stdout is a Pipe will not flash even
/// though its subsystem says it would — this is how shims silence things.
pub fn classify_stdio(_pid: u32) -> StdioHandles {
    unimplemented!("NtQueryObject on handle types via DuplicateHandle")
}

/// Resolve the working directory by reading the PEB / RTL_USER_PROCESS_PARAMETERS.
pub fn read_working_directory(_pid: u32) -> Option<String> {
    unimplemented!("NtQueryInformationProcess(ProcessBasicInformation) then walk PEB")
}

/// Read creation flags by inspecting the parent's CreateProcess args.
/// Not reliably available post-spawn; left as best-effort.
pub fn approximate_creation_flags(_handle_kind: HandleKind) -> u32 {
    unimplemented!("infer from stdio handle kinds when source is not available")
}
