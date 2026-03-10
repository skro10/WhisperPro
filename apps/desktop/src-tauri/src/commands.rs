use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter, State, Window};
use tracing::info;

use crate::*;

#[tauri::command]
pub(crate) fn get_settings(state: State<'_, AppState>) -> Result<UserSettings, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        let settings = get_settings_from_db(
            &app_state.db_path,
            &app_state.model_default_path,
            &app_state.whisper_cli_default_path,
        )?;

        info!(target: "settings", language = %settings.language, shortcut = %settings.shortcut, model = %settings.model_path, whisper_cli = %settings.whisper_cli_path, "settings loaded");
        Ok(settings)
    })
}

#[tauri::command]
pub(crate) fn list_input_devices() -> Result<Vec<InputDeviceInfo>, String> {
    crate::audio_capture::list_input_devices_impl()
}

#[tauri::command]
pub(crate) fn save_settings(app: AppHandle, state: State<'_, AppState>, mut settings: UserSettings) -> Result<(), String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        settings.widget_opacity = clamp_widget_opacity(settings.widget_opacity);
        settings.widget_pop_sound_volume =
            clamp_widget_pop_sound_volume(settings.widget_pop_sound_volume);
        settings.translation_target = normalize_translation_target(&settings.translation_target);
        let conn = open_db(&app_state.db_path)?;
        save_settings_impl(&conn, &settings)?;
        register_or_update_global_shortcut(&app, app_state, &settings.shortcut)?;
        app_state
            .widget_enabled
            .store(settings.widget_enabled, Ordering::SeqCst);
        if !settings.widget_enabled {
            apply_overlay_visibility(&app, false)?;
        }
        let _ = app.emit("settings-updated", settings.clone());
        info!(
            target: "settings",
            language = %settings.language,
            translation_target = %settings.translation_target,
            shortcut = %settings.shortcut,
            model = %settings.model_path,
            whisper_cli = %settings.whisper_cli_path,
            input_device_id = %settings.input_device_id,
            push_to_talk_hold = settings.push_to_talk_hold,
            secure_text_mode = settings.secure_text_mode,
            silence_gate_enabled = settings.silence_gate_enabled,
            compute_mode = %settings.compute_mode,
            keep_model_loaded = settings.keep_model_loaded,
            widget_enabled = settings.widget_enabled,
            widget_autohide = settings.widget_autohide,
            widget_opacity = settings.widget_opacity,
            widget_pop_sound_volume = settings.widget_pop_sound_volume,
            widget_pop_sound = %settings.widget_pop_sound,
            voice_commands_enabled = settings.voice_commands_enabled,
            onboarding_completed = settings.onboarding_completed,
            "settings saved"
        );
        Ok(())
    })
}

#[tauri::command]
pub(crate) fn save_widget_preferences(
    app: AppHandle,
    state: State<'_, AppState>,
    widget_opacity: f32,
    widget_pop_sound_volume: f32,
    widget_pop_sound: String,
) -> Result<UserSettings, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        let mut settings = get_settings_from_db(
            &app_state.db_path,
            &app_state.model_default_path,
            &app_state.whisper_cli_default_path,
        )?;

        settings.widget_opacity = clamp_widget_opacity(widget_opacity);
        settings.widget_pop_sound_volume = clamp_widget_pop_sound_volume(widget_pop_sound_volume);
        settings.widget_pop_sound = normalize_widget_pop_sound(&widget_pop_sound);

        let conn = open_db(&app_state.db_path)?;
        save_settings_impl(&conn, &settings)?;
        let _ = app.emit("settings-updated", settings.clone());
        info!(
            target: "settings",
            widget_opacity = settings.widget_opacity,
            widget_pop_sound_volume = settings.widget_pop_sound_volume,
            widget_pop_sound = %settings.widget_pop_sound,
            "widget preferences saved"
        );
        Ok(settings)
    })
}

#[tauri::command]
pub(crate) fn get_default_model_path(state: State<'_, AppState>) -> String {
    state.inner().model_default_path.to_string_lossy().to_string()
}

