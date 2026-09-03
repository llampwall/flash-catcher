' 10-nvidia-smi-from-pythonw-FIXED.vbs — identical to repro 9 plus creationflags=CREATE_NO_WINDOW,
' the same flag server.py already uses on its other subprocess calls (its _NO_WINDOW constant).
' Expected: NOTHING visible.
Option Explicit
Dim sh, r, cmd
Set sh = CreateObject("WScript.Shell")
r = MsgBox("REPRO 10 (FIX): same nvidia-smi call, with CREATE_NO_WINDOW" & vbCrLf & vbCrLf & _
  "Click OK, then WATCH THE SCREEN for ~3 seconds." & vbCrLf & _
  "Expected: nothing visible at all.", vbOKCancel + vbInformation, "popup repro 10")
If r <> vbOK Then WScript.Quit 0
cmd = """C:\ProgramData\miniconda3\pythonw.exe"" -c ""import subprocess; subprocess.run(['nvidia-smi','--query-gpu=memory.used,memory.total','--format=csv,noheader,nounits'], capture_output=True, text=True, timeout=3, creationflags=subprocess.CREATE_NO_WINDOW)"""
sh.Run cmd, 0, True
MsgBox "Repro 10 finished. Did anything flash? (expected: no)", vbInformation, "popup repro 10"
