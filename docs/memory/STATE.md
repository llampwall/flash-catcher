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

- Runtime testing with real ETW capture (requires elevation)
- Add more BUILTIN_RULES as new flash patterns are discovered
- v1.1 candidates: full stdio classification (PEB walk), working directory
- Consider `/api/rules` endpoint to expose BUILTIN_RULES via the running server

## Known deviations from spec (see DECISIONS.md)

- `classify_stdio` returns Unknown for all handles (PEB walk not implemented)
- `read_working_directory` returns None
- Both documented in `DECISIONS.md`

## Blockers

None — binary builds and runs.
