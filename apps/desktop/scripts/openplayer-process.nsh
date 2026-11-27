LangString OpenPlayerFilesUnavailable ${LANG_ENGLISH} "Windows could not access OpenPlayer's program files. Close the player if it is running, then select Retry."
LangString OpenPlayerFilesUnavailable ${LANG_SIMPCHINESE} "无法访问 OpenPlayer 的程序文件。若播放器仍在运行，请关闭后点击「重试」。"

!macro OpenPlayerProcessFunctions PREFIX FILE_ACCESS
Function ${PREFIX}EnsureOpenPlayerStopped
retry:
  IfFileExists "$INSTDIR\openplayer-desktop.exe" 0 wait_for_files
  Call ${PREFIX}RequestOpenPlayerClose

wait_for_files:
  StrCpy $0 25
check_files:
  StrCpy $1 "$INSTDIR\openplayer-desktop.exe"
  Call ${PREFIX}CheckOpenPlayerFile
  StrCmp $2 0 wait_for_release
  StrCpy $1 "$INSTDIR\libmpv-2.dll"
  Call ${PREFIX}CheckOpenPlayerFile
  StrCmp $2 0 wait_for_release
  Return

wait_for_release:
  StrCmp $0 0 files_unavailable
  Sleep 200
  IntOp $0 $0 - 1
  Goto check_files

files_unavailable:
  MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "$(OpenPlayerFilesUnavailable)" /SD IDCANCEL IDRETRY retry
  SetErrorLevel 1
  Abort
FunctionEnd

# Check the access required for the operation without modifying the installed file.
Function ${PREFIX}CheckOpenPlayerFile
  StrCpy $2 1
  IfFileExists "$1" 0 done
  # Share read/write/delete and open only an existing file.
  System::Call 'kernel32::CreateFileW(w r1, i ${FILE_ACCESS}, i 7, p 0, i 3, i 0x80, p 0)p.r2'
  StrCmp $2 -1 unavailable
  System::Call 'kernel32::CloseHandle(p r2)'
  StrCpy $2 1
  Return
unavailable:
  StrCpy $2 0
done:
FunctionEnd

Function ${PREFIX}RequestOpenPlayerClose
  GetFullPathName $2 "$INSTDIR\openplayer-desktop.exe"
  System::Call 'user32::GetDesktopWindow()p.rR0'
  System::Call 'user32::FindWindowExW(p rR0, p 0, p 0, p 0)p.r3'
next_window:
  StrCmp $3 0 done
  StrCpy $4 $3
  # Read the next window before asking this one to close.
  System::Call 'user32::FindWindowExW(p rR0, p r4, p 0, p 0)p.r3'
  # The player must manage its own platform and video windows during shutdown.
  System::Call 'user32::IsWindowVisible(p r4)i.r8'
  StrCmp $8 0 next_window
  System::Call 'user32::GetWindow(p r4, i 4)p.r8' # GW_OWNER
  StrCmp $8 0 0 next_window
  System::Call 'user32::GetWindowThreadProcessId(p r4, *i .r5)i.r8'
  StrCmp $8 0 next_window
  # PROCESS_QUERY_LIMITED_INFORMATION and SYNCHRONIZE.
  System::Call 'kernel32::OpenProcess(i 0x101000, i 0, i r5)p.r6'
  StrCmp $6 0 next_window
  System::Call 'kernel32::QueryFullProcessImageNameW(p r6, i 0, w .r7, *i ${NSIS_MAX_STRLEN})i.r8'
  StrCmp $8 0 close_handle
  System::Call 'kernel32::CompareStringOrdinal(w r2, i -1, w r7, i -1, i 1)i.r8'
  StrCmp $8 2 0 close_handle
  System::Call 'kernel32::WaitForSingleObject(p r6, i 0)i.r8'
  StrCmp $8 258 0 close_handle
  System::Call 'user32::GetWindowThreadProcessId(p r4, *i .r9)i.r8'
  StrCmp $8 0 close_handle
  StrCmp $5 $9 0 close_handle
  System::Call 'user32::PostMessageW(p r4, i 0x0010, p 0, p 0)'
close_handle:
  System::Call 'kernel32::CloseHandle(p r6)'
  Goto next_window
done:
FunctionEnd
!macroend
