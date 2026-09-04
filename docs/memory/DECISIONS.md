<!-- DO: Append new entries to current month. Rewrite Recent rollup. -->
<!-- DON'T: Edit or delete old entries. Don't log trivial changes. -->

# Decisions

## Recent (last 30 days)
- Investigation shifted from the ETW collector to an unelevated probe suite — flash-watcher did not catch the popups under investigation
- Visible console detected by the Windows Terminal `OpenConsole.exe -Embedding` handoff, sampled alongside the foreground window
- Three restart-time flash sources identified and fixed: `where.exe pwsh` in `lib/utils.js:29`, `cmd.exe` AutoRun doskey in ignition's cycle launcher, `nvidia-smi` in `ai-control-panel server.py:756`
- Repros shipped as numbered double-clickable `.vbs` pairs (unfixed / `-FIXED`), each self-tested under the same instruments
- PoolWatch task fix delivered as a UAC-elevating `.vbs` because `Set-ScheduledTask` and `schtasks /change` fail unelevated

## 2026-09

### 2026-09-02 — Attribute the three restart-time console flash sources and ship fixed repro pairs

- **Symptom:** Console windows flash across the screen during a broker restart; flash-watcher's ETW dashboard did not surface them
- **Root cause:** Broker restart 19:20–19:23 produced 16 Windows Terminal handoffs under `console-watch` plus a WMI process logger. 13 were `lib/utils.js:29` `where.exe pwsh` with no `windowsHide`, at boot of every PM2 node app, multiplied by telegram-bot crash-looping while the backend was down. 1 was `doskey.exe` from `cmd.exe` AutoRun (conda hook) inside ignition's console-less `start /min` cycle launcher. 1 was `ai-control-panel server.py:756` `nvidia-smi` without `CREATE_NO_WINDOW`, which fires on every GPU poll regardless of restart
- **Fix:** Numbered repro/`-FIXED` `.vbs` pairs 5–10 under `probes/no-admin/repro/`, each self-tested under the same instruments: exactly one handoff after 5, 7, 9 and none after 6, 8, 10
- **Prevention:** Any new console-spawn fix must be re-verified under `console-watch.ps1` plus the WMI logger, not by inspection and not by flash-watcher alone
- **Evidence:** 9683f18

### 2026-09-02 — Adopt an unelevated polling probe suite as the primary popup instrument

- **Why:** flash-watcher's ETW path missed the popups; a no-admin watcher that flags the Windows Terminal handoff (`OpenConsole.exe -Embedding`) and samples the foreground window catches them directly
- **Impact:** `probes/no-admin/` adds `console-watch.ps1` (polling watcher), `spawn-probe.mjs` (inherit/hidden/detached console modes), `run-on-desktop.ps1` (lpDesktop launcher), `tree-poll.ps1` and a second process-tree logger; repros 0–4 make each candidate double-clickable, including a guaranteed-popup control and the PoolWatch task
- **Evidence:** ac946cb, f905b75

### 2026-09-02 — PoolWatch task fix delivered as a UAC-elevating .vbs

- **Why:** `Set-ScheduledTask` and `schtasks /change` both return Access denied unelevated
- **Impact:** `probes/no-admin/repro/4-FIX-poolwatch-task-UAC.vbs` elevates pwsh onto `P:\software\bin\fix-poolwatch-task.ps1`, which swaps the task action to the wscript launcher and re-fires the task once for an eyes-on check
- **Evidence:** 4fea93d

## 2026-05

### 2026-05-23 — Switch ALLMIND dispatch from agent-spawn to internal invocation

- **Why:** Full-screen UAC elevation prompt would fire without user gate under agent-spawn; internal handler allows `requires_approval` gate and returns schtasks metadata instead of agent pid/output
- **Impact:** flash-watcher.* calls now routed through `lib/core/dispatch.js` in allmind; `start` sets `requires_approval: true`
- **Evidence:** d20edd2

### 2026-05-23 — Collector no longer backfills live aggregator on startup; DC events skip broadcast

- **Why:** Two bugs caused dashboard to "burst on open": (1) startup replayed all JSONL into live aggregator, surfacing rows that hadn't fired in days; (2) ~500 DC events flooded aggregator/ring/SSE within seconds of trace start
- **Impact:** Live view starts empty on each run; `Store::append` gains `broadcast: bool` parameter; DC events persist to JSONL only
- **Evidence:** ac47907

### 2026-05-23 — Publish agent.manifest.json; register flash-watcher with ALLMIND Core

- **Why:** Make flash-watcher dispatchable via Core registry/trace routing
- **Impact:** 5 capabilities (start, view, stop, status, classify-rules); `invocation_type=agent`, `runtime=rust`, `port=7790`, `requires_elevation=true`; capabilities.json rebuilt via `strap publish-manifest`
- **Evidence:** e831024

### 2026-05-22 — DC event false positives eliminated with is_dc gate

- **Symptom:** Services.exe×46, explorer.exe×28, node.exe×9 all showed `visible_flash=true` on dashboard open despite being hours-old processes
- **Root cause:** DC events (opcode 3, DCStart) for pre-existing processes passed through full pipeline; heuristic saw GUI parent → Console child and flagged them
- **Fix:** `is_dc: bool` added to `RawEvent::ProcessStart`; `visible_flash` gated on `!is_dc`; `pid_to_key` map added; `update_lifetime()` added to aggregator
- **Prevention:** Any new `visible_flash` predicate must check `!is_dc` first
- **Evidence:** 3bd4269

### 2026-05-22 — visible_flash heuristic: GUI parent spawns Console child

- **Why:** `classify_stdio` always returns Unknown (PEB walk not implemented), making Pipe/Null checks useless; the real signal for a visible flash is a Windows-subsystem parent creating a Console-subsystem child (which must allocate a new console window)
- **Impact:** `blame.rs` stores `subsystem` in `CachedNode`; `parent_subsystem(pid)` lookup added; orphan processes get own row via `self_name` fallback key
- **Evidence:** 2ab690f

### 2026-05-22 — ETW callback must use handle.spawn(), not blocking_send

- **Symptom:** Elevated process crashed immediately on first ETW event; window closed with no visible error
- **Root cause:** ferrisetw callbacks fire on native OS threads; `blocking_send` calls `Handle::current()` internally → panic "no current runtime"
- **Fix:** Capture `tokio::runtime::Handle::current()` before constructing the callback (on a tokio task), then use `handle.spawn()` inside the callback
- **Prevention:** Never call any tokio blocking API from a ferrisetw callback
- **Evidence:** b32bce2

### 2026-05-22 — Removed ShellExecuteW UAC relaunch from admin.rs

- **Why:** Auto-relaunch with `runas` verb causes double-elevation prompts and is unreliable in terminal contexts; callers are expected to launch from an elevated terminal
- **Impact:** `require_elevation_or_relaunch()` now bails with an error if not elevated; no silent self-relaunch
- **Evidence:** 055c0f1

### 2026-05-22 — EtwSession guard returned to run_collector caller

- **Why:** Original design leaked `KernelTrace` to a background thread; lifetime was unprovable and session could be dropped unexpectedly
- **Impact:** `start_kernel_session()` returns `(EtwSession, Receiver)`; caller holds the guard on the stack; no background thread needed
- **Evidence:** d42ffbe

### 2026-05-22 — classify_stdio and read_working_directory deferred to v1.1

- **Why:** Full implementation requires PEB walk (reading remote process memory); complexity out of scope for v0.1.0
- **Impact:** `classify_stdio` returns `Unknown` for all handles; `read_working_directory` returns `None`; documented in DECISIONS.md
- **Evidence:** c44aa90
