!include "LogicLib.nsh"

!macro NSIS_HOOK_POSTINSTALL
  ; Ensure Microsoft VC++ runtime is available on fresh Windows machines.
  IfFileExists "$WINDIR\System32\vcruntime140.dll" vc_runtime_present vc_runtime_missing
vc_runtime_missing:
  IfFileExists "$INSTDIR\resources\redist\vc_redist.x64.exe" vc_runtime_install vc_runtime_missing_file
vc_runtime_install:
  DetailPrint "Installing bundled Microsoft Visual C++ Redistributable (x64)..."
  ExecWait '"$INSTDIR\resources\redist\vc_redist.x64.exe" /install /passive /norestart' $0
  ${If} $0 != 0
    MessageBox MB_ICONEXCLAMATION|MB_OK "Le runtime Microsoft Visual C++ n'a pas pu être installé automatiquement (code: $0)."
  ${EndIf}
  Goto vc_runtime_present
vc_runtime_missing_file:
  MessageBox MB_ICONEXCLAMATION|MB_OK "Le runtime VC++ embarqué est introuvable dans l'installeur."
vc_runtime_present:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Full cleanup of WhisperPro user data.
  ; We intentionally keep shared system runtimes (VC++/WebView2).
  DetailPrint "Removing WhisperPro user data..."

  ; Main app data (db, logs, models, runtime files)
  RMDir /r "$LOCALAPPDATA\WhisperPro"

  ; Known transcript/temp artifact folders/files
  RMDir /r "$TEMP\whisperpro_transcripts"
  RMDir /r "$TEMP\whisperprotranscript"
  Delete "$TEMP\whisperpro_recording.wav"

  ; Legacy / alternate folders observed during earlier versions
  RMDir /r "$LOCALAPPDATA\whisperpro_transcripts"
  RMDir /r "$LOCALAPPDATA\whisperprotranscript"
  RMDir /r "$LOCALAPPDATA\WhisperPro\whisperpro_transcripts"
  RMDir /r "$LOCALAPPDATA\WhisperPro\whisperprotranscript"
  RMDir /r "$LOCALAPPDATA\com.whisperpro.app"
  RMDir /r "$LOCALAPPDATA\com.whisperpro.desktop"
!macroend
