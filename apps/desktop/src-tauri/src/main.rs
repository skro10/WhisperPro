#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use arboard::Clipboard;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, Window, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutEvent, ShortcutState};
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn apply_no_window(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserSettings {
    language: String,
    translation_target: String,
    shortcut: String,
    model_path: String,
    whisper_cli_path: String,
    compute_mode: String,
    keep_model_loaded: bool,
    widget_enabled: bool,
    widget_autohide: bool,
    widget_opacity: f32,
    widget_pop_sound_volume: f32,
    widget_pop_sound: String,
    voice_commands_enabled: bool,
    onboarding_completed: bool,
}

impl UserSettings {
    fn with_defaults(model_path: String, whisper_cli_path: String) -> Self {
        Self {
            language: "auto".to_string(),
            translation_target: "none".to_string(),
            shortcut: "Ctrl+Shift+Space".to_string(),
            model_path,
            whisper_cli_path,
            compute_mode: "auto".to_string(),
            keep_model_loaded: false,
            widget_enabled: true,
            widget_autohide: true,
            widget_opacity: 0.9,
            widget_pop_sound_volume: 0.65,
            widget_pop_sound: "sound1.mp3".to_string(),
            voice_commands_enabled: true,
            onboarding_completed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct TranscriptSegment {
    start_ms: i64,
    end_ms: i64,
    text: String,
}

#[derive(Debug, Clone, Serialize)]
struct TranscriptionResult {
    text: String,
    segments: Vec<TranscriptSegment>,
    model_path: String,
    wav_path: String,
}

#[derive(Debug, Clone, Serialize)]
struct WhisperEnvironmentReport {
    ready: bool,
    model_path: String,
    model_exists: bool,
    whisper_cli_path: String,
    whisper_cli_exists: bool,
    auto_updated: bool,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ComputeCapabilityReport {
    gpu_available: bool,
    supports_ngl: bool,
    supports_no_gpu_flag: bool,
    whisper_cli_path: String,
    details: String,
}

#[derive(Debug, Clone, Copy)]
enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
struct DictationStatusEvent {
    state: String,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct DictationTranscriptEvent {
    text: String,
    injected_text: String,
    translation_applied: bool,
    translation_target: String,
    wav_path: String,
    model_path: String,
    created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
struct ModelInfo {
    id: String,
    label: String,
    filename: String,
    installed: bool,
    active: bool,
    size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct ModelDownloadProgressEvent {
    model_id: String,
    status: String,
    progress_pct: Option<u8>,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ClearHistoryArtifactsPayload {
    #[serde(default, alias = "wav_paths", rename = "wavPaths")]
    wav_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct ModelCatalogEntry {
    id: &'static str,
    label: &'static str,
    filename: &'static str,
    download_url: &'static str,
}

struct CaptureSession {
    stop_tx: Sender<()>,
    worker: JoinHandle<Result<String, String>>,
    output_path: PathBuf,
}

struct AppState {
    capture: Mutex<Option<CaptureSession>>,
    db_path: PathBuf,
    log_path: PathBuf,
    model_default_path: PathBuf,
    whisper_cli_default_path: PathBuf,
    last_error: Mutex<Option<String>>,
    dictation_recording: AtomicBool,
    dictation_busy: AtomicBool,
    registered_shortcut: Mutex<Option<String>>,
    dictation_status: Mutex<DictationStatusEvent>,
    latest_dictation_transcript: Mutex<Option<DictationTranscriptEvent>>,
    last_successful_injection: Mutex<Option<(String, Instant)>>,
    widget_enabled: AtomicBool,
    overlay_last_position: Mutex<Option<(f64, f64)>>,
    overlay_hide_token: AtomicU64,
    whisper_server: Mutex<Option<WhisperServerRuntime>>,
    model_download_in_progress: Arc<AtomicBool>,
    model_download_cancel: Arc<AtomicBool>,
    model_download_active_id: Mutex<Option<String>>,
}

struct WhisperServerRuntime {
    child: std::process::Child,
    model_path: String,
    language: String,
    compute_mode: String,
    translate_to_english: bool,
    port: u16,
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<UserSettings, String> {
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
fn save_settings(app: AppHandle, state: State<'_, AppState>, mut settings: UserSettings) -> Result<(), String> {
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
fn get_default_model_path(state: State<'_, AppState>) -> String {
    state.inner().model_default_path.to_string_lossy().to_string()
}

#[tauri::command]
fn get_default_whisper_cli_path(state: State<'_, AppState>) -> String {
    state
        .inner()
        .whisper_cli_default_path
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
fn get_compute_capability(state: State<'_, AppState>) -> Result<ComputeCapabilityReport, String> {
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
fn auto_setup_runtime(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || {
        ensure_runtime_dependencies(&app, app_state)?;
        Ok("Moteur Whisper verifie et optimise.".to_string())
    })
}

#[tauri::command]
fn transcribe_wav(
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

        if is_probably_silent_wav(&wav)? {
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
fn translate_wav_to_english(state: State<'_, AppState>, wav_path: String) -> Result<TranscriptionResult, String> {
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
fn translate_text(text: String, target_lang: String, source_lang: Option<String>) -> Result<String, String> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return Ok(String::new());
    }

    let target = normalize_language(&target_lang);
    if target.is_empty() {
        return Err("Langue cible invalide.".to_string());
    }
    let source = source_lang
        .as_deref()
        .map(normalize_language)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "auto".to_string());
    let url = "https://translate.googleapis.com/translate_a/single";
    let client = reqwest::blocking::Client::new();
    let translate_once = |sl: &str| -> Result<String, String> {
        let params = [
            ("client", "gtx".to_string()),
            ("sl", sl.to_string()),
            ("tl", target.clone()),
            ("dt", "t".to_string()),
            ("q", cleaned.to_string()),
        ];

        let response = match client.post(url).form(&params).send() {
            Ok(resp) => resp,
            Err(_) => client
                .get(url)
                .query(&[
                    ("client", "gtx"),
                    ("sl", sl),
                    ("tl", target.as_str()),
                    ("dt", "t"),
                    ("q", cleaned),
                ])
                .send()
                .map_err(|e| format!("Traduction impossible (reseau): {e}"))?,
        };

        if !response.status().is_success() {
            return Err(format!(
                "Traduction impossible: serveur {}",
                response.status()
            ));
        }

        let payload = response
            .text()
            .map_err(|e| format!("Lecture reponse traduction impossible: {e}"))?;
        let json: serde_json::Value =
            serde_json::from_str(&payload).map_err(|e| format!("Reponse traduction invalide: {e}"))?;

        let mut out = String::new();
        if let Some(segments) = json.get(0).and_then(|v| v.as_array()) {
            for seg in segments {
                if let Some(part) = seg.get(0).and_then(|v| v.as_str()) {
                    out.push_str(part);
                }
            }
        }

        Ok(out.trim().to_string())
    };

    let first = translate_once(&source)?;
    if !first.is_empty() && !first.eq_ignore_ascii_case(cleaned) {
        return Ok(first);
    }

    if source != "auto" {
        let second = translate_once("auto")?;
        if !second.is_empty() {
            return Ok(second);
        }
    }

    if !first.is_empty() {
        return Ok(first);
    }

    Err("La traduction n'a renvoye aucun texte.".to_string())
}

#[tauri::command]
fn test_whisper_environment(state: State<'_, AppState>) -> Result<WhisperEnvironmentReport, String> {
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
fn get_last_error(state: State<'_, AppState>) -> Option<String> {
    state.inner().last_error.lock().clone()
}

#[tauri::command]
fn get_log_path(state: State<'_, AppState>) -> String {
    state.inner().log_path.to_string_lossy().to_string()
}

#[tauri::command]
fn get_dictation_status(state: State<'_, AppState>) -> DictationStatusEvent {
    state.inner().dictation_status.lock().clone()
}

#[tauri::command]
fn get_last_dictation_transcript(state: State<'_, AppState>) -> Option<DictationTranscriptEvent> {
    state.inner().latest_dictation_transcript.lock().clone()
}

#[tauri::command]
fn list_models(state: State<'_, AppState>) -> Result<Vec<ModelInfo>, String> {
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
async fn download_model(app: AppHandle, state: State<'_, AppState>, model_id: String) -> Result<String, String> {
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
fn cancel_model_download(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
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
fn set_active_model(state: State<'_, AppState>, model_id: String) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || set_active_model_impl(app_state, &model_id))
}

#[tauri::command]
fn delete_model(state: State<'_, AppState>, model_id: String) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || delete_model_impl(app_state, &model_id))
}

#[tauri::command]
fn generate_diagnostic_snapshot(state: State<'_, AppState>) -> Result<String, String> {
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
            "WhisperPro Diagnostic Snapshot\nDateEpochMs: {stamp}\n\nPaths\n- DB: {}\n- Log: {}\n- Model: {} ({})\n- whisper-cli: {} ({})\n\nSettings\n- language: {}\n- translation_target: {}\n- shortcut: {}\n- widget_enabled: {}\n- widget_autohide: {}\n- widget_opacity: {:.2}\n- widget_pop_sound_volume: {:.2}\n- widget_pop_sound: {}\n- voice_commands_enabled: {}\n- onboarding_completed: {}\n\nRuntime\n- dictation_state: {}\n- dictation_message: {}\n- dictation_recording: {}\n- dictation_busy: {}\n- last_error: {}\n",
            app_state.db_path.to_string_lossy(),
            app_state.log_path.to_string_lossy(),
            settings.model_path,
            if model_exists { "OK" } else { "KO" },
            settings.whisper_cli_path,
            if cli_exists { "OK" } else { "KO" },
            settings.language,
            settings.translation_target,
            settings.shortcut,
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
fn start_overlay_drag(window: Window) -> Result<(), String> {
    window
        .start_dragging()
        .map_err(|e| format!("Drag widget impossible: {e}"))
}

#[tauri::command]
fn open_path_in_explorer(path: String) -> Result<(), String> {
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
fn clear_history_artifacts(
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
fn quit_application(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if let Some(runtime) = state.inner().whisper_server.lock().as_mut() {
        let _ = runtime.child.kill();
        let _ = runtime.child.wait();
    }
    info!(target: "app", "quit requested from UI");
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn start_capture(state: State<'_, AppState>) -> Result<String, String> {
    start_capture_impl(state.inner())
}

#[tauri::command]
fn stop_capture(state: State<'_, AppState>) -> Result<String, String> {
    stop_capture_impl(state.inner())
}

#[tauri::command]
fn toggle_dictation_cycle(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let app_state = state.inner();
    with_error_log(app_state, || toggle_dictation_cycle_impl(&app, app_state))
}

#[tauri::command]
fn reset_runtime_state(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
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

fn start_capture_impl(app_state: &AppState) -> Result<String, String> {
    let mut guard = app_state.capture.lock();
    if guard.is_some() {
        return Err("Capture deja en cours".to_string());
    }

    let output_path = std::env::temp_dir().join("whisperpro_recording.wav");
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let output_path_for_thread = output_path.clone();

    let worker = thread::spawn(move || run_capture_loop(output_path_for_thread, stop_rx));

    *guard = Some(CaptureSession {
        stop_tx,
        worker,
        output_path: output_path.clone(),
    });

    info!(target: "audio", output = %output_path.to_string_lossy(), "capture started");
    Ok(output_path.to_string_lossy().to_string())
}

fn stop_capture_impl(app_state: &AppState) -> Result<String, String> {
    let mut guard = app_state.capture.lock();
    let Some(session) = guard.take() else {
        return Err("Aucune capture en cours".to_string());
    };

    let _ = session.stop_tx.send(());

    let worker_result = session
        .worker
        .join()
        .map_err(|_| "Le thread de capture a panique".to_string())?;

    let output = worker_result?;
    info!(target: "audio", output = %output, "capture stopped");
    Ok(session.output_path.to_string_lossy().to_string())
}

fn transcribe_wav_impl(app_state: &AppState, wav_path: &str) -> Result<TranscriptionResult, String> {
    let settings = get_settings_from_db(
        &app_state.db_path,
        &app_state.model_default_path,
        &app_state.whisper_cli_default_path,
    )?;

    let model_path = PathBuf::from(settings.model_path.clone());
    let whisper_cli_path = PathBuf::from(settings.whisper_cli_path.clone());
    let wav = PathBuf::from(wav_path);

    if !model_path.exists() {
        return Err(err_model_missing(&model_path));
    }

    if !whisper_cli_path.exists() {
        return Err(err_cli_missing(&whisper_cli_path));
    }

    if !wav.exists() {
        return Err(err_wav_missing(&wav));
    }

    if is_probably_silent_wav(&wav)? {
        return Ok(TranscriptionResult {
            text: String::new(),
            segments: vec![],
            model_path: model_path.to_string_lossy().to_string(),
            wav_path: wav.to_string_lossy().to_string(),
        });
    }

    transcribe_with_strategy(
        app_state,
        &whisper_cli_path,
        &model_path,
        &wav,
        &settings.language,
        &settings.compute_mode,
        false,
        false,
    )
}

fn toggle_dictation_cycle_impl(app: &AppHandle, app_state: &AppState) -> Result<String, String> {
    if app_state.dictation_busy.load(Ordering::SeqCst) {
        let message = "Traitement de dictee deja en cours".to_string();
        emit_dictation_status(app, "busy", &message);
        return Ok(message);
    }

    if !app_state.dictation_recording.load(Ordering::SeqCst) {
        let _wav_path = start_capture_impl(app_state)?;
        app_state.dictation_recording.store(true, Ordering::SeqCst);
        let message = "Ecoute en cours...".to_string();
        emit_dictation_status(app, "listening", &message);
        return Ok(message);
    }

    app_state.dictation_recording.store(false, Ordering::SeqCst);
    app_state.dictation_busy.store(true, Ordering::SeqCst);
    emit_dictation_status(app, "transcribing", "Transcription en cours...");

    let result = (|| -> Result<String, String> {
        let wav_path = stop_capture_impl(app_state)?;
        let transcript = transcribe_wav_impl(app_state, &wav_path)?;
        let settings = get_settings_from_db(
            &app_state.db_path,
            &app_state.model_default_path,
            &app_state.whisper_cli_default_path,
        )?;
        let post_processed_text = if settings.voice_commands_enabled {
            apply_voice_commands(&transcript.text)
        } else {
            transcript.text.clone()
        };
        if post_processed_text.trim().is_empty() {
            return Ok("Aucune parole detectee".to_string());
        }

        let text_for_injection = if normalize_translation_target(&settings.translation_target) != "none" {
            match translate_text(
                post_processed_text.clone(),
                settings.translation_target.clone(),
                Some(settings.language.clone()),
            ) {
                Ok(translated) if !translated.trim().is_empty() => translated,
                Ok(_) => post_processed_text.clone(),
                Err(e) => {
                    info!(target: "translate", reason = %e, "translation failed during injection, fallback to source text");
                    post_processed_text.clone()
                }
            }
        } else {
            post_processed_text.clone()
        };

        if should_skip_duplicate_injection(app_state, &text_for_injection) {
            return Ok("Texte deja injecte (doublon evite)".to_string());
        }

        let report = inject_text_with_retry(&text_for_injection)?;
        mark_successful_injection(app_state, &text_for_injection);
        info!(
            target: "inject",
            mode = report.mode,
            attempts = report.attempts,
            text_len = report.text_len,
            raw_len = transcript.text.chars().count(),
            "text injection succeeded"
        );
        let created_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Horodatage impossible: {e}"))?
            .as_millis() as u64;
        let payload = DictationTranscriptEvent {
            text: post_processed_text.clone(),
            injected_text: text_for_injection.clone(),
            translation_applied: normalize_translation_target(&settings.translation_target) != "none",
            translation_target: normalize_translation_target(&settings.translation_target),
            wav_path: wav_path.clone(),
            model_path: settings.model_path.clone(),
            created_at_ms,
        };
        *app_state.latest_dictation_transcript.lock() = Some(payload.clone());
        if let Err(e) = app.emit("dictation-transcript", payload) {
            error!(target: "inject", reason = %e, "broadcast dictation-transcript failed");
        }
        Ok("Texte injecte".to_string())
    })();

    app_state.dictation_busy.store(false, Ordering::SeqCst);
    match &result {
        Ok(message) => emit_dictation_status(app, "done", message),
        Err(message) => emit_dictation_status(app, "error", message),
    }
    result
}

fn register_or_update_global_shortcut(
    app: &AppHandle,
    app_state: &AppState,
    shortcut: &str,
) -> Result<(), String> {
    let shortcut = shortcut.trim();
    if shortcut.is_empty() {
        return Err("Raccourci global vide. Exemple valide: Ctrl+Shift+Space".to_string());
    }

    let mut registered = app_state.registered_shortcut.lock();
    if registered.as_deref() == Some(shortcut) {
        return Ok(());
    }

    if let Some(previous) = registered.as_ref() {
        let _ = app.global_shortcut().unregister(previous.as_str());
    }

    app.global_shortcut()
        .on_shortcut(shortcut, move |app_handle, _hotkey, event: ShortcutEvent| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            let handle = app_handle.clone();
            thread::spawn(move || {
                let state = handle.state::<AppState>();
                if let Err(e) = toggle_dictation_cycle_impl(&handle, state.inner()) {
                    record_error(state.inner(), &e);
                }
            });
        })
        .map_err(|e| format!("Impossible d'enregistrer le raccourci global '{shortcut}': {e}"))?;

    *registered = Some(shortcut.to_string());
    info!(target: "hotkey", shortcut = %shortcut, "global shortcut registered");
    Ok(())
}

fn emit_dictation_status(app: &AppHandle, state: &str, message: &str) {
    let app_state = app.state::<AppState>();
    let widget_enabled = app_state.widget_enabled.load(Ordering::SeqCst);
    let token = app_state.overlay_hide_token.fetch_add(1, Ordering::SeqCst) + 1;
    if widget_enabled && matches!(state, "listening" | "transcribing" | "busy") {
        let _ = show_overlay_near_cursor(app);
    }

    let payload = DictationStatusEvent {
        state: state.to_string(),
        message: message.to_string(),
    };
    *app.state::<AppState>().dictation_status.lock() = payload.clone();

    if let Err(e) = app.emit("dictation-status", payload.clone()) {
        error!(target: "overlay", reason = %e, "broadcast dictation-status failed");
    }

    if widget_enabled && matches!(state, "done" | "error") {
        let app_handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(560));
            let latest_token = app_handle
                .state::<AppState>()
                .overlay_hide_token
                .load(Ordering::SeqCst);
            if latest_token == token {
                let _ = apply_overlay_visibility(&app_handle, false);
            }
        });
    }
}

fn should_skip_duplicate_injection(app_state: &AppState, text: &str) -> bool {
    let guard = app_state.last_successful_injection.lock();
    if let Some((last_text, last_at)) = guard.as_ref() {
        if last_text == text && last_at.elapsed() < Duration::from_millis(1200) {
            return true;
        }
    }
    false
}

fn mark_successful_injection(app_state: &AppState, text: &str) {
    *app_state.last_successful_injection.lock() = Some((text.to_string(), Instant::now()));
}

struct InjectionReport {
    mode: &'static str,
    attempts: u8,
    text_len: usize,
}

fn inject_text_with_retry(text: &str) -> Result<InjectionReport, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(InjectionReport {
            mode: "clipboard-paste",
            attempts: 0,
            text_len: 0,
        });
    }

    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard indisponible: {e}"))?;
    let previous_clipboard = clipboard.get_text().ok();

    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("Ecriture clipboard impossible: {e}"))?;

    // Petite pause pour laisser la fenetre cible stable avant collage.
    thread::sleep(Duration::from_millis(120));

    let mut last_error: Option<String> = None;
    let mut attempts: u8 = 0;
    for _ in 0..2 {
        attempts += 1;
        match send_ctrl_v() {
            Ok(()) => {
                restore_clipboard_later(previous_clipboard);
                return Ok(InjectionReport {
                    mode: "clipboard-paste",
                    attempts,
                    text_len: text.chars().count(),
                });
            }
            Err(e) => {
                last_error = Some(e);
                thread::sleep(Duration::from_millis(80));
            }
        }
    }

    restore_clipboard_later(previous_clipboard);

    let reason = last_error.unwrap_or_else(|| "Erreur d'injection inconnue".to_string());
    error!(target: "inject", attempts = attempts, reason = %reason, "text injection failed");
    Err(reason)
}

fn restore_clipboard_later(previous_clipboard: Option<String>) {
    let Some(previous) = previous_clipboard else {
        return;
    };

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(700));
        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(previous);
        }
    });
}

fn send_ctrl_v() -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|e| format!("Injection clavier indisponible: {e}"))?;

    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| format!("Injection Ctrl down impossible: {e}"))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| format!("Injection V impossible: {e}"))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| format!("Injection Ctrl up impossible: {e}"))
}

fn with_error_log<T, F>(state: &AppState, f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    match f() {
        Ok(v) => Ok(v),
        Err(e) => {
            record_error(state, &e);
            Err(e)
        }
    }
}

fn record_error(state: &AppState, message: &str) {
    *state.last_error.lock() = Some(message.to_string());
    error!(target: "app", message = %message, "operation failed");
}

fn open_db(db_path: &PathBuf) -> Result<Connection, String> {
    Connection::open(db_path).map_err(|e| format!("Ouverture DB impossible: {e}"))
}

fn get_settings_from_db(
    db_path: &PathBuf,
    default_model_path: &Path,
    default_whisper_cli_path: &Path,
) -> Result<UserSettings, String> {
    let conn = open_db(db_path)?;
    let fallback_model_path = default_model_path.to_string_lossy().to_string();
    let fallback_whisper_cli_path = default_whisper_cli_path.to_string_lossy().to_string();

    let mut stmt = conn
        .prepare("SELECT language, translation_target, shortcut, model_path, whisper_cli_path, compute_mode, keep_model_loaded, widget_enabled, widget_autohide, voice_commands_enabled, onboarding_completed, widget_opacity, widget_pop_sound_volume, widget_pop_sound FROM settings WHERE id = 1")
        .map_err(|e| format!("Lecture settings impossible: {e}"))?;

    let mut rows = stmt
        .query([])
        .map_err(|e| format!("Lecture settings impossible: {e}"))?;

    if let Some(row) = rows
        .next()
        .map_err(|e| format!("Lecture settings impossible: {e}"))?
    {
        let model_path_from_db: String = row
            .get::<_, String>(3)
            .map_err(|e| format!("Lecture model_path impossible: {e}"))?;
        let whisper_cli_from_db: String = row
            .get::<_, String>(4)
            .map_err(|e| format!("Lecture whisper_cli_path impossible: {e}"))?;
        let compute_mode_from_db: String = row
            .get::<_, String>(5)
            .map_err(|e| format!("Lecture compute_mode impossible: {e}"))?;
        let keep_model_loaded: i64 = row
            .get::<_, i64>(6)
            .map_err(|e| format!("Lecture keep_model_loaded impossible: {e}"))?;
        let widget_enabled: i64 = row
            .get::<_, i64>(7)
            .map_err(|e| format!("Lecture widget_enabled impossible: {e}"))?;
        let widget_autohide: i64 = row
            .get::<_, i64>(8)
            .map_err(|e| format!("Lecture widget_autohide impossible: {e}"))?;
        let voice_commands_enabled: i64 = row
            .get::<_, i64>(9)
            .map_err(|e| format!("Lecture voice_commands_enabled impossible: {e}"))?;
        let onboarding_completed: i64 = row
            .get::<_, i64>(10)
            .map_err(|e| format!("Lecture onboarding_completed impossible: {e}"))?;
        let widget_opacity: f64 = row
            .get::<_, f64>(11)
            .map_err(|e| format!("Lecture widget_opacity impossible: {e}"))?;
        let widget_pop_sound_volume: f64 = row
            .get::<_, f64>(12)
            .map_err(|e| format!("Lecture widget_pop_sound_volume impossible: {e}"))?;
        let widget_pop_sound: String = row
            .get::<_, String>(13)
            .map_err(|e| format!("Lecture widget_pop_sound impossible: {e}"))?;

        let selected_model_path = resolve_active_model_path(&model_path_from_db, &fallback_model_path);
        let selected_cli_path = if whisper_cli_from_db.trim().is_empty() {
            fallback_whisper_cli_path.clone()
        } else {
            whisper_cli_from_db.clone()
        };

        let settings = UserSettings {
            language: row
                .get::<_, String>(0)
                .map_err(|e| format!("Lecture language impossible: {e}"))?,
            translation_target: normalize_translation_target(
                &row.get::<_, String>(1)
                    .map_err(|e| format!("Lecture translation_target impossible: {e}"))?,
            ),
            shortcut: row
                .get::<_, String>(2)
                .map_err(|e| format!("Lecture shortcut impossible: {e}"))?,
            model_path: selected_model_path.clone(),
            whisper_cli_path: selected_cli_path.clone(),
            compute_mode: normalize_compute_mode(&compute_mode_from_db),
            keep_model_loaded: keep_model_loaded != 0,
            widget_enabled: widget_enabled != 0,
            widget_autohide: widget_autohide != 0,
            widget_opacity: clamp_widget_opacity(widget_opacity as f32),
            widget_pop_sound_volume: clamp_widget_pop_sound_volume(widget_pop_sound_volume as f32),
            widget_pop_sound: normalize_widget_pop_sound(&widget_pop_sound),
            voice_commands_enabled: voice_commands_enabled != 0,
            onboarding_completed: onboarding_completed != 0,
        };

        if settings.model_path != model_path_from_db || settings.whisper_cli_path != whisper_cli_from_db {
            save_settings_impl(&conn, &settings)?;
        }
        return Ok(settings);
    }

    let defaults = UserSettings::with_defaults(fallback_model_path, fallback_whisper_cli_path);
    save_settings_impl(&conn, &defaults)?;
    Ok(defaults)
}

fn save_settings_impl(conn: &Connection, settings: &UserSettings) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (id, language, translation_target, shortcut, model_path, whisper_cli_path, compute_mode, keep_model_loaded, widget_enabled, widget_autohide, voice_commands_enabled, onboarding_completed, widget_opacity, widget_pop_sound_volume, widget_pop_sound) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(id) DO UPDATE SET language = excluded.language, translation_target = excluded.translation_target, shortcut = excluded.shortcut, model_path = excluded.model_path, whisper_cli_path = excluded.whisper_cli_path, compute_mode = excluded.compute_mode, keep_model_loaded = excluded.keep_model_loaded, widget_enabled = excluded.widget_enabled, widget_autohide = excluded.widget_autohide, voice_commands_enabled = excluded.voice_commands_enabled, onboarding_completed = excluded.onboarding_completed, widget_opacity = excluded.widget_opacity, widget_pop_sound_volume = excluded.widget_pop_sound_volume, widget_pop_sound = excluded.widget_pop_sound",
        params![
            settings.language,
            normalize_translation_target(&settings.translation_target),
            settings.shortcut,
            settings.model_path,
            settings.whisper_cli_path,
            normalize_compute_mode(&settings.compute_mode),
            if settings.keep_model_loaded { 1 } else { 0 },
            if settings.widget_enabled { 1 } else { 0 },
            if settings.widget_autohide { 1 } else { 0 },
            if settings.voice_commands_enabled { 1 } else { 0 },
            if settings.onboarding_completed { 1 } else { 0 },
            clamp_widget_opacity(settings.widget_opacity),
            clamp_widget_pop_sound_volume(settings.widget_pop_sound_volume),
            normalize_widget_pop_sound(&settings.widget_pop_sound)
        ],
    )
    .map_err(|e| format!("Sauvegarde settings impossible: {e}"))?;

    Ok(())
}

fn init_db(db_path: &PathBuf, default_model_path: &Path, default_whisper_cli_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Init DB impossible: {e}"))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            language TEXT NOT NULL,
            shortcut TEXT NOT NULL
        );",
    )
    .map_err(|e| format!("Migration settings impossible: {e}"))?;

    let _ = conn.execute("ALTER TABLE settings ADD COLUMN model_path TEXT NOT NULL DEFAULT ''", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN whisper_cli_path TEXT NOT NULL DEFAULT ''", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN translation_target TEXT NOT NULL DEFAULT 'none'", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN compute_mode TEXT NOT NULL DEFAULT 'auto'", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN keep_model_loaded INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN widget_enabled INTEGER NOT NULL DEFAULT 1", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN widget_autohide INTEGER NOT NULL DEFAULT 1", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN voice_commands_enabled INTEGER NOT NULL DEFAULT 1", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN onboarding_completed INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN widget_opacity REAL NOT NULL DEFAULT 0.9", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN widget_pop_sound_volume REAL NOT NULL DEFAULT 0.65", []);
    let _ = conn.execute("ALTER TABLE settings ADD COLUMN widget_pop_sound TEXT NOT NULL DEFAULT 'sound1.mp3'", []);

    let existing_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM settings WHERE id = 1", [], |row| row.get(0))
        .map_err(|e| format!("Verification settings impossible: {e}"))?;
    if existing_count == 0 {
        let defaults = UserSettings::with_defaults(
            default_model_path.to_string_lossy().to_string(),
            default_whisper_cli_path.to_string_lossy().to_string(),
        );
        save_settings_impl(&conn, &defaults)?;
    }
    Ok(())
}

fn resolve_active_model_path(model_path_from_db: &str, fallback_model_path: &str) -> String {
    let configured = model_path_from_db.trim();
    if !configured.is_empty() && Path::new(configured).exists() {
        return configured.to_string();
    }

    if Path::new(fallback_model_path).exists() {
        return fallback_model_path.to_string();
    }

    if let Some(model_dir) = Path::new(fallback_model_path).parent() {
        for entry in MODEL_CATALOG {
            let candidate = model_dir.join(entry.filename);
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }

        if let Ok(read_dir) = fs::read_dir(model_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let is_bin = path
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_lowercase() == "bin")
                    .unwrap_or(false);
                if is_bin {
                    return path.to_string_lossy().to_string();
                }
            }
        }
    }

    if !configured.is_empty() {
        configured.to_string()
    } else {
        fallback_model_path.to_string()
    }
}

fn resolve_app_dir() -> Result<PathBuf, String> {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());

    let app_dir = base.join("WhisperPro");
    fs::create_dir_all(&app_dir).map_err(|e| format!("Creation dossier app impossible: {e}"))?;
    Ok(app_dir)
}

