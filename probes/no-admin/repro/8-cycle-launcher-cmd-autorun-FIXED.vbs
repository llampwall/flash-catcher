' 8-cycle-launcher-cmd-autorun-FIXED.vbs — identical to repro 7 but cmd.exe is started with /d,
' which disables AutoRun, so no doskey.exe is ever launched. This is the fix for ignition.js:1052.
' Expected: NO flash. Only the minimized pwsh in the taskbar for ~3 s.
Option Explicit
Dim sh, fso, here, r, cmd
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
here = fso.GetParentFolderName(WScript.ScriptFullName)
r = MsgBox("REPRO 8 (FIX): same cycle launcher, cmd.exe /d (AutoRun off)" & vbCrLf & vbCrLf & _
  "Click OK, then WATCH THE SCREEN for ~4 seconds." & vbCrLf & _
  "Expected: no flash. Only a minimized pwsh in the taskbar for 3 s.", vbOKCancel + vbInformation, "popup repro 8")
If r <> vbOK Then WScript.Quit 0
cmd = """C:\nvm4w\nodejs\node.exe"" """ & fso.BuildPath(fso.GetParentFolderName(here), "cycle-launcher-probe.mjs") & """ --fixed"
sh.CurrentDirectory = here
sh.Run cmd, 0, True
WScript.Sleep 4000
MsgBox "Repro 8 finished. Did anything flash? (expected: no)", vbInformation, "popup repro 8"
