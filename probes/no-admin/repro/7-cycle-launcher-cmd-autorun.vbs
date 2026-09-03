' 7-cycle-launcher-cmd-autorun.vbs — allmind-ignition's launchDetachedCycle (ignition.js:1052):
' a console-less cmd.exe runs `start /min pwsh ...`. Before `start` even runs, cmd.exe's AutoRun
' (HKCU\Software\Microsoft\Command Processor\AutoRun = conda_hook.bat) launches doskey.exe, a
' console program, from a parent with no console => brand-new visible console. Measured once per
' restart at 2026-09-02 19:20:40.
' Expected: ONE brief console window/tab flashes immediately. A MINIMIZED pwsh also sits in the
' taskbar for ~3 s (that is the cycle window itself; the real cycle has it too, it is not a flash).
Option Explicit
Dim sh, fso, here, r, cmd
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
here = fso.GetParentFolderName(WScript.ScriptFullName)
r = MsgBox("REPRO 7: ignition cycle launcher (console-less cmd.exe + AutoRun doskey)" & vbCrLf & vbCrLf & _
  "Click OK, then WATCH THE SCREEN for ~4 seconds." & vbCrLf & _
  "Expected: one brief console window/tab flashes immediately." & vbCrLf & _
  "(A minimized pwsh in the taskbar for 3 s is normal in both 7 and 8.)", vbOKCancel + vbInformation, "popup repro 7")
If r <> vbOK Then WScript.Quit 0
cmd = """C:\nvm4w\nodejs\node.exe"" """ & fso.BuildPath(fso.GetParentFolderName(here), "cycle-launcher-probe.mjs") & """"
sh.CurrentDirectory = here
sh.Run cmd, 0, True
WScript.Sleep 4000
MsgBox "Repro 7 finished. Did a console window/tab flash right at the start?", vbInformation, "popup repro 7"