fn resolve_db_path(app_dir: &PathBuf) -> PathBuf {
    app_dir.join("whisperpro.db")
}

fn resolve_log_path(app_dir: &PathBuf) -> Result<PathBuf, String> {
    let log_dir = app_dir.join("logs");
    fs::create_dir_all(&log_dir).map_err(|e| format!("Creation dossier logs impossible: {e}"))?;
    Ok(log_dir.join("whisperpro.log"))
}

fn resolve_model_default_path(app_dir: &PathBuf) -> Result<PathBuf, String> {
    let model_dir = app_dir.join("models");
    fs::create_dir_all(&model_dir).map_err(|e| format!("Creation dossier modeles impossible: {e}"))?;
    Ok(model_dir.join("ggml-base.bin"))
}

fn resolve_whisper_cli_default_path(app_dir: &PathBuf) -> Result<PathBuf, String> {
    let bin_dir = app_dir.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|e| format!("Creation dossier bin impossible: {e}"))?;
    Ok(bin_dir.join("whisper-cli.exe"))
}

fn init_logger(log_path: &PathBuf) -> Result<WorkerGuard, String> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| format!("Ouverture log impossible: {e}"))?;

    let (non_blocking, guard) = tracing_appender::non_blocking(file);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = fmt::Subscriber::builder()
        .with_ansi(false)
        .with_env_filter(env_filter)
        .with_writer(non_blocking)
        .json()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| format!("Initialisation logger impossible: {e}"))?;

    Ok(guard)
}

