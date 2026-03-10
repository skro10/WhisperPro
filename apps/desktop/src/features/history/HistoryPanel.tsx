import type { HistoryPanelHandlers, HistoryPanelModel } from "../shared/panelContracts";

type HistoryPanelProps = {
  model: HistoryPanelModel;
  handlers: HistoryPanelHandlers;
};

export default function HistoryPanel({
  model,
  handlers
}: HistoryPanelProps) {
  const { uiText, historyItems, disabled } = model;
  const { onClearAllHistory, onRemoveHistoryItem, onCopyHistoryItem } = handlers;

  return (
    <aside className="panel history-panel">
      <div className="result-header">
        <h2>{uiText.history}</h2>
        <button type="button" className="ghost" disabled={historyItems.length === 0 || disabled} onClick={onClearAllHistory}>
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
                <button type="button" onClick={() => onCopyHistoryItem(item)}>
                  {uiText.copy}
                </button>
                <button type="button" onClick={() => onRemoveHistoryItem(item)}>
                  {uiText.delete}
                </button>
              </div>
            </article>
          ))}
        </div>
      )}
    </aside>
  );
}
