; NSIS Script for Zapret Interactive
; Compatible with Tauri v2 in-app updater and standalone installation

!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"

!ifndef PRODUCT_VERSION
  !define PRODUCT_VERSION "1.7.0"
!endif

!ifndef PRODUCT_NAME
  !define PRODUCT_NAME "Zapret Interactive"
!endif

!ifndef APP_EXE_NAME
  !define APP_EXE_NAME "Zapret Interactive.exe"
!endif

!ifndef SOURCE_DIR
  !define SOURCE_DIR "..\..\..\target\release"
!endif

!ifndef THIRDPARTY_DIR
  !define THIRDPARTY_DIR "..\..\..\thirdparty"
!endif

!define PRODUCT_PUBLISHER "Noktomezo"
!define PRODUCT_WEB_SITE "https://github.com/Noktomezo/ZapretInteractive"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
!define PRODUCT_UNINST_ROOT_KEY "HKCU"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
VIProductVersion "${PRODUCT_VERSION}.0"
VIAddVersionKey /LANG=1033 "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey /LANG=1033 "FileDescription" "${PRODUCT_NAME} Installer"
VIAddVersionKey /LANG=1033 "CompanyName" "${PRODUCT_PUBLISHER}"
VIAddVersionKey /LANG=1033 "LegalCopyright" "Copyright © 2026 ${PRODUCT_PUBLISHER}"
VIAddVersionKey /LANG=1033 "FileVersion" "${PRODUCT_VERSION}"
VIAddVersionKey /LANG=1033 "ProductVersion" "${PRODUCT_VERSION}"
VIAddVersionKey /LANG=1033 "OriginalFilename" "Zapret Interactive Setup.exe"

!ifndef OUTFILE
  !define OUTFILE "..\..\..\target\release\bundle\nsis\Zapret Interactive_${PRODUCT_VERSION}_x64-setup.exe"
!endif

OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\Zapret Interactive"
InstallDirRegKey HKCU "Software\${PRODUCT_NAME}" ""
RequestExecutionLevel user
Unicode true
SetCompressor /SOLID lzma

; Interface Configuration
!define MUI_ABORTWARNING
!define MUI_ICON "..\..\..\assets\app.ico"
!define MUI_UNICON "..\..\..\assets\app.ico"

!ifdef HEADER_BMP
  !define MUI_HEADERIMAGE
  !define MUI_HEADERIMAGE_BITMAP "${HEADER_BMP}"
  !define MUI_HEADERIMAGE_UNBITMAP "${HEADER_BMP}"
!endif

!ifdef SIDEBAR_BMP
  !define MUI_WELCOMEFINISHPAGE_BITMAP "${SIDEBAR_BMP}"
  !define MUI_UNWELCOMEFINISHPAGE_BITMAP "${SIDEBAR_BMP}"
!endif

; Modern UI Pages
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_WELCOME
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\${APP_EXE_NAME}"
!define MUI_FINISHPAGE_RUN_TEXT "Запустить ${PRODUCT_NAME}"
!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive
!insertmacro MUI_PAGE_FINISH

; Uninstaller Pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Language
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "English"

Var PassiveMode
Var RestartApp
Function .onInit
  ${GetOptions} $CMDLINE "/P" $0
  IfErrors +2 0
    StrCpy $PassiveMode 1

  ${GetOptions} $CMDLINE "/R" $0
  IfErrors +2 0
    StrCpy $RestartApp 1

  ; GPUI 2.0.0 launches updates without arguments, but uses this temp filename.
  StrCpy $0 $EXEFILE 24
  ${If} $0 == "ZapretInteractive-setup-"
    StrCpy $PassiveMode 1
    StrCpy $RestartApp 1
  ${EndIf}

  ; Tauri stores the selected path in the standard uninstall metadata.
  ReadRegStr $0 ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "InstallLocation"
  ${If} $0 != ""
    StrCpy $1 $0 1
    ${IfThen} $1 == '"' ${|} StrCpy $0 $0 -1 1 ${|}
    StrCpy $INSTDIR $0
  ${EndIf}

  ; Close running instances of app and winws if updating
  nsExec::Exec 'taskkill /F /IM "Zapret Interactive.exe" /IM "ZapretInteractive.exe" /IM "winws.exe" /T'
FunctionEnd

Function SkipIfPassive
  ${IfThen} $PassiveMode == 1 ${|} Abort ${|}
FunctionEnd

Section "MainSection" SEC01
  SetOutPath "$INSTDIR"
  SetOverwrite on

  ; Copy main executable
  File "/oname=${APP_EXE_NAME}" "${SOURCE_DIR}\Zapret Interactive.exe"

  ; Copy resources files
  SetOutPath "$INSTDIR\resources"
  File /r "${THIRDPARTY_DIR}\*.*"

  ; Shortcuts
  SetOutPath "$INSTDIR"
  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk" "$INSTDIR\${APP_EXE_NAME}" "" "$INSTDIR\${APP_EXE_NAME}" 0
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\Удалить ${PRODUCT_NAME}.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\uninstall.exe" 0
  CreateShortCut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\${APP_EXE_NAME}" "" "$INSTDIR\${APP_EXE_NAME}" 0

  ; Write Uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Registry Keys
  WriteRegStr HKCU "Software\${PRODUCT_NAME}" "" "$INSTDIR"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "InstallLocation" '$INSTDIR'
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\${APP_EXE_NAME}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegDWORD ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "NoModify" 1
  WriteRegDWORD ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "NoRepair" 1

  ${IfThen} $PassiveMode == 1 ${|} SetAutoClose true ${|}
SectionEnd

Function .onInstSuccess
  ${If} $RestartApp == 1
    Exec '"$INSTDIR\${APP_EXE_NAME}"'
  ${EndIf}
FunctionEnd

Section "Uninstall"
  nsExec::Exec 'taskkill /F /IM "ZapretInteractive.exe" /IM "Zapret Interactive.exe" /IM "winws.exe" /T'

  Delete "$DESKTOP\${PRODUCT_NAME}.lnk"
  Delete "$SMPROGRAMS\${PRODUCT_NAME}\*.*"
  RMDir "$SMPROGRAMS\${PRODUCT_NAME}"

  RMDir /r "$INSTDIR\thirdparty"
  RMDir /r "$INSTDIR\resources"
  Delete "$INSTDIR\${APP_EXE_NAME}"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  DeleteRegKey ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}"
  DeleteRegKey HKCU "Software\${PRODUCT_NAME}"
SectionEnd
