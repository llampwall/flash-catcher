# Claude Instructions

Repo: flash-watcher

- Be direct. Prefer simple solutions.
- Default to tests + lint + CI green.

Before searching for files with Glob/Grep, check docs/sys/lookup.json — a concept-to-files index. If your search term matches a key, you already know which files to read.

## Memory System

The `docs/memory/` directory holds three files that persist state across sessions:

- **STATE.md** — current truth: objective, active work, blockers, next actions. Rewrite freely.
- **CONSTRAINTS.md** — rules, hazards, infrastructure facts. Merge-only (never delete, move to Superseded).
- **DECISIONS.md** — audit trail of significant changes. Append-only for entries; rewrite the Recent rollup.

Run `/update-memory` to regenerate these files from git history.

## Rules

- When opening this repo, check if the brief shows `ACTION REQUIRED` — if so, offer to run `/update-memory`.
