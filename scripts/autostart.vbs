Set fso = CreateObject("Scripting.FileSystemObject")
Set sh = CreateObject("WScript.Shell")

root = fso.GetParentFolderName(fso.GetParentFolderName(WScript.ScriptFullName))
sh.CurrentDirectory = root

releaseExe = root & "\src-tauri\target\release\desktop-todo-tauri.exe"
debugExe = root & "\src-tauri\target\debug\desktop-todo-tauri.exe"

If fso.FileExists(releaseExe) Then
  sh.Run """" & releaseExe & """ --opened-at-login", 0, False
ElseIf fso.FileExists(debugExe) Then
  sh.Run """" & debugExe & """ --opened-at-login", 0, False
End If