#[tauri::command]
pub(crate) fn get_default_whisper_cli_path(state: State<'_, AppState>) -> String {
    state
        .inner()
        .whisper_cli_default_path
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
pub(crate) fn get_compute_capability(state: State<'_, AppState>) -> Result<ComputeCapabilityReport, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        let settings = get_settings_from_db(
            &app_state.db_path,
            &app_state.model_default_path,
            &app_state.whisper_cli_default_path,
        )?;
        Ok(detect_compute_capability_for_cli(Path::new(&settings.whisper_cli_path)))
    })
}

#[tauri::command]
pub(crate) fn auto_setup_runtime(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        ensure_runtime_dependencies(&app, app_state)?;
        Ok("Moteur Whisper verifie et optimise.".to_string())
    })
}

#[tauri::command]
pub(crate) fn transcribe_wav(
    state: State<'_, AppState>,
    wav_path: String,
    model_id: Option<String>,
) -> Result<TranscriptionResult, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        let mut settings = get_settings_from_db(
            &app_state.db_path,
            &app_state.model_default_path,
            &app_state.whisper_cli_default_path,
        )?;

        if let Some(id) = model_id.as_deref() {
            if let Some(entry) = model_catalog_entry(id) {
                let model_dir = models_dir(app_state)?;
                let candidate = model_dir.join(entry.filename);
                if candidate.exists() {
                    let candidate_path = candidate.to_string_lossy().to_string();
                    if settings.model_path != candidate_path {
                        settings.model_path = candidate_path;
                        let conn = open_db(&app_state.db_path)?;
                        save_settings_impl(&conn, &settings)?;
                    }
                }
            }
        }

        let model_path = PathBuf::from(settings.model_path.clone());
        let whisper_cli_path = PathBuf::from(settings.whisper_cli_path.clone());
        let wav = PathBuf::from(wav_path.clone());

        if !model_path.exists() {
            return Err(err_model_missing(&model_path));
        }

        if !whisper_cli_path.exists() {
            return Err(err_cli_missing(&whisper_cli_path));
        }

        if !wav.exists() {
            return Err(err_wav_missing(&wav));
        }

        if settings.silence_gate_enabled && is_probably_silent_wav(&wav)? {
            return Ok(TranscriptionResult {
                text: String::new(),
                segments: vec![],
                model_path: model_path.to_string_lossy().to_string(),
                wav_path: wav.to_string_lossy().to_string(),
            });
        }

        let mut result = transcribe_with_strategy(
            app_state,
            &whisper_cli_path,
            &model_path,
            &wav,
            &settings.language,
            &settings.compute_mode,
            false,
            false,
        )?;
        if settings.voice_commands_enabled {
            let processed = apply_voice_commands(&result.text);
            result.text = processed.clone();
            result.segments = if processed.trim().is_empty() {
                vec![]
            } else {
                vec![TranscriptSegment {
                    start_ms: 0,
                    end_ms: 0,
                    text: processed,
                }]
            };
        }
        info!(target: "asr", model = %result.model_path, wav = %result.wav_path, segments = result.segments.len(), "transcription completed");
        Ok(result)
    })
}

#[tauri::command]
pub(crate) fn translate_wav_to_english(state: State<'_, AppState>, wav_path: String) -> Result<TranscriptionResult, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        let settings = get_settings_from_db(
            &app_state.db_path,
            &app_state.model_default_path,
            &app_state.whisper_cli_default_path,
        )?;

        let model_path = PathBuf::from(settings.model_path.clone());
        let whisper_cli_path = PathBuf::from(settings.whisper_cli_path.clone());
        let wav = PathBuf::from(wav_path.clone());

        if !model_path.exists() {
            return Err(err_model_missing(&model_path));
        }

        if !whisper_cli_path.exists() {
            return Err(err_cli_missing(&whisper_cli_path));
        }

        if !wav.exists() {
            return Err(err_wav_missing(&wav));
        }

        let result = transcribe_with_strategy(
            app_state,
            &whisper_cli_path,
            &model_path,
            &wav,
            &settings.language,
            &settings.compute_mode,
            false,
            true,
        )?;
        info!(target: "asr", model = %result.model_path, wav = %result.wav_path, "translation completed");
        Ok(result)
    })
}

