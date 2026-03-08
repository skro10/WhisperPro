# WhisperPro

WhisperPro is a Windows desktop app for local-first dictation and transcription with Whisper.

[Project website](https://skro10.github.io/WhisperPro/) | [Latest installer](https://github.com/skro10/WhisperPro/releases/latest)

## Current release

- Version: `1.0.0`
- Distribution target: NSIS installer (`.exe`) only
- Portable packaging is not part of the release workflow anymore

## Main features

- one-click dictation flow (button + global shortcut)
- model download/manage/select directly in UI
- transcription history
- optional translation
- dynamic mini-widget with selectable pop sound and volume
- French/English UI language

## Project structure

- `apps/desktop`: React UI + Tauri desktop app
- `apps/desktop/src-tauri`: Rust backend commands/runtime orchestration
- `crates/core`: shared Rust crate(s)

## Development prerequisites

- Node.js 20+
- Rust stable (`cargo`)
- Visual Studio Build Tools (C++ workload)
- WebView2 Runtime

## Run in development

```powershell
cd apps/desktop
npm install
npm run tauri:dev
```

## Build installer

```powershell
cd apps/desktop
npm run tauri:build
```

Installer output:
- `target/release/bundle/nsis/WhisperPro_1.0.0_x64-setup.exe`

## GitHub distribution (free)

- GitHub Releases are automated by `.github/workflows/release.yml` on tag push (`v*`).
- Project page is deployed with GitHub Pages from `site/` via `.github/workflows/pages.yml`.
- Financial support links are configured in `.github/FUNDING.yml`.

## Runtime dependencies and models

- Runtime dependencies are handled automatically by the installer/startup checks.
- Whisper language models are not bundled by default and are managed from the app UI.