fn run_capture_loop(output_path: PathBuf, stop_rx: Receiver<()>) -> Result<String, String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "Aucun micro detecte".to_string())?;

    let config = device
        .default_input_config()
        .map_err(|e| format!("Configuration micro invalide: {e}"))?;

    let spec = hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = hound::WavWriter::create(&output_path, spec)
        .map_err(|e| format!("Impossible de creer le WAV: {e}"))?;
    let writer = std::sync::Arc::new(Mutex::new(Some(writer)));

    let writer_for_stream = std::sync::Arc::clone(&writer);
    let err_fn = |err| eprintln!("Erreur flux audio: {err}");

    let stream_config = config.clone().into();
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            build_stream_f32(&device, &stream_config, writer_for_stream, err_fn)
        }
        cpal::SampleFormat::I16 => {
            build_stream_i16(&device, &stream_config, writer_for_stream, err_fn)
        }
        cpal::SampleFormat::U16 => {
            build_stream_u16(&device, &stream_config, writer_for_stream, err_fn)
        }
        other => return Err(format!("Format audio non supporte: {other:?}")),
    }
    .map_err(|e| format!("Impossible de demarrer le flux audio: {e}"))?;

    stream
        .play()
        .map_err(|e| format!("Impossible de lire le flux audio: {e}"))?;

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(30));
    }

    drop(stream);

    if let Some(writer) = writer.lock().take() {
        writer
            .finalize()
            .map_err(|e| format!("Impossible de finaliser le WAV: {e}"))?;
    }

    Ok(output_path.to_string_lossy().to_string())
}

