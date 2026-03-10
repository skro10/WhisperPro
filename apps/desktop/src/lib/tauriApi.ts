import { invoke } from "@tauri-apps/api/core";

export function startCapture(): Promise<string> {
  return invoke<string>("start_capture");
}

export function stopCapture(): Promise<string> {
  return invoke<string>("stop_capture");
}

export function transcribeWav<T>(wavPath: string, modelId: string | null): Promise<T> {
  return invoke<T>("transcribe_wav", { wavPath, modelId });
}

export function translateWavToEnglish<T>(wavPath: string): Promise<T> {
  return invoke<T>("translate_wav_to_english", { wavPath });
}

export function translateText(
  text: string,
  targetLang: string,
  sourceLang?: string
): Promise<string> {
  const payload: { text: string; targetLang: string; sourceLang?: string } = {
    text,
    targetLang
  };
  if (sourceLang) payload.sourceLang = sourceLang;
  return invoke<string>("translate_text", payload);
}

export function getSettings<T>(): Promise<T> {
  return invoke<T>("get_settings");
}

export function saveSettings<S>(settings: S): Promise<void> {
  return invoke<void>("save_settings", { settings });
}

export function listModels<T>(): Promise<T> {
  return invoke<T>("list_models");
}

export function downloadModel(modelId: string): Promise<string> {
  return invoke<string>("download_model", { modelId });
}

export function cancelModelDownload(): Promise<string> {
  return invoke<string>("cancel_model_download");
}

export function setActiveModel(modelId: string): Promise<string> {
  return invoke<string>("set_active_model", { modelId });
}

export function deleteModel(modelId: string): Promise<string> {
  return invoke<string>("delete_model", { modelId });
}

export function getComputeCapability<T>(): Promise<T> {
  return invoke<T>("get_compute_capability");
}

export function autoSetupRuntime(): Promise<string> {
  return invoke<string>("auto_setup_runtime");
}

export function testWhisperEnvironment<T>(): Promise<T> {
  return invoke<T>("test_whisper_environment");
}

export function getDictationStatus<T>(): Promise<T> {
  return invoke<T>("get_dictation_status");
}

export function startOverlayDrag(): Promise<void> {
  return invoke<void>("start_overlay_drag");
}

export async function clearHistoryArtifacts(wavPaths: string[]): Promise<string> {
  try {
    return await invoke<string>("clear_history_artifacts", { payload: { wavPaths } });
  } catch (firstError) {
    try {
      return await invoke<string>("clear_history_artifacts", { wavPaths });
    } catch {
      try {
        return await invoke<string>("clear_history_artifacts", { wav_paths: wavPaths });
      } catch {
        throw firstError;
      }
    }
  }
}

export function quitApplication(): Promise<void> {
  return invoke<void>("quit_application");
}
