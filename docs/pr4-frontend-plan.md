# WhisperPro - PR4 Frontend (preparation)

Plan operationnel pour lancer la refactorisation frontend de facon progressive, sans changement fonctionnel volontaire.

## 1. Point de depart (2026-03-10)

- Frontend principal concentre dans `apps/desktop/src/App.tsx` (~1755 lignes).
- Textes UI centralises dans `apps/desktop/src/i18n.ts`.
- Build frontend OK (`npm run build`).
- Typecheck ajoute a partir de PR4 prep (`npm run typecheck`).

## 2. Contrats frontend a figer

## 2.1 Invokes Tauri utilises

- `start_capture`
- `stop_capture`
- `transcribe_wav`
- `translate_wav_to_english`
- `translate_text`
- `get_settings`
- `save_settings`
- `list_models`
- `download_model`
- `cancel_model_download`
- `set_active_model`
- `delete_model`
- `get_compute_capability`
- `auto_setup_runtime`
- `test_whisper_environment`
- `get_dictation_status`
- `clear_history_artifacts`
- `start_overlay_drag`
- `quit_application`

## 2.2 Events Tauri ecoutes

- `dictation-status`
- `dictation-transcript`
- `model-download-progress`
- `settings-updated`
- `ui-language-changed`

## 2.3 Etats UI critiques a conserver

- cycle dictee: `idle`, `listening`, `transcribing`, `busy`, `done`, `error`
- panneaux: `settingsOpen`, historique, vue texte source/traduite
- progression telechargement modele (starting/downloading/canceling/complete/error)

## 3. Strategie anti-regression

- Refacto par sous-PR (pas de big-bang).
- Chaque sous-PR garde les memes commandes/events.
- Pas de modification CSS globale non necessaire pendant extraction.
- Validation obligatoire a chaque etape:
  - `npm run build`
  - `npm run typecheck`
  - smoke manuel rapide (section 5)

## 4. Decoupage PR4 propose

## PR4.1 - Facade API et events

But: sortir tous les `invoke` et `listen` de `App.tsx`.

- Creer `src/lib/tauriApi.ts` (wrappers types pour invoke).
- Creer `src/lib/tauriEvents.ts` (abonnements + desabonnements).
- Remplacer les appels directs dans `App.tsx` par la facade.

Critere de sortie:
- `App.tsx` ne contient plus d'appel direct `invoke`/`listen`.
- Build + typecheck OK.

## PR4.2 - Types et constantes metier

But: reduire le bruit structurel dans `App.tsx`.

- Creer `src/features/shared/types.ts` (UserSettings, ModelInfo, etc.).
- Creer `src/features/shared/constants.ts` (defaults, options, limites).
- Conserver strictement les memes valeurs par defaut.

Critere de sortie:
- Aucun changement de comportement.
- Build + typecheck OK.

## PR4.3 - Extraction OverlayWidget

But: isoler la fenetre overlay et ses transitions d'etat.

- Creer `src/features/overlay/OverlayWidget.tsx`.
- Extraire la logique audio pop + state machine visuelle.
- Garder les memes classes CSS pour eviter les regressions visuelles.

Critere de sortie:
- Overlay identique (apparition, animation, autohide).
- Build + typecheck OK.

## PR4.4 - Extraction Settings/Models

But: sortir le bloc le plus volumineux de `MainApp`.

- Creer `src/features/settings/SettingsDrawer.tsx`.
- Creer `src/features/models/ModelLibrary.tsx`.
- Passer par props explicites (pas de changement de data flow initial).

Critere de sortie:
- Parametres, modeles, runtime repair inchanges.
- Build + typecheck OK.

## PR4.5 - Extraction Dictation/History + hook orchestrateur

But: terminer le decoupage de `MainApp`.

- Creer `src/features/dictation/DictationPanel.tsx`.
- Creer `src/features/history/HistoryPanel.tsx`.
- Introduire `src/features/app/useMainAppController.ts` pour centraliser orchestration.

Critere de sortie:
- `App.tsx` devient composeur (routing overlay/main + composition).
- Build + typecheck OK.

## 5. Smoke checklist PR4

- [ ] Lancement desktop (`npm run tauri:dev`)
- [ ] Start/Stop dictee
- [ ] Transcription visible et copiable
- [ ] Traduction ON/OFF conforme
- [ ] Ouverture/fermeture panneau settings
- [ ] Sauvegarde settings persistante
- [ ] Telechargement/annulation/suppression modele
- [ ] Overlay actif pendant dictee
- [ ] Historique: ajout/copie/suppression/clear
- [ ] Quit app propre

## 6. Regles de merge

- Une seule sous-PR frontend ouverte a la fois.
- Merge uniquement si check auto + smoke manuel OK.
- En cas de regression: rollback de la sous-PR et correctif cible.

## 7. Cloture PR4 (2026-03-10)

Sous-etapes executees:

- [x] PR4.1 - facade `tauriApi` + `tauriEvents`
- [x] PR4.2 - extraction `types.ts` + `constants.ts`
- [x] PR4.3 - extraction `OverlayWidget.tsx`
- [x] PR4.4 - extraction `SettingsDrawer.tsx` + `ModelLibrary.tsx`
- [x] PR4.5 - extraction `DictationPanel.tsx` + `HistoryPanel.tsx` + `useMainAppController.ts`
- [x] PR4.6 - contrats partages + reduction du volume de props
- [x] PR4.7 - harmonisation finale des noms (`dictation/history/settings`)

Validations executees:

- `npm run typecheck` OK
- `npm run build` OK

Smoke runtime:

- `npm run tauri:dev` demarre correctement apres liberation d'un process existant qui occupait le raccourci global.
- Blocage initial observe puis resolu localement: `HotKey already registered`.
- Les parcours UI complets restent a rejouer sur machine utilisateur (interaction fenetre requise):
  - start/stop dictee
  - traduction ON/OFF
  - settings save/reset
  - modeles download/activate/delete
  - overlay
  - historique
