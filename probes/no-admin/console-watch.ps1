# console-watch.ps1 — no-admin console-allocation + focus-steal watcher.
# Polls for new conhost.exe / OpenConsole.exe / WindowsTerminal.exe processes and records
# each one's command line (conhost "--headless" == hidden console; otherwise a visible window),
# its client (parent) process, and the client's ancestry chain. Also samples the foreground
# window every tick and logs every change of foreground process.
param(
  [int]$Seconds = 120,
  [int]$TickMs = 100,
  [string]$Out = "$PSScriptRoot\console-watch.jsonl",
  [string]$Label = ''
)
$ErrorActionPreference = 'Continue'

Add-Type -Namespace FW -Name Fg -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("user32.dll")] public static extern System.IntPtr GetForegroundWindow();
[System.Runtime.InteropServices.DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(System.IntPtr h, out uint pid);
[System.Runtime.InteropServices.DllImport("user32.dll", CharSet=System.Runtime.InteropServices.CharSet.Unicode)] public static extern int GetClassName(System.IntPtr h, System.Text.StringBuilder s, int n);
[System.Runtime.InteropServices.DllImport("user32.dll", CharSet=System.Runtime.InteropServices.CharSet.Unicode)] public static extern int GetWindowText(System.IntPtr h, System.Text.StringBuilder s, int n);
'@

function Get-ProcInfo([int]$id) {
  if ($id -le 0) { return $null }
  if ($script:cache.ContainsKey($id)) { return $script:cache[$id] }
  $p = Get-CimInstance Win32_Process -Filter "ProcessId=$id" -ErrorAction SilentlyContinue
  if (-not $p) { return $null }
  $info = [pscustomobject]@{ pid = $p.ProcessId; ppid = $p.ParentProcessId; name = $p.Name; cmd = $p.CommandLine }
  $script:cache[$id] = $info
  return $info
}
function Get-Chain([int]$id) {
  $chain = @(); $cur = $id; $n = 0
  while ($cur -gt 0 -and $n -lt 10) {
    $i = Get-ProcInfo $cur
    if (-not $i) { break }
    $chain += "$($i.name)#$($i.pid)"
    $cur = $i.ppid; $n++
  }
  return ($chain -join ' <- ')
}
function Emit($obj) { ($obj | ConvertTo-Json -Compress -Depth 4) | Add-Content -Path $Out -Encoding utf8 }

$script:cache = @{}
$known = @{}
Get-CimInstance Win32_Process -Filter "Name='conhost.exe' OR Name='OpenConsole.exe' OR Name='WindowsTerminal.exe'" | ForEach-Object { $known[[int]$_.ProcessId] = $true }

$sb = New-Object System.Text.StringBuilder 256
$lastFgPid = -1
$deadline = (Get-Date).AddSeconds($Seconds)
Emit @{ t = (Get-Date).ToString('o'); ev = 'start'; label = $Label; seconds = $Seconds; tick_ms = $TickMs; preexisting_hosts = $known.Count }

while ((Get-Date) -lt $deadline) {
  # foreground sampling
  $h = [FW.Fg]::GetForegroundWindow()
  [uint32]$fgPid = 0; [void][FW.Fg]::GetWindowThreadProcessId($h, [ref]$fgPid)
  if ([int]$fgPid -ne $lastFgPid) {
    [void]$sb.Clear(); [void][FW.Fg]::GetClassName($h, $sb, 256); $cls = $sb.ToString()
    [void]$sb.Clear(); [void][FW.Fg]::GetWindowText($h, $sb, 256); $title = $sb.ToString()
    $pi = Get-ProcInfo ([int]$fgPid)
    Emit @{ t = (Get-Date).ToString('o'); ev = 'focus'; pid = [int]$fgPid; name = $pi.name; class = $cls; title = $title; chain = (Get-Chain ([int]$fgPid)) }
    $lastFgPid = [int]$fgPid
  }
  # new console hosts
  $hosts = Get-CimInstance Win32_Process -Filter "Name='conhost.exe' OR Name='OpenConsole.exe' OR Name='WindowsTerminal.exe'" -ErrorAction SilentlyContinue
  foreach ($hp in $hosts) {
    $id = [int]$hp.ProcessId
    if ($known.ContainsKey($id)) { continue }
    $known[$id] = $true
    $client = Get-ProcInfo ([int]$hp.ParentProcessId)
    $headless = ($hp.CommandLine -match '--headless')
    Emit @{
      t = (Get-Date).ToString('o'); ev = 'console'; host = $hp.Name; host_pid = $id; host_cmd = $hp.CommandLine
      created = ($hp.CreationDate ? $hp.CreationDate.ToString('o') : $null)
      hidden = $headless
      client_pid = $hp.ParentProcessId; client = $client.name; client_cmd = $client.cmd
      chain = (Get-Chain ([int]$hp.ParentProcessId))
    }
  }
  Start-Sleep -Milliseconds $TickMs
}
Emit @{ t = (Get-Date).ToString('o'); ev = 'end'; label = $Label }
