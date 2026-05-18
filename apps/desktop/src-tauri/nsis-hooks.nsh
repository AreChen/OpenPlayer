!macro NSIS_HOOK_POSTINSTALL
  IfFileExists "$INSTDIR\resources\libmpv-2.dll" 0 +2
    CopyFiles /SILENT "$INSTDIR\resources\libmpv-2.dll" "$INSTDIR\libmpv-2.dll"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$INSTDIR\libmpv-2.dll"
!macroend
