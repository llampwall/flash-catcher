use anyhow::{Context, Result};
use std::os::windows::ffi::OsStrExt;

pub fn is_elevated() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret_len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut TOKEN_ELEVATION as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        let _ = CloseHandle(token);
        ok.is_ok() && elevation.TokenIsElevated != 0
    }
}

pub fn relaunch_elevated() -> Result<bool> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_NORMAL;

    let exe = std::env::current_exe().context("failed to get current exe path")?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args_str = args.join(" ");

    let exe_wide: Vec<u16> = exe
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let verb: Vec<u16> = "runas\0".encode_utf16().collect();
    let args_wide: Vec<u16> = args_str.encode_utf16().chain(std::iter::once(0)).collect();

    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(exe_wide.as_ptr()),
            PCWSTR(args_wide.as_ptr()),
            PCWSTR::null(),
            SW_NORMAL,
        )
    };

    // ShellExecuteW returns a value > 32 on success; <= 32 means error or declined
    if result.0 as usize > 32 {
        // Elevated child launched successfully — parent exits cleanly
        std::process::exit(0);
    }

    // UAC declined or error
    Ok(false)
}

pub fn require_elevation_or_relaunch() -> Result<()> {
    if is_elevated() {
        return Ok(());
    }
    relaunch_elevated()?;
    // If we get here, UAC was declined
    eprintln!(
        "Admin required for ETW capture. For UI-only access to existing data, run: flash-watcher view"
    );
    std::process::exit(1);
}
