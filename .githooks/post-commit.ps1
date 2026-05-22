$ErrorActionPreference = 'Stop'

$env:CODE_HOME = 'C:\Users\Jordan\.codex'

function Get-RepoRoot {
  $root = & git rev-parse --show-toplevel 2>$null
  if (-not $root) { return $null }
  return $root.Trim()
}

$repoRoot = Get-RepoRoot
if (-not $repoRoot) { exit 0 }
Set-Location $repoRoot

$gitDirRaw = (& git rev-parse --git-dir).Trim()
if ([IO.Path]::IsPathRooted($gitDirRaw)) {
  $gitDir = $gitDirRaw
} else {
  $gitDir = Join-Path $repoRoot $gitDirRaw
}

$lockFile = Join-Path $gitDir 'maintainer-hook.lock'
$logFile = Join-Path $gitDir 'maintainer-hook.log'

function Resolve-CodexPath {
  if ($env:CODEX_BIN -and $env:CODEX_BIN.Trim() -ne '') { return $env:CODEX_BIN }
  $cmd = Get-Command codex.cmd -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }
  $cmd = Get-Command codex -ErrorAction SilentlyContinue
  if ($cmd -and -not $cmd.Source.ToLower().EndsWith('.ps1')) { return $cmd.Source }
  $cmd = Get-Command codex.cmd -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }
  return $null
}

$coderPath = Resolve-CodexPath
if ($coderPath) { $coderPath = ($coderPath -replace '[\r\n\0]+','').Trim() }
$coderExt = if ($coderPath) { ([IO.Path]::GetExtension($coderPath).ToLower() -replace '[\r\n\0]+','').Trim() } else { '' }

function Write-LogLine([string]$line) {
  if ([string]::IsNullOrWhiteSpace($line)) { return }
  $maxAttempts = 10
  for ($i = 0; $i -lt $maxAttempts; $i++) {
    try {
      $fs = [System.IO.File]::Open($logFile, [System.IO.FileMode]::Append, [System.IO.FileAccess]::Write, [System.IO.FileShare]::ReadWrite)
      try {
        $sw = New-Object System.IO.StreamWriter($fs)
        $sw.WriteLine($line)
        $sw.Flush()
      } finally {
        if ($sw) { $sw.Dispose() }
        $fs.Dispose()
      }
      break
    } catch {
      Start-Sleep -Milliseconds 200
    }
  }
}

function Log([string]$msg) {
  $ts = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
  $line = "[$ts] $msg"
  Write-LogLine $line
  if ($env:MAINTAINER_HOOK_ECHO -eq '1') { Write-Host $line }
}

function Strip-Ansi([string]$text) {
  if ($null -eq $text) { return $null }
  $t = [string]$text
  $t = $t -replace '\x1b\[[0-9;?]*[ -/]*[@-~]', ''
  $t = $t -replace '\x1b\][^\x07]*\x07', ''
  return $t
}

function Write-OutputLines([string]$text) {
  if ([string]::IsNullOrWhiteSpace($text)) { return }
  $text -split "`r?`n" | ForEach-Object {
    $clean = Strip-Ansi $_
    if ($clean) {
      Write-LogLine $clean
      if ($env:MAINTAINER_HOOK_ECHO -eq '1') { Write-Host $clean }
    }
  }
}

