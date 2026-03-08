# WhisperPro

WhisperPro est une application Windows de dictee vocale locale basee sur Whisper, orientee utilisateur final.

[Site officiel](https://skro10.github.io/WhisperPro/) | [Telecharger l'installeur](https://github.com/skro10/WhisperPro/releases/latest) | [Support](https://buymeacoffee.com/skroproduction)

## Ce que fait WhisperPro

- dictee locale rapide via bouton ou raccourci global
- transcription avec gestion des modeles directement dans l'UI
- historique des transcriptions (copie et suppression)
- traduction optionnelle selon langue cible
- mini-widget discret et personnalisable (apparition, opacite, son)
- interface en francais et anglais

## Pour qui

WhisperPro est pense pour les personnes qui veulent dicter rapidement sans devoir configurer un environnement technique complexe.

## Installation (utilisateur final)

1. Ouvre la derniere release:
   [https://github.com/skro10/WhisperPro/releases/latest](https://github.com/skro10/WhisperPro/releases/latest)
2. Telecharge `WhisperPro_1.0.0_x64-setup.exe`
3. Lance l'installeur puis ouvre l'application

## Build local (developpement)

Prerequis:
- Node.js 20+
- Rust stable (`cargo`)
- Visual Studio Build Tools (C++ workload)
- WebView2 Runtime

Commandes:

```powershell
cd apps/desktop
npm install
npm run tauri:dev
```

Build installateur:

```powershell
cd apps/desktop
npm run tauri:build
```

Sortie:
- `target/release/bundle/nsis/WhisperPro_1.0.0_x64-setup.exe`

## Distribution GitHub

- release automatique via `.github/workflows/release.yml` sur tags `v*`
- site GitHub Pages via `.github/workflows/pages.yml` depuis `site/`
- lien de soutien via `.github/FUNDING.yml`

## Notes

- Les dependances runtime sont gerees automatiquement a l'installation / au premier lancement.
- Les modeles Whisper ne sont pas bundles par defaut et se gerent depuis l'interface.
