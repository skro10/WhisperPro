# QA Smoke - Sprint 2 (Hotkey + Widget + Injection)

Date: 2026-03-08
Projet: C:\Users\jerem\Desktop\WhisperPro

## Objectif

Valider le flux de dictée globale en usage réel:
- raccourci global,
- états widget,
- injection texte fiable,
- persistance settings widget.

## Pré-requis

- `whisper-cli.exe` installé et détecté.
- modèle `.bin` détecté.
- build app OK.

## Smoke test manuel

1. Lancer l'app:
   - `cd apps/desktop`
   - `npm.cmd run tauri:dev`

2. Vérifier présence widget:
   - Le widget overlay doit apparaître.
   - Le drag doit fonctionner.

3. Vérifier hotkey global:
   - Dans Settings, garder `Ctrl+Shift+Space` (ou raccourci custom).
   - Appui 1: état `listening`.
   - Appui 2: état `transcribing` puis `done`.

4. Vérifier injection texte (Notepad):
   - 5 cycles complets dictée.
   - Le texte collé doit correspondre au texte transcrit.
   - Le premier collage ne doit pas reprendre un ancien clipboard.

5. Vérifier settings widget:
   - Désactiver `Afficher le widget overlay` + sauvegarder.
   - Le widget doit se fermer.
   - Réactiver + sauvegarder.
   - Le widget doit réapparaître.

6. Vérifier auto-hide:
   - Activer `Auto-masquer le widget après succès`.
   - Après état `done`, le widget doit se masquer (~1.6s).
   - Nouveau cycle dictée => redevient visible.

7. Vérifier persistance:
   - Fermer puis relancer l'app.
   - Vérifier que les options widget restent identiques.

8. Vérifier reset session runtime:
   - En mode Usage ou Debug, cliquer `Reset session`.
   - Vérifier retour état `idle`.
   - Vérifier qu'un nouveau cycle dictée peut redémarrer normalement.

## Critères de PASS

- Hotkey stable sur 10 cycles.
- Widget met à jour ses états sans rester figé.
- Drag widget fonctionne.
- Injection correcte sur le premier cycle et les suivants.
- Toggle widget + auto-hide fonctionnent et persistent.

## En cas d'échec

- Consulter `Derniere erreur backend` dans l'UI.
- Consulter le log:
  - `%LOCALAPPDATA%\WhisperPro\logs\whisperpro.log`
