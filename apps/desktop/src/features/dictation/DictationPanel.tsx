import type {
  DictationPanelHandlers,
  DictationPanelModel,
  DictationPanelSetters
} from "../shared/panelContracts";

type DictationPanelProps = {
  model: DictationPanelModel;
  setters: DictationPanelSetters;
  handlers: DictationPanelHandlers;
};

export default function DictationPanel({
  model,
  setters,
  handlers
}: DictationPanelProps) {
  const {
    uiText,
    recording,
    disabled,
    modelsBusy,
    installedModels,
    activeModelId,
    activeModelLabel,
    shortcut,
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
  } = model;
  const { setTranslatedText, setTranscript } = setters;
  const {
    onStartRecording,
    onStopRecordingAndTranscribe,
    onActivateModel,
    onTranslationTargetChange,
    onShowOriginalText,
    onShowTranslatedText,
    onCopyVisibleText
  } = handlers;

  return (
    <section className="panel main-panel">
      <section className="hero-actions">
        <div className="cta-row controls-primary">
          {!recording ? (
            <button type="button" className="primary big" onClick={onStartRecording} disabled={disabled}>
              {uiText.startSpeaking}
            </button>
          ) : (
            <button type="button" className="danger big" onClick={onStopRecordingAndTranscribe} disabled={disabled}>
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
                if (nextId) onActivateModel(nextId);
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
            <div className="chip">{uiText.shortcut}: {shortcut}</div>
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
              onChange={(e) => onTranslationTargetChange(e.target.value)}
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
                <button type="button" className={textView === "source" ? "ghost" : "secondary"} onClick={onShowOriginalText}>
                  {uiText.original}
                </button>
                <button
                  type="button"
                  className={textView === "translated" ? "ghost" : "secondary"}
                  onClick={onShowTranslatedText}
                  disabled={!translatedText}
                >
                  {uiText.translated}
                </button>
              </>
            ) : null}
            <button type="button" onClick={onCopyVisibleText} disabled={!currentVisibleText || disabled || translating}>
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
  );
}
