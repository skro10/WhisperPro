import { useEffect, useMemo, useRef, useState } from "react";
import { getVersion as getAppVersion } from "@tauri-apps/api/app";

import {
  UI_LANGUAGE_STORAGE_KEY,
  UI_TEXT,
  UI_THEME_STORAGE_KEY,
  WIDGET_THEME_STORAGE_KEY,
  type UiLanguage,
  type UiTheme,
  type WidgetThemeMode
} from "../../i18n";
import {
  autoSetupRuntime,
  cancelModelDownload as cancelModelDownloadCmd,
  clearHistoryArtifacts as clearHistoryArtifactsCmd,
  deleteModel as deleteModelCmd,
  downloadModel as downloadModelCmd,
  getDefaultModelPath,
  getDefaultWhisperCliPath,
  getComputeCapability as getComputeCapabilityCmd,
  getSettings,
  listInputDevices,
  listModels,
  openExternalUrl,
  quitApplication as quitApplicationCmd,
  saveSettings as saveSettingsCmd,
  setActiveModel as setActiveModelCmd,
  startCapture,
  stopCapture,
  testWhisperEnvironment,
  transcribeWav,
  translateText as translateTextCmd,
  translateWavToEnglish
} from "../../lib/tauriApi";
import { emitEvent, listenEvent } from "../../lib/tauriEvents";
import {
  COMPUTE_MODE_VALUES,
  DEFAULT_WIDGET_POP_SOUND,
  HISTORY_KEY,
  MAX_HISTORY_ITEMS,
  TRANSLATION_VALUES,
  WIDGET_SOUND_GAIN,
  defaultSettings
} from "../shared/constants";
import type {
  ComputeCapabilityReport,
  DictationStatusEvent,
  DictationTranscriptEvent,
  HistoryItem,
  InputDeviceInfo,
  ModelDownloadProgressEvent,
  ModelInfo,
  TranscriptionResult,
  UserSettings,
  WhisperEnvironmentReport
} from "../shared/types";

const clamp01 = (value: number) => Math.max(0, Math.min(1, value));
const RELEASES_URL = "https://github.com/skro10/WhisperPro/releases/latest";
const RELEASES_API_URL = "https://api.github.com/repos/skro10/WhisperPro/releases/latest";
const normalizeVersion = (value: string) =>
  (value || "")
    .trim()
    .replace(/^v/i, "")
    .split("-")[0]
    .split(".")
    .map((part) => Number.parseInt(part, 10))
    .filter((part) => Number.isFinite(part));
const isVersionGreater = (candidate: string, current: string) => {
  const c = normalizeVersion(candidate);
  const t = normalizeVersion(current);
  const max = Math.max(c.length, t.length);
  for (let i = 0; i < max; i += 1) {
    const left = c[i] ?? 0;
    const right = t[i] ?? 0;
    if (left > right) return true;
    if (left < right) return false;
  }
  return false;
};
const isNoSpeechBackendMessage = (message: string) => {
  const normalized = (message || "")
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "");
  return (
    normalized.includes("aucune parole detectee") ||
    normalized.includes("aucune voix detectee") ||
    normalized.includes("no speech detected")
  );
};

const getWidgetSoundGain = (fileName: string) => {
  const key = (fileName || "").trim().toLowerCase();
  return WIDGET_SOUND_GAIN[key] ?? 1.0;
};

const loadInitialUiTheme = (): UiTheme => {
  try {
    const stored = localStorage.getItem(UI_THEME_STORAGE_KEY);
    return stored === "dark" ? "dark" : "light";
  } catch {
    return "light";
  }
};

const loadInitialWidgetThemeMode = (): WidgetThemeMode => {
  try {
    const stored = localStorage.getItem(WIDGET_THEME_STORAGE_KEY);
    if (stored === "light" || stored === "dark" || stored === "follow") return stored;
  } catch {
    // ignore
  }
  return "follow";
};

const loadInitialUiLanguage = (): UiLanguage => {
  try {
    const stored = localStorage.getItem(UI_LANGUAGE_STORAGE_KEY);
    if (stored === "fr" || stored === "en") return stored;
  } catch {
    // ignore
  }
  return "fr";
};