#[tauri::command]
pub(crate) fn translate_text(text: String, target_lang: String, source_lang: Option<String>) -> Result<String, String> {
    translate_text_impl(text, target_lang, source_lang)
}

#[tauri::command]
pub(crate) fn test_whisper_environment(state: State<'_, AppState>) -> Result<WhisperEnvironmentReport, String> {
    let app_state = state.inner();

    with_error_log(app_state, || {
        let mut settings = get_settings_from_db(
            &app_state.db_path,
            &app_state.model_default_path,
            &app_state.whisper_cli_default_path,
        )?;

        let mut notes: Vec<String> = Vec::new();
        let mut auto_updated = false;

        let settings_model_exists = Path::new(&settings.model_path).exists();
        let settings_cli_exists = Path::new(&settings.whisper_cli_path).exists();

        if !settings_model_exists {
            if let Some(detected_model) = first_existing_path(&candidate_model_paths(app_state, &settings)) {
                settings.model_path = detected_model.to_string_lossy().to_string();
                auto_updated = true;
                notes.push("Chemin modele auto-detecte et applique.".to_string());
            }
        }

        if !settings_cli_exists {
            if let Some(detected_cli) = first_existing_path(&candidate_cli_paths(app_state, &settings)) {
                settings.whisper_cli_path = detected_cli.to_string_lossy().to_string();
                auto_updated = true;
                notes.push("Chemin whisper-cli auto-detecte et applique.".to_string());
            }
        }

        if auto_updated {
            let conn = open_db(&app_state.db_path)?;
            save_settings_impl(&conn, &settings)?;
            info!(target: "settings", model = %settings.model_path, whisper_cli = %settings.whisper_cli_path, "settings auto-updated by environment test");
        }

        let model_exists = Path::new(&settings.model_path).exists();
        let whisper_cli_exists = Path::new(&settings.whisper_cli_path).exists();

        if !model_exists {
            notes.push(format!(
                "Modele introuvable. Place un .bin dans {} ou ajuste le chemin Settings.",
                app_state.model_default_path.to_string_lossy()
            ));
        }

        if !whisper_cli_exists {
            notes.push(format!(
                "whisper-cli introuvable. Place whisper-cli.exe dans {} ou ajuste le chemin Settings.",
                app_state.whisper_cli_default_path.to_string_lossy()
            ));
        }

        if model_exists && whisper_cli_exists && notes.is_empty() {
            notes.push("Environnement Whisper pret.".to_string());
        }

        Ok(WhisperEnvironmentReport {
            ready: model_exists && whisper_cli_exists,
            model_path: settings.model_path,
            model_exists,
            whisper_cli_path: settings.whisper_cli_path,
            whisper_cli_exists,
            auto_updated,
            notes,
        })
    })
}

#[tauri::command]
pub(crate) fn get_last_error(state: State<'_, AppState>) -> Option<String> {
    state.inner().last_error.lock().clone()
}

#[tauri::command]
pub(crate) fn get_log_path(state: State<'_, AppState>) -> String {
    state.inner().log_path.to_string_lossy().to_string()
}

#[tauri::command]
pub(crate) fn get_dictation_status(state: State<'_, AppState>) -> DictationStatusEvent {
    state.inner().dictation_status.lock().clone()
}

#[tauri::command]
pub(crate) fn get_last_dictation_transcript(state: State<'_, AppState>) -> Option<DictationTranscriptEvent> {
    state.inner().latest_dictation_transcript.lock().clone()
}

#[tauri::command]
pub(crate) fn list_models(state: State<'_, AppState>) -> Result<Vec<ModelInfo>, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        let settings = get_settings_from_db(
            &app_state.db_path,
            &app_state.model_default_path,
            &app_state.whisper_cli_default_path,
        )?;
        list_models_impl(app_state, &settings)
    })
}

