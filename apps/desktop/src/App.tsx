import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { UI_LANGUAGE_STORAGE_KEY, UI_TEXT, type UiLanguage } from "./i18n";

type UserSettings = {
  language: string;
  shortcut: string;
  model_path: string;
  whisper_cli_path: string;
  compute_mode: "auto" | "cpu" | "gpu";
  keep_model_loaded: boolean;
  widget_enabled: boolean;
  widget_autohide: boolean;
  widget_opacity: number;
  voice_commands_enabled: boolean;
  onboarding_completed: boolean;
};

type TranscriptionResult = {
  text: string;
  wav_path: string;
  model_path: string;
};

type WhisperEnvironmentReport = {
  ready: boolean;
  model_path: string;
  whisper_cli_path: string;
  notes: string[];
};

type ComputeCapabilityReport = {
  gpu_available: boolean;
  supports_ngl: boolean;
  supports_no_gpu_flag: boolean;
  whisper_cli_path: string;
  details: string;
};

type DictationStatusEvent = {
  state: string;
  message: string;
};

type DictationTranscriptEvent = {
  text: string;
  wav_path: string;
  model_path: string;
  created_at_ms: number;
};

type HistoryItem = {
  id: string;
  createdAt: string;
  text: string;
  wavPath: string;
  modelUsed?: string;
};

type ModelInfo = {
  id: string;
  label: string;
  filename: string;
  installed: boolean;
  active: boolean;
  size_bytes: number | null;
};

type ModelDownloadProgressEvent = {
  model_id: string;
  status: string;
  progress_pct: number | null;
  downloaded_bytes: number;
  total_bytes: number | null;
  message: string;
};

const HISTORY_KEY = "whisperpro_transcription_history";
const MAX_HISTORY_ITEMS = 20;

