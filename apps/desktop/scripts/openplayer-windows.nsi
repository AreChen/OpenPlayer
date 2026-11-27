Unicode true
!include "MUI2.nsh"
!include "WinVer.nsh"
!include "openplayer-media-shell.nsh"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenPlayer"
!define CAPABILITIES "Software\OpenPlayer\Capabilities"
!define MEDIA_CLASS "OpenPlayer.Media"
!define REGISTERED_APP_NAME "OpenPlayer"

Name "OpenPlayer"
OutFile "${INSTALLER}"
InstallDir "$LOCALAPPDATA\Programs\OpenPlayer"
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
VIProductVersion "${PRODUCT_VERSION}.0"
VIAddVersionKey /LANG=1033 "ProductName" "OpenPlayer"
VIAddVersionKey /LANG=1033 "ProductVersion" "${PRODUCT_VERSION}"
VIAddVersionKey /LANG=1033 "FileVersion" "${PRODUCT_VERSION}"
VIAddVersionKey /LANG=1033 "FileDescription" "OpenPlayer setup"
VIAddVersionKey /LANG=1033 "LegalCopyright" "OpenPlayer contributors"

!define MUI_ICON "${APP_ICON}"
!define MUI_UNICON "${APP_ICON}"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\openplayer-desktop.exe"
!define MUI_FINISHPAGE_RUN_TEXT "$(OpenPlayerFinishLaunch)"
!define MUI_FINISHPAGE_SHOWREADME
!define MUI_FINISHPAGE_SHOWREADME_TEXT "$(OpenPlayerFinishDefault)"
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION OpenDefaultPlayerSettings
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

LangString OpenPlayerFinishLaunch ${LANG_ENGLISH} "Launch OpenPlayer now"
LangString OpenPlayerFinishLaunch ${LANG_SIMPCHINESE} "立即打开 OpenPlayer"
LangString OpenPlayerFinishDefault ${LANG_ENGLISH} "Set as the default player"
LangString OpenPlayerFinishDefault ${LANG_SIMPCHINESE} "设置为默认播放器"

!include /CHARSET=UTF8 "openplayer-process.nsh"
!insertmacro OpenPlayerProcessFunctions "" 0x40000000
!insertmacro OpenPlayerProcessFunctions "un." 0x40010000

Function OpenDefaultPlayerSettings
  ${If} ${AtLeastBuild} 22000
    ClearErrors
    ExecShell "open" "ms-settings:defaultapps?registeredAppUser=${REGISTERED_APP_NAME}"
    ${IfNot} ${Errors}
      Return
    ${EndIf}
  ${EndIf}
  ExecShell "open" "ms-settings:defaultapps"
FunctionEnd

!macro RegisterMediaExtension EXT
  WriteRegStr HKCU "${CAPABILITIES}\FileAssociations" ".${EXT}" "${MEDIA_CLASS}"
  WriteRegStr HKCU "Software\Classes\.${EXT}\OpenWithProgids" "${MEDIA_CLASS}" ""
!macroend
!macro UnregisterMediaExtension EXT
  DeleteRegValue HKCU "Software\Classes\.${EXT}\OpenWithProgids" "${MEDIA_CLASS}"
  DeleteRegKey /ifempty HKCU "Software\Classes\.${EXT}\OpenWithProgids"
!macroend
!include "${MEDIA_ASSOCIATIONS}"

Section "OpenPlayer"
  SetShellVarContext current
  SetRegView 64
  Call EnsureOpenPlayerStopped
  SetOutPath "$INSTDIR"
  File /oname=openplayer-desktop.exe "${PLAYER_EXE}"
  File /oname=libmpv-2.dll "${MPV_DLL}"
  SetOutPath "$INSTDIR\licenses"
  File /oname=libmpv-LICENSE.LGPL "${MPV_LICENSE}"
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateShortcut "$SMPROGRAMS\OpenPlayer.lnk" "$INSTDIR\openplayer-desktop.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "OpenPlayer"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\openplayer-desktop.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  WriteRegStr HKCU "Software\Classes\${MEDIA_CLASS}" "" "OpenPlayer Media"
  WriteRegStr HKCU "Software\Classes\${MEDIA_CLASS}\DefaultIcon" "" "$INSTDIR\openplayer-desktop.exe,0"
  WriteRegStr HKCU "Software\Classes\${MEDIA_CLASS}\shell\open\command" "" '"$INSTDIR\openplayer-desktop.exe" "%1"'
  WriteRegStr HKCU "${CAPABILITIES}" "ApplicationName" "${REGISTERED_APP_NAME}"
  WriteRegStr HKCU "${CAPABILITIES}" "ApplicationDescription" "OpenPlayer Media Player"
  WriteRegStr HKCU "Software\RegisteredApplications" "${REGISTERED_APP_NAME}" "${CAPABILITIES}"
  !insertmacro RegisterMediaExtensions
  !insertmacro OP_REGISTER_MEDIA_SHELL_METADATA
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  SetRegView 64
  Call un.EnsureOpenPlayerStopped
  Delete "$SMPROGRAMS\OpenPlayer.lnk"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  DeleteRegValue HKCU "Software\RegisteredApplications" "${REGISTERED_APP_NAME}"
  DeleteRegKey HKCU "${CAPABILITIES}"
  DeleteRegKey HKCU "Software\Classes\${MEDIA_CLASS}"
  !insertmacro UnregisterMediaExtensions
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
  Delete "$INSTDIR\licenses\libmpv-LICENSE.LGPL"
  RMDir "$INSTDIR\licenses"
  Delete "$INSTDIR\libmpv-2.dll"
  Delete "$INSTDIR\openplayer-desktop.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
