import { LANGUAGE_OPTIONS } from "../shared/constants";
import type { UserSettings } from "../shared/types";
import type {
  SettingsDrawerHandlers,
  SettingsDrawerModel,
  SettingsDrawerSetters
} from "../shared/panelContracts";
import ModelLibrary from "../models/ModelLibrary";

type SettingsDrawerProps = {
  model: SettingsDrawerModel;
  setters: SettingsDrawerSetters;
  handlers: SettingsDrawerHandlers;
};

export default function SettingsDrawer({
  model,
  setters,
  handlers
}: SettingsDrawerProps) {
  const {
    open,
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
  } = model;
  const { setSettings, setShortcutDraft, setCapturingShortcut, setStatusLine } = setters;
  const {
    onRequestClose,
    onSaveSettingsSnapshot,
    onResetSettings,
    onRepairRuntime,
    onCancelModelDownload,
    onPreviewWidgetSound,
    onDownloadModel,
    onRemoveModel
  } = handlers;

  if (!open) return null;

  return (
    <div className="overlay" onClick={onRequestClose}>
      <section className="panel settings-drawer" onClick={(e) => e.stopPropagation()}>
        <div className="result-header settings-header">
          <h2>{uiText.options}</h2>
          <div className="inline-actions">
            <button
              type="button"
              className="primary"
              onClick={() => void onSaveSettingsSnapshot({ ...settings, shortcut: shortcutDraft.trim() || settings.shortcut })}
              disabled={settingsState === "saving"}
            >
              {settingsState === "saving" ? uiText.saving : uiText.save}
            </button>
            <button type="button" className="secondary" onClick={onResetSettings} disabled={settingsState === "saving"}>
              {uiText.reset}
            </button>
            <button type="button" className="ghost" onClick={onRequestClose}>
              {uiText.close}
            </button>
          </div>
        </div>
        {isDownloadInProgress ? (
          <div className="progress-box settings-progress settings-progress-sticky">
            <div className="inline-actions progress-header">
              <p className="history-time">{downloadProgress?.message ?? uiText.downloading}</p>
              <button type="button" className="secondary" onClick={onCancelModelDownload}>
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
            <button type="button" className="ghost" onClick={onRepairRuntime} disabled={runtimeSetupBusy}>
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
                void onSaveSettingsSnapshot(next, e.target.checked ? "Mini-widget activé" : "Mini-widget masqué");
              }}
              disabled={settingsState === "saving"}
            />
          </label>

          <label className="field">
            <span>
              {uiText.widgetPopSound}
              <button
                type="button"
                className="info-dot"
                title={uiText.tipWidgetPopSound}
                aria-label={uiText.ariaHelpWidgetPopSound}
              >
                ?
              </button>
            </span>
            <select
              value={settings.widget_pop_sound}
              onChange={(e) => setSettings((s) => ({ ...s, widget_pop_sound: e.target.value }))}
            >
              {widgetSoundOptions.map((soundFile) => (
                <option key={soundFile} value={soundFile}>
                  {widgetSoundLabel(soundFile)}
                </option>
              ))}
            </select>
            <div className="inline-actions">
              <button type="button" className="ghost" onClick={onPreviewWidgetSound} disabled={previewSoundPlaying}>
                {previewSoundPlaying ? uiText.previewingSound : uiText.previewSound}
              </button>
            </div>
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

          <label className="field">
            <span>
              {uiText.widgetPopSoundVolume} ({Math.round(settings.widget_pop_sound_volume * 100)}%)
              <button
                type="button"
                className="info-dot"
                title={uiText.tipWidgetPopSoundVolume}
                aria-label={uiText.ariaHelpWidgetPopSoundVolume}
              >
                ?
              </button>
            </span>
            <input
              type="range"
              min={0}
              max={1}
              step={0.05}
              value={settings.widget_pop_sound_volume}
              onChange={(e) => setSettings((s) => ({ ...s, widget_pop_sound_volume: Number(e.target.value) }))}
            />
            <p className="meta">{uiText.widgetPopSoundVolumeHint}</p>
          </label>
        </section>

        <section className="settings-group">
          <h3 className="settings-group-title">{uiText.sectionModels}</h3>
          <ModelLibrary
            uiText={uiText}
            modelPath={settings.model_path}
            models={models}
            modelsError={modelsError}
            modelsBusy={modelsBusy}
            isDownloadInProgress={isDownloadInProgress}
            downloadingModelId={downloadingModelId}
            modelDisplayLabel={modelDisplayLabel}
            onDownloadModel={onDownloadModel}
            onRemoveModel={onRemoveModel}
          />
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
  );
}
