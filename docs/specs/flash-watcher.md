# flash-watcher

**Version:** v0.1.0
**Spec Type:** greenfield
**Language/Runtime:** Rust (stable, edition 2021), Windows-only
**Created:** 2026-05-22

## Problem

Visible console-window flashes ("flashing windows") on the operator's Windows machine come from many sources — Claude Code's internal subprocess probes (`reg.exe MachineGuid`, `tasklist`, `Get-CimInstance Win32_Process`), chinvex ingest, AllMind background workers, hook-wrapper bash, and library shell-outs we don't control. Prior investigation used a PM2-managed `pwsh` script that polled WMI every 250ms — it silently died twice, missed sub-poll-interval spawns (cmd dies in 50ms, conhost lingers ~200ms — only the orphan conhost made it into the trace), and produced data ambiguous enough that we couldn't reliably attribute bursts to a source. We need a robust, standalone investigator that captures every spawn (including <10ms processes), groups them by full ancestry so a 50-flash burst becomes one expandable row, and exposes the data in a live + post-mortem UI.

## Solution

A single Rust binary, `flash-watcher`, that:

1. Subscribes to the Windows ETW kernel logger's process start/exit events (push-based, kernel-level — captures every spawn).
2. Enriches each raw event with Win32 process inspection (exe path, PE subsystem, integrity level, stdio handle kinds, working directory) to determine whether the spawn would actually produce a visible flash.
3. Walks the blame chain (pid → ppid → grandparent → root) using an in-process ancestry cache populated by every observed spawn, so parents that die before we can `OpenProcess` them are still resolvable.
4. Classifies each event against a built-in rule set (Claude Code probes, chinvex, our code, known-benign, unknown).
5. Appends every event to a rotating JSONL store at `data/events.jsonl` and broadcasts it to an embedded HTTP/SSE server on `127.0.0.1:7790`.
6. Serves a dark-themed web dashboard whose landing view is a **blame-chain aggregate table** — one row per unique ancestry path, with live counters, last-seen, total console-time, and an expand pane that shows per-spawn detail. Same dashboard handles live (SSE-driven) and post-mortem (JSONL-replay) modes because both paths read the same store.

Collector and viewer ship as one binary with three subcommands (`run`, `view`, `classify-rules`) — the "decoupled" property is satisfied by the shared JSONL substrate, not by separate processes.

## Technical Approach

### Capture: `src/etw.rs`

- Uses `ferrisetw::trace::KernelTrace` with the Process provider mask (`EVENT_TRACE_FLAG_PROCESS`).
- Session name: `flash-watcher-kernel` (a constant). Windows only allows one active kernel logger session per name — `stop_session()` is exposed for cleanup on Ctrl-C and for forcing recovery if a prior run leaked the session.
- Emits `RawEvent::{ProcessStart, ProcessExit}` over an `mpsc::Receiver` consumed by the collector loop.
- ETW provides pid, ppid, image file name, command line, and timestamp. Everything else (exe path, subsystem, integrity, stdio, cwd, creation flags) requires Win32 API calls against the live process via `src/process.rs::enrich`.

### Enrichment: `src/process.rs`

- `enrich(pid, ppid, image_file_name, command_line) -> ProcessInfo` opens the process with `PROCESS_QUERY_LIMITED_INFORMATION`. Short-lived processes (<50ms) frequently exit before the handle can be opened — when that happens the returned `ProcessInfo` carries whatever ETW provided and `Unknown` / `None` for every field that requires an open handle. This is acceptable: blame chain is still resolvable from the cache, and the JSONL captures that the spawn happened.
- `read_subsystem(exe_path) -> Subsystem` memory-maps the PE header and reads `IMAGE_OPTIONAL_HEADER.Subsystem`. Only `IMAGE_SUBSYSTEM_WINDOWS_CUI` (Console) produces visible flashes. This is what drives `FlashEvent.visible_flash`.
- `classify_stdio(pid) -> StdioHandles` uses `DuplicateHandle` + `NtQueryObject` to classify stdin/stdout/stderr handle types. A CONSOLE-subsystem process whose stdout is a Pipe will NOT actually flash even though its subsystem says it would (this is how shims like our pwsh-shim silence things). The `visible_flash` boolean combines subsystem + stdio analysis.
- `read_integrity_level`, `read_working_directory` are best-effort and may return defaults when the handle can't be opened.

### Blame chain: `src/blame.rs`

