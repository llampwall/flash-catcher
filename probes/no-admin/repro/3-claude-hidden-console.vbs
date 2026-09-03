' 3-claude-hidden-console.vbs — the candidate fix: same headless claude, windowsHide only (no detached),
' so claude owns a hidden console and everything under it inherits it.
' Expected (instrument said 3/3): nothing visible at all.
Option Explicit
Dim sh, fso, here, r, cmd
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
here = fso.GetParentFolderName(WScript.ScriptFullName)
r = MsgBox("REPRO 3: same headless claude, HIDDEN console (candidate fix)" & vbCrLf & vbCrLf & _
  "Click OK, then WATCH THE SCREEN for ~15 seconds." & vbCrLf & _
  "Expected: NOTHING visible.", vbOKCancel + vbInformation, "popup repro 3")
If r <> vbOK Then WScript.Quit 0
cmd = """C:\nvm4w\nodejs\node.exe"" """ & fso.BuildPath(fso.GetParentFolderName(here), "spawn-probe.mjs") & """ hidden --cwd P:\software\allmind -- C:\Users\Jordan\.local\bin\claude.exe -p ""reply ok"" --model haiku --no-session-persistence --strict-mcp-config"
sh.CurrentDirectory = here
sh.Run cmd, 0, True
MsgBox "Repro 3 finished. Did anything pop? (Expected: no.)", vbInformation, "popup repro 3"