function Invoke-Coder([string]$prompt, [int]$timeoutSec) {
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = $coderPath
  $psi.UseShellExecute = $false
  $psi.RedirectStandardInput = $true
  $psi.RedirectStandardOutput = $false
  $psi.RedirectStandardError = $false
  $psi.CreateNoWindow = $true
  $psi.Environment['CI'] = '1'
  $psi.Environment['TERM'] = 'dumb'
  $psi.Environment['NO_COLOR'] = '1'
  if ($env:CODE_HOME -and $env:CODE_HOME.Trim() -ne '') { $psi.Environment['CODE_HOME'] = $env:CODE_HOME }
  $quotedPrompt = '"' + ($prompt -replace '"','""') + '"'
  $argString = 'exec --full-auto --sandbox workspace-write ' + $quotedPrompt
  Log ("codex args: " + $argString)
  Log ("codex cwd: " + $repoRoot)
  $psi.FileName = 'cmd.exe'
  $psi.Arguments = '/c ""' + $coderPath + '" ' + $argString + ' >> "' + $logFile + '" 2>>&1"'
  $psi.WorkingDirectory = $repoRoot
  $p = New-Object System.Diagnostics.Process
  $p.StartInfo = $psi
  $null = $p.Start()
  if (-not $p.WaitForExit($timeoutSec * 1000)) {
    try { & taskkill /T /F /PID $p.Id | Out-Null } catch { try { $p.Kill() } catch {} }
    Log ('codex timed out after ' + $timeoutSec + 's (killed process tree)')
    Remove-Item -Force -ErrorAction SilentlyContinue $lockFile
    return 124
  }
  return $p.ExitCode
}

function Commit-NotesIfChanged([string]$message) {
  $paths = @(
    'docs/project_notes/operating_brief.md',
    'docs/project_notes/key_facts.md',
    'docs/project_notes/adrs.md',
    'docs/project_notes/bugs.md',
    'docs/project_notes/worklog.md',
    'AGENTS.md',
    'CLAUDE.md'
  )
  $changes = & git status --porcelain -- $paths 2>$null
  if (-not $changes) { return $false }
  & git add -- $paths 2>$null | Out-Null
  & git diff --cached --quiet 2>$null
  if ($LASTEXITCODE -eq 0) { return $false }
  & git commit -m $message 2>$null | Out-Null
  if ($LASTEXITCODE -eq 0) {
    Log ("auto-committed notes: " + $message)
    return $true
  }
  return $false
}

$lockAgeMinutes = 10
if (Test-Path $lockFile) {
  $age = (Get-Date) - (Get-Item $lockFile).LastWriteTime
  if ($age.TotalMinutes -gt $lockAgeMinutes) {
    Remove-Item -Force -ErrorAction SilentlyContinue $lockFile
    Log "removed stale lock (older than $lockAgeMinutes minutes)"
  }
}

$env:Path = $env:Path + ';C:\Users\Jordan\AppData\Roaming\npm'

