' 2-claude-detached-no-console.vbs — starts one headless claude the way mercenary.js:964 does
' (detached:true + windowsHide:true => DETACHED wins => claude has NO console).
' Expected (instrument said 3/3): a brief cmd.exe window/tab (~100 ms) about 1-2 s after start.
Option Explicit
Dim sh, fso, here, r, cmd
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
here = fso.GetParentFolderName(WScript.ScriptFullName)
r = MsgBox("REPRO 2: headless claude spawned DETACHED (no console)" & vbCrLf & vbCrLf & _
  "Click OK, then WATCH THE SCREEN for ~15 seconds." & vbCrLf & _
  "Expected: a brief cmd.exe window/tab flashes 1-2 s after start (Claude Code's startup REG QUERY)." & vbCrLf & vbCrLf & _
  "Runs one haiku turn that just replies 'ok'.", vbOKCancel + vbInformation, "popup repro 2")
If r <> vbOK Then WScript.Quit 0
cmd = """C:\nvm4w\nodejs\node.exe"" """ & fso.BuildPath(fso.GetParentFolderName(here), "spawn-probe.mjs") & """ detached-hidden --cwd P:\software\allmind -- C:\Users\Jordan\.local\bin\claude.exe -p ""reply ok"" --model haiku --no-session-persistence --strict-mcp-config"
sh.CurrentDirectory = here
sh.Run cmd, 0, True
MsgBox "Repro 2 finished. Did a cmd.exe window/tab flash?", vbInformation, "popup repro 2"
