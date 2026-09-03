# run-on-desktop.ps1 — launch a command on a separate (invisible) Windows desktop.
# Windows created by the process tree land on that desktop, so they cannot appear on, or take
# focus from, the interactive desktop. stdout/stderr are redirected to files (handles are
# desktop-independent). No SwitchDesktop is ever called.
#   .\run-on-desktop.ps1 -Desktop cc-quarantine -Cmd 'C:\...\node.exe' -Args '...' -Cwd P:\x -Out o.log -Err e.log
param(
  [string]$Desktop = 'cc-quarantine',
  [Parameter(Mandatory)][string]$Cmd,
  [string]$Args = '',
  [string]$Cwd = (Get-Location).Path,
  [string]$Out = "$PSScriptRoot\desktop-run.out.log",
  [string]$Err = "$PSScriptRoot\desktop-run.err.log",
  [int]$TimeoutSec = 600
)
$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class DeskLaunch {
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct STARTUPINFO {
    public int cb; public string lpReserved; public string lpDesktop; public string lpTitle;
    public int dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags;
    public short wShowWindow, cbReserved2; public IntPtr lpReserved2, hStdInput, hStdOutput, hStdError;
  }
  [StructLayout(LayoutKind.Sequential)]
  public struct PROCESS_INFORMATION { public IntPtr hProcess, hThread; public int dwProcessId, dwThreadId; }
  [StructLayout(LayoutKind.Sequential)]
  public struct SECURITY_ATTRIBUTES { public int nLength; public IntPtr lpSecurityDescriptor; public bool bInheritHandle; }

  [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern IntPtr CreateDesktop(string name, IntPtr device, IntPtr devmode, int flags, uint access, IntPtr sa);
  [DllImport("user32.dll", SetLastError = true)] public static extern bool CloseDesktop(IntPtr h);
  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern bool CreateProcess(string app, string cmdLine, IntPtr pa, IntPtr ta, bool inherit, uint flags,
    IntPtr env, string cwd, ref STARTUPINFO si, out PROCESS_INFORMATION pi);
  [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
  public static extern IntPtr CreateFile(string name, uint access, uint share, ref SECURITY_ATTRIBUTES sa, uint disp, uint flags, IntPtr tmpl);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern uint WaitForSingleObject(IntPtr h, uint ms);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool GetExitCodeProcess(IntPtr h, out int code);
  [DllImport("kernel32.dll", SetLastError = true)] public static extern bool CloseHandle(IntPtr h);

  public const uint GENERIC_ALL = 0x10000000, GENERIC_WRITE = 0x40000000, GENERIC_READ = 0x80000000;
  public const uint FILE_SHARE_RW = 3, CREATE_ALWAYS = 2, OPEN_EXISTING = 3, FILE_ATTRIBUTE_NORMAL = 0x80;
  public const int STARTF_USESTDHANDLES = 0x100;
  public const uint CREATE_UNICODE_ENVIRONMENT = 0x400;
}
'@

$hDesk = [DeskLaunch]::CreateDesktop($Desktop, [IntPtr]::Zero, [IntPtr]::Zero, 0, [DeskLaunch]::GENERIC_ALL, [IntPtr]::Zero)
if ($hDesk -eq [IntPtr]::Zero) { throw "CreateDesktop failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())" }

$sa = New-Object DeskLaunch+SECURITY_ATTRIBUTES
$sa.nLength = [Runtime.InteropServices.Marshal]::SizeOf($sa); $sa.bInheritHandle = $true
$hOut = [DeskLaunch]::CreateFile($Out, [DeskLaunch]::GENERIC_WRITE, [DeskLaunch]::FILE_SHARE_RW, [ref]$sa, [DeskLaunch]::CREATE_ALWAYS, [DeskLaunch]::FILE_ATTRIBUTE_NORMAL, [IntPtr]::Zero)
$hErr = [DeskLaunch]::CreateFile($Err, [DeskLaunch]::GENERIC_WRITE, [DeskLaunch]::FILE_SHARE_RW, [ref]$sa, [DeskLaunch]::CREATE_ALWAYS, [DeskLaunch]::FILE_ATTRIBUTE_NORMAL, [IntPtr]::Zero)
$hIn  = [DeskLaunch]::CreateFile('NUL', [DeskLaunch]::GENERIC_READ, [DeskLaunch]::FILE_SHARE_RW, [ref]$sa, [DeskLaunch]::OPEN_EXISTING, 0, [IntPtr]::Zero)

$si = New-Object DeskLaunch+STARTUPINFO
$si.cb = [Runtime.InteropServices.Marshal]::SizeOf($si)
$si.lpDesktop = $Desktop
$si.dwFlags = [DeskLaunch]::STARTF_USESTDHANDLES
$si.hStdInput = $hIn; $si.hStdOutput = $hOut; $si.hStdError = $hErr
$pi = New-Object DeskLaunch+PROCESS_INFORMATION

$cmdLine = if ($Args) { "`"$Cmd`" $Args" } else { "`"$Cmd`"" }
$t0 = Get-Date
$ok = [DeskLaunch]::CreateProcess($Cmd, $cmdLine, [IntPtr]::Zero, [IntPtr]::Zero, $true, [DeskLaunch]::CREATE_UNICODE_ENVIRONMENT, [IntPtr]::Zero, $Cwd, [ref]$si, [ref]$pi)
if (-not $ok) { $e = [Runtime.InteropServices.Marshal]::GetLastWin32Error(); [void][DeskLaunch]::CloseDesktop($hDesk); throw "CreateProcess failed: $e" }
"[desktop-run] desktop=$Desktop pid=$($pi.dwProcessId) cmd=$cmdLine"
[void][DeskLaunch]::CloseHandle($hIn); [void][DeskLaunch]::CloseHandle($hOut); [void][DeskLaunch]::CloseHandle($hErr)
$w = [DeskLaunch]::WaitForSingleObject($pi.hProcess, [uint32]($TimeoutSec * 1000))
[int]$code = -1; [void][DeskLaunch]::GetExitCodeProcess($pi.hProcess, [ref]$code)
"[desktop-run] wait=$w exit=$code elapsed=$(((Get-Date) - $t0).TotalSeconds)s"
[void][DeskLaunch]::CloseHandle($pi.hThread); [void][DeskLaunch]::CloseHandle($pi.hProcess)
[void][DeskLaunch]::CloseDesktop($hDesk)
