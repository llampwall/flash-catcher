<!-- DO: Rewrite freely. Keep under 30 lines. Current truth only. -->
<!-- DON'T: Add history, rationale, or speculation. No "we used to..." -->

# State

## Current Objective
Attribute and eliminate the console windows that flash on this machine. The ETW collector (v0.1.0) is shipped; the active instrument is the no-admin probe suite under `probes/`.

## Active Work
- Uncommitted rework in `src/` (dated 2026-05-27): conhost.exe pairing as visibility ground truth, replacing the GUI-parent→Console-child heuristic. Compiles; 2 dead-code warnings (`blame.rs` `subsystem` field, `parent_subsystem`).
- Untracked `probes/codex-popup-derisk.ps1` + `codex-popupfix-trigger.ps1` (2026-06-05): A/B de-risk of the codex console cascade and a Jordan-paced live trigger for the mercenary `detached: backend !== 'codex'` fix.

## Blockers
- flash-watcher itself did not catch the popups under investigation — the polling `console-watch.ps1` (Windows Terminal `OpenConsole.exe -Embedding` handoff) is what actually detected them.

## Next Actions
- [ ] Decide whether to commit or drop the 2026-05-27 conhost-pairing rework; clear its 2 warnings if committing
- [ ] Verify the conhost-pairing build against the three known repro pairs in `probes/no-admin/repro/`
- [ ] Commit or discard the untracked codex popup probes and their logs
- [ ] Fold the confirmed flash sources into `BUILTIN_RULES`

## Quick Reference
- Run: `flash-watcher run` (must be elevated)
- Build: `cargo build --release` / check: `cargo check`
- Dashboard: http://127.0.0.1:7790/ (health: `/api/health`)
- Entry point: `src/main.rs`; probes: `probes/no-admin/`

## Out of Scope (for now)
- Full stdio classification via PEB walk (v1.1)
- `read_working_directory` (returns None, v1.1)
- ConPTY shim for codex — only if the hidden-console inheritance fix fails

---
Last memory update: 2026-09-03
Commits covered through: 9683f18873298965edac9deff4a4219056374428

<!-- chinvex:last-commit:9683f18873298965edac9deff4a4219056374428 -->