fn build_stream_f32(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    writer: std::sync::Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    device.build_input_stream(
        config,
        move |data: &[f32], _| {
            if let Some(writer) = writer.lock().as_mut() {
                for &sample in data {
                    let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                    let _ = writer.write_sample(value);
                }
            }
        },
        err_fn,
        None,
    )
}

fn build_stream_i16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    writer: std::sync::Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    device.build_input_stream(
        config,
        move |data: &[i16], _| {
            if let Some(writer) = writer.lock().as_mut() {
                for &sample in data {
                    let _ = writer.write_sample(sample);
                }
            }
        },
        err_fn,
        None,
    )
}

fn build_stream_u16(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    writer: std::sync::Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    device.build_input_stream(
        config,
        move |data: &[u16], _| {
            if let Some(writer) = writer.lock().as_mut() {
                for &sample in data {
                    let normalized = sample as i32 - 32768;
                    let value = normalized as i16;
                    let _ = writer.write_sample(value);
                }
            }
        },
        err_fn,
        None,
    )
}

fn transcribe_with_strategy(
    app_state: &AppState,
    whisper_cli_path: &Path,
    model_path: &Path,
    wav_path: &Path,
    language: &str,
    compute_mode: &str,
    keep_model_loaded: bool,
    translate_to_english: bool,
) -> Result<TranscriptionResult, String> {
    if keep_model_loaded {
        match run_transcription_via_server(
            app_state,
            whisper_cli_path,
            model_path,
            wav_path,
            language,
            compute_mode,
            translate_to_english,
        ) {
            Ok(result) => return Ok(result),
            Err(e) => {
                info!(target: "asr", reason = %e, "server mode failed, fallback to cli mode");
            }
        }
    }

    run_transcription_cli(
        whisper_cli_path,
        model_path,
        wav_path,
        language,
        compute_mode,
        translate_to_english,
    )
}

fn run_transcription_cli(
    whisper_cli_path: &Path,
    model_path: &Path,
    wav_path: &Path,
    language: &str,
    compute_mode: &str,
    translate_to_english: bool,
) -> Result<TranscriptionResult, String> {
    let out_dir = std::env::temp_dir().join("whisperpro_transcripts");
    fs::create_dir_all(&out_dir).map_err(|e| format!("Creation dossier transcripts impossible: {e}"))?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Horodatage impossible: {e}"))?
        .as_millis();
    let out_prefix = out_dir.join(format!("transcript_{stamp}"));

    let normalized_lang = normalize_language(language);
    let normalized_compute = normalize_compute_mode(compute_mode);
    let capability = detect_compute_capability_for_cli(whisper_cli_path);
    if normalized_compute == "gpu" && !capability.gpu_available {
        return Err(format!(
            "Mode GPU indisponible avec ce whisper-cli.\n{}\nAction: utilise Auto/CPU ou installe une build whisper-cli avec backend GPU.",
            capability.details
        ));
    }
    let output = if normalized_compute == "cpu" {
        run_whisper_once(
            whisper_cli_path,
            model_path,
            wav_path,
            &normalized_lang,
            &out_prefix,
            translate_to_english,
            "cpu",
        )?
    } else if normalized_compute == "gpu" {
        run_whisper_once(
            whisper_cli_path,
            model_path,
            wav_path,
            &normalized_lang,
            &out_prefix,
            translate_to_english,
            "gpu",
        )
        .map_err(|err| {
            format!(
                "{err}\nAction: Passe le mode de calcul sur Auto ou CPU si le GPU n'est pas disponible."
            )
        })?
    } else {
        if !capability.gpu_available {
            run_whisper_once(
                whisper_cli_path,
                model_path,
                wav_path,
                &normalized_lang,
                &out_prefix,
                translate_to_english,
                "cpu",
            )?
        } else {
            match run_whisper_once(
                whisper_cli_path,
                model_path,
                wav_path,
                &normalized_lang,
                &out_prefix,
                translate_to_english,
                "gpu",
            ) {
            Ok(out) => out,
            Err(gpu_err) => {
                info!(target: "asr", error = %gpu_err, "gpu transcription failed, retrying on cpu");
                run_whisper_once(
                    whisper_cli_path,
                    model_path,
                    wav_path,
                    &normalized_lang,
                    &out_prefix,
                    translate_to_english,
                    "cpu",
                )
                .map_err(|cpu_err| {
                    format!(
                        "Echec en mode Auto.\nGPU: {gpu_err}\nCPU: {cpu_err}\nAction: Verifie le modele, la langue et le WAV dans Settings."
                    )
                })?
            }
        }
        }
    };

    let txt_path = out_prefix.with_extension("txt");
    let text = if txt_path.exists() {
        fs::read_to_string(&txt_path).map_err(|e| format!("Lecture transcript txt impossible: {e}"))?
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    let cleaned = text.trim().to_string();
    let segments = if cleaned.is_empty() {
        vec![]
    } else {
        vec![TranscriptSegment {
            start_ms: 0,
            end_ms: 0,
            text: cleaned.clone(),
        }]
    };

    Ok(TranscriptionResult {
        text: cleaned,
        segments,
        model_path: model_path.to_string_lossy().to_string(),
        wav_path: wav_path.to_string_lossy().to_string(),
    })
}

fn run_whisper_once(
    whisper_cli_path: &Path,
    model_path: &Path,
    wav_path: &Path,
    normalized_lang: &str,
    out_prefix: &Path,
    translate_to_english: bool,
    compute_mode: &str,
) -> Result<std::process::Output, String> {
    let (supports_ngl, supports_ng) = detect_whisper_gpu_flags(whisper_cli_path);
    let mut cmd = Command::new(whisper_cli_path);
    apply_no_window(&mut cmd);
    cmd.arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(wav_path)
        .arg("-l")
        .arg(normalized_lang)
        .arg("-of")
        .arg(out_prefix)
        .arg("-otxt")
        .arg("-np")
        .arg("-nt");

    // whisper.cpp has two common CLI variants:
    // - newer: -ngl <N> (GPU layers)
    // - older: -ng / --no-gpu
    // We adapt dynamically to keep compatibility with both.
    if compute_mode == "cpu" {
        if supports_ng {
            cmd.arg("-ng");
        } else if supports_ngl {
            cmd.arg("-ngl").arg("0");
        }
    } else if compute_mode == "gpu" && supports_ngl {
        cmd.arg("-ngl").arg("999");
    }

    if translate_to_english {
        cmd.arg("-tr");
    }

    let output = cmd.output().map_err(|e| {
        format!(
            "Impossible de lancer whisper-cli.\nProbleme: {e}\nAction: Verifie le chemin whisper-cli dans Settings et relance l'application."
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "Transcription echouee (whisper-cli code {:?}).\nstdout: {}\nstderr: {}",
            output.status.code(),
            stdout.trim(),
            stderr.trim()
        ));
    }

    Ok(output)
}

fn run_transcription_via_server(
    app_state: &AppState,
    whisper_cli_path: &Path,
    model_path: &Path,
    wav_path: &Path,
    language: &str,
    compute_mode: &str,
    translate_to_english: bool,
) -> Result<TranscriptionResult, String> {
    let server_exe = whisper_cli_path
        .parent()
        .map(|p| p.join("whisper-server.exe"))
        .ok_or_else(|| "Dossier bin whisper introuvable.".to_string())?;
    if !server_exe.exists() {
        return Err("whisper-server.exe introuvable.".to_string());
    }

    let normalized_lang = normalize_language(language);
    let normalized_compute = normalize_compute_mode(compute_mode);
    let capability = detect_compute_capability_for_cli(whisper_cli_path);

    let port = ensure_whisper_server_running(
        app_state,
        &server_exe,
        model_path,
        &normalized_lang,
        &normalized_compute,
        translate_to_english,
        &capability,
    )?;

    let url = format!("http://127.0.0.1:{port}/inference");
    let file_part = reqwest::blocking::multipart::Part::file(wav_path)
        .map_err(|e| format!("Lecture WAV pour serveur impossible: {e}"))?;
    let form = reqwest::blocking::multipart::Form::new()
        .text("response_format", "json")
        .part("file", file_part);
    let response = reqwest::blocking::Client::new()
        .post(url)
        .multipart(form)
        .send()
        .map_err(|e| format!("Inference serveur whisper impossible: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Serveur whisper en erreur (HTTP {}).",
            response.status()
        ));
    }

    let body = response
        .text()
        .map_err(|e| format!("Lecture reponse serveur whisper impossible: {e}"))?;
    let payload: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("JSON serveur whisper invalide: {e}"))?;

    let text = if let Some(t) = payload.get("text").and_then(|v| v.as_str()) {
        t.to_string()
    } else if let Some(segments) = payload.get("segments").and_then(|v| v.as_array()) {
        segments
            .iter()
            .filter_map(|s| s.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    } else {
        String::new()
    };

    let cleaned = text.trim().to_string();
    let segments = if cleaned.is_empty() {
        vec![]
    } else {
        vec![TranscriptSegment {
            start_ms: 0,
            end_ms: 0,
            text: cleaned.clone(),
        }]
    };

    Ok(TranscriptionResult {
        text: cleaned,
        segments,
        model_path: model_path.to_string_lossy().to_string(),
        wav_path: wav_path.to_string_lossy().to_string(),
    })
}