#[tauri::command]
pub(crate) async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<String, String> {
    let app_state = state.inner();
    if app_state
        .model_download_in_progress
        .swap(true, Ordering::SeqCst)
    {
        return Err("Un telechargement est deja en cours".to_string());
    }
    app_state
        .model_download_cancel
        .store(false, Ordering::SeqCst);
    {
        let mut active = app_state.model_download_active_id.lock();
        *active = Some(model_id.clone());
    }
    let db_path = app_state.db_path.clone();
    let model_default_path = app_state.model_default_path.clone();
    let whisper_cli_default_path = app_state.whisper_cli_default_path.clone();
    let model_id_for_error = model_id.clone();
    let app_for_job = app.clone();
    let download_cancel = app_state.model_download_cancel.clone();

    let job = tauri::async_runtime::spawn_blocking(move || {
        download_model_with_paths(
            &app_for_job,
            db_path,
            model_default_path,
            whisper_cli_default_path,
            model_id,
            download_cancel,
        )
    });

    let result = match job.await {
        Ok(Ok(message)) => Ok(message),
        Ok(Err(e)) => {
            if e == "Telechargement annule" {
                emit_model_download_progress(
                    &app,
                    ModelDownloadProgressEvent {
                        model_id: model_id_for_error,
                        status: "canceled".to_string(),
                        progress_pct: None,
                        downloaded_bytes: 0,
                        total_bytes: None,
                        message: "Telechargement annule".to_string(),
                    },
                );
                Ok("Telechargement annule".to_string())
            } else {
                emit_model_download_progress(
                    &app,
                    ModelDownloadProgressEvent {
                        model_id: model_id_for_error,
                        status: "error".to_string(),
                        progress_pct: None,
                        downloaded_bytes: 0,
                        total_bytes: None,
                        message: e.clone(),
                    },
                );
                Err(e)
            }
        }
        Err(e) => {
            let msg = format!("Telechargement interrompu: {e}");
            emit_model_download_progress(
                &app,
                ModelDownloadProgressEvent {
                    model_id: model_id_for_error,
                    status: "error".to_string(),
                    progress_pct: None,
                    downloaded_bytes: 0,
                    total_bytes: None,
                    message: msg.clone(),
                },
            );
            Err(msg)
        }
    };

    app_state
        .model_download_in_progress
        .store(false, Ordering::SeqCst);
    app_state
        .model_download_cancel
        .store(false, Ordering::SeqCst);
    {
        let mut active = app_state.model_download_active_id.lock();
        *active = None;
    }

    result
}

#[tauri::command]
pub(crate) fn cancel_model_download(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let app_state = state.inner();
    if !app_state.model_download_in_progress.load(Ordering::SeqCst) {
        return Ok("Aucun telechargement en cours".to_string());
    }
    app_state.model_download_cancel.store(true, Ordering::SeqCst);
    let active_id = app_state
        .model_download_active_id
        .lock()
        .clone()
        .unwrap_or_else(|| "inconnu".to_string());
    emit_model_download_progress(
        &app,
        ModelDownloadProgressEvent {
            model_id: active_id,
            status: "canceling".to_string(),
            progress_pct: None,
            downloaded_bytes: 0,
            total_bytes: None,
            message: "Annulation du telechargement...".to_string(),
        },
    );
    Ok("Annulation demandee".to_string())
}

#[tauri::command]
pub(crate) fn set_active_model(state: State<'_, AppState>, model_id: String) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || set_active_model_impl(app_state, &model_id))
}

#[tauri::command]
pub(crate) fn delete_model(state: State<'_, AppState>, model_id: String) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || delete_model_impl(app_state, &model_id))
}

