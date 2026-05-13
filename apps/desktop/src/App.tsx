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

type WindowCommand = "window_minimize" | "window_toggle_maximize" | "window_close";

const queueItems = ["No media loaded", "Drop files here", "History will resume here"];
const trackItems = ["Video", "Audio", "Subtitles", "Chapters"];

function runWindowCommand(command: WindowCommand) {
  invoke(command).catch((error: unknown) => {
    console.error(`Window command failed: ${command}`, error);
  });
}

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
      <section className="window-shell" aria-label="OpenPlayer desktop shell">
        <header className="titlebar" data-tauri-drag-region>
          <div className="titlebar-brand" data-tauri-drag-region>
            <span className="brand-mark" aria-hidden="true">
              OP
            </span>
            <div data-tauri-drag-region>
              <strong>OpenPlayer</strong>
              <span>No media loaded</span>
            </div>
          </div>

          <div className="titlebar-center" data-tauri-drag-region>
            <span>Studio Dark</span>
            <span className={`connection-dot connection-dot--${health.status}`} aria-hidden="true" />
          </div>

          <nav className="window-controls" aria-label="Window controls">
            <button type="button" aria-label="Minimize window" onClick={() => runWindowCommand("window_minimize")}>
              <span aria-hidden="true">_</span>
            </button>
            <button
              type="button"
              aria-label="Maximize or restore window"
              onClick={() => runWindowCommand("window_toggle_maximize")}
            >
              <span aria-hidden="true">□</span>
            </button>
            <button
              className="window-control-close"
              type="button"
              aria-label="Close window"
              onClick={() => runWindowCommand("window_close")}
            >
              <span aria-hidden="true">×</span>
            </button>
          </nav>
        </header>

        <div className="player-layout">
          <section className="stage" aria-label="Player surface placeholder">
            <div className="stage-vignette" aria-hidden="true" />
            <div className="drop-hint">
              <p className="eyebrow">Ready for playback core</p>
              <strong>Open a file, stream, or playlist.</strong>
              <span>Media controls are staged visually; engine wiring starts after this shell fix.</span>
            </div>

            <div className="transport" aria-label="Inactive transport preview">
              <div className="transport-row" aria-hidden="true">
                <span className="transport-time">00:00</span>
                <div className="timeline">
                  <span />
                </div>
                <span className="transport-time">00:00</span>
              </div>
              <div className="control-strip" aria-hidden="true">
                <span>Prev</span>
                <span className="control-primary">Play</span>
                <span>Next</span>
                <span>Sub</span>
                <span>1.0x</span>
              </div>
            </div>
          </section>

          <aside className="side-rail" aria-label="Playlist and media information">
            <section className="panel queue-panel" aria-label="Queue panel">
              <div className="panel-heading">
                <p className="eyebrow">Queue</p>
                <strong>Session</strong>
              </div>
              <ol className="queue-list">
                {queueItems.map((item) => (
                  <li key={item}>{item}</li>
                ))}
              </ol>
            </section>

            <section className="panel tracks-panel" aria-label="Tracks panel">
              <div className="panel-heading">
                <p className="eyebrow">Tracks</p>
                <strong>Media lanes</strong>
              </div>
              <div className="track-list">
                {trackItems.map((item) => (
                  <span key={item}>{item}</span>
                ))}
              </div>
            </section>

            <section className="panel status-panel" aria-label="Application status">
              <div className="panel-heading">
                <p className="eyebrow">Core</p>
                <strong>Runtime</strong>
              </div>
              <div className={`health-row health-row--${health.status}`} role="status" aria-live="polite">
                {health.status === "ready" && (
                  <span>
                    Rust core connected · {health.info.name} v{health.info.version}
                  </span>
                )}
                {health.status === "loading" && <span>Connecting to Rust core...</span>}
                {health.status === "error" && <span>Rust core error: {health.message}</span>}
              </div>
            </section>
          </aside>
        </div>
      </section>
    </main>
  );
}

export default App;