const LANGUAGE_OPTIONS: Array<{ value: string; label: string }> = [
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

const TRANSLATION_VALUES: Array<{ value: string }> = [
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

const COMPUTE_MODE_VALUES: Array<{ value: UserSettings["compute_mode"] }> = [
  { value: "auto" },
  { value: "cpu" },
  { value: "gpu" }
];

const defaultSettings: UserSettings = {
  language: "auto",
  shortcut: "Ctrl+Shift+Space",
  model_path: "",
  whisper_cli_path: "",
  compute_mode: "auto",
  keep_model_loaded: false,
  widget_enabled: true,
  widget_autohide: true,
  widget_opacity: 0.9,
  voice_commands_enabled: true,
  onboarding_completed: true
};

function OverlayWidget() {
  const [uiLanguage, setUiLanguage] = useState<UiLanguage>("fr");
  const [state, setState] = useState<string>("idle");
  const [widgetOpacity, setWidgetOpacity] = useState(defaultSettings.widget_opacity);
  const uiText = UI_TEXT[uiLanguage];
  const terminalStateAtRef = useRef<number>(0);
  const terminalLockUntilRef = useRef<number>(0);
  const stateRef = useRef<string>("idle");
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  const applyWidgetState = (nextState: string) => {
    const now = Date.now();
    const isTerminal = nextState === "done" || nextState === "error";
    const isActive = nextState === "listening" || nextState === "transcribing" || nextState === "busy";

    if (isTerminal) {
      if (stateRef.current === nextState) return;
      terminalStateAtRef.current = now;
      terminalLockUntilRef.current = now + 760;
      setState(nextState);
      return;
    }

    if (nextState === "idle") {
      if (terminalStateAtRef.current !== 0 && now - terminalStateAtRef.current < 1200) return;
      setState("idle");
      return;
    }

    if (isActive) {
      if (terminalLockUntilRef.current !== 0 && now < terminalLockUntilRef.current) return;
      terminalStateAtRef.current = 0;
      terminalLockUntilRef.current = 0;
      setState(nextState);
      return;
    }

    setState(nextState);
  };
  const visualState = useMemo(() => {
    if (state === "listening") return "listening";
    if (state === "transcribing" || state === "busy") return "transcribing";
    if (state === "done") return "done";
    if (state === "error") return "error";
    return "idle";
  }, [state]);
  const widgetLabel = useMemo(() => {
    if (visualState === "listening") return uiText.overlayListening;
    if (visualState === "transcribing") return uiText.overlayTranscribing;
    if (visualState === "error") return uiText.overlayError;
    return "";
  }, [visualState, uiText]);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(UI_LANGUAGE_STORAGE_KEY);
      if (raw === "fr" || raw === "en") setUiLanguage(raw);
    } catch {
      // ignore
    }
  }, []);  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key !== UI_LANGUAGE_STORAGE_KEY) return;
      if (event.newValue === "fr" || event.newValue === "en") {
        setUiLanguage(event.newValue);
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  useEffect(() => {
    let unlistenUiLanguage: (() => void) | null = null;
    const bootstrap = async () => {
      unlistenUiLanguage = await listen<{ language: UiLanguage }>("ui-language-changed", (event) => {
        const next = event.payload.language;
        if (next === "fr" || next === "en") {
          setUiLanguage(next);
        }
      });
    };
    void bootstrap();
    return () => {
      if (unlistenUiLanguage) unlistenUiLanguage();
    };
  }, []);

  useEffect(() => {
    document.documentElement.classList.add("overlay-mode");
    document.body.classList.add("overlay-mode");
    return () => {
      document.documentElement.classList.remove("overlay-mode");
      document.body.classList.remove("overlay-mode");
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let unlistenMoved: (() => void) | null = null;
    let unlistenSettings: (() => void) | null = null;

    const bootstrap = async () => {
      try {
        const [status, loadedSettings] = await Promise.all([
          invoke<DictationStatusEvent>("get_dictation_status"),
          invoke<UserSettings>("get_settings")
        ]);
        applyWidgetState(status.state);
        setWidgetOpacity(Math.max(0.25, Math.min(1, loadedSettings.widget_opacity ?? defaultSettings.widget_opacity)));
      } catch {
        // ignore
      }

      unlisten = await listen<DictationStatusEvent>("dictation-status", (event) => {
        applyWidgetState(event.payload.state);
      });

      const appWindow = getCurrentWindow();
      unlistenMoved = await appWindow.onMoved(() => {
        // overlay is cursor-anchored, no manual position persistence
      });

      unlistenSettings = await listen<UserSettings>("settings-updated", (event) => {
        setWidgetOpacity(Math.max(0.25, Math.min(1, event.payload.widget_opacity ?? defaultSettings.widget_opacity)));
      });
    };

    void bootstrap();

    return () => {
      if (unlisten) unlisten();
      if (unlistenMoved) unlistenMoved();
      if (unlistenSettings) unlistenSettings();
    };
  }, []);

  return (
    <main className="widget-root">
      <aside
        className={`dictation-overlay ${visualState}`}
        data-tauri-drag-region
        onMouseDown={() => {
          void invoke("start_overlay_drag");
        }}
      >
        <div className={`widget-dynamic ${visualState}`} style={{ opacity: widgetOpacity }}>
          <div className="widget-orb-wrap" aria-hidden="true">
            <div className="orb-stack">
              <span className="orb halo" />
              <span className="orb shell" />
              <span className="orb center-dot" />
              <span className="orb ring ring-a" />
              <span className="orb ring ring-b" />
            </div>
          </div>
          {visualState !== "idle" ? <div className="widget-copy">{widgetLabel}</div> : null}
          {visualState !== "idle" ? (
            <div className="widget-indicator" aria-hidden="true">
              {visualState === "listening" ? (
                <div className="audio-bars">
                  <span />
                  <span />
                  <span />
                  <span />
                  <span />
                </div>
              ) : null}
              {visualState === "transcribing" ? <div className="spinner-ring" /> : null}
            </div>
          ) : null}
        </div>
      </aside>
    </main>
  );
}

function MainApp() {
  const [settings, setSettings] = useState<UserSettings>(defaultSettings);
  const [savedSettings, setSavedSettings] = useState<UserSettings>(defaultSettings);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [uiLanguage, setUiLanguage] = useState<UiLanguage>("fr");
  const [settingsState, setSettingsState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [settingsError, setSettingsError] = useState("");
  const [shortcutDraft, setShortcutDraft] = useState(defaultSettings.shortcut);
  const [capturingShortcut, setCapturingShortcut] = useState(false);

  const [recording, setRecording] = useState(false);
  const [working, setWorking] = useState(false);
  const [wavPath, setWavPath] = useState("");
  const [transcript, setTranscript] = useState("");
  const [translatedText, setTranslatedText] = useState("");
  const [translationError, setTranslationError] = useState("");
  const [translationTarget, setTranslationTarget] = useState<string>("none");
  const [textView, setTextView] = useState<"source" | "translated">("source");

  const [historyItems, setHistoryItems] = useState<HistoryItem[]>([]);
  const [historyReady, setHistoryReady] = useState(false);
  const [statusLine, setStatusLine] = useState("Prêt à transcrire");
  const [errorLine, setErrorLine] = useState("");

  const [models, setModels] = useState<ModelInfo[]>([]);
  const [modelsBusy, setModelsBusy] = useState(false);
  const [modelsError, setModelsError] = useState("");
  const [downloadProgress, setDownloadProgress] = useState<ModelDownloadProgressEvent | null>(null);
  const [downloadingModelId, setDownloadingModelId] = useState<string | null>(null);
  const [computeCapability, setComputeCapability] = useState<ComputeCapabilityReport | null>(null);
  const [runtimeSetupBusy, setRuntimeSetupBusy] = useState(false);
  const uiText = UI_TEXT[uiLanguage];

  const modelDisplayLabel = (model: { id: string; label: string }) => uiText.modelLabels[model.id] ?? model.label;
  const activeModelLabel = useMemo(() => {
    const active = models.find((m) => m.active);
    return active ? modelDisplayLabel(active) : uiText.unknown;
  }, [models, uiText]);
  const installedModels = useMemo(() => models.filter((m) => m.installed), [models]);
  const activeModelId = useMemo(() => models.find((m) => m.active)?.id ?? "", [models]);
  const translationOptions = useMemo(
    () =>
      TRANSLATION_VALUES.map((option) => ({
        value: option.value,
        label: uiText.translationLabels[option.value as keyof typeof uiText.translationLabels]
      })),
    [uiText]
  );
  const computeModeOptions = useMemo(
    () =>
      COMPUTE_MODE_VALUES.map((option) => ({
        value: option.value,
        label: uiText.computeModeLabels[option.value]
      })),
    [uiText]
  );
  const translationTargetLabel = useMemo(
    () => translationOptions.find((o) => o.value === translationTarget)?.label ?? translationTarget,
    [translationTarget, translationOptions]
  );
  const settingsDirty = useMemo(
    () => JSON.stringify(settings) !== JSON.stringify(savedSettings),
    [settings, savedSettings]
  );
  const isDownloadInProgress = useMemo(() => {
    const status = downloadProgress?.status ?? "";
    return status === "starting" || status === "downloading" || status === "canceling";
  }, [downloadProgress]);

  const refreshModels = async () => {
    try {
      const list = await invoke<ModelInfo[]>("list_models");
      setModels(list);
      setModelsError("");
    } catch (e) {
      setModelsError(String(e));
    }
  };

  const modelLabelFromPath = (path: string) => {
    if (!path) return uiText.unknown;
    const normalized = path.replaceAll("\\", "/");
    return normalized.split("/").pop() ?? uiText.unknown;
  };

  const normalizeSettingsForUi = (loaded: UserSettings): UserSettings => {
    const normalizedOpacity = Math.max(0.25, Math.min(1, loaded.widget_opacity ?? defaultSettings.widget_opacity));
    return {
      ...loaded,
      keep_model_loaded: false,
      widget_opacity: normalizedOpacity
    };
  };

  const addHistoryItem = (text: string, sourceWavPath: string, modelPath: string, createdAtMs?: number) => {
    const cleaned = text.trim();
    if (!cleaned) return;

    const stamp = createdAtMs ?? Date.now();
    const createdAtIso = new Date(stamp).toISOString();

    setHistoryItems((current) => {
      const newItem: HistoryItem = {
        id: `${stamp}-${Math.random().toString(36).slice(2, 8)}`,
        createdAt: createdAtIso,
        text: cleaned,
        wavPath: sourceWavPath,
        modelUsed: modelLabelFromPath(modelPath)
      };
      const filtered = current.filter(
        (item) => !(item.text === newItem.text && item.wavPath === newItem.wavPath && item.createdAt === newItem.createdAt)
      );
      return [newItem, ...filtered].slice(0, MAX_HISTORY_ITEMS);
    });
  };

  const translateTextIfNeeded = async (sourceText: string, sourceLanguage: string) => {
    if (translationTarget === "none" || !sourceText.trim()) {
      setTranslatedText("");
      setTranslationError("");
      return;
    }

    try {
      const translated = await invoke<string>("translate_text", {
        text: sourceText,
        targetLang: translationTarget,
        sourceLang: sourceLanguage
      });
      setTranslatedText(translated);
      setTranslationError("");
      setTextView("translated");
      setStatusLine(uiText.statusTranslationDone);
    } catch (e) {
      setTranslationError(String(e));
      setTextView("source");
      setStatusLine(uiText.statusTranscriptionDoneNoTranslation);
    }
  };

  const formatShortcutFromEvent = (event: KeyboardEvent) => {
    const key = event.key;
    if (["Control", "Shift", "Alt", "Meta"].includes(key)) return "";

    const parts: string[] = [];
    if (event.ctrlKey) parts.push("Ctrl");
    if (event.altKey) parts.push("Alt");
    if (event.shiftKey) parts.push("Shift");
    if (event.metaKey) parts.push("Meta");

    let normalized = key.length === 1 ? key.toUpperCase() : key;
    if (normalized === " ") normalized = "Space";
    if (normalized === "Escape") normalized = "Esc";

    parts.push(normalized);
    return parts.join("+");
  };

  useEffect(() => {
    try {
      const raw = localStorage.getItem(HISTORY_KEY);
      if (raw) {
        const parsed = JSON.parse(raw) as HistoryItem[];
        if (Array.isArray(parsed)) setHistoryItems(parsed);
      }
    } catch {
      // ignore
    } finally {
      setHistoryReady(true);
    }
  }, []);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(UI_LANGUAGE_STORAGE_KEY);
      if (raw === "fr" || raw === "en") setUiLanguage(raw);
    } catch {
      // ignore
    }
  }, []);  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key !== UI_LANGUAGE_STORAGE_KEY) return;
      if (event.newValue === "fr" || event.newValue === "en") {
        setUiLanguage(event.newValue);
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  useEffect(() => {
    let unlistenUiLanguage: (() => void) | null = null;
    const bootstrap = async () => {
      unlistenUiLanguage = await listen<{ language: UiLanguage }>("ui-language-changed", (event) => {
        const next = event.payload.language;
        if (next === "fr" || next === "en") {
          setUiLanguage(next);
        }
      });
    };
    void bootstrap();
    return () => {
      if (unlistenUiLanguage) unlistenUiLanguage();
    };
  }, []);

  useEffect(() => {
    localStorage.setItem(UI_LANGUAGE_STORAGE_KEY, uiLanguage);
    void emit("ui-language-changed", { language: uiLanguage });
  }, [uiLanguage]);

  useEffect(() => {
    if (!historyReady) return;
    localStorage.setItem(HISTORY_KEY, JSON.stringify(historyItems));
  }, [historyItems, historyReady]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const bootstrap = async () => {
      try {
        const [loadedSettings, env] = await Promise.all([
          invoke<UserSettings>("get_settings"),
          invoke<WhisperEnvironmentReport>("test_whisper_environment")
        ]);

        setSettings({
          ...normalizeSettingsForUi(loadedSettings),
          model_path: env.model_path,
          whisper_cli_path: env.whisper_cli_path
        });
        setSavedSettings({
          ...normalizeSettingsForUi(loadedSettings),
          model_path: env.model_path,
          whisper_cli_path: env.whisper_cli_path
        });
        setShortcutDraft(loadedSettings.shortcut);
        await refreshModels();
        try {
          const capability = await invoke<ComputeCapabilityReport>("get_compute_capability");
          setComputeCapability(capability);
        } catch {
          setComputeCapability(null);
        }

        if (!env.ready) setStatusLine(uiText.statusConfigRequired);
      } catch (e) {
        setErrorLine(String(e));
      }

      unlisten = await listen<DictationStatusEvent>("dictation-status", (event) => {
        if (!working && !recording) setStatusLine(event.payload.message);
      });
    };

    void bootstrap();
    return () => {
      if (unlisten) unlisten();
    };
  }, [recording, working]);

  useEffect(() => {
    let unlistenTranscript: (() => void) | null = null;

    const bootstrap = async () => {
      unlistenTranscript = await listen<DictationTranscriptEvent>("dictation-transcript", (event) => {
        const payload = event.payload;
        setTranscript(payload.text);
        setWavPath(payload.wav_path);
        addHistoryItem(payload.text, payload.wav_path, payload.model_path, payload.created_at_ms);
        setTextView("source");
        const selectedModel = settings.model_path.replaceAll("\\", "/").toLowerCase();
        const usedModel = payload.model_path.replaceAll("\\", "/").toLowerCase();
        if (selectedModel && usedModel && selectedModel !== usedModel) {
          setErrorLine(
            `${uiText.warningSelectedModelMismatchPrefix} (${modelLabelFromPath(settings.model_path)}) ${uiText.warningSelectedModelMismatchConnector} (${modelLabelFromPath(payload.model_path)}).`
          );
        }
        void translateTextIfNeeded(payload.text, settings.language);
        if (translationTarget === "none") setStatusLine(`${uiText.statusTranscriptionDone} (${modelLabelFromPath(payload.model_path)})`);
      });
    };

    void bootstrap();
    return () => {
      if (unlistenTranscript) unlistenTranscript();
    };
  }, [translationTarget, settings.language]);

  useEffect(() => {
    let unlistenDownload: (() => void) | null = null;

    const bootstrap = async () => {
      unlistenDownload = await listen<ModelDownloadProgressEvent>("model-download-progress", (event) => {
        const payload = event.payload;
        setDownloadProgress(payload);
        if (payload.status === "error") setModelsError(payload.message);
        if (payload.status === "done" || payload.status === "error" || payload.status === "canceled") {
          setDownloadingModelId(null);
        }
      });
    };

    void bootstrap();
    return () => {
      if (unlistenDownload) unlistenDownload();
    };
  }, []);

  useEffect(() => {
    if (!capturingShortcut) return;

    const handler = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        setCapturingShortcut(false);
        setStatusLine(uiText.statusCaptureCancelled);
        return;
      }
      const combo = formatShortcutFromEvent(event);
      if (!combo) return;
      setShortcutDraft(combo);
      setSettings((s) => ({ ...s, shortcut: combo }));
      setCapturingShortcut(false);
      setStatusLine(`${uiText.statusShortcutDetectedPrefix}: ${combo}`);
    };

    window.addEventListener("keydown", handler, { capture: true });
    return () => window.removeEventListener("keydown", handler, { capture: true });
  }, [capturingShortcut]);

  const saveSettingsSnapshot = async (next: UserSettings, successLabel = uiText.settingsSaved) => {
    const shortcutFromDraft = shortcutDraft.trim();
    const payload: UserSettings = {
      ...next,
      shortcut: shortcutFromDraft || next.shortcut
    };
    if (!payload.shortcut.trim()) {
      setSettingsState("error");
      setSettingsError(uiText.shortcutEmptyExample);
      return false;
    }
    setSettingsState("saving");
    setSettingsError("");
    try {
      await invoke("save_settings", { settings: payload });
      setSettings(payload);
      setSavedSettings(payload);
      setShortcutDraft(payload.shortcut);
      setSettingsState("saved");
      setStatusLine(successLabel);
      try {
        const capability = await invoke<ComputeCapabilityReport>("get_compute_capability");
        setComputeCapability(capability);
      } catch {
        setComputeCapability(null);
      }
      return true;
    } catch (e) {
      setSettingsState("error");
      setSettingsError(String(e));
      return false;
    }
  };

  const closeSettingsPanel = async () => {
    if (settingsDirty) {
      const ok = await saveSettingsSnapshot(settings, uiText.settingsSaved);
      if (!ok) return;
    }
    setSettingsOpen(false);
  };

  const resetSettingsToDefaults = async () => {
    const shouldReset = window.confirm(uiText.confirmResetOptions);
    if (!shouldReset) return;
    const next: UserSettings = {
      ...settings,
      language: defaultSettings.language,
      shortcut: defaultSettings.shortcut,
      compute_mode: defaultSettings.compute_mode,
      keep_model_loaded: false,
      widget_enabled: defaultSettings.widget_enabled,
      widget_autohide: defaultSettings.widget_autohide,
      widget_opacity: defaultSettings.widget_opacity,
      voice_commands_enabled: defaultSettings.voice_commands_enabled
    };
    setShortcutDraft(next.shortcut);
    await saveSettingsSnapshot(next, uiText.settingsResetDone);
  };

  const downloadModel = async (modelId: string) => {
    setDownloadingModelId(modelId);
    setModelsError("");
    try {
      const message = await invoke<string>("download_model", { modelId });
      const loadedSettings = await invoke<UserSettings>("get_settings");
      const normalized = normalizeSettingsForUi(loadedSettings);
      setSettings(normalized);
      setSavedSettings(normalized);
      setShortcutDraft(loadedSettings.shortcut);
      await refreshModels();
      setStatusLine(message);
    } catch (e) {
      setModelsError(String(e));
    } finally {
      setDownloadingModelId(null);
    }
  };

  const cancelModelDownload = async () => {
    try {
      const message = await invoke<string>("cancel_model_download");
      setStatusLine(message);
    } catch (e) {
      setModelsError(String(e));
    }
  };

  const activateModel = async (modelId: string) => {
    setModelsBusy(true);
    setModelsError("");
    try {
      const message = await invoke<string>("set_active_model", { modelId });
      const loadedSettings = await invoke<UserSettings>("get_settings");
      const normalized = normalizeSettingsForUi(loadedSettings);
      setSettings(normalized);
      setSavedSettings(normalized);
      setShortcutDraft(loadedSettings.shortcut);
      await refreshModels();
      setStatusLine(message);
    } catch (e) {
      setModelsError(String(e));
    } finally {
      setModelsBusy(false);
    }
  };

  const removeModel = async (modelId: string) => {
    setModelsBusy(true);
    setModelsError("");
    try {
      const message = await invoke<string>("delete_model", { modelId });
      const loadedSettings = await invoke<UserSettings>("get_settings");
      const normalized = normalizeSettingsForUi(loadedSettings);
      setSettings(normalized);
      setSavedSettings(normalized);
      setShortcutDraft(loadedSettings.shortcut);
      await refreshModels();
      setStatusLine(message);
    } catch (e) {
      setModelsError(String(e));
    } finally {
      setModelsBusy(false);
    }
  };

  const repairRuntime = async () => {
    setRuntimeSetupBusy(true);
    setSettingsError("");
    try {
      const message = await invoke<string>("auto_setup_runtime");
      setStatusLine(message || uiText.statusEngineChecked);
      const loadedSettings = await invoke<UserSettings>("get_settings");
      const normalized = normalizeSettingsForUi(loadedSettings);
      setSettings(normalized);
      setSavedSettings(normalized);
      setShortcutDraft(loadedSettings.shortcut);
      const capability = await invoke<ComputeCapabilityReport>("get_compute_capability");
      setComputeCapability(capability);
    } catch (e) {
      setSettingsError(String(e));
      setStatusLine(uiText.statusEngineRepairFailed);
    } finally {
      setRuntimeSetupBusy(false);
    }
  };

  const startRecording = async () => {
    setErrorLine("");
    setTranscript("");
    setTranslatedText("");
    setTranslationError("");
    setTextView("source");
    setStatusLine(uiText.statusRecordingInProgress);
    try {
      await invoke<string>("start_capture");
      setRecording(true);
    } catch (e) {
      setErrorLine(String(e));
      setStatusLine(uiText.statusRecordingStartFailed);
    }
  };

  const stopRecordingAndTranscribe = async () => {
    setWorking(true);
    setErrorLine("");
    setStatusLine(uiText.statusTranscriptionInProgress);
    try {
      const outputPath = await invoke<string>("stop_capture");
      setRecording(false);
      setWavPath(outputPath);
      const result = await invoke<TranscriptionResult>("transcribe_wav", {
        wavPath: outputPath,
        modelId: activeModelId || null
      });
      setTranscript(result.text);
      addHistoryItem(result.text, outputPath, result.model_path);
      setTextView("source");
      const selectedModel = settings.model_path.replaceAll("\\", "/").toLowerCase();
      const usedModel = result.model_path.replaceAll("\\", "/").toLowerCase();
      if (selectedModel && usedModel && selectedModel !== usedModel) {
        setErrorLine(
          `${uiText.warningSelectedModelMismatchPrefix} (${modelLabelFromPath(settings.model_path)}) ${uiText.warningSelectedModelMismatchConnector} (${modelLabelFromPath(result.model_path)}).`
        );
      }

      if (translationTarget !== "none") {
        await translateTextIfNeeded(result.text, settings.language);
      } else {
        setStatusLine(result.text ? `${uiText.statusTranscriptionDone} (${modelLabelFromPath(result.model_path)})` : uiText.statusNoSpeechDetected);
      }
    } catch (e) {
      setRecording(false);
      setErrorLine(String(e));
      setStatusLine(uiText.statusTranscriptionError);
    } finally {
      setWorking(false);
    }
  };

  const currentVisibleText = textView === "translated" && translationTarget !== "none" ? translatedText : transcript;

  const copyVisibleText = async () => {
    if (!currentVisibleText.trim()) return;
    try {
      await navigator.clipboard.writeText(currentVisibleText);
      setStatusLine(textView === "translated" ? uiText.statusTranslationCopied : uiText.statusTextCopied);
    } catch {
      setStatusLine(uiText.copyImpossible);
    }
  };

  const cleanupHistoryArtifacts = async (paths: string[]) => {
    const unique = Array.from(new Set(paths.map((p) => p.trim()).filter(Boolean)));
    if (unique.length === 0) return uiText.statusHistoryEmpty;
    try {
      return await invoke<string>("clear_history_artifacts", { payload: { wavPaths: unique } });
    } catch (firstError) {
      try {
        return await invoke<string>("clear_history_artifacts", { wavPaths: unique });
      } catch {
        try {
          return await invoke<string>("clear_history_artifacts", { wav_paths: unique });
        } catch {
          throw firstError;
        }
      }
    }
  };

  const clearAllHistory = async () => {
    const paths = historyItems.map((item) => item.wavPath).filter(Boolean);
    try {
      const message = await cleanupHistoryArtifacts(paths);
      setHistoryItems([]);
      setWavPath("");
      setStatusLine(message || uiText.statusHistoryEmpty);
      setErrorLine("");
    } catch (e) {
      setHistoryItems([]);
      setWavPath("");
      setStatusLine(uiText.statusHistoryEmptyPartial);
      setErrorLine(`${uiText.errorDeleteFilesIncompletePrefix}: ${String(e)}`);
    }
  };

  const removeHistoryItem = async (item: HistoryItem) => {
    try {
      const message = await cleanupHistoryArtifacts([item.wavPath]);
      setStatusLine(message || uiText.statusEntryDeleted);
      setErrorLine("");
    } catch (e) {
      setErrorLine(`${uiText.errorDeleteFileImpossiblePrefix}: ${String(e)}`);
    } finally {
      setHistoryItems((current) => current.filter((entry) => entry.id !== item.id));
    }
  };

  const quitApplication = async () => {
    const shouldQuit = window.confirm(uiText.confirmQuitApp);
    if (!shouldQuit) return;
    try {
      await invoke("quit_application");
    } catch (e) {
      setErrorLine(`${uiText.errorQuitAppPrefix}: ${String(e)}`);
    }
  };

  const disabled = working || settingsState === "saving";

  return (
    <main className="app v2">
      <header className="topbar">
        <div className="brand-block">
          <h1>WhisperPro</h1>
          <p className="brand-subtitle">{uiText.topbarSubtitle}</p>
        </div>
        <div className="top-actions">
          <div className="ui-language-flags" role="group" aria-label={uiText.uiLanguageLabel}>
            <button
              type="button"
              className={uiLanguage === "fr" ? "active" : ""}
              onClick={() => setUiLanguage("fr")}
              title="Français"
              aria-label="Français"
            >
              <span className="flag-icon flag-fr" aria-hidden="true" />
            </button>
            <button
              type="button"
              className={uiLanguage === "en" ? "active" : ""}
              onClick={() => setUiLanguage("en")}
              title="English"
              aria-label="English"
            >
              <span className="flag-icon flag-en" aria-hidden="true" />
            </button>
          </div>
          <button type="button" className="ghost" onClick={() => setSettingsOpen(true)} disabled={disabled}>
            {uiText.options}
          </button>
          <button type="button" className="secondary" onClick={quitApplication} disabled={disabled}>
            {uiText.quit}
          </button>
        </div>
      </header>

      <section className="content-grid">
        <section className="panel main-panel">
          <section className="hero-actions">
            <div className="cta-row controls-primary">
              {!recording ? (
                <button type="button" className="primary big" onClick={startRecording} disabled={disabled}>
                  {uiText.startSpeaking}
                </button>
              ) : (
                <button type="button" className="danger big" onClick={stopRecordingAndTranscribe} disabled={disabled}>
                  {uiText.stopAndTranscribe}
                </button>
              )}

              <label className="model-select">
                <span>
                  {uiText.activeModel}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipActiveModel}
                    aria-label={uiText.ariaHelpActiveModel}
                  >
                    ?
                  </button>
                </span>
                <select
                  value={activeModelId}
                  onChange={(event) => {
                    const nextId = event.target.value;
                    if (nextId) void activateModel(nextId);
                  }}
                  disabled={modelsBusy || installedModels.length === 0}
                >
                  {installedModels.length === 0 ? <option value="">{uiText.noModelInstalled}</option> : null}
                  {installedModels.map((model) => (
                    <option key={model.id} value={model.id}>
                      {modelDisplayLabel(model)}
                    </option>
                  ))}
                </select>
              </label>
            </div>

            <div className="cta-row controls-secondary">
              <div className="chip-row">
                <div className="chip">{uiText.model}: {activeModelLabel}</div>
                <div className="chip">{uiText.shortcut}: {settings.shortcut}</div>
              </div>
              <label className="translation-select">
                <span>
                  {uiText.translation}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipTranslation}
                    aria-label={uiText.ariaHelpTranslation}
                  >
                    ?
                  </button>
                </span>
                <select
                  value={translationTarget}
                  onChange={(e) => {
                    const next = e.target.value;
                    setTranslationTarget(next);
                    if (next === "none") {
                      setTranslatedText("");
                      setTranslationError("");
                      setTextView("source");
                    } else if (transcript.trim()) {
                      void translateTextIfNeeded(transcript, settings.language);
                    }
                  }}
                >
                  {translationOptions.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          </section>

          <div className="status-stack">
            <p className="status">{statusLine}</p>
            {errorLine ? <p className="error">{errorLine}</p> : null}
          </div>

          <section className="editor-shell">
            <div className="result-header">
              <h2>{uiText.yourText}</h2>
              <div className="inline-actions">
                {translationTarget !== "none" ? (
                  <>
                    <button type="button" className={textView === "source" ? "ghost" : "secondary"} onClick={() => setTextView("source")}>
                      {uiText.original}
                    </button>
                    <button
                      type="button"
                      className={textView === "translated" ? "ghost" : "secondary"}
                      onClick={() => setTextView("translated")}
                      disabled={!translatedText}
                    >
                      {uiText.translated}
                    </button>
                  </>
                ) : null}
                <button type="button" onClick={copyVisibleText} disabled={!currentVisibleText || disabled}>
                  {uiText.copy}
                </button>
              </div>
            </div>

            <textarea
              className="result"
              value={currentVisibleText}
              onChange={(event) => {
                if (textView === "translated" && translationTarget !== "none") {
                  setTranslatedText(event.target.value);
                } else {
                  setTranscript(event.target.value);
                }
              }}
              placeholder={textView === "translated" ? uiText.translationPlaceholder : uiText.transcriptionPlaceholder}
            />

            <div className="editor-foot">
              {wavPath ? <p className="meta">{uiText.lastRecording}: {wavPath}</p> : null}
              {translationTarget !== "none" ? <p className="meta">{uiText.targetLanguage}: {translationTargetLabel}</p> : null}
              {translationError ? <p className="error">{translationError}</p> : null}
            </div>
          </section>
        </section>

        <aside className="panel history-panel">
          <div className="result-header">
            <h2>{uiText.history}</h2>
            <button type="button" className="ghost" disabled={historyItems.length === 0 || disabled} onClick={() => void clearAllHistory()}>
              {uiText.clear}
            </button>
          </div>

          {historyItems.length === 0 ? (
            <p className="meta">{uiText.noHistory}</p>
          ) : (
            <div className="history-list scrollable">
              {historyItems.map((item) => (
                <article className="history-item" key={item.id}>
                  <p className="history-time">{new Date(item.createdAt).toLocaleString()}</p>
                  <p className="history-time">{uiText.model}: {item.modelUsed || uiText.unknown}</p>
                  <p className="history-text">{item.text}</p>
                  <div className="inline-actions">
                    <button
                      type="button"
                      onClick={async () => {
                        try {
                          await navigator.clipboard.writeText(item.text);
                          setStatusLine(uiText.historyTextCopied);
                        } catch {
                          setStatusLine(uiText.copyImpossible);
                        }
                      }}
                    >
                      {uiText.copy}
                    </button>
                    <button type="button" onClick={() => void removeHistoryItem(item)}>
                      {uiText.delete}
                    </button>
                  </div>
                </article>
              ))}
            </div>
          )}
        </aside>
      </section>

      {settingsOpen ? (
        <div className="overlay" onClick={() => void closeSettingsPanel()}>
          <section className="panel settings-drawer" onClick={(e) => e.stopPropagation()}>
            <div className="result-header settings-header">
              <h2>{uiText.options}</h2>
              <div className="inline-actions">
                <button
                  type="button"
                  className="primary"
                  onClick={() => void saveSettingsSnapshot({ ...settings, shortcut: shortcutDraft.trim() || settings.shortcut })}
                  disabled={settingsState === "saving"}
                >
                  {settingsState === "saving" ? uiText.saving : uiText.save}
                </button>
                <button type="button" className="secondary" onClick={() => void resetSettingsToDefaults()} disabled={settingsState === "saving"}>
                  {uiText.reset}
                </button>
                <button type="button" className="ghost" onClick={() => void closeSettingsPanel()}>
                  {uiText.close}
                </button>
              </div>
            </div>
            {isDownloadInProgress ? (
              <div className="progress-box settings-progress settings-progress-sticky">
                <div className="inline-actions progress-header">
                  <p className="history-time">{downloadProgress?.message ?? uiText.downloading}</p>
                  <button type="button" className="secondary" onClick={() => void cancelModelDownload()}>
                    {uiText.cancel}
                  </button>
                </div>
                <div className="progress-track">
                  <span className="progress-fill" style={{ width: `${Math.min(100, Math.max(2, downloadProgress?.progress_pct ?? 0))}%` }} />
                </div>
              </div>
            ) : null}
            {settingsDirty ? <p className="settings-warning">{uiText.unsavedChanges}</p> : null}
            {settingsError ? <p className="error">{settingsError}</p> : null}

            <section className="settings-group">
              <h3 className="settings-group-title">{uiText.sectionGeneral}</h3>
              <label className="field">
                <span>
                  {uiText.language}
                  <button type="button" className="info-dot" title={uiText.tipRecognitionLanguage} aria-label={uiText.ariaHelpLanguage}>
                    ?
                  </button>
                </span>
                <select value={settings.language} onChange={(e) => setSettings((s) => ({ ...s, language: e.target.value }))}>
                  {LANGUAGE_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                  {!LANGUAGE_OPTIONS.some((option) => option.value === settings.language) ? (
                    <option value={settings.language}>{uiText.customLanguage} ({settings.language})</option>
                  ) : null}
                </select>
              </label>

              <label className="field">
                <span>
                  {uiText.computeMode}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipComputeMode}
                    aria-label={uiText.ariaHelpComputeMode}
                  >
                    ?
                  </button>
                </span>
                <select
                  value={settings.compute_mode}
                  onChange={(e) => setSettings((s) => ({ ...s, compute_mode: e.target.value as UserSettings["compute_mode"] }))}
                  disabled={runtimeSetupBusy}
                >
                  {computeModeOptions.map((option) => (
                    <option
                      key={option.value}
                      value={option.value}
                      disabled={option.value === "gpu" && computeCapability !== null && !computeCapability.gpu_available}
                    >
                      {option.label}
                    </option>
                  ))}
                </select>
                {computeCapability ? (
                  <p className="meta">
                    {computeCapability.gpu_available ? uiText.gpuDetected : uiText.gpuNotDetected} {computeCapability.details}
                  </p>
                ) : null}
                <button type="button" className="ghost" onClick={() => void repairRuntime()} disabled={runtimeSetupBusy}>
                  {runtimeSetupBusy ? uiText.checkRuntime : uiText.repairAcceleration}
                </button>
              </label>
            </section>

            <section className="settings-group">
              <h3 className="settings-group-title">{uiText.sectionShortcutInput}</h3>
              <label className="field">
                <span>
                  {uiText.keyboardShortcut}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipShortcut}
                    aria-label={uiText.ariaHelpShortcut}
                  >
                    ?
                  </button>
                </span>
                <div className="inline-actions">
                  <input
                    value={shortcutDraft}
                    onChange={(e) => {
                      const next = e.target.value;
                      setShortcutDraft(next);
                      setSettings((s) => ({ ...s, shortcut: next }));
                    }}
                    placeholder="Ctrl+Shift+Space"
                    readOnly={capturingShortcut}
                  />
                  <button
                    type="button"
                    className={capturingShortcut ? "secondary" : "ghost"}
                    onClick={() => {
                      const next = !capturingShortcut;
                      setCapturingShortcut(next);
                      setStatusLine(next ? uiText.statusPressShortcut : uiText.statusCaptureCancelled);
                    }}
                    disabled={settingsState === "saving"}
                  >
                    {capturingShortcut ? uiText.cancelCapture : uiText.detectKeys}
                  </button>
                </div>
                <p className="meta">{uiText.detectHint}</p>
              </label>

              <label className="field checkbox">
                <span>
                  {uiText.voicePunctuation}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipVoicePunctuation}
                    aria-label={uiText.ariaHelpPunctuation}
                  >
                    ?
                  </button>
                </span>
                <input
                  type="checkbox"
                  checked={settings.voice_commands_enabled}
                  onChange={(e) => setSettings((s) => ({ ...s, voice_commands_enabled: e.target.checked }))}
                />
              </label>
            </section>

            <section className="settings-group">
              <h3 className="settings-group-title">{uiText.sectionWidget}</h3>
              <label className="field checkbox">
                <span>
                  {uiText.showWidget}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipWidget}
                    aria-label={uiText.ariaHelpWidget}
                  >
                    ?
                  </button>
                </span>
                <input
                  type="checkbox"
                  checked={settings.widget_enabled}
                  onChange={(e) => {
                    const next = { ...settings, widget_enabled: e.target.checked };
                    setSettings(next);
                    void saveSettingsSnapshot(next, e.target.checked ? "Mini-widget activé" : "Mini-widget masqué");
                  }}
                  disabled={settingsState === "saving"}
                />
              </label>

              <label className="field">
                <span>
                  {uiText.widgetOpacity} ({Math.round(settings.widget_opacity * 100)}%)
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipWidgetOpacity}
                    aria-label={uiText.ariaHelpWidgetOpacity}
                  >
                    ?
                  </button>
                </span>
                <input
                  type="range"
                  min={0.25}
                  max={1}
                  step={0.05}
                  value={settings.widget_opacity}
                  onChange={(e) => setSettings((s) => ({ ...s, widget_opacity: Number(e.target.value) }))}
                />
                <p className="meta">{uiText.widgetOpacityHint}</p>
              </label>
            </section>

            <section className="settings-group">
              <h3 className="settings-group-title">{uiText.sectionModels}</h3>
              <div className="models-box">
              <div className="result-header">
                <h3>{uiText.library}</h3>
              </div>
              <p className="meta">{uiText.activeModelPath}: {settings.model_path || uiText.noneValue}</p>
              {modelsError ? <p className="error">{modelsError}</p> : null}
              {models.length === 0 ? (
                <p className="meta">{uiText.noModelReferenced}</p>
              ) : (
                <div className="history-list">
                  {models.map((model) => (
                    <article className="history-item" key={model.id}>
                      <p>
                        <strong>{modelDisplayLabel(model)}</strong> ({model.filename})
                      </p>
                      <p className="history-time">
                        {model.installed ? uiText.installed : uiText.notInstalled}
                        {model.active ? ` | ${uiText.active}` : ""}
                        {model.size_bytes ? ` | ${(model.size_bytes / (1024 * 1024)).toFixed(1)} MB` : ""}
                      </p>
                      {uiText.modelExperience[model.id] ? (
                        <div className="model-guide">
                          <p className="meta">
                            <strong>{uiText.idealFor}:</strong> {uiText.modelExperience[model.id].bestFor}
                          </p>
                          <p className="meta">
                            <strong>{uiText.advantages}:</strong> {uiText.modelExperience[model.id].pros}
                          </p>
                          <p className="meta">
                            <strong>{uiText.limits}:</strong> {uiText.modelExperience[model.id].cons}
                          </p>
                        </div>
                      ) : null}
                      <div className="inline-actions">
                        {!model.installed ? (
                          <button
                            type="button"
                            onClick={() => void downloadModel(model.id)}
                            disabled={modelsBusy || isDownloadInProgress}
                            title={uiText.tipDownloadModel}
                          >
                            {downloadingModelId === model.id && isDownloadInProgress ? uiText.downloading : uiText.download}
                          </button>
                        ) : null}
                        {model.installed ? (
                          <button
                            type="button"
                            onClick={() => void removeModel(model.id)}
                            disabled={modelsBusy || isDownloadInProgress}
                            title={uiText.tipDeleteModel}
                          >
                            {uiText.delete}
                          </button>
                        ) : null}
                      </div>
                    </article>
                  ))}
                </div>
              )}
              </div>
            </section>

            <section className="settings-group">
              <h3 className="settings-group-title">{uiText.sectionAdvanced}</h3>
              <details>
                <summary>{uiText.advancedPaths}</summary>
                <label className="field">
                  <span>
                    {uiText.whisperModelPath}
                    <button
                      type="button"
                      className="info-dot"
                      title={uiText.tipModelPath}
                      aria-label={uiText.ariaHelpModelPath}
                    >
                      ?
                    </button>
                  </span>
                  <input value={settings.model_path} onChange={(e) => setSettings((s) => ({ ...s, model_path: e.target.value }))} />
                </label>
                <label className="field">
                  <span>
                    whisper-cli.exe
                    <button
                      type="button"
                      className="info-dot"
                      title={uiText.tipWhisperCliPath}
                      aria-label={uiText.ariaHelpWhisperCli}
                    >
                      ?
                    </button>
                  </span>
                  <input value={settings.whisper_cli_path} onChange={(e) => setSettings((s) => ({ ...s, whisper_cli_path: e.target.value }))} />
                </label>
              </details>
            </section>

          </section>
        </div>
      ) : null}
    </main>
  );
}

function App() {
  const isOverlayWindow = useMemo(() => {
    const params = new URLSearchParams(window.location.search);
    return params.get("overlay") === "1";
  }, []);

  if (isOverlayWindow) return <OverlayWidget />;
  return <MainApp />;
}

export default App;