#[tauri::command]
pub(crate) fn generate_diagnostic_snapshot(state: State<'_, AppState>) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        let settings = get_settings_from_db(
            &app_state.db_path,
            &app_state.model_default_path,
            &app_state.whisper_cli_default_path,
        )?;

        let model_exists = Path::new(&settings.model_path).exists();
        let cli_exists = Path::new(&settings.whisper_cli_path).exists();
        let status = app_state.dictation_status.lock().clone();
        let last_error = app_state
            .last_error
            .lock()
            .clone()
            .unwrap_or_else(|| "Aucune".to_string());

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Horodatage impossible: {e}"))?
            .as_millis();

        let log_dir = app_state
            .log_path
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| "Dossier logs introuvable".to_string())?;
        fs::create_dir_all(&log_dir).map_err(|e| format!("Creation dossier logs impossible: {e}"))?;
        let snapshot_path = log_dir.join(format!("diagnostic-{stamp}.txt"));

        let content = format!(
            "WhisperPro Diagnostic Snapshot\nDateEpochMs: {stamp}\n\nPaths\n- DB: {}\n- Log: {}\n- Model: {} ({})\n- whisper-cli: {} ({})\n\nSettings\n- language: {}\n- translation_target: {}\n- shortcut: {}\n- input_device_id: {}\n- push_to_talk_hold: {}\n- secure_text_mode: {}\n- silence_gate_enabled: {}\n- widget_enabled: {}\n- widget_autohide: {}\n- widget_opacity: {:.2}\n- widget_pop_sound_volume: {:.2}\n- widget_pop_sound: {}\n- voice_commands_enabled: {}\n- onboarding_completed: {}\n\nRuntime\n- dictation_state: {}\n- dictation_message: {}\n- dictation_recording: {}\n- dictation_busy: {}\n- last_error: {}\n",
            app_state.db_path.to_string_lossy(),
            app_state.log_path.to_string_lossy(),
            settings.model_path,
            if model_exists { "OK" } else { "KO" },
            settings.whisper_cli_path,
            if cli_exists { "OK" } else { "KO" },
            settings.language,
            settings.translation_target,
            settings.shortcut,
            settings.input_device_id,
            settings.push_to_talk_hold,
            settings.secure_text_mode,
            settings.silence_gate_enabled,
            settings.widget_enabled,
            settings.widget_autohide,
            settings.widget_opacity,
            settings.widget_pop_sound_volume,
            settings.widget_pop_sound,
            settings.voice_commands_enabled,
            settings.onboarding_completed,
            status.state,
            status.message,
            app_state.dictation_recording.load(Ordering::SeqCst),
            app_state.dictation_busy.load(Ordering::SeqCst),
            last_error
        );

        fs::write(&snapshot_path, content).map_err(|e| format!("Ecriture snapshot impossible: {e}"))?;
        Ok(snapshot_path.to_string_lossy().to_string())
    })
}

#[tauri::command]
pub(crate) fn start_overlay_drag(window: Window) -> Result<(), String> {
    window
        .start_dragging()
        .map_err(|e| format!("Drag widget impossible: {e}"))
}

#[tauri::command]
pub(crate) fn open_path_in_explorer(path: String) -> Result<(), String> {
    let raw = path.trim();
    if raw.is_empty() {
        return Err("Chemin vide: impossible d'ouvrir l'explorateur.".to_string());
    }

    let target = PathBuf::from(raw);
    if target.is_file() {
        Command::new("explorer.exe")
            .arg(format!("/select,{}", target.to_string_lossy()))
            .spawn()
            .map_err(|e| format!("Ouverture explorateur impossible: {e}"))?;
        return Ok(());
    }

    if target.is_dir() {
        Command::new("explorer.exe")
            .arg(&target)
            .spawn()
            .map_err(|e| format!("Ouverture dossier impossible: {e}"))?;
        return Ok(());
    }

    let fallback = target
        .parent()
        .filter(|p| p.exists())
        .map(|p| p.to_path_buf())
        .ok_or_else(|| format!("Chemin introuvable: {}", target.to_string_lossy()))?;

    Command::new("explorer.exe")
        .arg(fallback)
        .spawn()
        .map_err(|e| format!("Ouverture dossier parent impossible: {e}"))?;
    Ok(())
}

