# Sprint 2 - Quickstart Transcription Locale

## Prerequis runtime

Place ces deux fichiers en local:

- `whisper-cli.exe`
- un modele Whisper.cpp `.bin` (ex: `ggml-base.bin`)

Chemins par defaut de l'app:

- CLI: `%LOCALAPPDATA%\\WhisperPro\\bin\\whisper-cli.exe`
- Modele: `%LOCALAPPDATA%\\WhisperPro\\models\\ggml-base.bin`

Tu peux aussi definir des chemins custom depuis `Settings`.

## Test manuel rapide

1. Lancer l'app:
   - `cd apps/desktop`
   - `npm.cmd run tauri:dev`
2. Dans `Settings`, verifier/ajuster:
   - `Chemin whisper-cli.exe`
   - `Chemin modele Whisper (.bin)`
3. Cliquer `Sauvegarder les settings`.
4. Dans `Dashboard`:
   - `Demarrer test micro`, parler 5-10 sec, `Arreter test micro`
   - `Transcrire le dernier WAV`
5. Verifier le texte affiche.

## Diagnostics

- En cas d'erreur, consulter:
  - `Derniere erreur backend` dans l'UI
  - `%LOCALAPPDATA%\\WhisperPro\\logs\\whisperpro.log`
