# flash-watcher — Session State

## Current objective

v0.1.0 shipped — full implementation complete.

## What was done (2026-05-22)

All 12 modules implemented and `cargo build --release` exits 0 with zero warnings.

**Commits (5) pushed to https://github.com/llampwall/flash-catcher.git:**
- `6efc855` feat(etw+process+admin)
- `36cdd20` feat(blame+classify)
- `d977b0f` feat(store+aggregate)
- `9447550` feat(web+frontend)
- `c44aa90` feat(cli+wiring)

**Binary:** `target/release/flash-watcher.exe`
**Dashboard:** http://127.0.0.1:7790/ when running `flash-watcher run`

## What remains

- Add more BUILTIN_RULES as new flash patterns are discovered
- Runtime false positives: explorer.exe/VSCode spawning Console helpers with CREATE_NO_WINDOW
  at runtime still shows visible_flash=true (needs PEB walk to detect CREATE_NO_WINDOW)
- v1.1 candidates: full stdio classification (PEB walk), working directory, CREATE_NO_WINDOW detection
- Consider `/api/rules` endpoint to expose BUILTIN_RULES via the running server

## Known deviations from spec (see DECISIONS.md)

- `classify_stdio` returns Unknown for all handles (PEB walk not implemented)
- `read_working_directory` returns None
- DC events are still ingested into the store/aggregator (just with visible_flash=false)
- Both documented in `DECISIONS.md`

## Working state (2026-05-22, session 3)

**Session 3 fix — commit 3bd4269:**
- `is_dc: bool` added to `RawEvent::ProcessStart`; set `true` for opcode 3 (DCStart)
- `visible_flash` now gated on `!is_dc` — eliminates entire DC burst false-positive set
  (services.exe×46, explorer.exe×28, node.exe×9 visible all go to 0 on restart)
- `pid_to_key` map tracks blame chain key per live pid
- ProcessExit now propagates `lifetime_ms` into aggregator via `update_lifetime()`
  (fixes total_console_time_ms always showing 0ms)
- `aggregate.rs` gains `update_lifetime(key, ms)` method

**Session 2 fixes (prior):**
- `blocking_send` panic in ETW callback → replaced with `handle.spawn()`
- ShellExecuteW/exit(0) relaunch removed from admin.rs
- Panic/error pause hook with 60s sleep fallback (stdin-safe)
- `EtwSession` guard keeps `KernelTrace` alive on stack in `run_collector`
- `C:\fw.log` entry probe for elevated-process diagnosis
- Log path changed to `C:\fw.log` (accessible from elevated context)
- Visible-flashes-only toggle in dashboard (default ON)

**6 fix commits pushed to origin/main** (b32bce2 → 6b54ac4)

## Bugfix (2026-05-22, session 2)

`b32bce2` fix(etw): replace blocking_send with handle.spawn — crash on first ETW event resolved.

Root cause: ferrisetw callbacks fire on a native OS thread; `blocking_send` called
`Handle::current()` internally → panic → elevated process closed immediately.
Fix: capture `Handle::current()` before callback, use `handle.spawn()` inside it.
Also fixed: retry branch had a no-op dummy callback; now uses real callback.

## Blockers

None — binary builds and runs. Crash-on-start bug fixed.
