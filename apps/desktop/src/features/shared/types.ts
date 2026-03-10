export type UserSettings = {
  language: string;
  translation_target: string;
  shortcut: string;
  model_path: string;
  whisper_cli_path: string;
  input_device_id: string;
  push_to_talk_hold: boolean;
  secure_text_mode: boolean;
  silence_gate_enabled: boolean;
  compute_mode: "auto" | "cpu" | "gpu";
  keep_model_loaded: boolean;
  widget_enabled: boolean;
  widget_autohide: boolean;
  widget_opacity: number;
  widget_pop_sound_volume: number;
  widget_pop_sound: string;
  voice_commands_enabled: boolean;
  onboarding_completed: boolean;
};

export type InputDeviceInfo = {
  id: string;
  name: string;
  is_default: boolean;
};

export type TranscriptionResult = {
  text: string;
  wav_path: string;
  model_path: string;
};

export type WhisperEnvironmentReport = {
  ready: boolean;
  model_path: string;
  whisper_cli_path: string;
  notes: string[];
};

export type ComputeCapabilityReport = {
  gpu_available: boolean;
  supports_ngl: boolean;
  supports_no_gpu_flag: boolean;
  whisper_cli_path: string;
  details: string;
};

export type DictationStatusEvent = {
  state: string;
  message: string;
};

export type DictationTranscriptEvent = {
  text: string;
  injected_text: string;
  translation_applied: boolean;
  translation_target: string;
  wav_path: string;
  model_path: string;
  created_at_ms: number;
};

export type HistoryItem = {
  id: string;
  createdAt: string;
  text: string;
  wavPath: string;
  modelUsed?: string;
};

export type ModelInfo = {
  id: string;
  label: string;
  filename: string;
  installed: boolean;
  active: boolean;
  size_bytes: number | null;
};

export type ModelDownloadProgressEvent = {
  model_id: string;
  status: string;
  progress_pct: number | null;
  downloaded_bytes: number;
  total_bytes: number | null;
  message: string;
};
