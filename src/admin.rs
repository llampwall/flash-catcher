use anyhow::Result;

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

pub fn require_elevation_or_relaunch() -> Result<()> {
    if is_elevated() {
        return Ok(());
    }
    anyhow::bail!(
        "ETW capture requires elevation. Run flash-watcher from an Administrator terminal \
         (right-click → Run as administrator, or open an elevated PowerShell)."
    );
}
