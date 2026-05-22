# Agents

Repo: flash-watcher

## Guardrails
- Keep changes small and reviewable.
- Put automation in `scripts/` or `tools/`.
- On Windows, if a `.ps1` sibling exists, prefer `pwsh -File <script>` over spawning `.cmd`/`.bat`; otherwise use `shell: true` (or `cmd.exe /c`) to avoid `spawn EINVAL`.

## No Python Replace for Newlines
Do not use ad‑hoc Python/regex replacements to edit files. For multiline/escape edits: show the 10–30 line snippet, edit directly, then re‑show the snippet (or `rg`) to verify.
LF is standard; CRLF only for .bat/.cmd.