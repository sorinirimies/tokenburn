; TokenBurn Windows Installer
; Built with NSIS (Nullsoft Scriptable Install System)
;
; NOTE: this script is compiled from a versioned copy at
; dist\installer_versioned.nsi (see scripts/ci/package_windows.sh), not from
; its own committed location (packaging\windows\installer.nsi). NSIS resolves
; every relative path used by File/LicenseData/Icon-style commands relative
; to the *compiled script's own directory*, not the process's working
; directory or the script's original location. Left unhandled, that means
; "target\...\release\tokenburn-gui.exe" would be looked up under
; dist\target\...\, and "LICENSE" under dist\LICENSE — neither of which
; exist, producing errors like:
;   LicenseData: open failed "LICENSE"
;   Error while loading icon from "packaging\windows\tokenburn.ico": can't open file
; `!cd` re-anchors that resolution to the actual repository root
; (substituted below by package_windows.sh via @REPO_ROOT_ABS@), so every
; relative path elsewhere in this file behaves exactly as if the script
; were compiled in place at the repo root.
!cd "@REPO_ROOT_ABS@"

!define PRODUCT_NAME "TokenBurn"
!define PRODUCT_VERSION "@VERSION@"
!define PRODUCT_PUBLISHER "Sorin Irimies"
!define PRODUCT_URL "https://github.com/sorinirimies/tokenburn"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "dist\tokenburn-${PRODUCT_VERSION}-windows-x86_64-setup.exe"
InstallDir "$PROGRAMFILES64\TokenBurn"
InstallDirRegKey HKLM "${PRODUCT_UNINST_KEY}" "InstallLocation"
RequestExecutionLevel admin
SetCompressor /SOLID lzma

!include "MUI2.nsh"

; UI
!define MUI_ABORTWARNING
!define MUI_ICON "packaging\windows\tokenburn.ico"
!define MUI_UNICON "packaging\windows\tokenburn.ico"

; Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; Components
Section "TokenBurn GUI" SecGUI
    SectionIn RO
    SetOutPath "$INSTDIR"
    File "target\x86_64-pc-windows-msvc\release\tokenburn-gui.exe"
    CreateDirectory "$SMPROGRAMS\TokenBurn"
    CreateShortcut "$SMPROGRAMS\TokenBurn\TokenBurn.lnk" "$INSTDIR\tokenburn-gui.exe"
    CreateShortcut "$DESKTOP\TokenBurn.lnk" "$INSTDIR\tokenburn-gui.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\tokenburn-gui.exe" "" "$INSTDIR\tokenburn-gui.exe"
SectionEnd

Section "TokenBurn CLI" SecCLI
    SetOutPath "$INSTDIR"
    File "target\x86_64-pc-windows-msvc\release\tokenburn.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\tokenburn.exe" "" "$INSTDIR\tokenburn.exe"
SectionEnd

Section "TokenBurn TUI" SecTUI
    SetOutPath "$INSTDIR"
    File "target\x86_64-pc-windows-msvc\release\tokenburn-tui.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\tokenburn-tui.exe" "" "$INSTDIR\tokenburn-tui.exe"
SectionEnd

Section "TokenBurn web dashboard" SecWeb
    SetOutPath "$INSTDIR"
    File "target\x86_64-pc-windows-msvc\release\tokenburn-web.exe"
    ; The live dashboard needs Topcoat's browser runtime bundle in assets\ next
    ; to the exe. Without it tokenburn-web still works, in static mode.
    !if /FileExists "dist\web-assets\manifest.toml"
        SetOutPath "$INSTDIR\assets"
        File /r "dist\web-assets\*.*"
        SetOutPath "$INSTDIR"
    !endif
    CreateDirectory "$SMPROGRAMS\TokenBurn"
    CreateShortcut "$SMPROGRAMS\TokenBurn\TokenBurn Web Dashboard.lnk" "$INSTDIR\tokenburn-web.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\tokenburn-web.exe" "" "$INSTDIR\tokenburn-web.exe"
SectionEnd

; PATH key in the system environment registry
!define PATH_KEY "SYSTEM\CurrentControlSet\Control\Session Manager\Environment"

Section "Add to PATH" SecPATH
    ; Append install directory to the system PATH via registry — no plugin required.
    ReadRegStr $0 HKLM "${PATH_KEY}" "Path"
    WriteRegExpandStr HKLM "${PATH_KEY}" "Path" "$0;$INSTDIR"
    ; Broadcast the change so running processes pick it up immediately.
    SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
SectionEnd

; Descriptions
LangString DESC_SecGUI ${LANG_ENGLISH} "TokenBurn desktop GUI — mouse-driven AI token usage report"
LangString DESC_SecCLI ${LANG_ENGLISH} "TokenBurn CLI — scriptable token usage report (tokenburn.exe)"
LangString DESC_SecWeb ${LANG_ENGLISH} "TokenBurn web dashboard — local live dashboard, opens in your browser"
LangString DESC_SecTUI ${LANG_ENGLISH} "TokenBurn TUI — keyboard-driven terminal AI token usage report"
LangString DESC_SecPATH ${LANG_ENGLISH} "Add TokenBurn to the system PATH"

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
    !insertmacro MUI_DESCRIPTION_TEXT ${SecGUI} $(DESC_SecGUI)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecCLI} $(DESC_SecCLI)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecWeb} $(DESC_SecWeb)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecTUI} $(DESC_SecTUI)
    !insertmacro MUI_DESCRIPTION_TEXT ${SecPATH} $(DESC_SecPATH)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Section -PostInstall
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME} ${PRODUCT_VERSION}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_URL}"
    WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
    WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "NoModify" 1
    WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "NoRepair" 1
    WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section Uninstall
    Delete "$INSTDIR\tokenburn-gui.exe"
    Delete "$INSTDIR\tokenburn-tui.exe"
    Delete "$INSTDIR\tokenburn.exe"
    Delete "$INSTDIR\tokenburn-web.exe"
    RMDir /r "$INSTDIR\assets"
    Delete "$INSTDIR\uninstall.exe"
    Delete "$SMPROGRAMS\TokenBurn\TokenBurn.lnk"
    Delete "$SMPROGRAMS\TokenBurn\TokenBurn Web Dashboard.lnk"
    Delete "$DESKTOP\TokenBurn.lnk"
    RMDir "$SMPROGRAMS\TokenBurn"
    RMDir "$INSTDIR"
    ; Remove install directory from the system PATH via registry.
    ReadRegStr $0 HKLM "${PATH_KEY}" "Path"
    ; Build cleaned PATH by removing our entry (exact match).
    ; Uses PowerShell for reliable semicolon-delimited string manipulation.
    ExecWait 'powershell -NoProfile -Command "\
        $p = [Environment]::GetEnvironmentVariable(\"Path\",\"Machine\"); \
        $clean = ($p -split \";\") | Where-Object { $_ -ne \"$INSTDIR\" }; \
        [Environment]::SetEnvironmentVariable(\"Path\", ($clean -join \";\"), \"Machine\")"'
    DeleteRegKey HKLM "${PRODUCT_UNINST_KEY}"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\tokenburn-gui.exe"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\tokenburn-tui.exe"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\tokenburn.exe"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\tokenburn-web.exe"
SectionEnd