fn ensure_whisper_server_running(
    app_state: &AppState,
    server_exe: &Path,
    model_path: &Path,
    language: &str,
    compute_mode: &str,
    translate_to_english: bool,
    capability: &ComputeCapabilityReport,
) -> Result<u16, String> {
    let mut guard = app_state.whisper_server.lock();

    if let Some(runtime) = guard.as_mut() {
        let same_config = runtime.model_path == model_path.to_string_lossy()
            && runtime.language == language
            && runtime.compute_mode == compute_mode
            && runtime.translate_to_english == translate_to_english;
        if same_config {
            if runtime.child.try_wait().ok().flatten().is_none() {
                return Ok(runtime.port);
            }
        }
        let _ = runtime.child.kill();
        let _ = runtime.child.wait();
        *guard = None;
    }

    let port: u16 = 8178;
    let mut cmd = Command::new(server_exe);
    apply_no_window(&mut cmd);
    cmd.arg("-m")
        .arg(model_path)
        .arg("-l")
        .arg(language)
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("-nt");

    if translate_to_english {
        cmd.arg("-tr");
    }
    if compute_mode == "cpu" {
        cmd.arg("-ng");
    }
    if compute_mode == "gpu" && !capability.gpu_available {
        return Err("Mode GPU indisponible pour le serveur whisper.".to_string());
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Demarrage whisper-server impossible: {e}"))?;
    *guard = Some(WhisperServerRuntime {
        child,
        model_path: model_path.to_string_lossy().to_string(),
        language: language.to_string(),
        compute_mode: compute_mode.to_string(),
        translate_to_english,
        port,
    });

    wait_for_server_ready(port)
}

fn wait_for_server_ready(port: u16) -> Result<u16, String> {
    let url = format!("http://127.0.0.1:{port}/");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(250))
        .build()
        .map_err(|e| format!("Client readiness impossible: {e}"))?;

    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        if let Ok(resp) = client.get(&url).send() {
            if resp.status().is_success() || resp.status().as_u16() == 404 {
                return Ok(port);
            }
        }
        thread::sleep(Duration::from_millis(150));
    }

    Err("whisper-server n'a pas demarre a temps.".to_string())
}

fn detect_whisper_gpu_flags(whisper_cli_path: &Path) -> (bool, bool) {
    let mut cmd = Command::new(whisper_cli_path);
    apply_no_window(&mut cmd);
    let output = cmd.arg("--help").output();
    let Ok(output) = output else {
        return (false, false);
    };

    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout).to_lowercase());
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&output.stderr).to_lowercase());

    let supports_ngl = text.contains("-ngl") || text.contains("--gpu-layers");
    let supports_ng = text.contains(" -ng,") || text.contains("--no-gpu");
    (supports_ngl, supports_ng)
}

fn detect_compute_capability_for_cli(whisper_cli_path: &Path) -> ComputeCapabilityReport {
    let (supports_ngl, supports_no_gpu_flag) = detect_whisper_gpu_flags(whisper_cli_path);
    let gpu_backend_dlls = [
        "ggml-cuda.dll",
        "ggml-vulkan.dll",
        "ggml-hipblas.dll",
        "ggml-opencl.dll",
        "ggml-sycl.dll",
        "ggml-kompute.dll",
    ];

    let mut found_gpu_runtime: Vec<String> = Vec::new();
    if let Some(parent) = whisper_cli_path.parent() {
        for dll in gpu_backend_dlls {
            if parent.join(dll).exists() {
                found_gpu_runtime.push(dll.to_string());
            }
        }
    }

    let gpu_available = !found_gpu_runtime.is_empty() || supports_ngl;
    let details = if !found_gpu_runtime.is_empty() {
        format!("Backends detectes: {}", found_gpu_runtime.join(", "))
    } else if supports_ngl {
        "Option -ngl detectee dans whisper-cli (GPU potentiellement disponible).".to_string()
    } else {
        "Aucun backend GPU detecte (ni DLL GPU whisper.cpp, ni option -ngl).".to_string()
    };

    ComputeCapabilityReport {
        gpu_available,
        supports_ngl,
        supports_no_gpu_flag,
        whisper_cli_path: whisper_cli_path.to_string_lossy().to_string(),
        details,
    }
}

fn err_model_missing(model_path: &Path) -> String {
    format!(
        "Modele introuvable.\nChemin: {}\nAction: Place un modele Whisper.cpp (.bin) a ce chemin ou mets a jour \"Chemin modele Whisper\" dans Settings.",
        model_path.to_string_lossy()
    )
}

fn err_cli_missing(cli_path: &Path) -> String {
    format!(
        "whisper-cli introuvable.\nChemin: {}\nAction: Place whisper-cli.exe a ce chemin ou mets a jour \"Chemin whisper-cli.exe\" dans Settings.",
        cli_path.to_string_lossy()
    )
}

fn err_wav_missing(wav_path: &Path) -> String {
    format!(
        "Fichier WAV introuvable.\nChemin: {}\nAction: Refais un enregistrement micro puis relance la transcription.",
        wav_path.to_string_lossy()
    )
}

fn is_probably_silent_wav(wav_path: &Path) -> Result<bool, String> {
    let mut reader =
        hound::WavReader::open(wav_path).map_err(|e| format!("Lecture WAV impossible: {e}"))?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample == 0 {
        return Ok(false);
    }

    let max_amp = ((1_i64 << (spec.bits_per_sample - 1)) - 1) as f64;
    if max_amp <= 0.0 {
        return Ok(false);
    }

    let mut sample_count: u64 = 0;
    let mut energy_sum: f64 = 0.0;
    let mut peak_norm: f64 = 0.0;
    let mut activity_count: u64 = 0;

    for s in reader.samples::<i16>() {
        let sample = s.map_err(|e| format!("Lecture sample WAV impossible: {e}"))? as f64;
        let norm = (sample.abs() / max_amp).clamp(0.0, 1.0);
        sample_count += 1;
        energy_sum += norm * norm;
        if norm > peak_norm {
            peak_norm = norm;
        }
        if norm > 0.018 {
            activity_count += 1;
        }
    }

    if sample_count == 0 {
        return Ok(true);
    }

    let rms = (energy_sum / sample_count as f64).sqrt();
    let activity_ratio = activity_count as f64 / sample_count as f64;
    let silent = rms < 0.0032 && peak_norm < 0.03 && activity_ratio < 0.0015;
    Ok(silent)
}

