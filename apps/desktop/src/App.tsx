import { useMemo } from "react";

import DictationPanel from "./features/dictation/DictationPanel";
import HistoryPanel from "./features/history/HistoryPanel";
import OverlayWidget from "./features/overlay/OverlayWidget";
import SettingsDrawer from "./features/settings/SettingsDrawer";
import { useMainAppController } from "./features/app/useMainAppController";

function MainApp() {
  const c = useMainAppController();

  return (
    <main className="app v2">
      <header className="topbar">
        <div className="brand-block">
          <h1>WhisperPro</h1>
          <p className="brand-subtitle">{c.uiText.topbarSubtitle}</p>
        </div>
        <div className="top-actions">
          <div className="ui-language-flags" role="group" aria-label={c.uiText.uiLanguageLabel}>
            <button
              type="button"
              className={c.uiLanguage === "fr" ? "active" : ""}
              onClick={() => c.setUiLanguage("fr")}
              title="Français"
              aria-label="Français"
            >
              <span className="flag-icon flag-fr" aria-hidden="true" />
            </button>
            <button
              type="button"
              className={c.uiLanguage === "en" ? "active" : ""}
              onClick={() => c.setUiLanguage("en")}
              title="English"
              aria-label="English"
            >
              <span className="flag-icon flag-en" aria-hidden="true" />
            </button>
          </div>
          <button type="button" className="ghost" onClick={() => c.setSettingsOpen(true)} disabled={c.disabled}>
            {c.uiText.options}
          </button>
          <button type="button" className="secondary" onClick={() => void c.quitApplication()} disabled={c.disabled}>
            {c.uiText.quit}
          </button>
        </div>
      </header>

      <section className="content-grid">
        <DictationPanel
          model={c.dictation.model}
          setters={c.dictation.setters}
          handlers={c.dictation.handlers}
        />

        <HistoryPanel
          model={c.history.model}
          handlers={c.history.handlers}
        />
      </section>

      <SettingsDrawer
        model={c.settings.model}
        setters={c.settings.setters}
        handlers={c.settings.handlers}
      />
    </main>
  );
}

function App() {
  const isOverlayWindow = useMemo(() => {
    const params = new URLSearchParams(window.location.search);
    return params.get("overlay") === "1";
  }, []);

  if (isOverlayWindow) return <OverlayWidget />;
  return <MainApp />;
}

export default App;