- `BlameCache` is a `DashMap<pid, CachedNode>` populated by every ProcessStart event. Never evicted — short-lived ancestors must remain resolvable for hours of cumulative captures.
- `walk(pid)` loops pid → ppid through the cache, building `Vec<BlameNode>`. If a parent is missing from the cache (collector started mid-system-lifetime), falls back to a one-shot Win32 `Process32First/Next` lookup.
- `chain_key(&nodes)` produces the deterministic UI grouping key: lowercased exe names joined with `<-`, e.g. `cmd.exe<-bash.exe<-claude.exe<-mercenary<-allmind`. This is the primary key for the aggregator.
- `mark_exited(pid)` tombstones rather than removes — late-arriving child events with this pid as their ppid must still resolve.

### Classification: `src/classify.rs`

- `BUILTIN_RULES` is a `&'static [ClassifyRule]` — match top-down, first hit wins. Rules combine `name_eq` (exact, case-insensitive on `process.name`), `cmdline_contains` (substring, case-insensitive), and `ancestor_name_eq` (any ancestor in the chain).
- Initial rule set covers the patterns identified in prior investigation: `reg.exe MachineGuid`, `tasklist.exe` under claude.exe, `powershell.exe Get-CimInstance Win32_Process`, chinvex.exe ancestors. New patterns are added here, not via runtime config — `flash-watcher classify-rules --pretty` dumps the active rules for inspection.
- Unknown classifications are the high-value signal: the UI emphasizes them so the operator can drive the rule set forward.

### Storage: `src/store.rs`

