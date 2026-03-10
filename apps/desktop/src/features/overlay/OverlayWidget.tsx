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
  const stateRef = useRef<string>("idle");
  const widgetPopSoundRef = useRef<string>(defaultSettings.widget_pop_sound);
  const widgetPopSoundVolumeRef = useRef<number>(defaultSettings.widget_pop_sound_volume);
  const popAudioRef = useRef<HTMLAudioElement | null>(null);
  const popAudioContextRef = useRef<AudioContext | null>(null);
  const popAudioSourceRef = useRef<MediaElementAudioSourceNode | null>(null);
  const popAudioGainRef = useRef<GainNode | null>(null);
  const isPoppingSoundRef = useRef(false);
  const disconnectPopAudioGraph = () => {
    try {
      popAudioSourceRef.current?.disconnect();
      popAudioGainRef.current?.disconnect();
    } catch {
      // ignore
    } finally {
      popAudioSourceRef.current = null;
      popAudioGainRef.current = null;
    }
  };
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  useEffect(() => {
    widgetPopSoundRef.current = widgetPopSound;
  }, [widgetPopSound]);

  useEffect(() => {
    widgetPopSoundVolumeRef.current = widgetPopSoundVolume;
  }, [widgetPopSoundVolume]);

  useEffect(() => {
    return () => {
      if (popAudioRef.current) {
        try {
          popAudioRef.current.pause();
        } catch {
          // ignore
        }
      }
      disconnectPopAudioGraph();
      if (popAudioContextRef.current && popAudioContextRef.current.state !== "closed") {
        void popAudioContextRef.current.close();
      }
      popAudioRef.current = null;
      popAudioContextRef.current = null;
    };
  }, [widgetPopSound]);

  const playWidgetPopSound = () => {
    const soundFile = widgetPopSoundRef.current;
    const soundVolume = widgetPopSoundVolumeRef.current;
    if (soundVolume <= 0) return;
    if (isPoppingSoundRef.current) return;
    isPoppingSoundRef.current = true;
    const play = async () => {
      try {
        if (popAudioRef.current) {
          popAudioRef.current.pause();
          popAudioRef.current.currentTime = 0;
        }
        disconnectPopAudioGraph();

        const audio = new Audio(`/sounds/${soundFile}`);
        audio.preload = "auto";
        const desiredVolume = clamp01(soundVolume * getWidgetSoundGain(soundFile));
        const AudioContextCtor =
          window.AudioContext ||
          (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;

        if (AudioContextCtor) {
          const context = popAudioContextRef.current ?? new AudioContextCtor();
          popAudioContextRef.current = context;
          if (context.state === "suspended") {
            await context.resume();
          }
          const source = context.createMediaElementSource(audio);
          const gain = context.createGain();
          gain.gain.value = desiredVolume;
          source.connect(gain);
          gain.connect(context.destination);
          popAudioSourceRef.current = source;
          popAudioGainRef.current = gain;
          audio.volume = 1;
        } else {
          audio.volume = desiredVolume;
        }

        popAudioRef.current = audio;
        audio.onended = () => {
          disconnectPopAudioGraph();
          isPoppingSoundRef.current = false;
        };
        audio.onerror = () => {
          disconnectPopAudioGraph();
          isPoppingSoundRef.current = false;
        };
        await audio.play();
      } catch {
        disconnectPopAudioGraph();
        isPoppingSoundRef.current = false;
      }
    };
    void play();
  };

  const applyWidgetState = (nextState: string) => {
    const isActive = nextState === "listening" || nextState === "transcribing" || nextState === "busy";
    if (isActive) {
      const wasActive =
        stateRef.current === "listening" || stateRef.current === "transcribing" || stateRef.current === "busy";
      if (!wasActive) {
        playWidgetPopSound();
      }
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
        const loadedSettings = await getSettings<UserSettings>();
        setWidgetOpacity(Math.max(0.25, Math.min(1, loadedSettings.widget_opacity ?? defaultSettings.widget_opacity)));
        setWidgetPopSoundVolume(Math.max(0, Math.min(1, loadedSettings.widget_pop_sound_volume ?? defaultSettings.widget_pop_sound_volume)));
        setWidgetPopSound((loadedSettings.widget_pop_sound || DEFAULT_WIDGET_POP_SOUND).trim() || DEFAULT_WIDGET_POP_SOUND);
        const status = await getDictationStatus<DictationStatusEvent>();
        applyWidgetState(status.state);
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
