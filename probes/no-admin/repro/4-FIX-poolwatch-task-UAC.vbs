' 4-FIX-poolwatch-task-UAC.vbs — applies the PoolWatch fix. Needs admin: you will get ONE UAC
' prompt, then an elevated pwsh window that shows before/after, fires the task once so you can
' see there is no popup, and closes itself.
' Rollback line is inside P:\software\bin\fix-poolwatch-task.ps1.
Option Explicit
Dim r, app
r = MsgBox("FIX: PoolWatch scheduled task -> wscript launcher" & vbCrLf & vbCrLf & _
  "Click OK, accept the UAC prompt, then watch the elevated window." & vbCrLf & _
  "It re-fires the task once; there should be NO popup this time.", vbOKCancel + vbInformation, "PoolWatch fix")
If r <> vbOK Then WScript.Quit 0
Set app = CreateObject("Shell.Application")
app.ShellExecute "C:\Program Files\PowerShell\7\pwsh.exe", "-NoProfile -ExecutionPolicy Bypass -File ""P:\software\bin\fix-poolwatch-task.ps1""", "", "runas", 1