export function useMainAppController() {
  const [settings, setSettings] = useState<UserSettings>(defaultSettings);
  const [savedSettings, setSavedSettings] = useState<UserSettings>(defaultSettings);
  const settingsRef = useRef<UserSettings>(defaultSettings);
  const savedSettingsRef = useRef<UserSettings>(defaultSettings);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [uiLanguage, setUiLanguage] = useState<UiLanguage>(loadInitialUiLanguage);
  const [uiTheme, setUiTheme] = useState<UiTheme>(loadInitialUiTheme);
  const [widgetThemeMode, setWidgetThemeMode] = useState<WidgetThemeMode>(loadInitialWidgetThemeMode);
  const [savedWidgetThemeMode, setSavedWidgetThemeMode] = useState<WidgetThemeMode>(loadInitialWidgetThemeMode);
  const [settingsState, setSettingsState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [settingsError, setSettingsError] = useState("");
  const [shortcutDraft, setShortcutDraft] = useState(defaultSettings.shortcut);
  const [capturingShortcut, setCapturingShortcut] = useState(false);

  const [recording, setRecording] = useState(false);
  const [working, setWorking] = useState(false);
  const [translating, setTranslating] = useState(false);
  const [wavPath, setWavPath] = useState("");
  const [transcript, setTranscript] = useState("");
  const [translatedText, setTranslatedText] = useState("");
  const [translationError, setTranslationError] = useState("");
  const [translationTarget, setTranslationTarget] = useState<string>("none");
  const [textView, setTextView] = useState<"source" | "translated">("source");
  const preferSourceTextViewRef = useRef(false);

  const [historyItems, setHistoryItems] = useState<HistoryItem[]>([]);
  const [historyReady, setHistoryReady] = useState(false);
  const [statusLine, setStatusLine] = useState(UI_TEXT[loadInitialUiLanguage()].statusReady);
  const [errorLine, setErrorLine] = useState("");
  const [updateReleaseUrl, setUpdateReleaseUrl] = useState("");
  const [appVersion, setAppVersion] = useState("");

  const [models, setModels] = useState<ModelInfo[]>([]);
  const [modelsBusy, setModelsBusy] = useState(false);
  const [modelsError, setModelsError] = useState("");
  const [downloadProgress, setDownloadProgress] = useState<ModelDownloadProgressEvent | null>(null);
  const [downloadingModelId, setDownloadingModelId] = useState<string | null>(null);
  const [computeCapability, setComputeCapability] = useState<ComputeCapabilityReport | null>(null);
  const [runtimeSetupBusy, setRuntimeSetupBusy] = useState(false);
  const [inputDevices, setInputDevices] = useState<InputDeviceInfo[]>([]);
  const [inputDevicesBusy, setInputDevicesBusy] = useState(false);
  const [widgetSoundOptions, setWidgetSoundOptions] = useState<string[]>([DEFAULT_WIDGET_POP_SOUND]);
  const [previewSoundPlaying, setPreviewSoundPlaying] = useState(false);
  const previewAudioRef = useRef<HTMLAudioElement | null>(null);
  const previewAudioContextRef = useRef<AudioContext | null>(null);
  const previewAudioSourceRef = useRef<MediaElementAudioSourceNode | null>(null);
  const previewAudioGainRef = useRef<GainNode | null>(null);
  const dictationCueContextRef = useRef<AudioContext | null>(null);
  const micMeterStreamRef = useRef<MediaStream | null>(null);
  const micMeterContextRef = useRef<AudioContext | null>(null);
  const micMeterAnimationRef = useRef<number | null>(null);
  const micPermissionRequestedRef = useRef(false);
  const [micMeterActive, setMicMeterActive] = useState(false);
  const [micLevel, setMicLevel] = useState(0);
  const uiText = UI_TEXT[uiLanguage];

  const disconnectPreviewAudioGraph = () => {
    try {
      previewAudioSourceRef.current?.disconnect();
      previewAudioGainRef.current?.disconnect();
    } catch {
      // ignore
    } finally {
      previewAudioSourceRef.current = null;
      previewAudioGainRef.current = null;
    }
  };

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
    () => JSON.stringify(settings) !== JSON.stringify(savedSettings) || widgetThemeMode !== savedWidgetThemeMode,
    [settings, savedSettings, widgetThemeMode, savedWidgetThemeMode]
  );
  const isDownloadInProgress = useMemo(() => {
    const status = downloadProgress?.status ?? "";
    return status === "starting" || status === "downloading" || status === "canceling";
  }, [downloadProgress]);

  useEffect(() => {
    settingsRef.current = settings;
  }, [settings]);

  useEffect(() => {
    savedSettingsRef.current = savedSettings;
  }, [savedSettings]);

  const refreshModels = async () => {
    try {
      const list = await listModels<ModelInfo[]>();
      setModels(list);
      setModelsError("");
    } catch (e) {
      setModelsError(String(e));
    }
  };

  const modelLabelFromPath = (path: string) => {
    if (!path) return uiText.unknown;
    const normalized = path.split("\\").join("/");
    return normalized.split("/").pop() ?? uiText.unknown;
  };

  const modelFileFromPath = (path: string) => {
    if (!path) return "";
    return path.split("\\").join("/").split("/").pop()?.toLowerCase() ?? "";
  };

  const normalizeSettingsForUi = (loaded: UserSettings): UserSettings => {
    const normalizedOpacity = Math.max(0.25, Math.min(1, loaded.widget_opacity ?? defaultSettings.widget_opacity));
    const normalizedPopSoundVolume = Math.max(
      0,
      Math.min(1, loaded.widget_pop_sound_volume ?? defaultSettings.widget_pop_sound_volume)
    );
    const normalizedPopSound = (loaded.widget_pop_sound || DEFAULT_WIDGET_POP_SOUND).trim() || DEFAULT_WIDGET_POP_SOUND;
    const normalizedTranslationTarget = (loaded.translation_target || "none").trim().toLowerCase() || "none";
    const normalizedInputDeviceId = (loaded.input_device_id || "").trim();
    const normalizedPushToTalkHold = Boolean(loaded.push_to_talk_hold);
    const normalizedSecureTextMode = Boolean(loaded.secure_text_mode);
    const normalizedSilenceGateEnabled =
      typeof loaded.silence_gate_enabled === "boolean" ? loaded.silence_gate_enabled : true;
    return {
      ...loaded,
      keep_model_loaded: false,
      translation_target: normalizedTranslationTarget,
      input_device_id: normalizedInputDeviceId,
      push_to_talk_hold: normalizedPushToTalkHold,
      secure_text_mode: normalizedSecureTextMode,
      silence_gate_enabled: normalizedSilenceGateEnabled,
      widget_opacity: normalizedOpacity,
      widget_pop_sound_volume: normalizedPopSoundVolume,
      widget_pop_sound: normalizedPopSound
    };
  };

  const updateModelMismatchWarning = async (usedModelPath: string) => {
    const usedModelFile = modelFileFromPath(usedModelPath);
    if (!usedModelFile) return;

    try {
      const latestSettings = normalizeSettingsForUi(await getSettings<UserSettings>());
      const selectedModelFile = modelFileFromPath(latestSettings.model_path);
      if (selectedModelFile && selectedModelFile !== usedModelFile) {
        setErrorLine(
          `${uiText.warningSelectedModelMismatchPrefix} (${selectedModelFile}) ${uiText.warningSelectedModelMismatchConnector} (${usedModelFile}).`
        );
      } else {
        setErrorLine("");
      }
      return;
    } catch {
      // fallback below
    }

    const fallbackSelected =
      models.find((model) => model.active)?.filename?.toLowerCase() ?? modelFileFromPath(settings.model_path);
    if (fallbackSelected && fallbackSelected !== usedModelFile) {
      setErrorLine(
        `${uiText.warningSelectedModelMismatchPrefix} (${fallbackSelected}) ${uiText.warningSelectedModelMismatchConnector} (${usedModelFile}).`
      );
    } else {
      setErrorLine("");
    }
  };

  const widgetSoundLabel = (fileName: string) => {
    const match = fileName.toLowerCase().match(/^sound(\d+)\.[a-z0-9]+$/);
    if (match) return `Sound ${match[1]}`;
    return fileName;
  };

  const addHistoryItem = (text: string, sourceWavPath: string, modelPath: string, createdAtMs?: number) => {
    if (settingsRef.current.secure_text_mode) return;
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

  const translateTextIfNeeded = async (sourceText: string, sourceLanguage: string, sourceWavPath?: string) => {
    if (translationTarget === "none" || !sourceText.trim()) {
      setTranslatedText("");
      setTranslationError("");
      setTranslating(false);
      return;
    }

    setTranslating(true);
    setStatusLine(uiText.statusTranslationInProgress);
    try {
      if (translationTarget === "en" && sourceWavPath) {
        const translatedFromWhisper = await translateWavToEnglish<TranscriptionResult>(sourceWavPath);
        const cleanedTranslated = translatedFromWhisper.text?.trim() ?? "";
        if (cleanedTranslated) {
          setTranslatedText(cleanedTranslated);
          setTranslationError("");
          if (!preferSourceTextViewRef.current) {
            setTextView("translated");
          }
          setStatusLine(uiText.statusTranslationDone);
          setTranslating(false);
          return;
        }
      }

      const normalizedSource = sourceLanguage.trim().toLowerCase();
      const sourceLangArg =
        normalizedSource && normalizedSource !== "auto" ? sourceLanguage : undefined;
      const translated = await translateTextCmd(sourceText, translationTarget, sourceLangArg);
      setTranslatedText(translated);
      setTranslationError("");
      if (!preferSourceTextViewRef.current) {
        setTextView("translated");
      }
      setStatusLine(uiText.statusTranslationDone);
    } catch (e) {
      setTranslationError(String(e));
      setTextView("source");
      setStatusLine(uiText.statusTranscriptionDoneNoTranslation);
    } finally {
      setTranslating(false);
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
    return () => {
      const audio = previewAudioRef.current;
      try {
        if (audio) {
          audio.pause();
        }
        disconnectPreviewAudioGraph();
        if (previewAudioContextRef.current && previewAudioContextRef.current.state !== "closed") {
          void previewAudioContextRef.current.close();
        }
        if (dictationCueContextRef.current && dictationCueContextRef.current.state !== "closed") {
          void dictationCueContextRef.current.close();
        }
        if (micMeterAnimationRef.current !== null) {
          cancelAnimationFrame(micMeterAnimationRef.current);
        }
        micMeterStreamRef.current?.getTracks().forEach((track) => track.stop());
        if (micMeterContextRef.current && micMeterContextRef.current.state !== "closed") {
          void micMeterContextRef.current.close();
        }
      } catch {
        // ignore
      }
      previewAudioRef.current = null;
      previewAudioContextRef.current = null;
      dictationCueContextRef.current = null;
      micMeterStreamRef.current = null;
      micMeterContextRef.current = null;
      micMeterAnimationRef.current = null;
    };
  }, []);

  useEffect(() => {
    const onStorage = (event: StorageEvent) => {
      if (event.key === UI_LANGUAGE_STORAGE_KEY) {
        if (event.newValue === "fr" || event.newValue === "en") {
          setUiLanguage(event.newValue);
        }
        return;
      }
      if (event.key === UI_THEME_STORAGE_KEY) {
        if (event.newValue === "light" || event.newValue === "dark") {
          setUiTheme(event.newValue);
        }
        return;
      }
      if (event.key === WIDGET_THEME_STORAGE_KEY) {
        if (event.newValue === "follow" || event.newValue === "light" || event.newValue === "dark") {
          setWidgetThemeMode(event.newValue);
          setSavedWidgetThemeMode(event.newValue);
        }
      }
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  useEffect(() => {
    let unlistenUiLanguage: (() => void) | null = null;
    let unlistenUiTheme: (() => void) | null = null;
    const bootstrap = async () => {
      unlistenUiLanguage = await listenEvent<{ language: UiLanguage }>("ui-language-changed", (payload) => {
        const next = payload.language;
        if (next === "fr" || next === "en") {
          setUiLanguage(next);
        }
      });
      unlistenUiTheme = await listenEvent<{ theme: UiTheme }>("ui-theme-changed", (payload) => {
        const next = payload.theme;
        if (next === "light" || next === "dark") {
          setUiTheme(next);
        }
      });
    };
    void bootstrap();
    return () => {
      if (unlistenUiLanguage) unlistenUiLanguage();
      if (unlistenUiTheme) unlistenUiTheme();
    };
  }, []);

  useEffect(() => {
    localStorage.setItem(UI_LANGUAGE_STORAGE_KEY, uiLanguage);
    void emitEvent("ui-language-changed", { language: uiLanguage });
  }, [uiLanguage]);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", uiTheme);
    localStorage.setItem(UI_THEME_STORAGE_KEY, uiTheme);
    void emitEvent("ui-theme-changed", { theme: uiTheme });
  }, [uiTheme]);

  useEffect(() => {
    localStorage.setItem(WIDGET_THEME_STORAGE_KEY, widgetThemeMode);
    void emitEvent("widget-theme-mode-changed", { mode: widgetThemeMode });
  }, [widgetThemeMode]);

  useEffect(() => {
    let cancelled = false;
    const loadSounds = async () => {
      try {
        const response = await fetch("/sounds/index.json", { cache: "no-store" });
        if (!response.ok) return;
        const list = (await response.json()) as string[];
        const normalized = Array.from(new Set(list.map((s) => String(s).trim()).filter(Boolean)));
        if (!cancelled && normalized.length > 0) {
          setWidgetSoundOptions(normalized);
          setSettings((current) => {
            if (normalized.includes(current.widget_pop_sound)) return current;
            return { ...current, widget_pop_sound: normalized[0] };
          });
        }
      } catch {
        // keep fallback sound list
      }
    };
    void loadSounds();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!historyReady) return;
    if (settings.secure_text_mode) {
      localStorage.removeItem(HISTORY_KEY);
      return;
    }
    localStorage.setItem(HISTORY_KEY, JSON.stringify(historyItems));
  }, [historyItems, historyReady, settings.secure_text_mode]);

  useEffect(() => {
    if (!settings.secure_text_mode) return;
    setHistoryItems([]);
    setWavPath("");
  }, [settings.secure_text_mode]);

  useEffect(() => {
    if (micPermissionRequestedRef.current) return;
    micPermissionRequestedRef.current = true;

    const primeMicPermission = async () => {
      try {
        if (!navigator.mediaDevices?.getUserMedia) return;
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        stream.getTracks().forEach((track) => track.stop());
      } catch {
        // User can deny; capture still works via Rust, only browser meter may stay unavailable.
      }
    };

    void primeMicPermission();
  }, []);

  useEffect(() => {
    const stopMeter = () => {
      if (micMeterAnimationRef.current !== null) {
        cancelAnimationFrame(micMeterAnimationRef.current);
      }
      micMeterAnimationRef.current = null;
      micMeterStreamRef.current?.getTracks().forEach((track) => track.stop());
      micMeterStreamRef.current = null;
      if (micMeterContextRef.current && micMeterContextRef.current.state !== "closed") {
        void micMeterContextRef.current.close();
      }
      micMeterContextRef.current = null;
      setMicLevel(0);
    };

    if (!micMeterActive) {
      stopMeter();
      return;
    }

    let cancelled = false;
    const startMeter = async () => {
      try {
        const mediaDevices = navigator.mediaDevices;
        if (!mediaDevices?.getUserMedia) return;

        const stream = await mediaDevices.getUserMedia({ audio: true });
        if (cancelled) {
          stream.getTracks().forEach((track) => track.stop());
          return;
        }

        const AudioContextCtor =
          window.AudioContext ||
          (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
        if (!AudioContextCtor) {
          stream.getTracks().forEach((track) => track.stop());
          return;
        }

        const context = new AudioContextCtor();
        micMeterContextRef.current = context;
        micMeterStreamRef.current = stream;

        const source = context.createMediaStreamSource(stream);
        const analyser = context.createAnalyser();
        analyser.fftSize = 256;
        analyser.smoothingTimeConstant = 0.8;
        source.connect(analyser);

        const buffer = new Uint8Array(analyser.fftSize);
        let lastPush = 0;
        const tick = (now: number) => {
          if (cancelled) return;
          analyser.getByteTimeDomainData(buffer);
          let sum = 0;
          for (let i = 0; i < buffer.length; i += 1) {
            const normalized = (buffer[i] - 128) / 128;
            sum += normalized * normalized;
          }
          const rms = Math.sqrt(sum / buffer.length);
          const boosted = Math.max(0, Math.min(1, rms * 3.2));
          if (now - lastPush > 50) {
            setMicLevel(boosted);
            lastPush = now;
          }
          micMeterAnimationRef.current = requestAnimationFrame(tick);
        };
        micMeterAnimationRef.current = requestAnimationFrame(tick);
      } catch {
        // If permission is denied or unavailable, keep meter hidden/idle.
      }
    };

    void startMeter();
    return () => {
      cancelled = true;
      stopMeter();
    };
  }, [micMeterActive]);

  useEffect(() => {
    if (!settingsOpen) return;
    void refreshInputDevices();
  }, [settingsOpen]);

  useEffect(() => {
    let cancelled = false;
    const loadVersion = async () => {
      try {
        const version = await getAppVersion();
        if (!cancelled) setAppVersion(version);
      } catch {
        if (!cancelled) setAppVersion("");
      }
    };
    void loadVersion();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const checkUpdates = async () => {
      try {
        let currentVersion = "0.0.0";
        try {
          currentVersion = await getAppVersion();
        } catch {
          // keep fallback
        }
        const response = await fetch(RELEASES_API_URL, { cache: "no-store" });
        if (!response.ok) return;
        const payload = (await response.json()) as { tag_name?: string; html_url?: string };
        const latestTag = (payload.tag_name || "").trim();
        if (!latestTag) return;
        if (isVersionGreater(latestTag, currentVersion) && !cancelled) {
          setUpdateReleaseUrl((payload.html_url || RELEASES_URL).trim() || RELEASES_URL);
        }
      } catch {
        // ignore network errors
      }
    };
    void checkUpdates();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const bootstrap = async () => {
      try {
        const [loadedSettings, env] = await Promise.all([
          getSettings<UserSettings>(),
          testWhisperEnvironment<WhisperEnvironmentReport>()
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
        setTranslationTarget((loadedSettings.translation_target || "none").trim().toLowerCase() || "none");
        setShortcutDraft(loadedSettings.shortcut);
        await Promise.all([refreshModels(), refreshInputDevices()]);
        try {
          const capability = await getComputeCapabilityCmd<ComputeCapabilityReport>();
          setComputeCapability(capability);
        } catch {
          setComputeCapability(null);
        }

        if (!env.ready) setStatusLine(uiText.statusConfigRequired);
      } catch (e) {
        setErrorLine(String(e));
      }

      unlisten = await listenEvent<DictationStatusEvent>("dictation-status", (payload) => {
        const backendNoSpeechDetected = isNoSpeechBackendMessage(payload.message || "");

        if (payload.state === "listening") {
          setRecording(true);
          setWorking(false);
          setMicMeterActive(true);
          setStatusLine(uiText.statusListeningNow);
          return;
        }
        if (payload.state === "transcribing" || payload.state === "busy") {
          setRecording(false);
          setWorking(true);
          setMicMeterActive(false);
          setStatusLine(uiText.statusTranscriptionInProgress);
          return;
        }
        if (payload.state === "done") {
          setRecording(false);
          setWorking(false);
          setMicMeterActive(false);
          if (backendNoSpeechDetected) {
            setStatusLine(uiText.statusNoSpeechDetected);
          } else {
            setStatusLine(uiText.statusTranscriptionDone);
          }
          return;
        }
        if (payload.state === "idle") {
          setRecording(false);
          setWorking(false);
          setMicMeterActive(false);
          setStatusLine(uiText.statusReady);
          return;
        }
        if (payload.state === "error") {
          setRecording(false);
          setWorking(false);
          setMicMeterActive(false);
          setStatusLine(uiText.statusTranscriptionError);
          return;
        }
        setStatusLine(payload.message);
      });
    };

    void bootstrap();
    return () => {
      if (unlisten) unlisten();
    };
  }, [
    uiText.statusConfigRequired,
    uiText.statusListeningNow,
    uiText.statusTranscriptionInProgress,
    uiText.statusTranscriptionDone,
    uiText.statusReady,
    uiText.statusTranscriptionError
  ]);

  useEffect(() => {
    let unlistenTranscript: (() => void) | null = null;

    const bootstrap = async () => {
      unlistenTranscript = await listenEvent<DictationTranscriptEvent>("dictation-transcript", (payload) => {
        setTranscript(payload.text);
        setWavPath(payload.wav_path);
        addHistoryItem(payload.text, payload.wav_path, payload.model_path, payload.created_at_ms);
        if (translationTarget !== "none" || payload.translation_applied) {
          if (payload.translation_applied) {
            const usedTarget = (payload.translation_target || "").trim().toLowerCase();
            if (usedTarget && usedTarget !== "none" && usedTarget !== translationTarget) {
              setTranslationTarget(usedTarget);
            }
            setTranslatedText(payload.injected_text || "");
            setTranslationError("");
            if (!preferSourceTextViewRef.current) {
              setTextView("translated");
            }
            setStatusLine(uiText.statusTranslationDone);
          } else {
            setTranslatedText("");
            if (!preferSourceTextViewRef.current) {
              setTextView("translated");
            }
          }
        } else {
          preferSourceTextViewRef.current = false;
          setTextView("source");
        }
        void updateModelMismatchWarning(payload.model_path);
        if (!payload.translation_applied) {
          void translateTextIfNeeded(payload.text, settings.language, payload.wav_path);
        }
        if (translationTarget === "none") setStatusLine(`${uiText.statusTranscriptionDone} (${modelLabelFromPath(payload.model_path)})`);
      });
    };

    void bootstrap();
    return () => {
      if (unlistenTranscript) unlistenTranscript();
    };
  }, [translationTarget, settings.language, settings.model_path, models, uiText.statusTranslationDone, uiText.statusTranscriptionDone]);

  useEffect(() => {
    let unlistenDownload: (() => void) | null = null;

    const bootstrap = async () => {
      unlistenDownload = await listenEvent<ModelDownloadProgressEvent>("model-download-progress", (payload) => {
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
  }, [capturingShortcut, uiText.statusCaptureCancelled, uiText.statusShortcutDetectedPrefix]);

  const saveSettingsSnapshot = async (next: UserSettings, successLabel = uiText.settingsSaved) => {
    const shortcutFromDraft = shortcutDraft.trim();
    const shortcutFromNext = (next.shortcut || "").trim();
    const effectiveShortcut = shortcutFromNext || shortcutFromDraft;
    const payload: UserSettings = {
      ...next,
      shortcut: effectiveShortcut
    };
    if (!payload.shortcut.trim()) {
      setSettingsState("error");
      setSettingsError(uiText.shortcutEmptyExample);
      return false;
    }
    setSettingsState("saving");
    setSettingsError("");
    try {
      await saveSettingsCmd(payload);
      setSettings(payload);
      setSavedSettings(payload);
      setSavedWidgetThemeMode(widgetThemeMode);
      setShortcutDraft(payload.shortcut);
      setSettingsState("saved");
      setStatusLine(successLabel);
      try {
        const capability = await getComputeCapabilityCmd<ComputeCapabilityReport>();
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

  const mergeBackendSettingsPreservingDraft = (loaded: UserSettings) => {
    const backend = normalizeSettingsForUi(loaded);
    const current = settingsRef.current;
    const saved = savedSettingsRef.current;
    const merged: UserSettings = { ...backend };

    (Object.keys(current) as Array<keyof UserSettings>).forEach((key) => {
      if (current[key] !== saved[key]) {
        (merged as Record<string, string | number | boolean>)[key] = current[key];
      }
    });

    setSavedSettings(backend);
    setSettings(merged);
    setShortcutDraft(merged.shortcut);
    setTranslationTarget((prev) => (prev === merged.translation_target ? prev : merged.translation_target));
  };

  const closeSettingsPanel = () => {
    const stable = savedSettingsRef.current;
    setSettings(stable);
    setWidgetThemeMode(savedWidgetThemeMode);
    setShortcutDraft(stable.shortcut);
    setTranslationTarget(stable.translation_target);
    setSettingsError("");
    setSettingsState("idle");
    setCapturingShortcut(false);
    setSettingsOpen(false);
  };

  const resetSettingsToDefaults = async () => {
    const shouldReset = window.confirm(uiText.confirmResetOptions);
    if (!shouldReset) return;

    let defaultModelPath = settings.model_path;
    let defaultWhisperCliPath = settings.whisper_cli_path;
    try {
      const [modelPath, cliPath] = await Promise.all([getDefaultModelPath(), getDefaultWhisperCliPath()]);
      defaultModelPath = modelPath || defaultModelPath;
      defaultWhisperCliPath = cliPath || defaultWhisperCliPath;
    } catch {
      // keep current paths if defaults are unavailable
    }

    const next: UserSettings = {
      ...settings,
      language: defaultSettings.language,
      translation_target: defaultSettings.translation_target,
      shortcut: defaultSettings.shortcut,
      model_path: defaultModelPath,
      whisper_cli_path: defaultWhisperCliPath,
      input_device_id: defaultSettings.input_device_id,
      push_to_talk_hold: defaultSettings.push_to_talk_hold,
      secure_text_mode: defaultSettings.secure_text_mode,
      silence_gate_enabled: defaultSettings.silence_gate_enabled,
      compute_mode: defaultSettings.compute_mode,
      keep_model_loaded: false,
      widget_enabled: defaultSettings.widget_enabled,
      widget_autohide: defaultSettings.widget_autohide,
      widget_opacity: defaultSettings.widget_opacity,
      widget_pop_sound_volume: defaultSettings.widget_pop_sound_volume,
      widget_pop_sound: defaultSettings.widget_pop_sound,
      voice_commands_enabled: defaultSettings.voice_commands_enabled,
      onboarding_completed: defaultSettings.onboarding_completed
    };
    setCapturingShortcut(false);
    setShortcutDraft(next.shortcut);
    setTranslationTarget(next.translation_target);
    await saveSettingsSnapshot(next, uiText.settingsResetDone);
  };

  const downloadModel = async (modelId: string) => {
    setDownloadingModelId(modelId);
    setModelsError("");
    setDownloadProgress({
      model_id: modelId,
      status: "starting",
      progress_pct: 0,
      downloaded_bytes: 0,
      total_bytes: null,
      message: uiText.downloading
    });
    setStatusLine(uiText.downloading);
    try {
      const message = await downloadModelCmd(modelId);
      const loadedSettings = await getSettings<UserSettings>();
      mergeBackendSettingsPreservingDraft(loadedSettings);
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
      const message = await cancelModelDownloadCmd();
      setStatusLine(message);
    } catch (e) {
      setModelsError(String(e));
    }
  };

  const activateModel = async (modelId: string) => {
    setModelsBusy(true);
    setModelsError("");
    try {
      const message = await setActiveModelCmd(modelId);
      const loadedSettings = await getSettings<UserSettings>();
      mergeBackendSettingsPreservingDraft(loadedSettings);
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
      const message = await deleteModelCmd(modelId);
      const loadedSettings = await getSettings<UserSettings>();
      mergeBackendSettingsPreservingDraft(loadedSettings);
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
      const message = await autoSetupRuntime();
      setStatusLine(message || uiText.statusEngineChecked);
      const loadedSettings = await getSettings<UserSettings>();
      mergeBackendSettingsPreservingDraft(loadedSettings);
      const capability = await getComputeCapabilityCmd<ComputeCapabilityReport>();
      setComputeCapability(capability);
    } catch (e) {
      setSettingsError(String(e));
      setStatusLine(uiText.statusEngineRepairFailed);
    } finally {
      setRuntimeSetupBusy(false);
    }
  };

  const playDictationStartCue = async () => {
    try {
      const AudioContextCtor =
        window.AudioContext ||
        (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
      if (!AudioContextCtor) return;

      const context = dictationCueContextRef.current ?? new AudioContextCtor();
      dictationCueContextRef.current = context;
      if (context.state === "suspended") {
        await context.resume();
      }

      const oscillator = context.createOscillator();
      const gain = context.createGain();
      oscillator.type = "sine";
      oscillator.frequency.value = 880;
      gain.gain.setValueAtTime(0.0001, context.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.06, context.currentTime + 0.015);
      gain.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + 0.12);
      oscillator.connect(gain);
      gain.connect(context.destination);
      oscillator.start();
      oscillator.stop(context.currentTime + 0.13);
    } catch {
      // no-op: status feedback still visible even if audio cue cannot play
    }
  };

  const refreshInputDevices = async () => {
    setInputDevicesBusy(true);
    try {
      const list = await listInputDevices<InputDeviceInfo[]>();
      setInputDevices(Array.isArray(list) ? list : []);
    } catch (e) {
      setSettingsError(String(e));
    } finally {
      setInputDevicesBusy(false);
    }
  };

  const startRecording = async () => {
    if (installedModels.length === 0) {
      setErrorLine(uiText.noModelRequiredHint);
      setStatusLine(uiText.statusNoModelReady);
      setSettingsOpen(true);
      return;
    }

    preferSourceTextViewRef.current = false;
    setErrorLine("");
    setTranscript("");
    setTranslatedText("");
    setTranslationError("");
    setTextView("source");
    setStatusLine(uiText.statusRecordingInProgress);
    try {
      await startCapture();
      setRecording(true);
      setMicMeterActive(true);
      setStatusLine(uiText.statusListeningNow);
      void playDictationStartCue();
    } catch (e) {
      setErrorLine(String(e));
      setMicMeterActive(false);
      setStatusLine(uiText.statusRecordingStartFailed);
    }
  };

  const stopRecordingAndTranscribe = async () => {
    setWorking(true);
    setMicMeterActive(false);
    setErrorLine("");
    setStatusLine(uiText.statusTranscriptionInProgress);
    try {
      const outputPath = await stopCapture();
      setRecording(false);
      setWavPath(outputPath);
      const result = await transcribeWav<TranscriptionResult>(outputPath, activeModelId || null);
      setTranscript(result.text);
      addHistoryItem(result.text, outputPath, result.model_path);
      if (translationTarget !== "none") {
        setTranslatedText("");
        if (!preferSourceTextViewRef.current) {
          setTextView("translated");
        }
      } else {
        preferSourceTextViewRef.current = false;
        setTextView("source");
      }
      await updateModelMismatchWarning(result.model_path);

      if (translationTarget !== "none") {
        void translateTextIfNeeded(result.text, settings.language, outputPath);
      } else {
        setStatusLine(result.text ? `${uiText.statusTranscriptionDone} (${modelLabelFromPath(result.model_path)})` : uiText.statusNoSpeechDetected);
      }

      if (settings.secure_text_mode && translationTarget === "none") {
        try {
          await cleanupHistoryArtifacts([outputPath]);
          setWavPath("");
        } catch {
          // ignore cleanup errors in secure mode flow
        }
      }
    } catch (e) {
      setRecording(false);
      setMicMeterActive(false);
      const message = String(e);
      setErrorLine(message);
      if (message.toLowerCase().includes("modele introuvable")) {
        setStatusLine(uiText.statusNoModelReady);
        setSettingsOpen(true);
      } else {
        setStatusLine(uiText.statusTranscriptionError);
      }
    } finally {
      setWorking(false);
    }
  };

  const currentVisibleText = textView === "translated" ? translatedText : transcript;

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
    return clearHistoryArtifactsCmd(unique);
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

  const copyHistoryItem = async (item: HistoryItem) => {
    try {
      await navigator.clipboard.writeText(item.text);
      setStatusLine(uiText.historyTextCopied);
    } catch {
      setStatusLine(uiText.copyImpossible);
    }
  };

  const quitApplication = async () => {
    const shouldQuit = window.confirm(uiText.confirmQuitApp);
    if (!shouldQuit) return;
    try {
      await quitApplicationCmd();
    } catch (e) {
      setErrorLine(`${uiText.errorQuitAppPrefix}: ${String(e)}`);
    }
  };

  const previewWidgetSound = async () => {
    const selected = (settings.widget_pop_sound || DEFAULT_WIDGET_POP_SOUND).trim() || DEFAULT_WIDGET_POP_SOUND;
    try {
      if (previewAudioRef.current) {
        previewAudioRef.current.pause();
        previewAudioRef.current.currentTime = 0;
      }
      disconnectPreviewAudioGraph();

      const audio = new Audio(`/sounds/${selected}`);
      audio.preload = "auto";
      const desiredVolume = clamp01(settings.widget_pop_sound_volume * getWidgetSoundGain(selected));
      const AudioContextCtor =
        window.AudioContext ||
        (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;

      if (AudioContextCtor) {
        const context = previewAudioContextRef.current ?? new AudioContextCtor();
        previewAudioContextRef.current = context;
        if (context.state === "suspended") {
          await context.resume();
        }
        const source = context.createMediaElementSource(audio);
        const gain = context.createGain();
        gain.gain.value = desiredVolume;
        source.connect(gain);
        gain.connect(context.destination);
        previewAudioSourceRef.current = source;
        previewAudioGainRef.current = gain;
        audio.volume = 1;
      } else {
        audio.volume = desiredVolume;
      }

      previewAudioRef.current = audio;
      setPreviewSoundPlaying(true);
      audio.onended = () => {
        disconnectPreviewAudioGraph();
        setPreviewSoundPlaying(false);
      };
      audio.onerror = () => {
        disconnectPreviewAudioGraph();
        setPreviewSoundPlaying(false);
      };
      await audio.play();
    } catch {
      disconnectPreviewAudioGraph();
      setPreviewSoundPlaying(false);
      setStatusLine(uiText.previewSoundError);
    }
  };

  const handleTranslationTargetChange = (next: string) => {
    setTranslationTarget(next);
    const nextSettings = { ...settings, translation_target: next };
    setSettings(nextSettings);
    setSavedSettings((prev) => ({ ...prev, translation_target: next }));
    void saveSettingsCmd(nextSettings).catch(() => {
      // keep UI responsive even if persistence fails
    });
    if (next === "none") {
      preferSourceTextViewRef.current = false;
      setTranslatedText("");
      setTranslationError("");
      setTextView("source");
    } else if (transcript.trim()) {
      void translateTextIfNeeded(transcript, settings.language, wavPath);
    }
  };

  const showOriginalText = () => {
    preferSourceTextViewRef.current = true;
    setTextView("source");
  };

  const showTranslatedText = () => {
    preferSourceTextViewRef.current = false;
    setTextView("translated");
  };

  const togglePushToTalkHold = async () => {
    const next = { ...settingsRef.current, push_to_talk_hold: !settingsRef.current.push_to_talk_hold };
    setSettings(next);
    try {
      await saveSettingsCmd(next);
      setSavedSettings(next);
    } catch (e) {
      setSettingsError(String(e));
    }
  };

  const openReleasePage = async () => {
    const target = updateReleaseUrl || RELEASES_URL;
    try {
      await openExternalUrl(target);
    } catch {
      window.open(target, "_blank", "noopener,noreferrer");
    }
  };

  const toggleUiTheme = () => {
    setUiTheme((current) => (current === "light" ? "dark" : "light"));
  };

  const disabled = working || settingsState === "saving";

  return {
    uiLanguage,
    setUiLanguage,
    uiTheme,
    setUiTheme,
    toggleUiTheme,
    uiText,
    disabled,
    micLevel,
    micMeterActive,
    appVersion,
    updateReleaseUrl,
    openReleasePage,
    settingsOpen,
    setSettingsOpen,
    quitApplication,
    dictation: {
      model: {
        uiText,
        recording,
        disabled,
        modelsBusy,
        installedModels,
        activeModelId,
        activeModelLabel,
        shortcut: settings.shortcut,
        pushToTalkHold: settings.push_to_talk_hold,
        translationTarget,
        translationOptions,
        statusLine,
        errorLine,
        textView,
        translatedText,
        currentVisibleText,
        translating,
        wavPath,
        translationTargetLabel,
        translationError,
        modelDisplayLabel
      },
      setters: {
        setTextView,
        setTranslatedText,
        setTranscript
      },
      handlers: {
        onStartRecording: () => void startRecording(),
        onStopRecordingAndTranscribe: () => void stopRecordingAndTranscribe(),
        onOpenSettings: () => setSettingsOpen(true),
        onTogglePushToTalkHold: () => void togglePushToTalkHold(),
        onActivateModel: (modelId: string) => void activateModel(modelId),
        onTranslationTargetChange: handleTranslationTargetChange,
        onShowOriginalText: showOriginalText,
        onShowTranslatedText: showTranslatedText,
        onCopyVisibleText: () => void copyVisibleText()
      }
    },
    history: {
      model: {
        uiText,
        historyItems,
        disabled
      },
      handlers: {
        onClearAllHistory: () => void clearAllHistory(),
        onRemoveHistoryItem: (item: HistoryItem) => void removeHistoryItem(item),
        onCopyHistoryItem: (item: HistoryItem) => void copyHistoryItem(item)
      }
    },
    settings: {
      model: {
        open: settingsOpen,
        uiText,
        settings,
        settingsState,
        settingsError,
        settingsDirty,
        shortcutDraft,
        capturingShortcut,
        runtimeSetupBusy,
        inputDevices,
        inputDevicesBusy,
        computeModeOptions,
        computeCapability,
        isDownloadInProgress,
        downloadProgress,
        models,
        modelsError,
        modelsBusy,
        downloadingModelId,
        widgetSoundOptions,
        previewSoundPlaying,
        widgetThemeMode,
        widgetSoundLabel,
        modelDisplayLabel
      },
      setters: {
        setSettings,
        setShortcutDraft,
        setCapturingShortcut,
        setStatusLine
      },
      handlers: {
        onRequestClose: () => void closeSettingsPanel(),
        onSaveSettingsSnapshot: saveSettingsSnapshot,
        onResetSettings: () => void resetSettingsToDefaults(),
        onRepairRuntime: () => void repairRuntime(),
        onRefreshInputDevices: () => void refreshInputDevices(),
        onCancelModelDownload: () => void cancelModelDownload(),
        onWidgetSoundChange: (soundFile: string) =>
          setSettings((current) => ({ ...current, widget_pop_sound: soundFile })),
        onWidgetThemeModeChange: (mode: WidgetThemeMode) => setWidgetThemeMode(mode),
        onWidgetSoundVolumeChange: (volume: number) =>
          setSettings((current) => ({ ...current, widget_pop_sound_volume: clamp01(volume) })),
        onWidgetOpacityChange: (opacity: number) =>
          setSettings((current) => ({ ...current, widget_opacity: Math.max(0.25, Math.min(1, opacity)) })),
        onPreviewWidgetSound: () => void previewWidgetSound(),
        onDownloadModel: (modelId: string) => void downloadModel(modelId),
        onRemoveModel: (modelId: string) => void removeModel(modelId)
      }
    }
  };
}
