# WhisperPro - Etat Reel + Plan de Stabilisation UX

Date: 2026-03-08
Projet: C:\Users\jerem\Desktop\WhisperPro

## 1) Pourquoi ce document

Le projet avance bien techniquement, mais l'experience utilisateur est devenue confuse pendant les iterations Sprint 2.
Objectif: figer l'etat reel, lister les problemes concrets observes, puis definir un plan de remise a plat avant d'aller plus loin.

## 2) Etat reel (ce qui fonctionne aujourd'hui)

### 2.1 Fonctionnalites backend valides

- Capture micro locale (WAV) OK.
- Transcription locale via `whisper-cli` OK.
- Settings SQLite persistants OK:
  - `language`
  - `shortcut`
  - `model_path`
  - `whisper_cli_path`
- Test environnement Whisper + auto-detection chemins OK.
- Logging backend + derniere erreur + chemin log OK.

### 2.2 Fonctionnalites dictée globales deja implementees

- Raccourci global configurable (par `shortcut`) active au lancement.
- Cycle toggle dictée:
  - appui 1: start capture
  - appui 2: stop + transcription + injection
- Injection actuellement forcee en `clipboard + Ctrl+V` (plus stable que frappe directe).

### 2.3 UI actuelle

- Dashboard et Settings dans la fenetre principale.
- Overlay de statut affiche dans la fenetre principale (pas un widget externe).

## 3) Frictions utilisateurs confirmees

### 3.1 Friction focus/fenetre

- L'application se minimise pour tenter de redonner le focus a la cible.
- Cela cree une sensation "bordel/pas pratique" et complique les tests.

### 3.2 Friction injection

- Cas observes de texte partiel/tronque selon l'application cible.
- Le debug peut dire "texte injecte" mais le resultat visuel cible n'est pas toujours complet.

### 3.3 Friction experience globale

- Trop de signaux dans la fenetre principale pendant un usage qui devrait etre "sans regarder l'app".
- Besoin d'un vrai widget/overlay systeme discret et clair.

## 4) Decision de produit (validee)

Avant d'ajouter de nouvelles features, on fait une phase "stabilisation + UX propre".

## 5) Plan de stabilisation (ordre strict)

### WP-UX-001 - Stabiliser le flux dictée/injection

- Retirer la minimisation automatique de la fenetre principale.
- Ajouter un delai/fenetre de securite configurable avant collage.
- Ajouter retry injection (1 tentative supplementaire) si collage vide detecte.
- Instrumenter logs injection:
  - mode injection utilise
  - taille texte
  - statut final

Criteres d'acceptation:
- 20 dictées testees sur Notepad + navigateur + WordPad.
- 0 texte tronque sur ces cas de reference.

### WP-UX-002 - Widget overlay hors fenetre principale

- Creer une fenetre Tauri dediee "widget" (toujours au-dessus, non focalisable autant que possible).
- Etats widget:
  - `idle`
  - `listening`
  - `transcribing`
  - `done`
  - `error`
- Le widget lit les evenements backend `dictation-status`.
- L'application principale peut rester en arriere-plan sans reduire de force.

Criteres d'acceptation:
- L'utilisateur comprend l'etat sans ouvrir WhisperPro.
- Le widget n'interrompt pas la saisie de texte cible.

### WP-UX-003 - Nettoyage interface principale

- Garder Dashboard pour debug/tests, mais separer clairement:
  - mode debug (visible)
  - mode usage quotidien (minimal)
- Clarifier les messages utilisateurs en francais simple.

## 6) Plan implementation technique (prochain dev)

1. Supprimer `prepare_target_focus_for_injection` (ou la desactiver par flag).
2. Centraliser l'injection dans un service unique avec resultat structure:
   - `ok`
   - `error`
   - `mode_used`
3. Emission evenement backend normalisee:
   - `dictation-status` avec `state`, `message`, `timestamp`.
4. Ajouter une vraie fenetre widget Tauri:
   - taille compacte
   - always-on-top
   - deco reduite
   - drag simple
5. Raccorder widget a l'evenement global et tester sur 3 applications cibles.

## 7) Backlog court (Sprints)

### Sprint S2-A (stabilisation)

- [x] WP-UX-001.1 retirer minimisation forcee
- [x] WP-UX-001.2 hardening collage + retry
- [x] WP-UX-001.3 logs inject/hotkey
- [ ] QA manuelle 20 cas

### Sprint S2-B (widget)

- [x] WP-UX-002.1 fenetre widget Tauri
- [x] WP-UX-002.2 design etats widget
- [x] WP-UX-002.3 integration evenements backend
- [ ] QA ergonomie + non-interference focus

### Sprint S2-C (polish)

- [x] WP-UX-003.1 simplification Dashboard
- [x] WP-UX-003.2 messages erreur actionnables (UI + actions rapides ouvrir dossiers/logs)
- [x] WP-UX-003.3 checklist QA Sprint 2 finale

## 8) Ce qu'il reste a faire globalement (macro)

- UX dictée sans friction (objectif principal actuel)
- Commandes vocales de ponctuation
- Export texte/SRT
- Packaging/signature
- Licensing achat unique

## 9) Conclusion

Ce n'est pas "trop". C'est une phase normale de produit: on a prouve la faisabilite technique, maintenant on consolide l'ergonomie.
La bonne strategie est exactement celle proposee par l'utilisateur: documenter, stabiliser, puis reprendre l'interface proprement.
