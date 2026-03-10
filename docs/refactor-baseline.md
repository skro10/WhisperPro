# WhisperPro - Baseline anti-regression (PR1)

Document de reference pour verifier que la refactorisation ne change pas le comportement v1.0.

## 1) Contrat commandes Tauri

Commandes exposees par le backend:

- `start_capture`
- `stop_capture`
- `toggle_dictation_cycle`
- `reset_runtime_state`
- `transcribe_wav`
- `test_whisper_environment`
- `get_settings`
- `save_settings`
- `get_default_model_path`
- `get_default_whisper_cli_path`
- `get_compute_capability`
- `auto_setup_runtime`
- `translate_wav_to_english`
- `translate_text`
- `list_models`
- `download_model`
- `cancel_model_download`
- `set_active_model`
- `delete_model`
- `get_last_error`
- `get_log_path`
- `get_dictation_status`
- `get_last_dictation_transcript`
- `generate_diagnostic_snapshot`
- `start_overlay_drag`
- `clear_history_artifacts`
- `open_path_in_explorer`
- `quit_application`

## 2) Contrat evenements Tauri

Evenements emis:

- `dictation-status`
- `dictation-transcript`
- `model-download-progress`
- `settings-updated`

## 3) Etats de dictee a conserver

Les valeurs d'etat suivantes ne doivent pas changer:

- `idle`
- `listening`
- `transcribing`
- `busy`
- `done`
- `error`

## 4) Smoke tests manuels

Rejouer cette checklist avant/apres chaque phase de refacto:

- [ ] `npm run tauri:dev` demarre correctement.
- [ ] Start capture fonctionne.
- [ ] Stop + transcribe fonctionne.
- [ ] Cas silence retourne une transcription vide sans crash.
- [ ] Injection texte fonctionne dans une application cible.
- [ ] Traduction OFF: texte source conserve.
- [ ] Traduction ON: texte traduit visible et injectable.
- [ ] Raccourci global demarre/arrete la dictee.
- [ ] Overlay visible pendant `listening/transcribing`.
- [ ] Overlay masque apres `done/error`.
- [ ] Telechargement modele avec progression.
- [ ] Annulation telechargement modele.
- [ ] Activation d'un modele installe.
- [ ] Suppression modele + fallback modele actif.
- [ ] Settings sauvegardes et persistants apres relance.
- [ ] `quit_application` ferme proprement.

## 5) Validation CI minimum

La phase est validee seulement si la CI passe avec:

- `npm run build`
- `npm run typecheck`
- `cargo check --workspace`
- `cargo test --workspace`

## 6) Depannage local (Tauri build path stale)

Symptome possible:

- erreur `failed to read plugin permissions ... app_hide.toml`
- chemin absolu qui pointe vers un ancien dossier de travail.

Correctif local:

1. `cargo clean`
2. relancer `cargo check --workspace`
3. relancer `cargo test --workspace`

## 7) Depannage local (raccourci global deja reserve)

Symptome possible au lancement `tauri:dev`:

- `Impossible d'enregistrer le raccourci global ... HotKey already registered`

Correctif local:

1. fermer toute instance precedente de WhisperPro (ou autre app qui reserve le meme raccourci)
2. relancer `npm run tauri:dev`
