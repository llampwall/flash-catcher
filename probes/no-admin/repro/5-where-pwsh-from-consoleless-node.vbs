' 5-where-pwsh-from-consoleless-node.vbs — what every PM2-hosted AllMind node process does at boot:
' allmind lib/utils.js:29 runs execFileSync('where.exe', ['pwsh']) with NO windowsHide, from a
' process that has NO console (PM2 forks detached). Measured 2026-09-02 19:20-19:22: 13 of 16
' visible windows during one broker restart were this call (the WT window title was literally
' "C:\WINDOWS\system32\where.exe"), multiplied by telegram-bot crash-looping while the backend was down.
' Expected: ONE brief console window/tab flashes right after you click OK.
Option Explicit
Dim sh, fso, here, r, cmd
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
here = fso.GetParentFolderName(WScript.ScriptFullName)
r = MsgBox("REPRO 5: where.exe pwsh from a console-less node (utils.js:29 shape, NO windowsHide)" & vbCrLf & vbCrLf & _
  "Click OK, then WATCH THE SCREEN for ~3 seconds." & vbCrLf & _
  "Expected: one brief console window/tab flashes almost immediately.", vbOKCancel + vbInformation, "popup repro 5")
If r <> vbOK Then WScript.Quit 0
cmd = """C:\nvm4w\nodejs\node.exe"" """ & fso.BuildPath(fso.GetParentFolderName(here), "spawn-probe.mjs") & """ detached-hidden -- C:\nvm4w\nodejs\node.exe -e ""require('child_process').execFileSync('where.exe',['pwsh'],{encoding:'utf8'})"""
sh.CurrentDirectory = here
sh.Run cmd, 0, True
MsgBox "Repro 5 finished. Did a console window/tab flash?", vbInformation, "popup repro 5"
