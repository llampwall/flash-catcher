<!-- DO: Add bullets. Edit existing bullets in place with (updated YYYY-MM-DD). -->
<!-- DON'T: Delete bullets. Don't write prose. Don't duplicate — search first. -->

# Constraints

## Infrastructure
- Web server binds port 7790 (axum); dashboard + REST API + SSE all served there
- Event store: `data/events.jsonl`; rotated to `data/events-<utc-ts>.jsonl.gz` when file exceeds 50 MB
- Log file: `C:\fw.log` — written from elevated context; always accessible regardless of working directory
- Binary: `target/release/flash-watcher.exe`; requires elevation at runtime
- Probe suite lives in `probes/no-admin/` and runs unelevated; numbered repros in `probes/no-admin/repro/` as double-clickable `.vbs` launchers

## Rules
- MUST run elevated — ETW kernel session requires admin; manifest sets `requires_elevation: true`
- ferrisetw callbacks fire on native OS threads (not tokio workers) — NEVER call `blocking_send` from callback; always capture `Handle::current()` before the callback and use `handle.spawn()` inside it
- DC events (opcode 3, DCStart) fire for every process already running when ETW trace starts — must be excluded from `visible_flash` calculation and from aggregator/ring/SSE broadcast; they are still persisted to JSONL
- Collector startup does NOT replay historical JSONL into the live aggregator — live dashboard starts empty; history is accessible via `flash-watcher view`
- ALLMIND dispatch uses `invocation_type: internal` (not agent-spawn) — flash-watcher.* calls are handled by `lib/core/dispatch.js` in allmind, not spawned as a subprocess
- Repro naming convention: odd-numbered `.vbs` reproduces the flash, the next even number is the `-FIXED` counterpart; each pair must self-test as exactly one handoff on the odd, zero on the even (added 2026-09-03)
- A spawn is only proven fixed when re-run under the same instrument that caught it — `console-watch.ps1` plus a WMI process logger (added 2026-09-03)

## Key Facts
- Dashboard URL: http://127.0.0.1:7790/
- ETW provider: kernel PROCESS_PROVIDER (opcode 1/3 = ProcessStart/DCStart, opcode 2/4 = ProcessExit/DCStop)
- Manifest registered in ALLMIND via `strap publish-manifest`; capabilities.json rebuilt after manifest changes
- `classify_stdio` returns `Unknown` for all handles — PEB walk not implemented (v1.1)
- `read_working_directory` returns `None` — not implemented (v1.1)
- Visible-console signature on this machine: a Windows Terminal handoff, i.e. `OpenConsole.exe -Embedding`; `probes/no-admin/console-watch.ps1` polls for it and samples the foreground window (added 2026-09-03)
- Confirmed restart-time flash sources (2026-09-02 broker restart, 16 handoffs): 13× `lib/utils.js:29` `where.exe pwsh` with no `windowsHide` at boot of every PM2 node app; 1× `doskey.exe` from `cmd.exe` AutoRun (conda hook) inside ignition's console-less `start /min` cycle launcher; 1× `ai-control-panel server.py:756` `nvidia-smi` without `CREATE_NO_WINDOW` (added 2026-09-03)
- `Set-ScheduledTask` and `schtasks /change` both return Access denied unelevated — the PoolWatch task fix runs via `probes/no-admin/repro/4-FIX-poolwatch-task-UAC.vbs` onto `P:\software\bin\fix-poolwatch-task.ps1` (added 2026-09-03)

## Hazards
- DC burst: ~500 synthetic ProcessStart events fire within seconds of trace start for every running process — these will flood dashboard and aggregator if not filtered by `is_dc`
- `CREATE_NO_WINDOW` gap: explorer.exe and VSCode spawn Console helpers at runtime with this flag; heuristic (GUI parent → Console child) marks them `visible_flash=true` incorrectly; no fix until PEB walk
- Elevated window closes on panic/crash without error visible — panic hook + 60s stdin-read fallback required in `main.rs`
- `start` capability sets `requires_approval: true` in manifest — full-screen UAC elevation prompt will fire without user gate if approval is skipped
- flash-watcher was blind to the 2026-09-02 popups: it is not a trustworthy sole instrument for popup attribution — corroborate with `console-watch.ps1` and a WMI process logger (added 2026-09-03)
- Node `spawn` with `detached: true` silently drops `windowsHide` — a console-less parent spawning a CUI child then gets a fresh VISIBLE console (added 2026-09-03)
- Sub-poll-interval spawns (cmd, reg, wmic dying in <50ms) fail `enrich_raw` — the PE subsystem read needs OpenProcess plus a file read and the process is already gone; without a name-table fallback these exact flashes are undercounted (added 2026-09-03)
- `/update-memory` bumping the STATE.md anchor is itself a commit, so the next run always sees "new commits" and bumps again — check whether the only delta is `docs/memory/` before writing anything (added 2026-09-06)
- `src/` carries an uncommitted conhost-pairing rework dated 2026-05-27 — the committed binary still uses the old GUI-parent heuristic; do not assume `main` behavior matches the working tree (added 2026-09-03)

## Superseded
