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

## Console-window suppression on Windows: what works and what does not

This repo exists to attribute stray console windows. The findings below were **measured**
on this machine with `probes/no-admin/console-watch.ps1` plus a WMI process logger, not
reasoned out. Read this before proposing any fix — every approach under "does not work"
has already been tried here and looked correct while failing.

### The master rule

`DETACHED_PROCESS` and `CREATE_NO_WINDOW` are mutually exclusive in Win32. When both are
requested, **`CREATE_NO_WINDOW` is silently ignored**. In Node, `detached: true` maps to
`DETACHED_PROCESS` and `windowsHide: true` maps to `CREATE_NO_WINDOW`, so
`spawn(cmd, args, { detached: true, windowsHide: true })` does **not** hide anything.

The consequence is the actual bug, and it is counterintuitive: a console-**less** parent
(anything detached, any GUI-subsystem process such as `pythonw.exe`, any PM2-hosted app)
that spawns a **console-subsystem** child without an effective hide flag causes the OS to
allocate the child a **brand new visible console**. There is no parent console to inherit,
so Windows makes one. Asking for `detached + windowsHide` is therefore worse than asking
for neither.

A binary's subsystem is what decides, and it is readable from the PE header (offset
`e_lfanew + 4 + 20 + 68`): `2` = GUI, never allocates a console; `3` = CONSOLE, gets one.
`python.exe` and `node.exe` and `cmd.exe` are all `3`. `pythonw.exe` is `2`.

### What works

- **`windowsHide: true` WITHOUT `detached: true`.** The child gets its own hidden console.
  Call `.unref()` if it must outlive the parent.
- **Make the final binary GUI-subsystem** — spawn `pythonw.exe`, never `python.exe`, for
  anything long-lived. This is why the PM2 chinvex services are silent.
- **`cmd.exe /d`** whenever a *console-less* parent runs `cmd.exe`. `/d` skips AutoRun.
  `HKCU\Software\Microsoft\Command Processor\AutoRun` on this box runs conda's hook, which
  spawns `doskey.exe` — a console child this codebase never passes flags to, so it cannot
  be hidden any other way. Landed at `allmind-ignition/ignition.js:1165`.
  A `cmd.exe` that already has a hidden console does not need `/d`: `doskey` inherits it.

### What does NOT work (all tried, all failed)

- **Passing `detached: true` and `windowsHide: true` together.** See the master rule. This
  is the single most common wrong fix because it reads as belt-and-braces.
- **A `subprocess.Popen` monkey-patch that ORs in `CREATE_NO_WINDOW`** (chinvex
  `src/chinvex/__init__.py`). It cannot help where the flag is dropped by the master rule,
  and it cannot reach a child spawned by a non-Python launcher stub.
- **Passing `CREATE_NO_WINDOW` to a venv launcher** (`<venv>\Scripts\python.exe`, and any
  `Scripts\*.exe` console script). Those are ~249 KB **redirector stubs**, not copies of
  the interpreter — the real one is ~103 KB at `C:\Python313\`. The stub does its own
  `CreateProcess` of the base interpreter and **forwards none of the caller's creation
  flags**. Compare file sizes to tell a stub from a real interpreter.
- **PowerShell `-WindowStyle Hidden`, Task Scheduler `<Hidden>true</Hidden>`,
  `-NoProfile`.** These affect only the immediate process, never a grandchild. See
  `docs/WINDOW_FIX_VERIFICATION.md` for a fix that was declared "100% CONFIDENT" on these
  grounds and did not hold; treat that document as a historical record of a wrong
  conclusion, not as guidance.
- **flash-watcher itself as the sole instrument.** It was blind to the 2026-09-02 popups.
  Corroborate with `console-watch.ps1` and a WMI process logger.

### Proving a fix

A spawn is only proven fixed when re-run under the instrument that caught it. Follow the
repro convention in `probes/no-admin/repro/`: an odd-numbered `.vbs` reproduces the flash,
the next even number is its `-FIXED` counterpart, and the pair must self-test as exactly
one console handoff on the odd and zero on the even.
