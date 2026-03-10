import { useMemo, useState } from "react";
import type { WidgetThemeMode } from "../../i18n";

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

type SettingsSection = "general" | "widget" | "models" | "advanced";

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
  } = model;
  const { setSettings, setShortcutDraft, setCapturingShortcut, setStatusLine } = setters;
  const {
    onRequestClose,
    onSaveSettingsSnapshot,
    onResetSettings,
    onRepairRuntime,
    onRefreshInputDevices,
    onCancelModelDownload,
    onWidgetSoundChange,
    onWidgetThemeModeChange,
    onWidgetSoundVolumeChange,
    onWidgetOpacityChange,
    onPreviewWidgetSound,
    onDownloadModel,
    onRemoveModel
  } = handlers;
  const [activeSection, setActiveSection] = useState<SettingsSection>("general");

  const sectionTabs = useMemo(
    () =>
      [
        { id: "general" as const, label: uiText.sectionGeneral },
        { id: "widget" as const, label: uiText.sectionWidget },
        { id: "models" as const, label: uiText.sectionModels },
        { id: "advanced" as const, label: uiText.sectionAdvanced }
      ] satisfies Array<{ id: SettingsSection; label: string }>,
    [uiText]
  );

  if (!open) return null;

  return (
    <div className="overlay settings-overlay" onClick={onRequestClose}>
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
            <button type="button" className="danger" onClick={onResetSettings} disabled={settingsState === "saving"}>
              {uiText.reset}
            </button>
            <button type="button" className="ghost" onClick={onRequestClose}>
              {uiText.close}
            </button>
          </div>
        </div>

        <div className="settings-notice-stack">
          {isDownloadInProgress ? (
            <div className="progress-box settings-progress">
              <div className="inline-actions progress-header">
                <p className="history-time">{downloadProgress?.message ?? uiText.downloading}</p>
                <button type="button" className="secondary" onClick={onCancelModelDownload}>
                  {uiText.cancel}
                </button>
              </div>
              <div className="progress-track">
                <span
                  className="progress-fill"
                  style={{ width: `${Math.min(100, Math.max(2, downloadProgress?.progress_pct ?? 0))}%` }}
                />
              </div>
            </div>
          ) : null}
          {settingsDirty ? <p className="settings-warning">{uiText.unsavedChanges}</p> : null}
          {settingsError ? <p className="error settings-error">{settingsError}</p> : null}
        </div>

        <div className="settings-tabs" role="tablist" aria-label={uiText.options}>
          {sectionTabs.map((tab) => (
            <button
              key={tab.id}
              type="button"
              role="tab"
              aria-selected={activeSection === tab.id}
              className={`settings-tab ${activeSection === tab.id ? "active" : ""}`}
              onClick={() => setActiveSection(tab.id)}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className="settings-content">
          {activeSection === "general" ? (
            <section className="settings-group settings-pane" role="tabpanel">
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
                  {uiText.inputDevice}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipInputDevice}
                    aria-label={uiText.ariaHelpInputDevice}
                  >
                    ?
                  </button>
                </span>
                <div className="inline-actions">
                  <select
                    value={settings.input_device_id}
                    onChange={(e) => setSettings((s) => ({ ...s, input_device_id: e.target.value }))}
                    disabled={inputDevicesBusy}
                  >
                    <option value="">{uiText.inputDeviceDefault}</option>
                    {inputDevices.map((device) => (
                      <option key={device.id} value={device.id}>
                        {device.name}
                        {device.is_default ? ` (${uiText.inputDeviceDefault})` : ""}
                      </option>
                    ))}
                    {settings.input_device_id && !inputDevices.some((d) => d.id === settings.input_device_id) ? (
                      <option value={settings.input_device_id}>
                        {uiText.inputDeviceUnavailable}
                      </option>
                    ) : null}
                  </select>
                  <button
                    type="button"
                    className="ghost"
                    onClick={onRefreshInputDevices}
                    disabled={inputDevicesBusy}
                  >
                    {uiText.refreshInputDevices}
                  </button>
                </div>
              </label>

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
              </label>

              <label className="field checkbox compact">
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

              <label className="field checkbox compact">
                <span>
                  {uiText.pushToTalkHold}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipPushToTalkHold}
                    aria-label={uiText.ariaHelpPushToTalkHold}
                  >
                    ?
                  </button>
                </span>
                <input
                  type="checkbox"
                  checked={settings.push_to_talk_hold}
                  onChange={(e) => setSettings((s) => ({ ...s, push_to_talk_hold: e.target.checked }))}
                />
              </label>

              <label className="field checkbox compact">
                <span>
                  {uiText.secureTextMode}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipSecureTextMode}
                    aria-label={uiText.ariaHelpSecureTextMode}
                  >
                    ?
                  </button>
                </span>
                <input
                  type="checkbox"
                  checked={settings.secure_text_mode}
                  onChange={(e) => setSettings((s) => ({ ...s, secure_text_mode: e.target.checked }))}
                />
              </label>

              <label className="field checkbox compact">
                <span>
                  {uiText.silenceGate}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipSilenceGate}
                    aria-label={uiText.ariaHelpSilenceGate}
                  >
                    ?
                  </button>
                </span>
                <input
                  type="checkbox"
                  checked={settings.silence_gate_enabled}
                  onChange={(e) => setSettings((s) => ({ ...s, silence_gate_enabled: e.target.checked }))}
                />
              </label>

              <div className="settings-inline-actions">
                {computeCapability ? (
                  <p className="meta">
                    {computeCapability.gpu_available ? uiText.gpuDetected : uiText.gpuNotDetected} {computeCapability.details}
                  </p>
                ) : null}
                <button type="button" className="ghost" onClick={onRepairRuntime} disabled={runtimeSetupBusy}>
                  {runtimeSetupBusy ? uiText.checkRuntime : uiText.repairAcceleration}
                </button>
              </div>
            </section>
          ) : null}

          {activeSection === "widget" ? (
            <section className="settings-group settings-pane" role="tabpanel">
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
                    void onSaveSettingsSnapshot(next, e.target.checked ? "Mini-widget active" : "Mini-widget hidden");
                  }}
                  disabled={settingsState === "saving"}
                />
              </label>

              <label className="field">
                <span>
                  {uiText.widgetTheme}
                  <button
                    type="button"
                    className="info-dot"
                    title={uiText.tipWidgetTheme}
                    aria-label={uiText.ariaHelpWidgetTheme}
                  >
                    ?
                  </button>
                </span>
                <select value={widgetThemeMode} onChange={(e) => onWidgetThemeModeChange(e.target.value as WidgetThemeMode)}>
                  <option value="follow">{uiText.widgetThemeFollowApp}</option>
                  <option value="light">{uiText.widgetThemeLight}</option>
                  <option value="dark">{uiText.widgetThemeDark}</option>
                </select>
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
                <select value={settings.widget_pop_sound} onChange={(e) => onWidgetSoundChange(e.target.value)}>
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
                  onChange={(e) => onWidgetOpacityChange(Number(e.target.value))}
                />
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
                  onChange={(e) => onWidgetSoundVolumeChange(Number(e.target.value))}
                />
              </label>
            </section>
          ) : null}

          {activeSection === "models" ? (
            <section className="settings-group settings-pane" role="tabpanel">
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
          ) : null}

          {activeSection === "advanced" ? (
            <section className="settings-group settings-pane" role="tabpanel">
              <details open>
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
                  <input
                    value={settings.whisper_cli_path}
                    onChange={(e) => setSettings((s) => ({ ...s, whisper_cli_path: e.target.value }))}
                  />
                </label>
              </details>
            </section>
          ) : null}
        </div>
      </section>
    </div>
  );
}
