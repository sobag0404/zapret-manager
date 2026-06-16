!define APP_NAME "Zapret Manager"
!define COMPANY_NAME "ZapretManager"
!define INSTALL_DIR "$PROGRAMFILES64\ZapretManager"

Name "${APP_NAME}"
OutFile "ZapretManagerSetup.exe"
InstallDir "${INSTALL_DIR}"
RequestExecutionLevel admin

Section "Install"
  SetOutPath "$INSTDIR"
  File /r "..\..\app\tauri\target\release\Zapret Manager.exe"
  File /r "..\..\service\target\release\zapret-manager-service.exe"
  SetOutPath "$INSTDIR\engine"
  File /r "..\..\engine\*"
  SetOutPath "$INSTDIR\profiles"
  File /r "..\..\profiles\*"
  SetOutPath "$INSTDIR\strategies"
  File /r "..\..\strategies\*"
  CreateDirectory "$INSTDIR\logs"
  CreateDirectory "$INSTDIR\snapshots"
  CreateDirectory "$INSTDIR\updater"
  WriteUninstaller "$INSTDIR\uninstaller.exe"
SectionEnd

Section "Uninstall"
  ExecWait '"$INSTDIR\ZapretManagerService.exe" emergency-disable'
  Delete "$INSTDIR\uninstaller.exe"
  RMDir /r "$INSTDIR"
SectionEnd
