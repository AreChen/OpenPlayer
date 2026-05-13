import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type AppInfo = {
  name: string;
  version: string;
  stage: "skeleton";
};

type HealthState =
  | { status: "loading" }
  | { status: "ready"; info: AppInfo }
  | { status: "error"; message: string };

const queueItems = ["No file loaded", "Queue persistence", "History sync"];
const trackItems = ["Video track", "Audio lanes", "Subtitle sources"];

function App() {
  const [health, setHealth] = useState<HealthState>({ status: "loading" });

  useEffect(() => {
    let isMounted = true;

    invoke<AppInfo>("app_health")
      .then((info) => {
        if (isMounted) {
          setHealth({ status: "ready", info });
        }
      })
      .catch((error: unknown) => {
        if (isMounted) {
          setHealth({
            status: "error",
            message: error instanceof Error ? error.message : String(error),
          });
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <main className="app-shell">
      <section className="studio-frame" aria-label="OpenPlayer desktop shell">
        <header className="top-bar">
          <div>
            <p className="eyebrow">OpenPlayer</p>
            <h1>Studio Dark desktop shell</h1>
          </div>
          <div className={`status-pill status-pill--${health.status}`}>
            {health.status === "ready" ? `v${health.info.version}` : health.status}
          </div>
        </header>

        <div className="workspace-grid">
          <section className="player-surface" aria-label="Player surface placeholder">
            <div className="surface-mark" aria-hidden="true">
              <span>OP</span>
            </div>
            <div className="surface-copy">
              <p className="eyebrow">Player Surface</p>
              <strong>Playback engine connects in a later phase.</strong>
              <span>Surface, transport, and state boundaries are visible without shipping playback behavior.</span>
            </div>
            <div className="transport" aria-label="Inactive transport preview">
              <div className="transport-controls" aria-hidden="true">
                <span>Prev</span>
                <span className="control-primary">Play</span>
                <span>Next</span>
              </div>
              <div className="timeline">
                <span />
              </div>
              <div className="transport-meta">
                <span>00:00</span>
                <span>Awaiting media backend</span>
                <span>00:00</span>
              </div>
            </div>
          </section>

          <aside className="queue-panel" aria-label="Queue panel">
            <div className="panel-heading">
              <p className="eyebrow">Queue</p>
              <strong>Session outline</strong>
            </div>
            <ol className="queue-list">
              {queueItems.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ol>
          </aside>

          <section className="tracks-panel" aria-label="Tracks panel">
            <div className="panel-heading">
              <p className="eyebrow">Tracks</p>
              <strong>MediaBackend contract</strong>
            </div>
            <div className="track-list">
              {trackItems.map((item) => (
                <span key={item}>{item}</span>
              ))}
            </div>
          </section>

          <section className="themes-panel" aria-label="Themes and plugins panel">
            <div className="panel-heading">
              <p className="eyebrow">Themes + Plugins</p>
              <strong>Manifest boundaries</strong>
            </div>
            <p>
              Studio Dark tokens, theme manifests, and application plugin contracts are present as shell boundaries.
            </p>
          </section>
        </div>

        <footer className={`health-row health-row--${health.status}`} role="status" aria-live="polite">
          {health.status === "ready" && (
            <span>
              Rust core connected: {health.info.name} is in {health.info.stage} stage.
            </span>
          )}
          {health.status === "loading" && <span>Connecting to Rust core...</span>}
          {health.status === "error" && <span>Rust core error: {health.message}</span>}
        </footer>
      </section>
    </main>
  );
}

export default App;
