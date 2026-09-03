' 1-poolwatch-task.vbs — fires the PoolWatch scheduled task once (it normally runs every 15 min).
' Expected (instrument said 3/3): a pwsh console tab/window appears and takes focus.
Option Explicit
Dim sh, r
Set sh = CreateObject("WScript.Shell")
r = MsgBox("REPRO 1: PoolWatch scheduled task" & vbCrLf & vbCrLf & _
  "Click OK, then WATCH THE SCREEN for ~8 seconds." & vbCrLf & _
  "Expected: a pwsh console window/tab pops and takes focus." & vbCrLf & vbCrLf & _
  "Nothing on this box is changed by this.", vbOKCancel + vbInformation, "popup repro 1")
If r <> vbOK Then WScript.Quit 0
sh.Run "schtasks /run /tn PoolWatch", 0, True
WScript.Sleep 8000
MsgBox "Repro 1 finished. Did a window pop and take focus?", vbInformation, "popup repro 1"
