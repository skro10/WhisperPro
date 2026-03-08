# WhisperPro

Application desktop Windows local-first de dictee/transcription basee sur Whisper.

## Structure

- `apps/desktop`: UI React + shell Tauri
- `crates/core`: logique Rust partagee (audio, inference, etc.)
- `packages/ui`: composants UI mutualisables (placeholder v1)

## Prerequis (developpement)

- Node.js 20+
- Rust stable + `cargo`
- Visual Studio Build Tools (C++ workload)
- WebView2 Runtime

## Demarrage rapide (dev)

1. Ouvrir `apps/desktop`.
2. Executer:
   - `npm install`
   - `npm run tauri:dev`

## Packaging utilisateur final (sans configuration manuelle)

Avant `npm run tauri:build`, place `whisper-cli.exe` dans:
- `apps/desktop/src-tauri/resources/bin/whisper-cli.exe`

Au premier lancement de l'application installee:
- WhisperPro copie automatiquement ce binaire vers `LOCALAPPDATA\\WhisperPro\\bin\\whisper-cli.exe`
- le chemin est configure automatiquement dans les settings
- l'utilisateur final n'a pas besoin de chercher/configurer ce binaire

Les modeles de langue ne sont pas encore bundles (gestion via UI a venir).
