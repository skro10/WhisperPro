import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { UI_LANGUAGE_STORAGE_KEY, UI_TEXT, type UiLanguage } from "../../i18n";
import { getDictationStatus, getSettings, startOverlayDrag } from "../../lib/tauriApi";
import { listenEvent } from "../../lib/tauriEvents";
import {
  DEFAULT_WIDGET_POP_SOUND,
  WIDGET_SOUND_GAIN,
  defaultSettings
} from "../shared/constants";
import type { DictationStatusEvent, UserSettings } from "../shared/types";

const clamp01 = (value: number) => Math.max(0, Math.min(1, value));

const getWidgetSoundGain = (fileName: string) => {
  const key = (fileName || "").trim().toLowerCase();
  return WIDGET_SOUND_GAIN[key] ?? 1.0;
};

export default function OverlayWidget() {
  const [uiLanguage, setUiLanguage] = useState<UiLanguage>("fr");
  const [state, setState] = useState<string>("idle");
  const [widgetOpacity, setWidgetOpacity] = useState(defaultSettings.widget_opacity);
  const [widgetPopSoundVolume, setWidgetPopSoundVolume] = useState(defaultSettings.widget_pop_sound_volume);
  const [widgetPopSound, setWidgetPopSound] = useState(defaultSettings.widget_pop_sound);
  const uiText = UI_TEXT[uiLanguage];
  const terminalStateAtRef = useRef<number>(0);
  const terminalLockUntilRef = useRef<number>(0);
  const stateRef = useRef<string>("idle");
  const popAudioRef = useRef<HTMLAudioElement | null>(null);
  const isPoppingSoundRef = useRef(false);
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  useEffect(() => {
    const audio = new Audio(`/sounds/${widgetPopSound}`);
    audio.preload = "auto";
    popAudioRef.current = audio;
    return () => {
      popAudioRef.current = null;
    };
  }, [widgetPopSound]);

  const playWidgetPopSound = () => {
    if (widgetPopSoundVolume <= 0) return;
    const audio = popAudioRef.current;
    if (!audio || isPoppingSoundRef.current) return;
    isPoppingSoundRef.current = true;
    try {
      audio.pause();
      audio.currentTime = 0;
      audio.volume = clamp01(widgetPopSoundVolume * getWidgetSoundGain(widgetPopSound));
      void audio.play().catch(() => {
        // no sound asset yet or playback blocked by platform policy
      }).finally(() => {
        isPoppingSoundRef.current = false;
      });
    } catch {
      isPoppingSoundRef.current = false;
    }
  };

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
      const wasActive =
        stateRef.current === "listening" || stateRef.current === "transcribing" || stateRef.current === "busy";
      if (!wasActive) {
        playWidgetPopSound();
      }
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
          getDictationStatus<DictationStatusEvent>(),
          getSettings<UserSettings>()
        ]);
        applyWidgetState(status.state);
        setWidgetOpacity(Math.max(0.25, Math.min(1, loadedSettings.widget_opacity ?? defaultSettings.widget_opacity)));
        setWidgetPopSoundVolume(Math.max(0, Math.min(1, loadedSettings.widget_pop_sound_volume ?? defaultSettings.widget_pop_sound_volume)));
        setWidgetPopSound((loadedSettings.widget_pop_sound || DEFAULT_WIDGET_POP_SOUND).trim() || DEFAULT_WIDGET_POP_SOUND);
      } catch {
        // ignore
      }

      unlisten = await listenEvent<DictationStatusEvent>("dictation-status", (payload) => {
        applyWidgetState(payload.state);
      });

      const appWindow = getCurrentWindow();
      unlistenMoved = await appWindow.onMoved(() => {
        // overlay is cursor-anchored, no manual position persistence
      });

      unlistenSettings = await listenEvent<UserSettings>("settings-updated", (payload) => {
        setWidgetOpacity(Math.max(0.25, Math.min(1, payload.widget_opacity ?? defaultSettings.widget_opacity)));
        setWidgetPopSoundVolume(
          Math.max(0, Math.min(1, payload.widget_pop_sound_volume ?? defaultSettings.widget_pop_sound_volume))
        );
        setWidgetPopSound((payload.widget_pop_sound || DEFAULT_WIDGET_POP_SOUND).trim() || DEFAULT_WIDGET_POP_SOUND);
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
          void startOverlayDrag();
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