$subject = (& git log -1 --pretty=%s 2>&1).ToString().Trim()
if ($subject -eq '...') { Log 'skipping: subject "..."'; exit 0 }
if ($subject -match '\\[skip maintainer\\]') { Log 'skipping: [skip maintainer]'; exit 0 }
if ($subject -match '^(docs|notes):') { Log ("skipping maintainer for notes commit: " + $subject); exit 0 }
if ($subject -match '^(chore|ci|style|refactor|test)(\([^)]+\))?:') { Log ("skipping maintainer for low-signal commit: " + $subject); exit 0 }
if ($subject -match '(?i)\b(release|bump version)\b') { Log ("skipping maintainer for release/bump: " + $subject); exit 0 }
if ($subject -match '(?i)\b(dependabot|renovate)\b') { Log ("skipping maintainer for bot commit: " + $subject); exit 0 }
$parents = (& git show -s --pretty=%P 2>$null).ToString().Trim()
if ($parents -and ($parents -split '\s+').Count -gt 1) { Log 'skipping: merge commit'; exit 0 }
$changed = & git show -1 --name-only 2>$null
if ($changed -and ($changed | Where-Object { $_ -notmatch '^\\.githooks[\\\\/]' }) -eq $null) { Log 'skipping: .githooks-only commit'; exit 0 }
if ($changed -and ($changed | Where-Object { $_ -notmatch '^(docs/project_notes/|AGENTS\\.md$|CLAUDE\\.md$)' }) -eq $null) { Log 'skipping: notes/agents-only commit'; exit 0 }
if ($changed -and ($changed | Where-Object { $_ -notmatch '\.md$' }) -eq $null -and ($changed | Where-Object { $_ -match '^docs/project_notes/' }) -eq $null) { Log 'skipping: docs-only commit outside project_notes'; exit 0 }
$lockfiles = @(
  'package-lock.json','pnpm-lock.yaml','yarn.lock','bun.lockb','npm-shrinkwrap.json','composer.lock','poetry.lock','Pipfile.lock','Cargo.lock','Gemfile.lock','go.sum'
)
if ($changed -and ($changed | Where-Object { $lockfiles -notcontains $_ }) -eq $null) { Log 'skipping: lockfiles-only commit'; exit 0 }
if ($env:MAINTAINER_SKIP -eq '1') { Log 'skipping: MAINTAINER_SKIP=1'; exit 0 }
if (-not $env:MAINTAINER_ASYNC -or $env:MAINTAINER_ASYNC.Trim() -eq '') { $env:MAINTAINER_ASYNC = '1' }
if ($env:MAINTAINER_ASYNC -ne '0' -and $env:MAINTAINER_ASYNC_CHILD -ne '1') {
  Log 'spawning async maintainer'
  $env:MAINTAINER_ASYNC_CHILD = '1'
  $ps = if (Get-Command pwsh -ErrorAction SilentlyContinue) { 'pwsh' } else { 'powershell' }
  Start-Process -FilePath $ps -ArgumentList ('-NoProfile -ExecutionPolicy Bypass -File "' + $PSCommandPath + '"') -WindowStyle Hidden | Out-Null
  exit 0
}

if ($env:EVERYCODE_MAINTAINER_RUNNING -eq '1') { Log 'skipping: maintainer already running'; exit 0 }
if (Test-Path $lockFile) { Log 'skipping: lock file present'; exit 0 }

$env:EVERYCODE_MAINTAINER_RUNNING = '1'
New-Item -ItemType File -Force -Path $lockFile | Out-Null

