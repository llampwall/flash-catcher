' 9-nvidia-smi-from-pythonw.vbs — what the ai-control-panel PM2 app does on every GPU read:
' control-panel/server.py:756 (gpu_vram_live) and :792 (gpu_free_gb) call
' subprocess.run(["nvidia-smi", ...]) WITHOUT creationflags=CREATE_NO_WINDOW, from pythonw.exe,
' which is a GUI-subsystem process with no console. Measured 2026-09-02 19:20:22: a "Terminal"
' window took focus for 1.7 s. This one fires whenever the panel is polled, restart or not.
' Expected: ONE console window/tab flashes right after you click OK.
Option Explicit
Dim sh, r, cmd
Set sh = CreateObject("WScript.Shell")
r = MsgBox("REPRO 9: nvidia-smi from console-less pythonw (server.py:756 shape)" & vbCrLf & vbCrLf & _
  "Click OK, then WATCH THE SCREEN for ~3 seconds." & vbCrLf & _
  "Expected: one console window/tab flashes almost immediately.", vbOKCancel + vbInformation, "popup repro 9")
If r <> vbOK Then WScript.Quit 0
cmd = """C:\ProgramData\miniconda3\pythonw.exe"" -c ""import subprocess; subprocess.run(['nvidia-smi','--query-gpu=memory.used,memory.total','--format=csv,noheader,nounits'], capture_output=True, text=True, timeout=3)"""
sh.Run cmd, 0, True
MsgBox "Repro 9 finished. Did a console window/tab flash?", vbInformation, "popup repro 9"
