# WhisperPro - Plan de Refactorisation Progressive (v1.0)

Ce document est notre feuille de route pour modulariser l'application sans regression fonctionnelle visible.

## 1. Objectif

- Modulariser progressivement l'architecture Rust + React.
- Conserver le comportement utilisateur existant pendant toute la transition.
- Reduire le risque de regression via des gates qualite systematiques.

## 2. Principe de securite

- Refactorisation progressive uniquement (pas de big bang).
- Une PR = un perimetre reduit et validable.
- Aucune PR de refacto ne modifie volontairement l'UX ou le wording des statuts.
- On valide chaque phase avec:
  - checks CI,
  - tests automatises,
  - smoke tests manuels.

Note: Le "zero regression absolu" n'existe pas, mais ce process rend les regressions rares, visibles vite, et faciles a corriger.

## 2.1 Etat courant (2026-03-10)

- PR1 validee (baseline/tests/CI).
- PR2 validee (extraction `state/settings_db/commands`).
- PR3 validee (extraction `audio/transcription/translation/injection`).
- PR3.1 validee (reduction `main.rs` via `models/runtime_setup/overlay`).
- PR4 validee (frontend modularise: `tauriApi/tauriEvents`, panels, hook orchestrateur, contrats partages).
- Prochaine etape active: preparation du chantier suivant (post-refacto frontend).

## 3. Baseline obligatoire (avant decoupage)

### 3.1 Contrats a figer

- Commandes Tauri exposees (noms, payloads, retours).
- Evenements emis:
  - `dictation-status`
  - `dictation-transcript`
  - `model-download-progress`
  - `settings-updated`
- Flux critiques:
  - capture -> transcription -> injection,
  - traduction optionnelle,
  - gestion des modeles,
  - widget overlay,
  - raccourci global.

### 3.2 Smoke checklist baseline

Cocher cette checklist avant et apres chaque phase:

- [ ] Lancement app OK (`npm run tauri:dev`).
- [ ] Demarrer capture audio.
- [ ] Arreter + transcrire.
- [ ] Cas "silence" gere.
- [ ] Injection texte dans application cible.
- [ ] Traduction desactivee: texte source uniquement.
- [ ] Traduction activee: texte traduit visible/injecte.
- [ ] Raccourci global demarre/stoppe la dictee.
- [ ] Overlay visible en ecoute/transcription.
- [ ] Overlay se masque en fin de cycle.
- [ ] Telechargement modele avec progression.
- [ ] Annulation telechargement modele.
- [ ] Activation modele.
- [ ] Suppression modele + fallback modele actif.
- [ ] Sauvegarde settings persistante apres relance.
- [ ] Fermeture app propre (`quit_application`).

## 4. Plan d'action doux (PR par PR)

### PR1 - Filets de securite (obligatoire)

But: preparer la suite sans toucher au comportement.

- Ajouter `docs/refactor-baseline.md`:
  - contrats Tauri,
  - evenements,
  - resultats attendus de smoke tests.
- Ajouter premiers tests Rust cibles:
  - normalisation (`language`, `compute_mode`, `translation_target`),
  - `apply_voice_commands`,
  - fallback path settings.
- Ajouter tests DB/settings (init + migration + lecture/ecriture).
- Renforcer CI (build + tests + check workspace).

Critere de sortie PR1:
- [ ] Tous checks CI verts.
- [ ] Smoke checklist baseline validee.

### PR2 - Extraction noyau Rust (state + settings + commandes)

But: commencer la modularisation Rust avec impact minimal.

- Creer modules:
  - `state.rs`
  - `settings_db.rs`
  - `commands.rs`
- Deplacer d'abord fonctions utilitaires et types.
- Garder strictement les memes commandes Tauri et signatures.

Critere de sortie PR2:
- [ ] Aucune difference fonctionnelle visible.
- [ ] Smoke checklist baseline validee.

### PR3 - Extraction audio/transcription/translation/injection

But: isoler le coeur dictee.

- Creer modules:
  - `audio_capture.rs`
  - `transcription.rs`
  - `translation.rs`
  - `injection.rs`
- Deplacer la logique du cycle dictee sans changer les contrats.

Critere de sortie PR3:
- [ ] Dictee complete inchangee.
- [ ] Traduction/fallback inchanges.
- [ ] Smoke checklist baseline validee.

### PR4 - Extraction modeles/runtime/overlay

But: isoler les blocs techniques peripheriques.

- Creer modules:
  - `models.rs`
  - `runtime_setup.rs`
  - `overlay.rs`
- Preserver:
  - progression download modele,
  - auto setup runtime,
  - comportement overlay.

Critere de sortie PR4:
- [ ] Gestion modeles identique.
- [ ] Overlay identique.
- [ ] Smoke checklist baseline validee.

### PR5 - Facade API Frontend

But: retirer les appels Tauri bruts de `App.tsx`.

- Creer:
  - `src/lib/tauriApi.ts` (tous les `invoke`)
  - `src/lib/events.ts` (tous les `listen`)
- `App.tsx` ne garde que la logique UI/etat.

Critere de sortie PR5:
- [ ] Aucun changement UX.
- [ ] Meme comportement evenementiel.
- [ ] Smoke checklist baseline validee.

### PR6 - Decoupage React par features

But: rendre l'UI maintenable et testable.

- Extraire:
  - `features/recording`
  - `features/settings`
  - `features/models`
  - `features/history`
  - `features/overlay`
- Creer un hook orchestrateur (ex: `useDictationController`).
- Conserver `i18n.ts` comme source unique des textes.

Critere de sortie PR6:
- [ ] Ecrans inchanges pour l'utilisateur.
- [ ] Parcours dictee/parametres/historique inchanges.
- [ ] Smoke checklist baseline validee.

## 5. Regles anti-regression (a respecter en continu)

- Aucune suppression/renommage de commande Tauri sans migration explicite.
- Aucune modification de schema DB sans migration compatible.
- Conserver les noms d'evenements Tauri.
- Conserver les valeurs d'etat metier (`idle`, `listening`, `transcribing`, `done`, `error`, `busy`).
- Limiter chaque PR a un axe technique principal.
- Tester sur Windows reel a chaque phase.

## 6. Suivi operationnel

Creer une issue par phase (PR1 -> PR6) avec:

- Scope.
- Risques.
- Checklist baseline copiee.
- Resultat des tests CI.
- Resultat smoke tests.
- Decision Go/No-Go.

## 7. Plan de rollback simple

Si une regression est detectee:

1. Bloquer le merge de la PR concernee.
2. Revenir au dernier tag/commit valide baseline.
3. Ouvrir un correctif cible.
4. Repasser la checklist baseline complete.

## 8. Definition of Done globale

La refactorisation est terminee quand:

- [ ] `main.rs` et `App.tsx` ne portent plus la majorite de la logique metier.
- [ ] Contrats Tauri et UX v1.0 conserves.
- [ ] Tests et CI stables.
- [ ] Documentation d'architecture mise a jour.
