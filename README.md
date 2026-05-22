# flash-watcher

Standalone Windows console-flash investigator. Headless ETW collector + local web UI for diagnosing the source of visible console-window flashes.

## Original Prompt

> Standalone Windows console-flash investigator: headless ETW (kernel process-start events, admin-elevated) collector that writes append-only JSONL, plus a local web UI at http://localhost:7790 with SSE live tail. Default landing view groups events by full blame-chain ancestry (cmd ← bash ← claude ← mercenary ← allmind) into aggregated, expandable rows with live counts, last-seen, total console-time; expand a row for per-spawn detail (full command line, executable path, creation flags, subsystem CONSOLE vs WINDOWS, session id, integration level, working directory, stdio handle types, conhost.exe pairing, spawn/exit timestamps, exit code). Sort by most-recent / highest-count / longest-lifetime. Same UI handles live and post-mortem because both read the same JSONL. Rust collector binary, decoupled from the viewer. Replaces the unreliable PM2-managed WMI-polling pwsh watcher we kept losing.

## Status

v0.1.0 — implemented 2026-05-22. Dashboard at **http://127.0.0.1:7790/** when running in collector or view mode.

## Dev

```powershell
# Build
cargo build --release

# Run collector + UI (must be elevated)
.\target\release\flash-watcher.exe run --bind 127.0.0.1:7790 --data-dir data --open

# View-only against existing JSONL (no admin)
.\target\release\flash-watcher.exe view --data-dir data --open

# Dump active classification rules
.\target\release\flash-watcher.exe classify-rules --pretty
```