fn normalize_language(language: &str) -> String {
    let trimmed = language.trim();
    if trimmed.is_empty() {
        return "fr".to_string();
    }

    let lower = trimmed.to_lowercase();
    if let Some((prefix, _)) = lower.split_once('-') {
        prefix.to_string()
    } else {
        lower
    }
}

fn normalize_compute_mode(mode: &str) -> String {
    match mode.trim().to_lowercase().as_str() {
        "cpu" => "cpu".to_string(),
        "gpu" => "gpu".to_string(),
        _ => "auto".to_string(),
    }
}

fn normalize_translation_target(target: &str) -> String {
    let t = target.trim().to_lowercase();
    if t.is_empty() {
        "none".to_string()
    } else {
        t
    }
}

fn clamp_widget_opacity(value: f32) -> f32 {
    value.clamp(0.25, 1.0)
}

fn clamp_widget_pop_sound_volume(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn normalize_widget_pop_sound(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "sound1.mp3".to_string()
    } else {
        trimmed.to_string()
    }
}

fn apply_voice_commands(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Padded string makes word-boundary style replacements simpler.
    let mut out = format!(" {} ", trimmed.to_lowercase().replace('’', "'"));

    // Allow users to dictate literal words without converting to punctuation.
    let escapes = [
        (" le mot point ", " __WORD_POINT__ "),
        (" mot point ", " __WORD_POINT__ "),
        (" le mot virgule ", " __WORD_VIRGULE__ "),
        (" mot virgule ", " __WORD_VIRGULE__ "),
    ];
    for (from, to) in escapes {
        out = out.replace(from, to);
    }

    let replacements = [
        (" nouvelle ligne ", "\n"),
        (" retour a la ligne ", "\n"),
        (" retour à la ligne ", "\n"),
        (" retour ligne ", "\n"),
        (" ponctuation point d'interrogation ", "? "),
        (" point d'interrogation ", "? "),
        (" point d interrogation ", "? "),
        (" ponctuation point d'exclamation ", "! "),
        (" point d'exclamation ", "! "),
        (" point d exclamation ", "! "),
        (" ponctuation point virgule ", "; "),
        (" point virgule ", "; "),
        (" ponctuation deux-points ", ": "),
        (" deux-points ", ": "),
        (" deux points ", ": "),
        (" ponctuation virgule ", ", "),
        (" virgule ", ", "),
        (" ponctuation point ", ". "),
        (" point final ", ". "),
        (" ouvrir parenthèse ", " ("),
        (" ouvrir parenthese ", " ("),
        (" fermer parenthèse ", ") "),
        (" fermer parenthese ", ") "),
    ];

    for (from, to) in replacements {
        out = out.replace(from, to);
    }

    out = out
        .replace(" .", ".")
        .replace(" ,", ",")
        .replace(" ;", ";")
        .replace(" :", ":")
        .replace(" ?", "?")
        .replace(" !", "!")
        .replace("( ", "(")
        .replace(" )", ")");

    out = out.replace("\r\n", "\n").replace('\r', "\n");
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out = out.replace(" \n", "\n").replace("\n ", "\n");
    out = out
        .replace("__WORD_POINT__", "point")
        .replace("__WORD_VIRGULE__", "virgule");

    capitalize_sentences(out.trim())
}

fn capitalize_sentences(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut capitalize_next = true;

    for c in input.chars() {
        if capitalize_next && c.is_alphabetic() {
            for up in c.to_uppercase() {
                output.push(up);
            }
            capitalize_next = false;
            continue;
        }

        output.push(c);

        if matches!(c, '.' | '!' | '?' | '\n') {
            capitalize_next = true;
        } else if !c.is_whitespace() {
            capitalize_next = false;
        }
    }

    output
}

const MODEL_CATALOG: [ModelCatalogEntry; 5] = [
    ModelCatalogEntry {
        id: "tiny",
        label: "Tiny",
        filename: "ggml-tiny.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
    },
    ModelCatalogEntry {
        id: "base",
        label: "Base",
        filename: "ggml-base.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
    },
    ModelCatalogEntry {
        id: "small",
        label: "Small",
        filename: "ggml-small.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
    },
    ModelCatalogEntry {
        id: "medium",
        label: "Medium",
        filename: "ggml-medium.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
    },
    ModelCatalogEntry {
        id: "large-v3",
        label: "Large v3",
        filename: "ggml-large-v3.bin",
        download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
    },
];

fn model_catalog_entry(model_id: &str) -> Option<ModelCatalogEntry> {
    MODEL_CATALOG.iter().find(|m| m.id == model_id).copied()
}

fn models_dir(app_state: &AppState) -> Result<PathBuf, String> {
    app_state
        .model_default_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "Dossier modeles introuvable".to_string())
}

fn list_models_impl(app_state: &AppState, settings: &UserSettings) -> Result<Vec<ModelInfo>, String> {
    let model_dir = models_dir(app_state)?;
    fs::create_dir_all(&model_dir).map_err(|e| format!("Creation dossier modeles impossible: {e}"))?;

    let active_path = PathBuf::from(settings.model_path.clone());
    let active_name = active_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
        .to_lowercase();

    let models = MODEL_CATALOG
        .iter()
        .map(|entry| {
            let path = model_dir.join(entry.filename);
            let installed = path.exists();
            let size_bytes = if installed {
                fs::metadata(&path).ok().map(|m| m.len())
            } else {
                None
            };
            let active = active_name == entry.filename.to_lowercase();

            ModelInfo {
                id: entry.id.to_string(),
                label: entry.label.to_string(),
                filename: entry.filename.to_string(),
                installed,
                active,
                size_bytes,
            }
        })
        .collect();

    Ok(models)
}

fn set_active_model_impl(app_state: &AppState, model_id: &str) -> Result<String, String> {
    let entry = model_catalog_entry(model_id)
        .ok_or_else(|| format!("Modele inconnu: {model_id}"))?;
    let model_dir = models_dir(app_state)?;
    let target = model_dir.join(entry.filename);
    if !target.exists() {
        return Err(format!(
            "Modele non installe: {}. Telecharge-le d'abord.",
            entry.label
        ));
    }

    let mut settings = get_settings_from_db(
        &app_state.db_path,
        &app_state.model_default_path,
        &app_state.whisper_cli_default_path,
    )?;
    settings.model_path = target.to_string_lossy().to_string();
    let conn = open_db(&app_state.db_path)?;
    save_settings_impl(&conn, &settings)?;
    Ok(format!("Modele actif: {}", entry.label))
}

fn emit_model_download_progress(app: &AppHandle, payload: ModelDownloadProgressEvent) {
    if let Err(e) = app.emit("model-download-progress", payload) {
        error!(target: "models", reason = %e, "broadcast model-download-progress failed");
    }
}

fn models_dir_from_default_path(model_default_path: &Path) -> Result<PathBuf, String> {
    model_default_path
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "Dossier modeles introuvable".to_string())
}

fn set_active_model_with_paths(
    db_path: &Path,
    model_default_path: &Path,
    whisper_cli_default_path: &Path,
    model_id: &str,
) -> Result<String, String> {
    let entry = model_catalog_entry(model_id).ok_or_else(|| format!("Modele inconnu: {model_id}"))?;
    let model_dir = models_dir_from_default_path(model_default_path)?;
    let target = model_dir.join(entry.filename);
    if !target.exists() {
        return Err(format!(
            "Modele non installe: {}. Telecharge-le d'abord.",
            entry.label
        ));
    }

    let mut settings = get_settings_from_db(
        &db_path.to_path_buf(),
        &model_default_path.to_path_buf(),
        &whisper_cli_default_path.to_path_buf(),
    )?;
    settings.model_path = target.to_string_lossy().to_string();
    let conn = open_db(&db_path.to_path_buf())?;
    save_settings_impl(&conn, &settings)?;
    Ok(format!("Modele actif: {}", entry.label))
}

