import type { UserSettings } from "./types";

export const HISTORY_KEY = "whisperpro_transcription_history";
export const MAX_HISTORY_ITEMS = 20;
export const DEFAULT_WIDGET_POP_SOUND = "sound1.mp3";

export const WIDGET_SOUND_GAIN: Record<string, number> = {
  "sound1.mp3": 1.0,
  "sound2.mp3": 0.78,
  "sound3.mp3": 0.95,
  "sound4.mp3": 0.74,
  "sound5.mp3": 0.96,
  "sound6.mp3": 0.8,
  "sound7.mp3": 0.84,
  "sound8.mp3": 1.0,
  "sound9.mp3": 0.92
};

export const LANGUAGE_OPTIONS: Array<{ value: string; label: string }> = [
  { value: "auto", label: "Auto (détection)" },
  { value: "fr-FR", label: "Français" },
  { value: "en-US", label: "English" },
  { value: "es-ES", label: "Español" },
  { value: "de-DE", label: "Deutsch" },
  { value: "it-IT", label: "Italiano" },
  { value: "pt-PT", label: "Português" },
  { value: "nl-NL", label: "Nederlands" },
  { value: "ru-RU", label: "Русский" },
  { value: "uk-UA", label: "Українська" },
  { value: "pl-PL", label: "Polski" },
  { value: "tr-TR", label: "Türkçe" },
  { value: "ar-SA", label: "العربية" },
  { value: "hi-IN", label: "हिन्दी" },
  { value: "ja-JP", label: "日本語" },
  { value: "ko-KR", label: "한국어" },
  { value: "zh-CN", label: "中文" },
  { value: "sv-SE", label: "Svenska" }
];

export const TRANSLATION_VALUES: Array<{ value: string }> = [
  { value: "none" },
  { value: "en" },
  { value: "fr" },
  { value: "es" },
  { value: "de" },
  { value: "it" },
  { value: "pt" },
  { value: "nl" },
  { value: "ru" },
  { value: "uk" },
  { value: "pl" },
  { value: "tr" },
  { value: "ar" },
  { value: "hi" },
  { value: "ja" },
  { value: "ko" },
  { value: "zh" },
  { value: "sv" }
];

export const COMPUTE_MODE_VALUES: Array<{ value: UserSettings["compute_mode"] }> = [
  { value: "auto" },
  { value: "cpu" },
  { value: "gpu" }
];

export const defaultSettings: UserSettings = {
  language: "auto",
  translation_target: "none",
  shortcut: "Ctrl+Shift+Space",
  model_path: "",
  whisper_cli_path: "",
  input_device_id: "",
  push_to_talk_hold: false,
  secure_text_mode: false,
  silence_gate_enabled: true,
  compute_mode: "auto",
  keep_model_loaded: false,
  widget_enabled: true,
  widget_autohide: true,
  widget_opacity: 0.9,
  widget_pop_sound_volume: 0.65,
  widget_pop_sound: DEFAULT_WIDGET_POP_SOUND,
  voice_commands_enabled: true,
  onboarding_completed: true
};
