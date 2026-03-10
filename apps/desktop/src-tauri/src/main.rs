#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::fs;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutEvent, ShortcutState};
use tracing::{error, info};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};

mod state;
mod settings_db;
mod commands;
mod injection;
mod audio_capture;
mod transcription;
mod translation;
mod models;
mod runtime_setup;
mod overlay;

use settings_db::{
    get_settings_from_db, init_db, open_db, save_settings_impl,
};
use commands::*;
use audio_capture::*;
use transcription::*;
use translation::*;
use models::*;
use runtime_setup::*;
use overlay::*;
use injection::*;
use state::*;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn apply_no_window(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
}

fn start_capture_impl(app_state: &AppState) -> Result<String, String> {
    let mut guard = app_state.capture.lock();
    if guard.is_some() {
        return Err("Capture deja en cours".to_string());
    }
    let settings = get_settings_from_db(
        &app_state.db_path,
        &app_state.model_default_path,
        &app_state.whisper_cli_default_path,
    )?;

    let output_path = std::env::temp_dir().join("whisperpro_recording.wav");
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let output_path_for_thread = output_path.clone();
    let preferred_input_device_id = settings.input_device_id.trim().to_string();

    let worker = thread::spawn(move || {
        let preferred = if preferred_input_device_id.is_empty() {
            None
        } else {
            Some(preferred_input_device_id)
        };
        run_capture_loop(output_path_for_thread, stop_rx, preferred)
    });

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

    if settings.silence_gate_enabled && is_probably_silent_wav(&wav)? {
        return Ok(TranscriptionResult {
            text: String::new(),
            segments: vec![],
            model_path: model_path.to_string_lossy().to_string(),
            wav_path: wav.to_string_lossy().to_string(),
        });
    }

    let mut transcription_input = wav.clone();
    let normalized_temp = maybe_create_normalized_wav_copy(&wav)?;
    if let Some(normalized_path) = normalized_temp.as_ref() {
        info!(
            target: "audio",
            original = %wav.to_string_lossy(),
            normalized = %normalized_path.to_string_lossy(),
            "low input level detected, using normalized wav for transcription"
        );
        transcription_input = normalized_path.clone();
    }

    let mut result = transcribe_with_strategy(
        app_state,
        &whisper_cli_path,
        &model_path,
        &transcription_input,
        &settings.language,
        &settings.compute_mode,
        false,
        false,
    )?;

    if let Some(path) = normalized_temp {
        let _ = fs::remove_file(path);
    }

    result.wav_path = wav.to_string_lossy().to_string();
    Ok(result)
}

fn ensure_dictation_model_ready(app_state: &AppState) -> Result<(), String> {
    let settings = get_settings_from_db(
        &app_state.db_path,
        &app_state.model_default_path,
        &app_state.whisper_cli_default_path,
    )?;
    let model_path = PathBuf::from(settings.model_path);
    if !model_path.exists() {
        return Err("Aucun modele actif. Ouvre Options > Modeles, telecharge un modele (Base recommande), puis active-le.".to_string());
    }
    Ok(())
}

fn toggle_dictation_cycle_impl(app: &AppHandle, app_state: &AppState) -> Result<String, String> {
    if app_state.dictation_busy.load(Ordering::SeqCst) {
        let message = "Traitement de dictee deja en cours".to_string();
        emit_dictation_status(app, "busy", &message);
        return Ok(message);
    }

    if !app_state.dictation_recording.load(Ordering::SeqCst) {
        if let Err(message) = ensure_dictation_model_ready(app_state) {
            emit_dictation_status(app, "error", &message);
            return Ok(message);
        }
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
            let handle = app_handle.clone();
            thread::spawn(move || {
                let state = handle.state::<AppState>();
                let app_state = state.inner();
                let settings = match get_settings_from_db(
                    &app_state.db_path,
                    &app_state.model_default_path,
                    &app_state.whisper_cli_default_path,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        record_error(app_state, &e);
                        return;
                    }
                };

                let result = if settings.push_to_talk_hold {
                    if event.state == ShortcutState::Pressed {
                        if app_state.dictation_recording.load(Ordering::SeqCst)
                            || app_state.dictation_busy.load(Ordering::SeqCst)
                        {
                            Ok("Dictee deja active".to_string())
                        } else {
                            toggle_dictation_cycle_impl(&handle, app_state)
                        }
                    } else if event.state == ShortcutState::Released {
                        if app_state.dictation_recording.load(Ordering::SeqCst) {
                            toggle_dictation_cycle_impl(&handle, app_state)
                        } else {
                            Ok("Dictee inactive".to_string())
                        }
                    } else {
                        Ok("Evenement raccourci ignore".to_string())
                    }
                } else if event.state == ShortcutState::Pressed {
                    toggle_dictation_cycle_impl(&handle, app_state)
                } else {
                    Ok("Evenement raccourci ignore".to_string())
                };

                if let Err(e) = result {
                    record_error(state.inner(), &e);
                }
            });
        })
        .map_err(|e| format!("Impossible d'enregistrer le raccourci global '{shortcut}': {e}"))?;

    *registered = Some(shortcut.to_string());
    info!(target: "hotkey", shortcut = %shortcut, "global shortcut registered");
    Ok(())
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

