' 0-control-known-popup.vbs — calibration: a console-less cmd.exe runs ping, which MUST pop a window.
' Use this first so you know what a popup looks like on this box (a Windows Terminal tab/window
' titled "ping -n 3 127.0.0.1" for ~2 s).
Option Explicit
Dim sh, fso, here, r, cmd
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
here = fso.GetParentFolderName(WScript.ScriptFullName)
r = MsgBox("CONTROL: a guaranteed popup" & vbCrLf & vbCrLf & _
  "Click OK, then WATCH THE SCREEN for ~3 seconds." & vbCrLf & _
  "Expected: a terminal window/tab titled 'ping -n 3 127.0.0.1' appears for ~2 s.", vbOKCancel + vbInformation, "popup control")
If r <> vbOK Then WScript.Quit 0
cmd = """C:\nvm4w\nodejs\node.exe"" """ & fso.BuildPath(fso.GetParentFolderName(here), "spawn-probe.mjs") & """ detached -- cmd.exe /c ""ping -n 3 127.0.0.1"""
sh.CurrentDirectory = here
sh.Run cmd, 0, True
MsgBox "Control finished. You should have seen the ping window.", vbInformation, "popup control"
