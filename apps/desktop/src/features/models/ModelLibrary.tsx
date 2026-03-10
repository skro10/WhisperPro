import { UI_TEXT } from "../../i18n";
import type { ModelInfo } from "../shared/types";

type ModelLibraryProps = {
  uiText: (typeof UI_TEXT)["fr"];
  modelPath: string;
  models: ModelInfo[];
  modelsError: string;
  modelsBusy: boolean;
  isDownloadInProgress: boolean;
  downloadingModelId: string | null;
  modelDisplayLabel: (model: { id: string; label: string }) => string;
  onDownloadModel: (modelId: string) => void;
  onRemoveModel: (modelId: string) => void;
};

export default function ModelLibrary({
  uiText,
  modelPath,
  models,
  modelsError,
  modelsBusy,
  isDownloadInProgress,
  downloadingModelId,
  modelDisplayLabel,
  onDownloadModel,
  onRemoveModel
}: ModelLibraryProps) {
  return (
    <div className="models-box">
      <div className="result-header">
        <h3>{uiText.library}</h3>
      </div>
      <p className="meta">{uiText.activeModelPath}: {modelPath || uiText.noneValue}</p>
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
                    onClick={() => onDownloadModel(model.id)}
                    disabled={modelsBusy || isDownloadInProgress}
                    title={uiText.tipDownloadModel}
                  >
                    {downloadingModelId === model.id && isDownloadInProgress ? uiText.downloading : uiText.download}
                  </button>
                ) : null}
                {model.installed ? (
                  <button
                    type="button"
                    onClick={() => onRemoveModel(model.id)}
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
  );
}
