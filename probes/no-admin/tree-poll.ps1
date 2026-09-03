# tree-poll.ps1 — fast new-process logger (no admin). Every tick: Get-Process (cheap) → for each new
# pid fetch ppid/cmdline/creation via CIM. Short-lived children are caught if they live ≳ one tick.
param([int]$Seconds = 300, [int]$TickMs = 40, [string]$Out = "$PSScriptRoot\tree-poll.jsonl")
$known = @{}
Get-Process | ForEach-Object { $known[$_.Id] = $true }
$deadline = (Get-Date).AddSeconds($Seconds)
while ((Get-Date) -lt $deadline) {
  $ps = Get-Process
  foreach ($p in $ps) {
    if ($known.ContainsKey($p.Id)) { continue }
    $known[$p.Id] = $true
    $ci = Get-CimInstance Win32_Process -Filter "ProcessId=$($p.Id)" -ErrorAction SilentlyContinue
    $rec = @{ t = (Get-Date).ToString('o'); pid = $p.Id; name = $p.ProcessName; ppid = $ci.ParentProcessId; cmd = $ci.CommandLine; created = ($ci.CreationDate ? $ci.CreationDate.ToString('o') : $null) }
    ($rec | ConvertTo-Json -Compress) | Add-Content -Path $Out -Encoding utf8
  }
  Start-Sleep -Milliseconds $TickMs
}
