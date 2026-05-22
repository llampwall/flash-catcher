use crate::event::{HandleKind, IntegrityLevel, ProcessInfo, StdioHandles, Subsystem};
use anyhow::Result;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel,
    TOKEN_INFORMATION_CLASS, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
    PROCESS_QUERY_LIMITED_INFORMATION,
};

pub fn enrich(pid: u32, ppid: u32, image_file_name: &str, command_line: &str) -> Result<ProcessInfo> {
    let exe_path = query_exe_path(pid);
    let subsystem = exe_path
        .as_deref()
        .map(read_subsystem)
        .unwrap_or(Subsystem::Unknown);
    let integrity_level = read_integrity_level(pid);
    let stdio = classify_stdio(pid);
    let session_id = query_session_id(pid);
    let working_directory = read_working_directory(pid);

    Ok(ProcessInfo {
        pid,
        ppid,
        name: image_file_name.to_string(),
        exe_path,
        command_line: if command_line.is_empty() {
            None
        } else {
            Some(command_line.to_string())
        },
        working_directory,
        session_id,
        integrity_level,
        subsystem,
        creation_flags: 0,
        stdio,
    })
}

fn query_exe_path(pid: u32) -> Option<String> {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return None;
        };
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            Default::default(),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        if result.is_ok() {
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        } else {
            None
        }
    }
}

fn query_session_id(pid: u32) -> u32 {
    unsafe {
        let Ok(proc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return 0;
        };
        let mut token = HANDLE::default();
        if OpenProcessToken(proc, TOKEN_QUERY, &mut token).is_err() {
            let _ = CloseHandle(proc);
            return 0;
        }
        let _ = CloseHandle(proc);

        let mut session_id = 0u32;
        let mut ret_len = 0u32;
        // TokenSessionId = 12
        let _ = GetTokenInformation(
            token,
            TOKEN_INFORMATION_CLASS(12),
            Some(&mut session_id as *mut u32 as *mut _),
            4,
            &mut ret_len,
        );
        let _ = CloseHandle(token);
        session_id
    }
}

pub fn read_subsystem(exe_path: &str) -> Subsystem {
    let bytes = match std::fs::read(exe_path) {
        Ok(b) => b,
        Err(_) => return Subsystem::Unknown,
    };

    if bytes.len() < 64 {
        return Subsystem::Unknown;
    }
    if bytes[0] != b'M' || bytes[1] != b'Z' {
        return Subsystem::Unknown;
    }

    let pe_offset = u32::from_le_bytes([bytes[0x3C], bytes[0x3D], bytes[0x3E], bytes[0x3F]]) as usize;

    if pe_offset + 4 > bytes.len() {
        return Subsystem::Unknown;
    }
    if &bytes[pe_offset..pe_offset + 4] != b"PE\0\0" {
        return Subsystem::Unknown;
    }

    // IMAGE_FILE_HEADER is 20 bytes; optional header starts at pe_offset + 4 + 20
    let opt_header_offset = pe_offset + 4 + 20;
    // Subsystem at offset 0x44 (68) within optional header for both PE32 and PE32+
    let subsystem_offset = opt_header_offset + 0x44;
    if subsystem_offset + 2 > bytes.len() {
        return Subsystem::Unknown;
    }

    let sub = u16::from_le_bytes([bytes[subsystem_offset], bytes[subsystem_offset + 1]]);
    match sub {
        3 => Subsystem::Console,  // IMAGE_SUBSYSTEM_WINDOWS_CUI
        2 => Subsystem::Windows,  // IMAGE_SUBSYSTEM_WINDOWS_GUI
        _ => Subsystem::Unknown,
    }
}

pub fn read_integrity_level(pid: u32) -> IntegrityLevel {
    unsafe {
        let Ok(proc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return IntegrityLevel::Unknown;
        };
        let mut token = HANDLE::default();
        if OpenProcessToken(proc, TOKEN_QUERY, &mut token).is_err() {
            let _ = CloseHandle(proc);
            return IntegrityLevel::Unknown;
        }
        let _ = CloseHandle(proc);

        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut needed);
        if needed == 0 {
            let _ = CloseHandle(token);
            return IntegrityLevel::Unknown;
        }

        let mut buf = vec![0u8; needed as usize];
        if GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buf.as_mut_ptr() as *mut _),
            needed,
            &mut needed,
        )
        .is_err()
        {
            let _ = CloseHandle(token);
            return IntegrityLevel::Unknown;
        }
        let _ = CloseHandle(token);

        let label = buf.as_ptr() as *const TOKEN_MANDATORY_LABEL;
        let sid = (*label).Label.Sid;
        let count = *GetSidSubAuthorityCount(sid) as usize;
        if count == 0 {
            return IntegrityLevel::Unknown;
        }
        let level = *GetSidSubAuthority(sid, (count - 1) as u32);

        match level {
            0x0000 => IntegrityLevel::Untrusted,
            0x1000 => IntegrityLevel::Low,
            0x2000 => IntegrityLevel::Medium,
            0x3000 => IntegrityLevel::High,
            _ => IntegrityLevel::System,
        }
    }
}

/// Classify stdio handle types. For v1, returns Unknown for all handles.
/// DEVIATION: Full PEB-based stdio classification requires NtQueryInformationProcess +
/// ReadProcessMemory to walk RTL_USER_PROCESS_PARAMETERS, which is out of scope for v1.
/// The `visible_flash` field uses PE subsystem as the primary signal.
pub fn classify_stdio(_pid: u32) -> StdioHandles {
    StdioHandles {
        stdin: HandleKind::Unknown,
        stdout: HandleKind::Unknown,
        stderr: HandleKind::Unknown,
    }
}

/// Working directory — best-effort, returns None for v1.
/// DEVIATION: PEB walk via NtQueryInformationProcess not implemented; field is optional.
pub fn read_working_directory(_pid: u32) -> Option<String> {
    None
}

/// Creation flags — returns 0; not reliably available post-spawn.
#[allow(dead_code)]
pub fn approximate_creation_flags(_handle_kind: HandleKind) -> u32 {
    0
}

/// Walk the system snapshot to find the PPID of a process.
pub fn snapshot_ppid(pid: u32) -> Option<u32> {
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return None;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut entry).is_err() {
            let _ = CloseHandle(snap);
            return None;
        }
        loop {
            if entry.th32ProcessID == pid {
                let ppid = entry.th32ParentProcessID;
                let _ = CloseHandle(snap);
                return Some(ppid);
            }
            if Process32NextW(snap, &mut entry).is_err() {
                break;
            }
        }
        let _ = CloseHandle(snap);
        None
    }
}

/// Look up the exe name for a pid via toolhelp snapshot.
pub fn snapshot_name(pid: u32) -> Option<String> {
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return None;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut entry).is_err() {
            let _ = CloseHandle(snap);
            return None;
        }
        loop {
            if entry.th32ProcessID == pid {
                let _ = CloseHandle(snap);
                let name = entry.szExeFile;
                let len = name.iter().position(|&c| c == 0).unwrap_or(name.len());
                return Some(String::from_utf16_lossy(&name[..len]));
            }
            if Process32NextW(snap, &mut entry).is_err() {
                break;
            }
        }
        let _ = CloseHandle(snap);
        None
    }
}
