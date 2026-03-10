import type { Dispatch, SetStateAction } from "react";

import { UI_TEXT } from "../../i18n";
import type { WidgetThemeMode } from "../../i18n";
import type {
  ComputeCapabilityReport,
  HistoryItem,
  ModelDownloadProgressEvent,
  ModelInfo,
  UserSettings
} from "./types";

export type UiText = (typeof UI_TEXT)["fr"];

export type DictationPanelModel = {
  uiText: UiText;
  recording: boolean;
  disabled: boolean;
  modelsBusy: boolean;
  installedModels: ModelInfo[];
  activeModelId: string;
  activeModelLabel: string;
  shortcut: string;
  translationTarget: string;
  translationOptions: Array<{ value: string; label: string }>;
  statusLine: string;
  errorLine: string;
  textView: "source" | "translated";
  translatedText: string;
  currentVisibleText: string;
  translating: boolean;
  wavPath: string;
  translationTargetLabel: string;
  translationError: string;
  modelDisplayLabel: (model: { id: string; label: string }) => string;
};

export type DictationPanelSetters = {
  setTextView: Dispatch<SetStateAction<"source" | "translated">>;
  setTranslatedText: Dispatch<SetStateAction<string>>;
  setTranscript: Dispatch<SetStateAction<string>>;
};

export type DictationPanelHandlers = {
  onStartRecording: () => void;
  onStopRecordingAndTranscribe: () => void;
  onActivateModel: (modelId: string) => void;
  onTranslationTargetChange: (next: string) => void;
  onShowOriginalText: () => void;
  onShowTranslatedText: () => void;
  onCopyVisibleText: () => void;
};

export type HistoryPanelModel = {
  uiText: UiText;
  historyItems: HistoryItem[];
  disabled: boolean;
};

export type HistoryPanelHandlers = {
  onClearAllHistory: () => void;
  onRemoveHistoryItem: (item: HistoryItem) => void;
  onCopyHistoryItem: (item: HistoryItem) => void;
};

export type SettingsDrawerModel = {
  open: boolean;
  uiText: UiText;
  settings: UserSettings;
  settingsState: "idle" | "saving" | "saved" | "error";
  settingsError: string;
  settingsDirty: boolean;
  shortcutDraft: string;
  capturingShortcut: boolean;
  runtimeSetupBusy: boolean;
  computeModeOptions: Array<{ value: UserSettings["compute_mode"]; label: string }>;
  computeCapability: ComputeCapabilityReport | null;
  isDownloadInProgress: boolean;
  downloadProgress: ModelDownloadProgressEvent | null;
  models: ModelInfo[];
  modelsError: string;
  modelsBusy: boolean;
  downloadingModelId: string | null;
  widgetSoundOptions: string[];
  previewSoundPlaying: boolean;
  widgetThemeMode: WidgetThemeMode;
  widgetSoundLabel: (fileName: string) => string;
  modelDisplayLabel: (model: { id: string; label: string }) => string;
};

export type SettingsDrawerSetters = {
  setSettings: Dispatch<SetStateAction<UserSettings>>;
  setShortcutDraft: Dispatch<SetStateAction<string>>;
  setCapturingShortcut: Dispatch<SetStateAction<boolean>>;
  setStatusLine: Dispatch<SetStateAction<string>>;
};

export type SettingsDrawerHandlers = {
  onRequestClose: () => void;
  onSaveSettingsSnapshot: (next: UserSettings, successLabel?: string) => Promise<boolean>;
  onResetSettings: () => void;
  onRepairRuntime: () => void;
  onCancelModelDownload: () => void;
  onWidgetSoundChange: (soundFile: string) => void;
  onWidgetThemeModeChange: (mode: WidgetThemeMode) => void;
  onWidgetSoundVolumeChange: (volume: number) => void;
  onWidgetOpacityChange: (opacity: number) => void;
  onPreviewWidgetSound: () => void;
  onDownloadModel: (modelId: string) => void;
  onRemoveModel: (modelId: string) => void;
};