#[tauri::command]
pub(crate) fn open_external_url(url: String) -> Result<(), String> {
    let target = url.trim();
    if target.is_empty() {
        return Err("URL vide: impossible d'ouvrir la page.".to_string());
    }

    if !(target.starts_with("http://") || target.starts_with("https://")) {
        return Err("URL invalide: seul http/https est accepte.".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer.exe")
            .arg(target)
            .spawn()
            .map_err(|e| format!("Ouverture URL impossible: {e}"))?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("Ouverture URL impossible: {e}"))?;
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn clear_history_artifacts(
    state: State<'_, AppState>,
    payload: ClearHistoryArtifactsPayload,
) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        let mut targets: HashSet<PathBuf> = HashSet::new();
        let sidecar_exts = ["txt", "json", "srt", "vtt", "csv", "tsv", "lrc"];

        for raw in payload.wav_paths {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let wav = PathBuf::from(trimmed);
            targets.insert(wav.clone());
            if let (Some(parent), Some(stem)) = (wav.parent(), wav.file_stem()) {
                for ext in sidecar_exts {
                    targets.insert(parent.join(format!("{}.{}", stem.to_string_lossy(), ext)));
                }
            }
        }

        let temp_dir = std::env::temp_dir();
        let _ = targets.insert(temp_dir.join("whisperpro_recording.wav"));

        let app_root = app_state
            .db_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| temp_dir.clone());

        // Purge transcript artifact directories explicitly.
        let mut transcript_dirs: Vec<PathBuf> = vec![
            temp_dir.join("whisperpro_transcripts"),
            temp_dir.join("whisperprotranscript"),
            app_root.join("whisperpro_transcripts"),
            app_root.join("whisperprotranscript"),
        ];
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let base = PathBuf::from(local_app_data);
            transcript_dirs.push(base.join("WhisperPro").join("whisperpro_transcripts"));
            transcript_dirs.push(base.join("WhisperPro").join("whisperprotranscript"));
            transcript_dirs.push(base.join("whisperpro_transcripts"));
            transcript_dirs.push(base.join("whisperprotranscript"));
        }

        for dir in transcript_dirs {
            if !dir.exists() || !dir.is_dir() {
                continue;
            }
            if let Ok(read_dir) = fs::read_dir(&dir) {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let _ = targets.insert(path);
                    }
                }
            }
        }
        let temp_norm = normalize_windows_path_string(&temp_dir);
        let app_root_norm = normalize_windows_path_string(&app_root);

        let mut deleted = 0u32;
        let mut skipped = 0u32;
        for path in targets {
            let path_norm = normalize_windows_path_string(&path);
            let allowed = path_norm.starts_with(&temp_norm) || path_norm.starts_with(&app_root_norm);
            if !allowed {
                skipped += 1;
                continue;
            }
            if path.exists() && path.is_file() {
                match remove_file_with_retry(&path) {
                    Ok(_) => deleted += 1,
                    Err(_) => skipped += 1,
                }
            }
        }

        Ok(format!(
            "Historique vide. Fichiers supprimes: {deleted}. Ignorés: {skipped}."
        ))
    })
}

fn remove_file_with_retry(path: &Path) -> Result<(), std::io::Error> {
    let mut last_err: Option<std::io::Error> = None;
    for _ in 0..20 {
        match fs::remove_file(path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::other("suppression fichier impossible")))
}

fn normalize_windows_path_string(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('/', "\\").to_lowercase();
    raw.trim_start_matches("\\\\?\\").to_string()
}

#[tauri::command]
pub(crate) fn quit_application(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(runtime) = state.inner().whisper_server.lock().as_mut() {
        let _ = runtime.child.kill();
        let _ = runtime.child.wait();
    }
    info!(target: "app", "quit requested from UI");
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub(crate) fn start_capture(state: State<'_, AppState>) -> Result<String, String> {
    start_capture_impl(state.inner())
}

#[tauri::command]
pub(crate) fn stop_capture(state: State<'_, AppState>) -> Result<String, String> {
    stop_capture_impl(state.inner())
}

#[tauri::command]
pub(crate) fn toggle_dictation_cycle(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || toggle_dictation_cycle_impl(&app, app_state))
}

#[tauri::command]
pub(crate) fn reset_runtime_state(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        let mut notes: Vec<String> = Vec::new();

        match stop_capture_impl(app_state) {
            Ok(path) => notes.push(format!("Capture stoppee ({path})")),
            Err(e) => {
                if e != "Aucune capture en cours" {
                    notes.push(format!("Stop capture partiel: {e}"));
                }
            }
        }

        app_state.dictation_recording.store(false, Ordering::SeqCst);
        app_state.dictation_busy.store(false, Ordering::SeqCst);
        *app_state.last_error.lock() = None;

        let base = "Etat runtime reinitialise".to_string();
        let message = if notes.is_empty() {
            base
        } else {
            format!("{base}. {}", notes.join(" | "))
        };

        emit_dictation_status(&app, "idle", &message);
        info!(target: "runtime", message = %message, "runtime state reset");
        Ok(message)
    })
}