- `Store::open(data_dir)` creates `data_dir/events.jsonl` in append mode and a `broadcast::Sender<FlashEvent>` with bounded capacity (256). SSE subscribers receive new events; lagging subscribers drop events rather than blocking the collector.
- `append(&event)` serializes one JSON line, writes + flushes, and broadcasts. Disk-write failures bubble up to the collector loop, which logs and continues (we'd rather miss disk persistence than crash the capture).
- `rotate_if_needed(max_bytes)` checks the active file size; when exceeded, renames to `events-<utc-ts>.jsonl`, gzips it, and reopens the writer. Called once per minute by a timer task.
- `read_all(data_dir)` globs `events*.jsonl(.gz)?`, parses oldest-first, returns `Vec<FlashEvent>`. Used by `view` mode and by the initial-page-load backfill.

### Aggregation: `src/aggregate.rs`

- `Aggregator` holds `HashMap<chain_key, BlameChainRow>`. `ingest(&event)` upserts by `event.blame.key`, bumps `count` and `visible_count`, updates `last_seen`, adds `lifetime_ms` to `total_console_time_ms`, and pushes `event_id` to `recent_event_ids` (capped at 50, FIFO).
- `snapshot(sort)` clones values into a `Vec` and sorts by `MostRecent` (default), `HighestCount`, or `LongestLifetime`. Snapshots are cheap — the aggregator is in-process behind a `Mutex` and rows count is bounded by distinct ancestry paths (low hundreds in practice).
- The aggregator is built incrementally by the collector loop and is the source of truth for `GET /api/rows`. On startup in `run` mode, the collector replays the on-disk JSONL through the aggregator before opening the ETW session so historical bursts are visible from the first page load.

### Web server: `src/web.rs`

`build_router(state) -> Router` composes:

| Method | Path | Handler | Purpose |
|---|---|---|---|
| GET | `/` | `index_handler` | serves embedded `web/index.html` |
| GET | `/static/*file` | tower-http `ServeDir` over embedded assets | `app.js`, `styles.css` |
| GET | `/api/rows?sort=…` | `rows_handler` | aggregated blame-chain rows |
| GET | `/api/events/:event_id` | `event_detail_handler` | full per-spawn detail |
| GET | `/api/events?chain=<key>&limit=<n>` | `events_for_chain_handler` | events under one row |
| GET | `/api/stream` | `sse_stream_handler` | live FlashEvent stream (SSE) |
| GET | `/api/health` | `health_handler` | etw active? store path? counts? |

`serve(state, addr)` binds with `tokio::net::TcpListener::bind`, runs `axum::serve`, and shuts down gracefully on Ctrl-C.

### Admin gate: `src/admin.rs`

- `is_elevated()` opens the current process token with `OpenProcessToken` + `GetTokenInformation(TokenElevation)`.
- `require_elevation_or_relaunch()` — in `run` mode, if not elevated, calls `relaunch_elevated()` (`ShellExecuteW` with `runas` verb so Windows shows a UAC prompt), and the parent exits cleanly. If the user declines elevation, the parent prints a message pointing at `flash-watcher view` (no admin required) and exits with code 1.
- `view` mode never calls this — view-only is read-only against the JSONL and runs unprivileged.

### CLI: `src/cli.rs`

Subcommands (already scaffolded with `clap` derive):

- `run --bind 127.0.0.1:7790 --data-dir data [--open] [--skip-admin-check]` — admin gate → ETW start → web server → optional browser-open
- `view --bind 127.0.0.1:7790 --data-dir data [--open]` — UI-only against existing JSONL, no ETW
- `classify-rules [--pretty]` — dump active classification rules as JSON

### Web UI: `web/index.html` + `web/app.js` + `web/styles.css`

- Landing table renders rows returned by `GET /api/rows`. Each row shows chain, count, visible count, last-seen, total console-time, classification. Unknown rows render with a bright accent border.
- Sort `<select>` triggers `GET /api/rows?sort=…` and replaces the table body.
- Classification `<select>` filters client-side (no server roundtrip).
- Pause button suspends the SSE update loop without closing the stream. Clear-view empties the local DOM table but doesn't touch the store.
- Each row is clickable — expand pane uses the `<template id="detail-template">` and loads `GET /api/events?chain=<key>&limit=50` for the per-spawn detail table.
- Live updates: `EventSource('/api/stream')` keeps the connection open; each incoming event is folded into the local row state and the affected row's DOM is updated in place. The server-side aggregator stays the source of truth — on reconnect the client re-fetches `/api/rows`.

## Interaction Flows

Flow 1: Launch live capture (happy path)
1. Operator runs `flash-watcher run --open` from a non-elevated shell → binary checks `is_elevated()`, sees false → calls `relaunch_elevated()` → Windows shows UAC prompt → operator accepts.
2. Elevated child process starts → starts ETW kernel session named `flash-watcher-kernel` → opens JSONL store at `./data/events.jsonl` → replays existing JSONL into the aggregator → binds web server on `127.0.0.1:7790` → opens the default browser to `http://127.0.0.1:7790/`.
3. Browser loads `index.html` → `GET /api/rows?sort=most-recent` populates the table → `EventSource('/api/stream')` opens → status bar shows `ETW: active, N events, M chains`.
4. A new spawn happens → ETW emits ProcessStart → enrich+blame+classify → store.append → broadcast → SSE pushes event → browser folds it into the matching row, count ticks up, last-seen updates, row sorts to top.

Flow 2: Inspect a burst (happy path)
1. Operator sees `cmd.exe<-claude.exe<-pwsh-real.exe<-…` row count tick from 12 → 27 in five seconds.
2. Operator clicks the row → client issues `GET /api/events?chain=cmd.exe<-claude.exe<-pwsh-real.exe<-…&limit=50` → detail pane renders per-spawn rows with timestamp, lifetime, exit code, subsystem, stdout handle kind, full command line.
3. Operator sees every row has `cmdLine` matching `reg.exe MachineGuid` → confirms Claude Code probe → no new rule needed.

Flow 3: View post-mortem (happy path, no admin)
1. Operator runs `flash-watcher view --data-dir P:\software\flash-watcher\data --open` from an unprivileged shell → no admin check → opens the web UI.
2. Page load → `GET /api/rows` returns the aggregator snapshot, which was built by replaying every `events*.jsonl(.gz)` in `data_dir` on startup → operator browses historical bursts and expands rows for detail.
3. No SSE updates arrive (collector isn't running) → status bar shows `ETW: inactive (view mode)` → table is static.

Flow 4: UAC declined (error path)
1. Operator runs `flash-watcher run` → UAC prompt → operator clicks "No".
2. `relaunch_elevated()` returns `Ok(false)` → parent prints `"Admin required for ETW capture. For UI-only access to existing data, run: flash-watcher view"` to stderr and exits with code 1.
3. No ETW session created, no web server bound, no half-state on disk.

Flow 5: Prior session leaked (error path, recovered)
1. Operator runs `flash-watcher run` → admin granted → `start_kernel_session()` calls into ferrisetw → ETW returns `ERROR_ALREADY_EXISTS` (a previous run crashed without `Drop`-ing the session).
2. Collector startup logic catches the specific error → calls `stop_session("flash-watcher-kernel")` to force-stop the leaked session → retries `start_kernel_session()` once → succeeds.
3. UI loads normally; collector logs a warning line about the recovery for diagnostics.

Flow 6: Short-lived process captured (edge case)
1. A library shell-out spawns `cmd.exe` that exits in 12ms — too fast for any polling approach.
2. ETW ProcessStart fires at T+0ms → `enrich()` tries `OpenProcess` → fails with `ERROR_INVALID_PARAMETER` (process already exited) → returns ProcessInfo with name/cmdline from ETW and `Unknown`/`None` for the open-handle fields.
3. `blame.walk(pid)` resolves the parent chain from `BlameCache` (parent was recorded when it started, before this cmd.exe) → full chain available.
4. Event written to JSONL with `visible_flash = true` if classification rules + ETW image data indicate a CONSOLE subsystem binary → row updates in UI.

Flow 7: Inspect classification rules (happy path)
1. Operator runs `flash-watcher classify-rules --pretty` → binary calls `classify::dump_rules(true)` → prints JSON array of all `BUILTIN_RULES` to stdout → operator pipes to a file or grep to confirm what's recognized.

Flow 8: Conhost pairing detail (edge case)
1. A console-subsystem child spawns conhost.exe as a sibling (Windows allocates one per console process).
2. Collector observes both ProcessStart events within a few ms; pairing logic in `etw.rs` matches conhost to its allocator (same session, conhost spawned within 100ms of the allocator, conhost's parent matches the allocator's session host).
3. Resulting `FlashEvent.conhost` is populated → detail pane shows the paired conhost pid + spawned_at + exited_at → operator can confirm a visible window actually opened (vs a CONSOLE-subsystem process whose stdout is a pipe and therefore reuses an existing console).

## Wiring Map

```json
{
  "existing_seams": [
    { "file": "src/main.rs", "symbol": "run_collector", "action": "implement", "reason": "stub: wire admin gate -> ETW session -> store -> web server" },
    { "file": "src/main.rs", "symbol": "run_viewer", "action": "implement", "reason": "stub: read JSONL into aggregator, serve UI without ETW" },
    { "file": "src/main.rs", "symbol": "print_classify_rules", "action": "implement", "reason": "stub: call classify::dump_rules and print" },
    { "file": "src/admin.rs", "symbol": "is_elevated", "action": "implement", "reason": "OpenProcessToken + GetTokenInformation(TokenElevation)" },
    { "file": "src/admin.rs", "symbol": "relaunch_elevated", "action": "implement", "reason": "ShellExecuteW runas, exit parent" },
    { "file": "src/admin.rs", "symbol": "require_elevation_or_relaunch", "action": "implement", "reason": "compose is_elevated + relaunch_elevated for run mode" },
    { "file": "src/etw.rs", "symbol": "start_kernel_session", "action": "implement", "reason": "ferrisetw KernelTrace Process mask -> mpsc::Receiver<RawEvent>; recover on ERROR_ALREADY_EXISTS" },
    { "file": "src/etw.rs", "symbol": "enrich_raw", "action": "implement", "reason": "delegate to process::enrich" },
    { "file": "src/etw.rs", "symbol": "stop_session", "action": "implement", "reason": "ControlTrace EVENT_TRACE_CONTROL_STOP for leaked-session recovery" },
    { "file": "src/process.rs", "symbol": "enrich", "action": "implement", "reason": "OpenProcess + image-name + cwd + token + stdio; tolerate handle-open failure" },
    { "file": "src/process.rs", "symbol": "read_subsystem", "action": "implement", "reason": "mmap PE header, return Console/Windows" },
    { "file": "src/process.rs", "symbol": "read_integrity_level", "action": "implement", "reason": "OpenProcessToken + GetTokenInformation(TokenIntegrityLevel)" },
    { "file": "src/process.rs", "symbol": "classify_stdio", "action": "implement", "reason": "DuplicateHandle + NtQueryObject for stdin/stdout/stderr" },
    { "file": "src/process.rs", "symbol": "read_working_directory", "action": "implement", "reason": "NtQueryInformationProcess + PEB walk" },
    { "file": "src/blame.rs", "symbol": "BlameCache::record", "action": "implement", "reason": "DashMap insert" },
    { "file": "src/blame.rs", "symbol": "BlameCache::mark_exited", "action": "implement", "reason": "tombstone for lazy GC" },
    { "file": "src/blame.rs", "symbol": "BlameCache::walk", "action": "implement", "reason": "loop ppid lookups, fallback to Process32 snapshot" },
    { "file": "src/blame.rs", "symbol": "chain_key", "action": "implement", "reason": "lowercased exe names joined with '<-'" },
    { "file": "src/classify.rs", "symbol": "classify", "action": "implement", "reason": "match BUILTIN_RULES top-down, return (Classification, rule_name)" },
    { "file": "src/store.rs", "symbol": "Store::open", "action": "implement", "reason": "mkdir, open append handle, create broadcast channel" },
    { "file": "src/store.rs", "symbol": "Store::append", "action": "implement", "reason": "serialize line, write+flush, broadcast" },
    { "file": "src/store.rs", "symbol": "Store::read_all", "action": "implement", "reason": "glob events*.jsonl(.gz)?, parse lines" },
    { "file": "src/store.rs", "symbol": "Store::rotate_if_needed", "action": "implement", "reason": "size check, rename+gzip+reopen" },
    { "file": "src/aggregate.rs", "symbol": "Aggregator::ingest", "action": "implement", "reason": "upsert by chain key, update counters" },
    { "file": "src/aggregate.rs", "symbol": "Aggregator::snapshot", "action": "implement", "reason": "clone+sort per SortBy" },
    { "file": "src/web.rs", "symbol": "build_router", "action": "implement", "reason": "compose all handlers + ServeDir for embedded /static" },
    { "file": "src/web.rs", "symbol": "serve", "action": "implement", "reason": "tokio TcpListener bind + axum::serve + Ctrl-C shutdown" },
    { "file": "src/web.rs", "symbol": "index_handler", "action": "implement", "reason": "include_str! of web/index.html with text/html" },
    { "file": "src/web.rs", "symbol": "rows_handler", "action": "implement", "reason": "parse_sort + aggregator.snapshot" },
    { "file": "src/web.rs", "symbol": "event_detail_handler", "action": "implement", "reason": "lookup by event_id across in-memory ring + JSONL" },
    { "file": "src/web.rs", "symbol": "events_for_chain_handler", "action": "implement", "reason": "filter by blame.key, return last N" },
    { "file": "src/web.rs", "symbol": "sse_stream_handler", "action": "implement", "reason": "store.subscribe -> map FlashEvent -> Sse::Event::json" },
    { "file": "src/web.rs", "symbol": "health_handler", "action": "implement", "reason": "ETW alive flag + counts + uptime" },
    { "file": "src/web.rs", "symbol": "parse_sort", "action": "implement", "reason": "match raw query to SortBy, default MostRecent" },
    { "file": "src/event.rs", "symbol": "FlashEvent::new_id", "action": "implement", "reason": "ulid or monotonic id string" },
    { "file": "web/app.js", "symbol": "default", "action": "implement", "reason": "EventSource client, sort/filter, expand pane, paused-state" },
    { "file": "web/styles.css", "symbol": "default", "action": "implement", "reason": "dark theme; unknown-class accent border; row hover/expand styles" },
    { "file": "web/index.html", "symbol": "default", "action": "implement", "reason": "already scaffolded shape; final polish pass during UI implementation" }
  ],
  "new_modules": [
    { "file": "src/conhost.rs", "exports": ["pair_conhost"], "imports": ["crate::event::ConhostPairing"] }
  ],
  "public_interfaces": [
    { "type": "route", "path": "/", "method": "GET", "description": "Serves the landing-view HTML" },
    { "type": "route", "path": "/static/*file", "method": "GET", "description": "Serves embedded web assets (app.js, styles.css)" },
    { "type": "route", "path": "/api/rows", "method": "GET", "description": "Aggregated blame-chain rows; query: sort=most-recent|highest-count|longest-lifetime" },
    { "type": "route", "path": "/api/events/:event_id", "method": "GET", "description": "Full FlashEvent detail by id" },
    { "type": "route", "path": "/api/events", "method": "GET", "description": "Events for one blame-chain row; query: chain=<key>&limit=<n>" },
    { "type": "route", "path": "/api/stream", "method": "GET", "description": "SSE stream of new FlashEvents as the collector appends them" },
    { "type": "route", "path": "/api/health", "method": "GET", "description": "Collector + store diagnostics (ETW active, store path, counts, uptime)" }
  ]
}
```

## Acceptance Criteria

- [ ] `cargo build --release` produces `target/release/flash-watcher.exe` with no warnings beyond unused-imports (resolved during implementation).
- [ ] `flash-watcher run` from a non-elevated shell triggers a UAC prompt and on accept runs elevated; on decline exits with code 1 and a message pointing at `view` mode.
- [ ] Once the collector is running, `curl http://127.0.0.1:7790/api/health` returns `{etw_session_active: true, store_path: "<resolved data dir>", total_events: …, aggregator_rows: …, uptime_seconds: …}`.
- [ ] Spawning a known short-lived process (e.g. `cmd.exe /c exit 0`) from an unrelated process produces a corresponding entry in `data/events.jsonl` within 1 second, with non-null pid, ppid, name="cmd.exe", and a populated `blame.ancestors` array.
- [ ] `GET /api/rows?sort=highest-count` returns rows sorted by `count` descending; switching to `?sort=most-recent` returns them sorted by `last_seen` descending.
- [ ] Opening the dashboard in a browser shows a live count that increases as new spawns happen, without a page reload (SSE).
- [ ] Expanding a row issues `GET /api/events?chain=<key>` and renders per-spawn detail with command line, lifetime, exit code, subsystem, stdout handle kind.
- [ ] `flash-watcher view --data-dir <path with events.jsonl>` opens the dashboard against the existing JSONL with no admin prompt and no ETW session created.
- [ ] `flash-watcher classify-rules --pretty` prints the BUILTIN_RULES as indented JSON to stdout and exits 0.
- [ ] After a leaked ETW session (simulated by starting collector twice without graceful shutdown), the second `run` recovers by calling `stop_session` and starting cleanly, logging a single warning.
- [ ] When `data/events.jsonl` exceeds the configured rotation threshold, it is renamed to `events-<utc-ts>.jsonl`, gzipped to `.gz`, and a fresh `events.jsonl` is opened — verified by inspecting the data dir after capture.

## Constraints

- Windows-only. Cargo target = `x86_64-pc-windows-msvc`. `cfg(target_os = "windows")` is implicit; no cross-platform pretense.
- ETW kernel logger requires admin (the `runas`/UAC path is the supported launch mechanism). View mode is the unprivileged fallback.
- Bind address defaults to `127.0.0.1:7790`. Do not expose to LAN — there is no auth, and the dashboard reveals every process command line on the machine.
- Single binary; web assets are embedded at compile time (`include_str!`). No runtime dependency on the `web/` directory.
- JSONL is the canonical record. Aggregator is derived state — must be rebuildable from the JSONL on startup.
- `BlameCache` is allowed to grow unboundedly across a single capture session; processes can be arbitrarily deep and we'd rather use memory than lose ancestry. Acceptable because a session is bounded by process lifetime.
- The store's broadcast channel uses a bounded capacity (256); SSE subscribers that lag drop events rather than blocking the collector.

## Out of Scope (v1)

- Linux/macOS support.
- WMI fallback path (we chose ETW + admin specifically; if admin is unavailable, the operator uses `view` mode against an existing JSONL).
- Per-event ack/dismiss UI ("mark this burst as known"). Classification rules are the durable mechanism; adding them is a code change, not a runtime config.
- Authentication / multi-user dashboard. Loopback-only.
- Remote ETW capture (capturing from machine A, viewing on machine B).
- CSV / Parquet export. JSONL is the format; consumers can `jq` it.
- Native GUI (egui/iced/Tauri). The browser dashboard is the only frontend.
- PM2 integration. v1 is launched manually with UAC; if it proves valuable enough to want auto-start, that's a follow-up.
- Stdout/stderr capture of the spawned processes themselves. We capture spawn metadata, not their output.

## Notes

- `src/conhost.rs` is the one new module flagged in the wiring map. It carries the pairing logic between a CONSOLE-subsystem process and its allocated conhost.exe sibling. Could be folded into `src/etw.rs` if the pairing logic stays under ~100 LOC — decide during implementation.
- The `chinvex-tunnel` and `pythonw → os.system` cases identified in prior investigation will surface here as Unknown-classified rows until rules are added — that's the point. The dashboard is built to expose them.
- Classification rule format intentionally avoids regex on cmdline at v1; substring + exact-name is sufficient for the patterns observed so far. If a future pattern needs regex, extend `ClassifyRule` then.
- `event.visible_flash` is the field downstream consumers care about — counts in the UI also break out a `visible_count` so the operator can distinguish "100 spawns of which only 3 visually flashed" from "100 visible flashes."
- The pairing logic for conhost vs allocator depends on Windows session id + tight timing — this is heuristic, not API-guaranteed. Document the heuristic in `src/conhost.rs` when implementing so the assumption is visible.
