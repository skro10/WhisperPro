Place `whisper-cli.exe` in this folder before running `npm run tauri:build`.

Bundled installer behavior:
- ships this binary as an app resource
- app copies it to LOCALAPPDATA\\WhisperPro\\bin\\whisper-cli.exe on first launch
- user does not need to configure a path manually

Models are intentionally not bundled here.
