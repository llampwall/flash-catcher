' 6-where-pwsh-from-consoleless-node-FIXED.vbs — identical to repro 5 but the execFileSync passes
' windowsHide:true (CREATE_NO_WINDOW), which is the one-line fix for allmind lib/utils.js:29.
' Expected: NOTHING visible.
Option Explicit
Dim sh, fso, here, r, cmd
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
here = fso.GetParentFolderName(WScript.ScriptFullName)
r = MsgBox("REPRO 6 (FIX): same where.exe pwsh call, but with windowsHide:true" & vbCrLf & vbCrLf & _
  "Click OK, then WATCH THE SCREEN for ~3 seconds." & vbCrLf & _
  "Expected: nothing visible at all.", vbOKCancel + vbInformation, "popup repro 6")
If r <> vbOK Then WScript.Quit 0
cmd = """C:\nvm4w\nodejs\node.exe"" """ & fso.BuildPath(fso.GetParentFolderName(here), "spawn-probe.mjs") & """ detached-hidden -- C:\nvm4w\nodejs\node.exe -e ""require('child_process').execFileSync('where.exe',['pwsh'],{encoding:'utf8',windowsHide:true})"""
sh.CurrentDirectory = here
sh.Run cmd, 0, True
MsgBox "Repro 6 finished. Did anything flash? (expected: no)", vbInformation, "popup repro 6"
