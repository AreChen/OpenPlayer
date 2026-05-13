import { useEffect, useRef, useState, type ChangeEvent, type DragEvent } from "react";
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

type MediaItem = {
  name: string;
  type: string;
  size: number;
  url: string;
};

type WindowCommand = "window_minimize" | "window_toggle_maximize" | "window_close";

const playableNamePattern = /\.(3gp|aac|avi|flac|m4a|m4v|mkv|mov|mp3|mp4|mpeg|mpg|oga|ogg|ogv|opus|wav|webm)$/i;

function runWindowCommand(command: WindowCommand) {
  invoke(command).catch((error: unknown) => {
    console.error(`Window command failed: ${command}`, error);
  });
}

function formatTime(value: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return "00:00";
  }

  const totalSeconds = Math.floor(value);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
  }

  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

function pickMediaFile(files: FileList | File[]) {
  return Array.from(files).find(
    (file) => file.type.startsWith("video/") || file.type.startsWith("audio/") || playableNamePattern.test(file.name),
  );
}

function App() {
  const [health, setHealth] = useState<HealthState>({ status: "loading" });
  const [media, setMedia] = useState<MediaItem | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [duration, setDuration] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const [volumeLevel, setVolumeLevel] = useState(0.82);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

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

  useEffect(() => {
    return () => {
      if (media?.url) {
        URL.revokeObjectURL(media.url);
      }
    };
  }, [media?.url]);

  function openFiles(files: FileList | File[]) {
    const file = pickMediaFile(files);
    if (!file) {
      setPlaybackError("No supported media file was found in that selection.");
      return;
    }

    setMedia({
      name: file.name,
      type: file.type || "media file",
      size: file.size,
      url: URL.createObjectURL(file),
    });
    setCurrentTime(0);
    setDuration(0);
    setIsPlaying(false);
    setPlaybackError(null);
  }

  function handleFileInput(event: ChangeEvent<HTMLInputElement>) {
    if (event.currentTarget.files?.length) {
      openFiles(event.currentTarget.files);
      event.currentTarget.value = "";
    }
  }

  function handleDrop(event: DragEvent<HTMLElement>) {
    event.preventDefault();
    event.stopPropagation();
    if (event.dataTransfer.files.length) {
      openFiles(event.dataTransfer.files);
    }
  }

  function togglePlayback() {
    const video = videoRef.current;
    if (!media || !video) {
      fileInputRef.current?.click();
      return;
    }

    if (video.paused) {
      video.play().catch((error: unknown) => {
        setPlaybackError(error instanceof Error ? error.message : String(error));
      });
    } else {
      video.pause();
    }
  }

  function seekTo(value: number) {
    const video = videoRef.current;
    if (!video || !Number.isFinite(value)) {
      return;
    }
    video.currentTime = value;
    setCurrentTime(value);
  }

  function setVolume(value: number) {
    const nextVolume = Math.min(1, Math.max(0, value));
    setVolumeLevel(nextVolume);
    if (videoRef.current) {
      videoRef.current.volume = nextVolume;
    }
  }

  const progress = duration > 0 ? (currentTime / duration) * 100 : 0;
  const queueItems = media ? [media.name, media.type, `${(media.size / (1024 * 1024)).toFixed(1)} MiB`] : ["No media loaded", "Drop files here", "Open a local file"];
  const trackItems = media ? ["Local file", "HTML5 renderer", isPlaying ? "Playing" : "Paused", "Subtitles later"] : ["Video", "Audio", "Subtitles", "Chapters"];

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
              <span>{media?.name ?? "No media loaded"}</span>
            </div>
          </div>

          <div className="titlebar-center" data-tauri-drag-region>
            <span>{isPlaying ? "Playing" : "Studio Dark"}</span>
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
          <section
            className={`stage ${media ? "stage--loaded" : ""}`}
            aria-label="Player surface"
            onDragOver={(event) => event.preventDefault()}
            onDrop={handleDrop}
          >
            <div className="stage-vignette" aria-hidden="true" />
            <input
              ref={fileInputRef}
              className="media-file-input"
              type="file"
              tabIndex={-1}
              aria-hidden="true"
              accept="audio/*,video/*,.mkv,.avi,.mov,.mp4,.webm,.mp3,.flac,.wav,.m4a"
              onChange={handleFileInput}
            />

            {media ? (
              <video
                ref={videoRef}
                className="media-view"
                src={media.url}
                onLoadedMetadata={(event) => {
                  event.currentTarget.volume = volumeLevel;
                  setDuration(event.currentTarget.duration);
                }}
                onTimeUpdate={(event) => setCurrentTime(event.currentTarget.currentTime)}
                onPlay={() => setIsPlaying(true)}
                onPause={() => setIsPlaying(false)}
                onEnded={() => setIsPlaying(false)}
                onError={() => setPlaybackError("This file could not be decoded by the current preview renderer.")}
              />
            ) : (
              <div className="drop-hint">
                <p className="eyebrow">Local playback preview</p>
                <strong>Open or drop a media file.</strong>
                <span>MP4, WebM, MP3, WAV and other WebView-supported formats can play here now.</span>
              </div>
            )}

            {playbackError && <div className="playback-error">{playbackError}</div>}

            <div className="transport" aria-label="Playback controls">
              <div className="transport-row">
                <span className="transport-time">{formatTime(currentTime)}</span>
                <input
                  className="seek-slider"
                  type="range"
                  min="0"
                  max={duration || 0}
                  step="0.1"
                  value={Math.min(currentTime, duration || 0)}
                  aria-label="Seek playback position"
                  style={{ "--progress": `${progress}%` } as React.CSSProperties}
                  onChange={(event) => seekTo(Number(event.currentTarget.value))}
                  disabled={!media || duration <= 0}
                />
                <span className="transport-time">{formatTime(duration)}</span>
              </div>
              <div className="control-strip">
                <button type="button" onClick={() => fileInputRef.current?.click()}>
                  Open
                </button>
                <button className="control-primary" type="button" onClick={togglePlayback}>
                  {isPlaying ? "Pause" : media ? "Play" : "Open"}
                </button>
                <button type="button" onClick={() => seekTo(0)} disabled={!media}>
                  Restart
                </button>
                <label className="volume-control">
                  <span>Vol</span>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.01"
                    value={volumeLevel}
                    aria-label="Volume"
                    onChange={(event) => setVolume(Number(event.currentTarget.value))}
                  />
                </label>
              </div>
            </div>
          </section>

          <aside className="side-rail" aria-label="Playlist and media information">
            <section className="panel queue-panel" aria-label="Queue panel">
              <div className="panel-heading">
                <p className="eyebrow">Queue</p>
                <strong>{media ? "Current media" : "Session"}</strong>
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