fn download_model_with_paths(
    app: &AppHandle,
    db_path: PathBuf,
    model_default_path: PathBuf,
    whisper_cli_default_path: PathBuf,
    model_id: String,
    download_cancel: Arc<AtomicBool>,
) -> Result<String, String> {
    let entry = model_catalog_entry(&model_id)
        .ok_or_else(|| format!("Modele inconnu: {}", model_id))?;
    emit_model_download_progress(
        app,
        ModelDownloadProgressEvent {
            model_id: model_id.clone(),
            status: "starting".to_string(),
            progress_pct: Some(0),
            downloaded_bytes: 0,
            total_bytes: None,
            message: format!("Preparation du telechargement: {}", entry.label),
        },
    );
    let model_dir = models_dir_from_default_path(&model_default_path)?;
    fs::create_dir_all(&model_dir).map_err(|e| format!("Creation dossier modeles impossible: {e}"))?;

    let target = model_dir.join(entry.filename);

    if !target.exists() {
        let mut response = reqwest::blocking::get(entry.download_url)
            .map_err(|e| format!("Telechargement impossible: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "Telechargement echoue: code HTTP {}",
                response.status()
            ));
        }
        let total_bytes = response.content_length();

        let tmp = target.with_extension("download");
        let mut file = fs::File::create(&tmp).map_err(|e| format!("Creation fichier modele impossible: {e}"))?;

        let mut downloaded_bytes: u64 = 0;
        let mut last_pct: u8 = 0;
        let mut buffer = [0_u8; 64 * 1024];

        loop {
            if download_cancel.load(Ordering::SeqCst) {
                let _ = fs::remove_file(&tmp);
                return Err("Telechargement annule".to_string());
            }
            let read = response
                .read(&mut buffer)
                .map_err(|e| format!("Lecture flux telechargement impossible: {e}"))?;
            if read == 0 {
                break;
            }

            file.write_all(&buffer[..read])
                .map_err(|e| format!("Ecriture modele impossible: {e}"))?;
            downloaded_bytes += read as u64;

            let next_pct = total_bytes
                .map(|total| ((downloaded_bytes.saturating_mul(100)) / total.max(1)) as u8)
                .unwrap_or(0);
            if next_pct != last_pct {
                last_pct = next_pct;
                emit_model_download_progress(
                    app,
                    ModelDownloadProgressEvent {
                        model_id: model_id.clone(),
                        status: "downloading".to_string(),
                        progress_pct: Some(next_pct.min(99)),
                        downloaded_bytes,
                        total_bytes,
                        message: format!("Telechargement {}%", next_pct.min(99)),
                    },
                );
            }
        }

        fs::rename(&tmp, &target)
            .map_err(|e| format!("Finalisation modele impossible: {e}"))?;
        emit_model_download_progress(
            app,
            ModelDownloadProgressEvent {
                model_id: model_id.clone(),
                status: "downloading".to_string(),
                progress_pct: Some(100),
                downloaded_bytes: total_bytes.unwrap_or(0),
                total_bytes,
                message: "Telechargement termine".to_string(),
            },
        );
    }

    let settings = get_settings_from_db(
        &db_path,
        &model_default_path,
        &whisper_cli_default_path,
    )?;
    let installed_count = MODEL_CATALOG
        .iter()
        .filter(|m| model_dir.join(m.filename).exists())
        .count();
    let active_exists = {
        let configured = settings.model_path.trim();
        !configured.is_empty() && Path::new(configured).exists()
    };
    let should_auto_activate = installed_count <= 1 || !active_exists;

    if should_auto_activate {
        set_active_model_with_paths(&db_path, &model_default_path, &whisper_cli_default_path, &model_id)?;
    }

    emit_model_download_progress(
        app,
        ModelDownloadProgressEvent {
            model_id: model_id.clone(),
            status: "done".to_string(),
            progress_pct: Some(100),
            downloaded_bytes: 0,
            total_bytes: None,
            message: format!("Modele pret: {}", entry.label),
        },
    );
    if should_auto_activate {
        Ok(format!("Modele pret et actif: {}", entry.label))
    } else {
        Ok(format!("Modele telecharge: {} (modele actif conserve)", entry.label))
    }
}

fn delete_model_impl(app_state: &AppState, model_id: &str) -> Result<String, String> {
    let entry = model_catalog_entry(model_id)
        .ok_or_else(|| format!("Modele inconnu: {model_id}"))?;
    let model_dir = models_dir(app_state)?;
    let target = model_dir.join(entry.filename);
    if target.exists() {
        fs::remove_file(&target).map_err(|e| format!("Suppression modele impossible: {e}"))?;
    }

    let mut settings = get_settings_from_db(
        &app_state.db_path,
        &app_state.model_default_path,
        &app_state.whisper_cli_default_path,
    )?;
    let active_name = PathBuf::from(settings.model_path.clone())
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
        .to_lowercase();
    if active_name == entry.filename.to_lowercase() {
        if let Some(next) = MODEL_CATALOG.iter().find(|m| model_dir.join(m.filename).exists()) {
            settings.model_path = model_dir.join(next.filename).to_string_lossy().to_string();
        } else {
            settings.model_path = app_state.model_default_path.to_string_lossy().to_string();
        }
        let conn = open_db(&app_state.db_path)?;
        save_settings_impl(&conn, &settings)?;
    }

    Ok(format!("Modele supprime: {}", entry.label))
}

fn candidate_model_paths(app_state: &AppState, settings: &UserSettings) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if !settings.model_path.trim().is_empty() {
        paths.push(PathBuf::from(settings.model_path.clone()));
    }

    paths.push(app_state.model_default_path.clone());

    if let Some(model_dir) = app_state.model_default_path.parent() {
        for name in ["ggml-base.bin", "ggml-small.bin", "ggml-tiny.bin", "ggml-medium.bin"] {
            paths.push(model_dir.join(name));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        for name in ["ggml-base.bin", "ggml-small.bin", "ggml-tiny.bin"] {
            paths.push(cwd.join(name));
        }
    }

    dedupe_paths(paths)
}

fn candidate_cli_paths(app_state: &AppState, settings: &UserSettings) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if !settings.whisper_cli_path.trim().is_empty() {
        paths.push(PathBuf::from(settings.whisper_cli_path.clone()));
    }

    paths.push(app_state.whisper_cli_default_path.clone());

    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("whisper-cli.exe"));
        paths.push(cwd.join("build").join("bin").join("Release").join("whisper-cli.exe"));
        paths.push(cwd.join("whisper.cpp").join("build").join("bin").join("Release").join("whisper-cli.exe"));
    }

    dedupe_paths(paths)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for p in paths {
        let key = p.to_string_lossy().to_string().to_lowercase();
        if seen.insert(key) {
            out.push(p);
        }
    }

    out
}

fn first_existing_path(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|p| p.exists()).cloned()
}

fn ensure_runtime_dependencies(app: &AppHandle, app_state: &AppState) -> Result<(), String> {
    let whisper_bin_dir = app_state
        .whisper_cli_default_path
        .parent()
        .ok_or_else(|| "Dossier bin whisper introuvable.".to_string())?
        .to_path_buf();
    fs::create_dir_all(&whisper_bin_dir)
        .map_err(|e| format!("Creation dossier dependances impossible: {e}"))?;

    if !app_state.whisper_cli_default_path.exists() {
        let _ = copy_runtime_from_resources(app, app_state);
    }

    let vendor = detect_gpu_vendor();
    let current_capability = if app_state.whisper_cli_default_path.exists() {
        Some(detect_compute_capability_for_cli(
            &app_state.whisper_cli_default_path,
        ))
    } else {
        None
    };

    let needs_runtime = !app_state.whisper_cli_default_path.exists();
    let needs_nvidia_upgrade = matches!(vendor, GpuVendor::Nvidia)
        && current_capability
            .as_ref()
            .map(|c| !c.gpu_available)
            .unwrap_or(true);

    if needs_runtime || needs_nvidia_upgrade {
        let reason = if needs_nvidia_upgrade {
            "nvidia-gpu-detected"
        } else {
            "missing-runtime"
        };
        let install_note = install_runtime_from_official_release(vendor, &whisper_bin_dir)?;
        info!(target: "bootstrap", reason = reason, note = %install_note, "runtime dependency installed from official release");
    }

    let mut settings = get_settings_from_db(
        &app_state.db_path,
        &app_state.model_default_path,
        &app_state.whisper_cli_default_path,
    )?;
    if settings.whisper_cli_path.trim().is_empty() || !Path::new(&settings.whisper_cli_path).exists() {
        settings.whisper_cli_path = app_state
            .whisper_cli_default_path
            .to_string_lossy()
            .to_string();
        let conn = open_db(&app_state.db_path)?;
        save_settings_impl(&conn, &settings)?;
    }

    Ok(())
}

fn copy_runtime_from_resources(app: &AppHandle, app_state: &AppState) -> Result<(), String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("bin").join("whisper-cli.exe"));
        candidates.push(resource_dir.join("whisper-cli.exe"));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("whisper-cli.exe"));
            candidates.push(exe_dir.join("resources").join("bin").join("whisper-cli.exe"));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("whisper-cli.exe"));
        candidates.push(cwd.join("src-tauri").join("resources").join("bin").join("whisper-cli.exe"));
        candidates.push(
            cwd.join("apps")
                .join("desktop")
                .join("src-tauri")
                .join("resources")
                .join("bin")
                .join("whisper-cli.exe"),
        );
    }

    let source = first_existing_path(&dedupe_paths(candidates)).ok_or_else(|| {
        "Dependance absente: whisper-cli.exe non trouve dans les ressources de l'application."
            .to_string()
    })?;

    fs::copy(&source, &app_state.whisper_cli_default_path).map_err(|e| {
        format!(
            "Copie de whisper-cli impossible depuis {}: {e}",
            source.to_string_lossy()
        )
    })?;
    Ok(())
}

fn detect_gpu_vendor() -> GpuVendor {
    let mut nvidia = Command::new("nvidia-smi");
    apply_no_window(&mut nvidia);
    if let Ok(output) = nvidia.output() {
        if output.status.success() {
            return GpuVendor::Nvidia;
        }
    }

    let mut combined = String::new();
    let mut wmic = Command::new("cmd");
    wmic.args(["/C", "wmic path win32_VideoController get name"]);
    apply_no_window(&mut wmic);
    if let Ok(output) = wmic.output() {
        combined.push_str(&String::from_utf8_lossy(&output.stdout).to_lowercase());
        combined.push_str(&String::from_utf8_lossy(&output.stderr).to_lowercase());
    }
    if combined.is_empty() {
        let mut ps = Command::new("powershell");
        ps.args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name) -join \"`n\"",
        ]);
        apply_no_window(&mut ps);
        if let Ok(output) = ps.output() {
            combined.push_str(&String::from_utf8_lossy(&output.stdout).to_lowercase());
            combined.push_str(&String::from_utf8_lossy(&output.stderr).to_lowercase());
        }
    }

    if combined.contains("nvidia") {
        GpuVendor::Nvidia
    } else if combined.contains("amd") || combined.contains("radeon") {
        GpuVendor::Amd
    } else if combined.contains("intel") {
        GpuVendor::Intel
    } else {
        GpuVendor::Unknown
    }
}

