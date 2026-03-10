import { useEffect, useMemo, useRef, useState } from "react";

import { UI_LANGUAGE_STORAGE_KEY, UI_TEXT, type UiLanguage } from "../../i18n";
import {
  autoSetupRuntime,
  cancelModelDownload as cancelModelDownloadCmd,
  clearHistoryArtifacts as clearHistoryArtifactsCmd,
  deleteModel as deleteModelCmd,
  downloadModel as downloadModelCmd,
  getComputeCapability as getComputeCapabilityCmd,
  getSettings,
  listModels,
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
  ModelDownloadProgressEvent,
  ModelInfo,
  TranscriptionResult,
  UserSettings,
  WhisperEnvironmentReport
} from "../shared/types";

const clamp01 = (value: number) => Math.max(0, Math.min(1, value));

const getWidgetSoundGain = (fileName: string) => {
  const key = (fileName || "").trim().toLowerCase();
  return WIDGET_SOUND_GAIN[key] ?? 1.0;
};

export function useMainAppController() {
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
  const [translating, setTranslating] = useState(false);
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
  const [widgetSoundOptions, setWidgetSoundOptions] = useState<string[]>([DEFAULT_WIDGET_POP_SOUND]);
  const [previewSoundPlaying, setPreviewSoundPlaying] = useState(false);
  const previewAudioRef = useRef<HTMLAudioElement | null>(null);
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
    const normalizedPopSoundVolume = Math.max(0, Math.min(1, loaded.widget_pop_sound_volume ?? defaultSettings.widget_pop_sound_volume));
    const normalizedPopSound = (loaded.widget_pop_sound || DEFAULT_WIDGET_POP_SOUND).trim() || DEFAULT_WIDGET_POP_SOUND;
    const normalizedTranslationTarget = (loaded.translation_target || "none").trim().toLowerCase() || "none";
    return {
      ...loaded,
      keep_model_loaded: false,
      translation_target: normalizedTranslationTarget,
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
          setTextView("translated");
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
      setTextView("translated");
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
      if (!audio) return;
      try {
        audio.pause();
      } catch {
        // ignore
      }
      previewAudioRef.current = null;
    };
  }, []);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(UI_LANGUAGE_STORAGE_KEY);
      if (raw === "fr" || raw === "en") setUiLanguage(raw);
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
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
      unlistenUiLanguage = await listenEvent<{ language: UiLanguage }>("ui-language-changed", (payload) => {
        const next = payload.language;
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
    void emitEvent("ui-language-changed", { language: uiLanguage });
  }, [uiLanguage]);

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
    localStorage.setItem(HISTORY_KEY, JSON.stringify(historyItems));
  }, [historyItems, historyReady]);

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
        await refreshModels();
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
        if (!working && !recording && !translating) setStatusLine(payload.message);
      });
    };

    void bootstrap();
    return () => {
      if (unlisten) unlisten();
    };
  }, [recording, working, translating, uiText.statusConfigRequired]);

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
            setTextView("translated");
            setStatusLine(uiText.statusTranslationDone);
          } else {
            setTranslatedText("");
            setTextView("translated");
          }
        } else {
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
      await saveSettingsCmd(payload);
      setSettings(payload);
      setSavedSettings(payload);
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
      widget_pop_sound_volume: defaultSettings.widget_pop_sound_volume,
      widget_pop_sound: defaultSettings.widget_pop_sound,
      voice_commands_enabled: defaultSettings.voice_commands_enabled
    };
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
      const message = await deleteModelCmd(modelId);
      const loadedSettings = await getSettings<UserSettings>();
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
      const message = await autoSetupRuntime();
      setStatusLine(message || uiText.statusEngineChecked);
      const loadedSettings = await getSettings<UserSettings>();
      const normalized = normalizeSettingsForUi(loadedSettings);
      setSettings(normalized);
      setSavedSettings(normalized);
      setShortcutDraft(loadedSettings.shortcut);
      const capability = await getComputeCapabilityCmd<ComputeCapabilityReport>();
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
      await startCapture();
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
      const outputPath = await stopCapture();
      setRecording(false);
      setWavPath(outputPath);
      const result = await transcribeWav<TranscriptionResult>(outputPath, activeModelId || null);
      setTranscript(result.text);
      addHistoryItem(result.text, outputPath, result.model_path);
      if (translationTarget !== "none") {
        setTranslatedText("");
        setTextView("translated");
      } else {
        setTextView("source");
      }
      await updateModelMismatchWarning(result.model_path);

      if (translationTarget !== "none") {
        void translateTextIfNeeded(result.text, settings.language, outputPath);
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
      const audio = new Audio(`/sounds/${selected}`);
      audio.preload = "auto";
      audio.volume = clamp01(settings.widget_pop_sound_volume * getWidgetSoundGain(selected));
      previewAudioRef.current = audio;
      setPreviewSoundPlaying(true);
      audio.onended = () => setPreviewSoundPlaying(false);
      audio.onerror = () => setPreviewSoundPlaying(false);
      await audio.play();
    } catch {
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
      setTranslatedText("");
      setTranslationError("");
      setTextView("source");
    } else if (transcript.trim()) {
      void translateTextIfNeeded(transcript, settings.language, wavPath);
    }
  };

  const disabled = working || settingsState === "saving";

  return {
    uiLanguage,
    setUiLanguage,
    uiText,
    disabled,
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
        onActivateModel: (modelId: string) => void activateModel(modelId),
        onTranslationTargetChange: handleTranslationTargetChange,
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
        onCancelModelDownload: () => void cancelModelDownload(),
        onPreviewWidgetSound: () => void previewWidgetSound(),
        onDownloadModel: (modelId: string) => void downloadModel(modelId),
        onRemoveModel: (modelId: string) => void removeModel(modelId)
      }
    }
  };
}
