; NSIS hooks for the Tauri Windows installer.
; This file is referenced from src-tauri/tauri.conf.json via:
; bundle.windows.nsis.installerHooks = "windows/installer-hooks.nsh"

!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Préparation de l'installation de Norsys Workspaces..."
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Configuration post-installation de Norsys Workspaces..."

  ; The custom installer wizard/helper will be wired here once the Rust commands are added.
  ; Keep this hook non-blocking for now so the NSIS build and installation flow remain valid.
  IfFileExists "$INSTDIR\dev-workspace-manager.exe" 0 +3
    DetailPrint "Application installée dans $INSTDIR"
    Goto +2
  DetailPrint "Application introuvable dans $INSTDIR"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Préparation de la désinstallation de Norsys Workspaces..."
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DetailPrint "Nettoyage post-désinstallation de Norsys Workspaces..."

  ; Remove the per-user installer configuration file if it exists.
  Delete "$TEMP\norsys-workspaces-install.json"
!macroend