fn install_runtime_from_official_release(vendor: GpuVendor, install_dir: &Path) -> Result<String, String> {
    let assets = fetch_whisper_release_assets()?;
    let preferred_asset = match vendor {
        GpuVendor::Nvidia => assets
            .cublas_12_4_x64
            .as_deref()
            .or(assets.cublas_11_8_x64.as_deref())
            .or(assets.cpu_x64.as_deref()),
        GpuVendor::Amd | GpuVendor::Intel | GpuVendor::Unknown => assets.cpu_x64.as_deref(),
    }
    .ok_or_else(|| "Aucun runtime Windows x64 compatible trouve dans la release whisper.cpp.".to_string())?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Horodatage impossible: {e}"))?
        .as_millis();
    let zip_path = install_dir.join(format!("runtime-{stamp}.zip"));
    let mut response = reqwest::blocking::Client::new()
        .get(preferred_asset)
        .header("User-Agent", "WhisperPro/1.0")
        .send()
        .map_err(|e| format!("Telechargement runtime whisper impossible: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Telechargement runtime whisper echoue (HTTP {}).",
            response.status()
        ));
    }
    let mut zip_file = fs::File::create(&zip_path)
        .map_err(|e| format!("Creation archive runtime impossible: {e}"))?;
    std::io::copy(&mut response, &mut zip_file)
        .map_err(|e| format!("Ecriture archive runtime impossible: {e}"))?;

    let file = fs::File::open(&zip_path).map_err(|e| format!("Lecture archive runtime impossible: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Archive runtime invalide: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Lecture entree archive impossible: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let entry_name = entry.name().replace('\\', "/");
        let lower = entry_name.to_lowercase();
        if !(lower.ends_with(".exe") || lower.ends_with(".dll")) {
            continue;
        }

        let Some(file_name) = Path::new(&entry_name).file_name() else {
            continue;
        };
        let target = install_dir.join(file_name);
        let mut out = fs::File::create(&target)
            .map_err(|e| format!("Creation fichier runtime impossible ({}): {e}", target.to_string_lossy()))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(|e| format!("Ecriture fichier runtime impossible ({}): {e}", target.to_string_lossy()))?;
    }

    let _ = fs::remove_file(&zip_path);

    let cli = install_dir.join("whisper-cli.exe");
    if !cli.exists() {
        return Err("Installation runtime incomplete: whisper-cli.exe introuvable apres extraction.".to_string());
    }

    Ok(format!("Runtime installe depuis {}", assets.tag))
}

struct WhisperReleaseAssets {
    tag: String,
    cpu_x64: Option<String>,
    cublas_11_8_x64: Option<String>,
    cublas_12_4_x64: Option<String>,
}

fn fetch_whisper_release_assets() -> Result<WhisperReleaseAssets, String> {
    let response = reqwest::blocking::Client::new()
        .get("https://api.github.com/repos/ggml-org/whisper.cpp/releases/latest")
        .header("User-Agent", "WhisperPro/1.0")
        .send()
        .map_err(|e| format!("Lecture release whisper.cpp impossible: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Lecture release whisper.cpp echouee (HTTP {}).",
            response.status()
        ));
    }

    let body = response
        .text()
        .map_err(|e| format!("Lecture reponse release whisper.cpp impossible: {e}"))?;
    let payload: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Reponse release whisper.cpp invalide: {e}"))?;
    let tag = payload
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("latest")
        .to_string();

    let mut cpu_x64: Option<String> = None;
    let mut cublas_11_8_x64: Option<String> = None;
    let mut cublas_12_4_x64: Option<String> = None;

    if let Some(assets) = payload.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            let name = asset
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();
            let url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if url.is_empty() {
                continue;
            }

            if name == "whisper-bin-x64.zip" {
                cpu_x64 = Some(url.clone());
            } else if name == "whisper-cublas-11.8.0-bin-x64.zip" {
                cublas_11_8_x64 = Some(url.clone());
            } else if name == "whisper-cublas-12.4.0-bin-x64.zip" {
                cublas_12_4_x64 = Some(url.clone());
            }
        }
    }

    Ok(WhisperReleaseAssets {
        tag,
        cpu_x64,
        cublas_11_8_x64,
        cublas_12_4_x64,
    })
}

fn apply_overlay_visibility(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        if let Some(existing) = app.get_webview_window("overlay") {
            let _ = existing.show();
            return Ok(());
        }
        let builder = WebviewWindowBuilder::new(
            app,
            "overlay",
            WebviewUrl::App("index.html?overlay=1".into()),
        )
        .title("WhisperPro Widget")
        .inner_size(240.0, 88.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .focused(false)
        .skip_taskbar(true)
        .visible(true);
        let _window = builder
            .build()
            .map_err(|e| format!("Creation fenetre widget impossible: {e}"))?;
    } else if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
    }
    Ok(())
}

fn show_overlay_near_cursor(app: &AppHandle) -> Result<(), String> {
    apply_overlay_visibility(app, true)?;
    let Some(window) = app.get_webview_window("overlay") else {
        return Ok(());
    };

    if let Some((cursor_x, cursor_y)) = current_cursor_position() {
        let target_x = (cursor_x - 110.0).max(0.0);
        let target_y = (cursor_y - 120.0).max(0.0);
        let state = app.state::<AppState>();
        let previous = *state.overlay_last_position.lock();

        if let Some((start_x, start_y)) = previous {
            let dx = target_x - start_x;
            let dy = target_y - start_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > 8.0 {
                let steps = ((distance / 42.0).ceil() as i32).clamp(3, 8);
                for i in 1..=steps {
                    let t = i as f64 / steps as f64;
                    let eased = 1.0 - (1.0 - t).powi(3);
                    let x = start_x + dx * eased;
                    let y = start_y + dy * eased;
                    let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
                    thread::sleep(Duration::from_millis(10));
                }
            } else {
                let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                    x: target_x,
                    y: target_y,
                }));
            }
        } else {
            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
                x: target_x,
                y: target_y,
            }));
        }
        *state.overlay_last_position.lock() = Some((target_x, target_y));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn current_cursor_position() -> Option<(f64, f64)> {
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }
    unsafe extern "system" {
        fn GetCursorPos(lp_point: *mut Point) -> i32;
    }

    let mut pt = Point { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut pt as *mut Point) };
    if ok != 0 {
        Some((pt.x as f64, pt.y as f64))
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn current_cursor_position() -> Option<(f64, f64)> {
    None
}

fn main() {
    let app_dir = resolve_app_dir().expect("app dir resolution failed");
    let db_path = resolve_db_path(&app_dir);
    let log_path = resolve_log_path(&app_dir).expect("log path resolution failed");
    let model_default_path = resolve_model_default_path(&app_dir).expect("model path resolution failed");
    let whisper_cli_default_path =
        resolve_whisper_cli_default_path(&app_dir).expect("whisper cli path resolution failed");
    let _log_guard = init_logger(&log_path).expect("logger initialization failed");

    init_db(&db_path, &model_default_path, &whisper_cli_default_path).expect("db initialization failed");
    info!(
        target: "app",
        db = %db_path.to_string_lossy(),
        log = %log_path.to_string_lossy(),
        model_default = %model_default_path.to_string_lossy(),
        whisper_cli_default = %whisper_cli_default_path.to_string_lossy(),
        "app initialized"
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            capture: Mutex::new(None),
            db_path,
            log_path,
            model_default_path,
            whisper_cli_default_path,
            last_error: Mutex::new(None),
            dictation_recording: AtomicBool::new(false),
            dictation_busy: AtomicBool::new(false),
            registered_shortcut: Mutex::new(None),
            dictation_status: Mutex::new(DictationStatusEvent {
                state: "idle".to_string(),
                message: "En attente".to_string(),
            }),
            latest_dictation_transcript: Mutex::new(None),
            last_successful_injection: Mutex::new(None),
            widget_enabled: AtomicBool::new(true),
            overlay_last_position: Mutex::new(None),
            overlay_hide_token: AtomicU64::new(0),
            whisper_server: Mutex::new(None),
            model_download_in_progress: Arc::new(AtomicBool::new(false)),
            model_download_cancel: Arc::new(AtomicBool::new(false)),
            model_download_active_id: Mutex::new(None),
        })
        .setup(|app| {
            let app_handle_for_bootstrap = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_handle_inner = app_handle_for_bootstrap.clone();
                let result = tauri::async_runtime::spawn_blocking(move || {
                    let state = app_handle_inner.state::<AppState>();
                    ensure_runtime_dependencies(&app_handle_inner, state.inner())
                })
                .await;
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        error!(target: "bootstrap", reason = %e, "runtime dependency bootstrap failed");
                    }
                    Err(e) => {
                        error!(target: "bootstrap", reason = %e, "runtime dependency bootstrap task join failed");
                    }
                }
            });

            let state = app.state::<AppState>();
            let settings = get_settings_from_db(
                &state.db_path,
                &state.model_default_path,
                &state.whisper_cli_default_path,
            )
            .map_err(|e| anyhow::anyhow!(e))?;

            register_or_update_global_shortcut(app.handle(), state.inner(), &settings.shortcut)
                .map_err(|e| anyhow::anyhow!(e))?;
            state
                .widget_enabled
                .store(settings.widget_enabled, Ordering::SeqCst);
            if !settings.widget_enabled {
                apply_overlay_visibility(app.handle(), false).map_err(|e| anyhow::anyhow!(e))?;
            }

            if let Some(main_window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                main_window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { .. } = event {
                        app_handle.exit(0);
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_capture,
            stop_capture,
            toggle_dictation_cycle,
            reset_runtime_state,
            transcribe_wav,
            test_whisper_environment,
            get_settings,
            save_settings,
            get_default_model_path,
            get_default_whisper_cli_path,
            get_compute_capability,
            auto_setup_runtime,
            translate_wav_to_english,
            translate_text,
            list_models,
            download_model,
            cancel_model_download,
            set_active_model,
            delete_model,
            get_last_error,
            get_log_path,
            get_dictation_status,
            get_last_dictation_transcript,
            generate_diagnostic_snapshot,
            start_overlay_drag,
            clear_history_artifacts,
            open_path_in_explorer,
            quit_application
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
