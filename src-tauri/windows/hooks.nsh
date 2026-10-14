; Until 0.1.21 the app was called "Quick Translate". Its installer put it in a
; folder, an uninstall entry and shortcuts under that name, so an update to
; Swipelate would otherwise leave a second copy behind. The settings live in
; a folder named after the identifier, which did not change, and are kept.
!define OLD_NAME "Quick Translate"
!define OLD_UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${OLD_NAME}"
!define RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"

!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $R0 HKCU "${OLD_UNINSTKEY}" "InstallLocation"
  ${If} $R0 != ""
    nsExec::Exec 'taskkill /f /im quick-translate.exe'
    Pop $R1
    Sleep 500
    ; The location is written with quotes around it.
    StrCpy $R1 $R0 1
    ${If} $R1 == '"'
      StrCpy $R0 $R0 "" 1
      StrCpy $R0 $R0 -1
    ${EndIf}
    ; Only ever the old app's own folder.
    StrLen $R2 "${OLD_NAME}"
    IntOp $R2 0 - $R2
    StrCpy $R1 $R0 "" $R2
    ${If} $R1 == "${OLD_NAME}"
      RMDir /r "$R0"
    ${Else}
      ; A folder the user chose: let the old uninstaller remove only its own files.
      ExecWait '"$R0\uninstall.exe" /S _?=$R0'
      Delete "$R0\uninstall.exe"
      RMDir "$R0"
    ${EndIf}
    Delete "$SMPROGRAMS\${OLD_NAME}.lnk"
    Delete "$DESKTOP\${OLD_NAME}.lnk"
    DeleteRegKey HKCU "${OLD_UNINSTKEY}"
    DeleteRegKey HKCU "Software\${MANUFACTURER}\${OLD_NAME}"
    ; Starting with Windows stays on when it was on.
    ReadRegStr $R1 HKCU "${RUN_KEY}" "${OLD_NAME}"
    ${If} $R1 != ""
      DeleteRegValue HKCU "${RUN_KEY}" "${OLD_NAME}"
      WriteRegStr HKCU "${RUN_KEY}" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --hidden'
    ${EndIf}
  ${EndIf}
!macroend
