use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct UserSettings {
    pub(crate) language: String,
    pub(crate) translation_target: String,
    pub(crate) shortcut: String,
    pub(crate) model_path: String,
    pub(crate) whisper_cli_path: String,
    pub(crate) input_device_id: String,
    pub(crate) push_to_talk_hold: bool,
    pub(crate) secure_text_mode: bool,
    pub(crate) silence_gate_enabled: bool,
    pub(crate) compute_mode: String,
    pub(crate) keep_model_loaded: bool,
    pub(crate) widget_enabled: bool,
    pub(crate) widget_autohide: bool,
    pub(crate) widget_opacity: f32,
    pub(crate) widget_pop_sound_volume: f32,
    pub(crate) widget_pop_sound: String,
    pub(crate) voice_commands_enabled: bool,
    pub(crate) onboarding_completed: bool,
}

impl UserSettings {
    pub(crate) fn with_defaults(model_path: String, whisper_cli_path: String) -> Self {
        Self {
            language: "auto".to_string(),
            translation_target: "none".to_string(),
            shortcut: "Ctrl+Shift+Space".to_string(),
            model_path,
            whisper_cli_path,
            input_device_id: String::new(),
            push_to_talk_hold: false,
            secure_text_mode: false,
            silence_gate_enabled: true,
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
pub(crate) struct InputDeviceInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TranscriptSegment {
    pub(crate) start_ms: i64,
    pub(crate) end_ms: i64,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TranscriptionResult {
    pub(crate) text: String,
    pub(crate) segments: Vec<TranscriptSegment>,
    pub(crate) model_path: String,
    pub(crate) wav_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WhisperEnvironmentReport {
    pub(crate) ready: bool,
    pub(crate) model_path: String,
    pub(crate) model_exists: bool,
    pub(crate) whisper_cli_path: String,
    pub(crate) whisper_cli_exists: bool,
    pub(crate) auto_updated: bool,
    pub(crate) notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapabilityReport {
    pub(crate) gpu_available: bool,
    pub(crate) supports_ngl: bool,
    pub(crate) supports_no_gpu_flag: bool,
    pub(crate) whisper_cli_path: String,
    pub(crate) details: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DictationStatusEvent {
    pub(crate) state: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DictationTranscriptEvent {
    pub(crate) text: String,
    pub(crate) injected_text: String,
    pub(crate) translation_applied: bool,
    pub(crate) translation_target: String,
    pub(crate) wav_path: String,
    pub(crate) model_path: String,
    pub(crate) created_at_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ModelInfo {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) filename: String,
    pub(crate) installed: bool,
    pub(crate) active: bool,
    pub(crate) size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ModelDownloadProgressEvent {
    pub(crate) model_id: String,
    pub(crate) status: String,
    pub(crate) progress_pct: Option<u8>,
    pub(crate) downloaded_bytes: u64,
    pub(crate) total_bytes: Option<u64>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ClearHistoryArtifactsPayload {
    #[serde(default, alias = "wav_paths", rename = "wavPaths")]
    pub(crate) wav_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ModelCatalogEntry {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) filename: &'static str,
    pub(crate) download_url: &'static str,
}

pub(crate) struct CaptureSession {
    pub(crate) stop_tx: Sender<()>,
    pub(crate) worker: JoinHandle<Result<String, String>>,
    pub(crate) output_path: PathBuf,
}

pub(crate) struct WhisperServerRuntime {
    pub(crate) child: std::process::Child,
    pub(crate) model_path: String,
    pub(crate) language: String,
    pub(crate) compute_mode: String,
    pub(crate) translate_to_english: bool,
    pub(crate) port: u16,
}

pub(crate) struct AppState {
    pub(crate) capture: Mutex<Option<CaptureSession>>,
    pub(crate) db_path: PathBuf,
    pub(crate) log_path: PathBuf,
    pub(crate) model_default_path: PathBuf,
    pub(crate) whisper_cli_default_path: PathBuf,
    pub(crate) last_error: Mutex<Option<String>>,
    pub(crate) dictation_recording: AtomicBool,
    pub(crate) dictation_busy: AtomicBool,
    pub(crate) registered_shortcut: Mutex<Option<String>>,
    pub(crate) dictation_status: Mutex<DictationStatusEvent>,
    pub(crate) latest_dictation_transcript: Mutex<Option<DictationTranscriptEvent>>,
    pub(crate) last_successful_injection: Mutex<Option<(String, Instant)>>,
    pub(crate) widget_enabled: AtomicBool,
    pub(crate) overlay_last_position: Mutex<Option<(f64, f64)>>,
    pub(crate) overlay_hide_token: AtomicU64,
    pub(crate) whisper_server: Mutex<Option<WhisperServerRuntime>>,
    pub(crate) model_download_in_progress: Arc<AtomicBool>,
    pub(crate) model_download_cancel: Arc<AtomicBool>,
    pub(crate) model_download_active_id: Mutex<Option<String>>,
}