try {
  Log 'post-commit hook start'
  $env:CODE_HOME = 'C:\Users\Jordan\.codex'
  Log ("CODE_HOME=" + $env:CODE_HOME)
  if (-not (Test-Path $coderPath)) { Log ("Codex not found: " + $coderPath); return }
  Log ("codex bin: " + $coderPath)
  Log ("codex ext: " + $coderExt)
  $subject = (& git log -1 --pretty=%s 2>&1).ToString().Trim()
  if ($subject -eq '...') {
    Log 'skipping: subject "..."'
    return
  }
  if ($subject -match '\\[skip maintainer\\]') {
    Log 'skipping: [skip maintainer]'
    return
  }
  if ($subject -match '^(docs|notes):') {
    Log ("skipping maintainer for notes commit: " + $subject)
    return
  }
  if ($subject -match '^(chore|ci|style|refactor|test)(\([^)]+\))?:') {
    Log ("skipping maintainer for low-signal commit: " + $subject)
    return
  }
  if ($subject -match '(?i)\b(release|bump version)\b') {
    Log ("skipping maintainer for release/bump: " + $subject)
    return
  }
  if ($subject -match '(?i)\b(dependabot|renovate)\b') {
    Log ("skipping maintainer for bot commit: " + $subject)
    return
  }
  $parents = (& git show -s --pretty=%P 2>$null).ToString().Trim()
  if ($parents -and ($parents -split '\s+').Count -gt 1) {
    Log 'skipping: merge commit'
    return
  }
  $changed = & git show -1 --name-only 2>$null
  if ($changed -and ($changed | Where-Object { $_ -notmatch '^\\.githooks[\\\\/]' }) -eq $null) {
    Log 'skipping: .githooks-only commit'
    return
  }
  if ($changed -and ($changed | Where-Object { $_ -notmatch '^(docs/project_notes/|AGENTS\\.md$|CLAUDE\\.md$)' }) -eq $null) {
    Log 'skipping: notes/agents-only commit'
    return
  }
  if ($changed -and ($changed | Where-Object { $_ -notmatch '\.md$' }) -eq $null -and ($changed | Where-Object { $_ -match '^docs/project_notes/' }) -eq $null) {
    Log 'skipping: docs-only commit outside project_notes'
    return
  }
  $lockfiles = @(
    'package-lock.json','pnpm-lock.yaml','yarn.lock','bun.lockb','npm-shrinkwrap.json','composer.lock','poetry.lock','Pipfile.lock','Cargo.lock','Gemfile.lock','go.sum'
  )
  if ($changed -and ($changed | Where-Object { $lockfiles -notcontains $_ }) -eq $null) {
    Log 'skipping: lockfiles-only commit'
    return
  }

  $requiredRel = @(
    'AGENTS.md',
    'CLAUDE.md',
    'docs/project_notes/operating_brief.md',
    'docs/project_notes/key_facts.md',
    'docs/project_notes/adrs.md',
    'docs/project_notes/bugs.md',
    'docs/project_notes/worklog.md'
  )
  $missing = $requiredRel | Where-Object { -not (Test-Path (Join-Path $repoRoot $_)) }
  if ($missing) {
    $bootstrapPrompt = (
      'You are a non-interactive maintainer. Use the project-context skill to create/update AGENTS.md and CLAUDE.md files in the root, and create/update the specified context files in docs/project_notes/.' +
      ' Do not touch any other paths.'
    )
    Log 'bootstrapping docs/project_notes'
    Log ("bootstrapping using: " + $coderPath)
    $exitCode = Invoke-Coder $bootstrapPrompt 600
    Log ("bootstrap exit code: " + $exitCode)
  }

  $summary = & git show -1 --name-status --stat --no-color 2>&1
  $full = & git show -1 --unified=0 --no-color 2>&1
  $summaryText = ($summary | ForEach-Object { Strip-Ansi $_ }) -join "`n"
  $fullText = ($full | ForEach-Object { Strip-Ansi $_ }) -join "`n"
  Log "commit summary:"
  $summary | ForEach-Object {
    $clean = Strip-Ansi $_
    if ($clean) {
      Write-LogLine $clean
      if ($env:MAINTAINER_HOOK_ECHO -eq '1') { Write-Host $clean }
    }
  }

  $promptLines = @(
    'You are a non-interactive maintainer. Use the project-context skill to create/update AGENTS.md and CLAUDE.md files in the root, and create/update the specified context files in docs/project_notes/.',
    'Rules:',
    '- Only edit those files (plus AGENTS.md and CLAUDE.md if required by the skill); do not touch anything else.',
    '- If nothing meaningful changed for the notes, make no edits.',
    '- Enforce no duplication: worklog links should not repeat; ADRs are constraints; key_facts are lookup truths; bugs are recurring/scary only.',
    '',
    'Commit summary:',
    $summaryText,
    '',
    'Commit diff:',
    $fullText
  )
  $prompt = $promptLines -join "`n"
  $maxPromptChars = 40000
  if ($prompt.Length -gt $maxPromptChars) {
    $keep = [Math]::Max(0, $maxPromptChars - 200)
    $prompt = $prompt.Substring(0, $keep) + "`n...[truncated]..."
  }

  Log 'running maintainer'
  Log ("maintainer using: " + $coderPath)
  $exitCode = Invoke-Coder $prompt 240
  Log ("maintainer exit code: " + $exitCode)
  Commit-NotesIfChanged 'docs: update project notes [skip maintainer]' | Out-Null
  Log 'post-commit hook end'
}
catch {
  Log ("error: " + $_.Exception.Message)
}
finally {
  Remove-Item -Force -ErrorAction SilentlyContinue $lockFile
  Remove-Item Env:EVERYCODE_MAINTAINER_RUNNING -ErrorAction SilentlyContinue
}
