use anyhow::Result;

/// Return true if the current process token is elevated (admin).
/// ETW kernel-logger session creation requires this.
pub fn is_elevated() -> bool {
    unimplemented!("OpenProcessToken + GetTokenInformation(TokenElevation)")
}

/// If not elevated, relaunch the current binary via ShellExecuteW with the
/// `runas` verb so Windows shows a UAC prompt. The parent process exits.
/// If elevation is declined, return Ok(false) so the caller can fall back
/// to view-only mode.
pub fn relaunch_elevated() -> Result<bool> {
    unimplemented!("ShellExecuteW(\"runas\", current_exe, original args, NULL, SW_NORMAL)")
}

/// Convenience: ensure we are admin OR exit with a clear error.
/// Returns Ok(()) only when running elevated.
pub fn require_elevation_or_relaunch() -> Result<()> {
    unimplemented!("if !is_elevated() -> relaunch_elevated; else Ok(())")
}
