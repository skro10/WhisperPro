# WhisperPro 1.0 - Release Notes

## Scope

WhisperPro 1.0 targets end users on Windows with a simplified UX:
- local dictation and transcription
- model management from the UI
- optional translation flow
- global shortcut and dynamic mini-widget

## Distribution

- Official package: NSIS installer only.
- Portable distribution has been removed from this release workflow.

## Runtime bootstrap

At installation/startup, the app handles runtime checks for:
- VC++ redistributable
- WebView2
- Whisper runtime dependencies

Language models are managed from the UI and are not bundled by default.

## Data paths

User data remains under `%LOCALAPPDATA%\WhisperPro` (standard Windows behavior):
- settings database
- logs
- models
- runtime binaries

## Uninstall cleanup

Uninstall hook removes WhisperPro user folders, including legacy paths used by previous builds.

