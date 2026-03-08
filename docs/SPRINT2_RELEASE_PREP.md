# Sprint 2 - Release Prep (Etat consolidé)

Date: 2026-03-08
Projet: C:\Users\jerem\Desktop\WhisperPro

## 1) Résumé exécutif

Sprint 2 est globalement réussi sur l'axe critique: dictée locale Windows utilisable avec hotkey globale + injection + widget dédié.
Le socle est désormais exploitable en test terrain.

## 2) Livré pendant Sprint 2

### Backend

- Pipeline local `whisper-cli` stabilisé.
- Hotkey globale configurable (`shortcut`) avec rebind au save settings.
- Cycle dictée toggle:
  - press 1: start capture
  - press 2: stop + transcription + injection
- Injection renforcée:
  - collage `clipboard + Ctrl+V`
  - retry
  - restauration clipboard différée
- État dictée centralisé backend:
  - `idle`, `listening`, `transcribing`, `done`, `busy`, `error`
- Commandes backend ajoutées:
  - `get_dictation_status`
  - `start_overlay_drag`

### Frontend

- Widget overlay externe (fenêtre dédiée Tauri).
- Sync widget robuste:
  - écoute événements
  - fallback polling
- Widget compact:
  - always-on-top
  - non focus
  - drag + position mémorisée
- Settings étendus:
  - `widget_enabled`
  - `widget_autohide`
- Main app simplifiée:
  - mode `Usage` (par défaut)
  - mode `Debug`

### Persistance

- SQLite migre et persiste:
  - `widget_enabled`
  - `widget_autohide`

### QA/Docs

- Checklist smoke Sprint 2: `docs/QA_SMOKE_SPRINT2.md`
- Script smoke Sprint 2: `scripts/smoke-check-sprint2.ps1`

## 3) Etat de stabilité actuel

### Vert (validé)

- Build frontend OK.
- Cargo check workspace OK.
- Hotkey active et persistante.
- Premier collage corrigé (bug clipboard historique traité).
- Toggle widget + auto-hide persistants.

### Jaune (à confirmer terrain)

- Synchronisation widget sur sessions longues (confirmer en test > 20 cycles).
- Compatibilité injection multi-apps (Notepad/Chrome/WordPad) à valider sur volume.
- Ergonomie fine des messages d'erreur (encore orientés technique sur certains cas).

## 4) Critères de clôture Sprint 2 (reste)

- [ ] Exécuter smoke test Sprint 2 complet en manuel et obtenir PASS.
- [ ] Confirmer 20 cycles dictée sans freeze widget.
- [ ] Confirmer 20 cycles injection sans collage incorrect.
- [ ] Corriger les 2-3 messages d'erreur les moins compréhensibles.

## 5) Plan immédiat Sprint 3 (proposé)

### S3-1 Fiabilité produit

- hardening final états widget (timeout/heartbeat)
- hardening injection selon app cible
- meilleure télémétrie locale (logs plus exploitables)

### S3-2 Qualité de texte

- post-traitement dictée:
  - ponctuation vocale
  - normalisation espaces/majuscules
  - commandes simples (`nouvelle ligne`, `point`, `virgule`)

### S3-3 Productisation

- packaging installateur propre
- parcours onboarding 1er lancement
- préparation bloc licensing achat unique

## 6) Commandes utiles

```powershell
cd C:\Users\jerem\Desktop\WhisperPro\apps\desktop
npm.cmd run tauri:dev
```

```powershell
cd C:\Users\jerem\Desktop\WhisperPro
powershell -ExecutionPolicy Bypass -File .\scripts\smoke-check-sprint2.ps1
```

```powershell
cd C:\Users\jerem\Desktop\WhisperPro
powershell -ExecutionPolicy Bypass -File .\scripts\qa-campaign-20-cycles.ps1
```

```powershell
cd C:\Users\jerem\Desktop\WhisperPro
powershell -ExecutionPolicy Bypass -File .\scripts\analyze-qa-campaign.ps1
```

```powershell
cd C:\Users\jerem\Desktop\WhisperPro\apps\desktop\src-tauri
cargo.exe check --workspace
```

## 7) Décision recommandée

Passer en phase "stabilisation finale Sprint 2" (tests manuels complets), puis démarrer Sprint 3 sur la qualité de texte et l'onboarding.