fn normalize_widget_pop_sound_volume_from_db(value: f32) -> f32 {
    if value.is_finite() && value > 2.0 {
        // Legacy format stored as percentage (e.g. 65 or 200).
        return clamp_widget_pop_sound_volume(value / 100.0);
    }
    clamp_widget_pop_sound_volume(value)
}

fn normalize_widget_pop_sound(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "sound1.mp3".to_string()
    } else {
        trimmed.to_string()
    }
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
            list_input_devices,
            save_settings,
            save_widget_preferences,
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
            open_external_url,
            quit_application
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings_db::resolve_active_model_path;
    use rusqlite::{params, Connection};
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn make_temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let seq = TEST_COUNTER.fetch_add(1, AtomicOrdering::SeqCst);
        dir.push(format!("whisperpro-tests-{name}-{stamp}-{seq}"));
        fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    fn make_legacy_settings_db(db_path: &Path) {
        let conn = Connection::open(db_path).expect("open legacy db");
        conn.execute_batch(
            "CREATE TABLE settings (
                id INTEGER PRIMARY KEY CHECK(id = 1),
                language TEXT NOT NULL,
                shortcut TEXT NOT NULL
            );",
        )
        .expect("create legacy schema");
        conn.execute(
            "INSERT INTO settings (id, language, shortcut) VALUES (1, ?1, ?2)",
            params!["en-US", "Ctrl+Alt+Space"],
        )
        .expect("insert legacy settings");
    }

    #[test]
    fn normalize_language_handles_empty_and_locale() {
        assert_eq!(normalize_language(""), "fr");
        assert_eq!(normalize_language("  EN-us "), "en");
        assert_eq!(normalize_language("de"), "de");
    }

    #[test]
    fn normalize_compute_and_translation_targets_are_stable() {
        assert_eq!(normalize_compute_mode("gpu"), "gpu");
        assert_eq!(normalize_compute_mode(" CPU "), "cpu");
        assert_eq!(normalize_compute_mode("weird"), "auto");

        assert_eq!(normalize_translation_target(""), "none");
        assert_eq!(normalize_translation_target(" FR "), "fr");
    }

    #[test]
    fn widget_pop_sound_volume_supports_legacy_percent_and_normalized_range() {
        assert!((normalize_widget_pop_sound_volume_from_db(0.65) - 0.65).abs() < 0.0001);
        assert!((normalize_widget_pop_sound_volume_from_db(65.0) - 0.65).abs() < 0.0001);
        assert!((normalize_widget_pop_sound_volume_from_db(200.0) - 1.0).abs() < 0.0001);
        assert!((clamp_widget_pop_sound_volume(2.4) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn voice_commands_apply_punctuation_and_literal_escape_words() {
        let punctuated = apply_voice_commands("bonjour virgule test point d'interrogation");
        assert_eq!(punctuated, "Bonjour, test?");

        let literal_words = apply_voice_commands("le mot point et le mot virgule");
        assert_eq!(literal_words, "Point et virgule");
    }

    #[test]
    fn voice_commands_remain_stable_with_irregular_whitespace() {
        let punctuated = apply_voice_commands("bonjour   virgule\n\t test   point   d interrogation");
        assert_eq!(punctuated, "Bonjour, test?");
    }

    #[test]
    fn voice_commands_do_not_force_uppercase_after_single_line_break() {
        let text = apply_voice_commands("bonjour ponctuation virgule nouvelle ligne test");
        assert_eq!(text, "Bonjour,\ntest");
    }

    #[test]
    fn voice_commands_keep_uppercase_for_paragraph_breaks() {
        let text = apply_voice_commands("bonjour point final nouvelle ligne nouvelle ligne test");
        assert_eq!(text, "Bonjour.\nTest");
    }

    #[test]
    fn resolve_active_model_path_prefers_existing_configured_path() {
        let temp = make_temp_dir("resolve-model");
        let configured = temp.join("configured.bin");
        let fallback = temp.join("fallback.bin");
        fs::write(&configured, b"configured").expect("write configured model");
        fs::write(&fallback, b"fallback").expect("write fallback model");

        let resolved = resolve_active_model_path(
            &configured.to_string_lossy(),
            &fallback.to_string_lossy(),
        );

        assert_eq!(resolved, configured.to_string_lossy());
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn init_db_creates_defaults_and_migrates_legacy_schema() {
        let temp = make_temp_dir("init-db");
        let db_path = temp.join("whisperpro.db");
        let default_model_path = temp.join("models").join("ggml-base.bin");
        let default_cli_path = temp.join("bin").join("whisper-cli.exe");
        fs::create_dir_all(default_model_path.parent().expect("model parent"))
            .expect("create model dir");
        fs::create_dir_all(default_cli_path.parent().expect("cli parent")).expect("create cli dir");

        make_legacy_settings_db(&db_path);

        init_db(&db_path, &default_model_path, &default_cli_path).expect("migrate schema");

        let settings =
            get_settings_from_db(&db_path, &default_model_path, &default_cli_path).expect("load settings");

        assert_eq!(settings.language, "en-US");
        assert_eq!(settings.shortcut, "Ctrl+Alt+Space");
        assert_eq!(settings.compute_mode, "auto");
        assert_eq!(settings.translation_target, "none");
        assert_eq!(settings.widget_pop_sound, "sound1.mp3");

        let _ = fs::remove_dir_all(temp);
    }
}

