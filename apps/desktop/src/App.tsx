import { useEffect, useRef, useState, type CSSProperties, type ChangeEvent, type DragEvent, type PointerEvent, type SyntheticEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type MediaItem = {
  id: string;
  name: string;
  file: File;
  type: string;
  size: number;
  url: string;
};

type DragIntent = {
  pointerId: number;
  startX: number;
  startY: number;
};

type WindowCommand = "window_minimize" | "window_toggle_maximize" | "window_close";
type IconName = "close" | "folder" | "list" | "maximize" | "minimize" | "pause" | "play" | "restart" | "volume";

const playableExtensions = ["3gp", "aac", "avi", "flac", "m4a", "m4v", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "oga", "ogg", "ogv", "opus", "wav", "webm"];
const playableNamePattern = new RegExp(`\\.(${playableExtensions.join("|")})$`, "i");
let mediaItemIdCounter = 0;

function nextMediaItemId() {
  mediaItemIdCounter += 1;
  return `file:${mediaItemIdCounter}`;
}

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

function isSupportedMediaName(name: string) {
  return playableNamePattern.test(name);
}

function pickMediaFiles(files: FileList | File[]) {
  return Array.from(files).filter((file) => file.type.startsWith("video/") || file.type.startsWith("audio/") || isSupportedMediaName(file.name));
}

function mediaItemFromBrowserFile(file: File): MediaItem {
  return {
    id: nextMediaItemId(),
    name: file.name,
    file,
    type: file.type || "media file",
    size: file.size,
    url: URL.createObjectURL(file),
  };
}

function revokeObjectUrls(items: MediaItem[]) {
  for (const item of items) {
    URL.revokeObjectURL(item.url);
  }
}

function Icon({ name }: { name: IconName }) {
  const paths: Record<IconName, string> = {
    close: "M6 6l12 12M18 6 6 18",
    folder: "M3 7.5h6l2 2h10v8.5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7.5Z",
    list: "M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01",
    maximize: "M7 7h10v10H7z",
    minimize: "M6 12h12",
    pause: "M8 6h3v12H8zM13 6h3v12h-3z",
    play: "M8 5v14l11-7z",
    restart: "M5 12a7 7 0 1 0 2-4.9M5 5v5h5",
    volume: "M4 10v4h4l5 4V6l-5 4H4Z M16 9a4 4 0 0 1 0 6",
  };

  return (
    <svg aria-hidden="true" className="icon" fill="none" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" viewBox="0 0 24 24">
      <path d={paths[name]} />
    </svg>
  );
}

function App() {
  const [queue, setQueue] = useState<MediaItem[]>([]);
  const [currentIndex, setCurrentIndex] = useState<number | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isPlaylistOpen, setIsPlaylistOpen] = useState(false);
  const [duration, setDuration] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const [volumeLevel, setVolumeLevel] = useState(0.82);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const [mpvEmbedPath, setMpvEmbedPath] = useState("");
  const [mpvEmbedStatus, setMpvEmbedStatus] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const dragIntentRef = useRef<DragIntent | null>(null);
  const pendingAutoplayRef = useRef(false);
  const media = currentIndex === null ? null : (queue[currentIndex] ?? null);

  useEffect(() => {
    return () => revokeObjectUrls(queue);
  }, [queue]);

  useEffect(() => {
    setCurrentTime(0);
    setDuration(0);
    setIsPlaying(false);
    setPlaybackError(null);
  }, [media?.id]);

  function replaceQueue(nextQueue: MediaItem[]) {
    pendingAutoplayRef.current = false;
    setQueue(nextQueue);
    setCurrentIndex(nextQueue.length ? 0 : null);
    setIsPlaylistOpen(nextQueue.length > 1);
  }

  function openBrowserMediaFiles() {
    fileInputRef.current?.click();
  }

  function openFiles(files: FileList | File[]) {
    const nextQueue = pickMediaFiles(files).map(mediaItemFromBrowserFile);
    if (!nextQueue.length) {
      setPlaybackError("No supported media file was found in that selection.");
      return;
    }

    setPlaybackError(null);
    replaceQueue(nextQueue);
  }

  function handleFileInputChange(event: ChangeEvent<HTMLInputElement>) {
    if (event.currentTarget.files?.length) {
      openFiles(event.currentTarget.files);
    }
    event.currentTarget.value = "";
  }

  function handleDrop(event: DragEvent<HTMLElement>) {
    event.preventDefault();
    event.stopPropagation();
    if (event.dataTransfer.files.length) {
      openFiles(event.dataTransfer.files);
    }
  }

  function chooseQueueItem(index: number) {
    if (!queue[index] || index === currentIndex) {
      return;
    }

    pendingAutoplayRef.current = false;
    setCurrentIndex(index);
  }

  function advanceToNextQueueItem() {
    if (currentIndex === null) {
      return false;
    }

    const nextIndex = currentIndex + 1;
    if (!queue[nextIndex]) {
      return false;
    }

    pendingAutoplayRef.current = true;
    setCurrentIndex(nextIndex);
    return true;
  }

  function handleLoadedMetadata(event: SyntheticEvent<HTMLVideoElement>) {
    event.currentTarget.volume = volumeLevel;
    setDuration(event.currentTarget.duration);
  }

  function handleTimeUpdate(event: SyntheticEvent<HTMLVideoElement>) {
    setCurrentTime(event.currentTarget.currentTime);
  }

  function handleCanPlay(event: SyntheticEvent<HTMLVideoElement>) {
    event.currentTarget.volume = volumeLevel;
    if (!pendingAutoplayRef.current) {
      return;
    }

    pendingAutoplayRef.current = false;
    event.currentTarget.play().catch((error: unknown) => {
      setPlaybackError(error instanceof Error ? error.message : String(error));
    });
  }

  function beginWindowDragIntent(event: PointerEvent<HTMLElement>) {
    if (event.button !== 0) {
      return;
    }

    dragIntentRef.current = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function continueWindowDragIntent(event: PointerEvent<HTMLElement>) {
    const intent = dragIntentRef.current;
    if (!intent || intent.pointerId !== event.pointerId) {
      return;
    }

    const distance = Math.hypot(event.clientX - intent.startX, event.clientY - intent.startY);
    if (distance < 4) {
      return;
    }

    clearWindowDragIntent(event);
    getCurrentWindow().startDragging().catch((error: unknown) => {
      console.error("Window drag failed", error);
    });
  }

  function clearWindowDragIntent(event: PointerEvent<HTMLElement>) {
    if (dragIntentRef.current?.pointerId === event.pointerId && event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    dragIntentRef.current = null;
  }

  function toggleFullscreen() {
    const window = getCurrentWindow();
    window
      .isFullscreen()
      .then((isFullscreen) => window.setFullscreen(!isFullscreen))
      .catch((error: unknown) => {
        console.error("Fullscreen toggle failed", error);
      });
  }

  function togglePlayback() {
    const video = videoRef.current;
    if (!media || !video) {
      openBrowserMediaFiles();
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

  function openMpvEmbedDebug() {
    const path = mpvEmbedPath.trim();
    if (!path) {
      setMpvEmbedStatus("Enter a local path for the mpv embed spike.");
      return;
    }

    invoke<{ hwnd: number; path: string; status: string }>("mpv_embed_open_path", { path })
      .then((snapshot) => {
        setMpvEmbedStatus(`mpv ${snapshot.status}: HWND ${snapshot.hwnd}`);
      })
      .catch((error: unknown) => {
        setMpvEmbedStatus(error instanceof Error ? error.message : String(error));
      });
  }

  function stopMpvEmbedDebug() {
    invoke("mpv_embed_stop")
      .then(() => setMpvEmbedStatus("mpv stopped"))
      .catch((error: unknown) => {
        setMpvEmbedStatus(error instanceof Error ? error.message : String(error));
      });
  }

  function togglePlaylist() {
    setIsPlaylistOpen((isOpen) => !isOpen);
  }

  function seekTo(value: number) {
    const video = videoRef.current;
    if (!video || !Number.isFinite(value)) {
      return;
    }
    video.currentTime = value;
    setCurrentTime(value);
  }

  function commitSeekTo(value: number) {
    seekTo(value);
  }

  function setVolume(value: number) {
    const nextVolume = Math.min(1, Math.max(0, value));
    setVolumeLevel(nextVolume);
    if (videoRef.current) {
      videoRef.current.volume = nextVolume;
    }
  }

  function commitVolume(value: number) {
    setVolume(value);
  }

  const progress = duration > 0 ? (currentTime / duration) * 100 : 0;
  const queueItems = queue.length ? queue : media ? [media] : [];

  return (
    <main className="app-shell">
      <section className={`window-shell ${media ? "window-shell--loaded" : ""}`} aria-label="OpenPlayer">
        <section
          className={`stage ${media ? "stage--loaded" : ""}`}
          aria-label="Player surface"
          onDragOver={(event) => event.preventDefault()}
          onDrop={handleDrop}
        >
          <input
            ref={fileInputRef}
            className="media-file-input"
            type="file"
            tabIndex={-1}
            aria-hidden="true"
            accept={`audio/*,video/*,${playableExtensions.map((extension) => `.${extension}`).join(",")}`}
            multiple
            onChange={handleFileInputChange}
          />

          {media ? (
            <video
              key={media.id}
              ref={videoRef}
              className="media-view"
              src={media.url}
              playsInline
              onCanPlay={handleCanPlay}
              onLoadedMetadata={handleLoadedMetadata}
              onDurationChange={handleLoadedMetadata}
              onTimeUpdate={handleTimeUpdate}
              onPlay={() => {
                setIsPlaying(true);
                setPlaybackError(null);
              }}
              onPause={() => setIsPlaying(false)}
              onEnded={() => {
                setIsPlaying(false);
                if (!advanceToNextQueueItem()) {
                  setCurrentTime(duration);
                }
              }}
              onError={() => setPlaybackError("This file could not be decoded by the native HTML video renderer.")}
            />
          ) : (
            <div className="empty-open">
              <span>Open media</span>
              <small>or drop a file anywhere</small>
            </div>
          )}

          <div
            className="drag-surface"
            aria-hidden="true"
            onDoubleClick={toggleFullscreen}
            onPointerCancel={clearWindowDragIntent}
            onPointerDown={beginWindowDragIntent}
            onPointerMove={continueWindowDragIntent}
            onPointerUp={clearWindowDragIntent}
          />

          <div className="window-controls" aria-label="Window controls">
            <button type="button" aria-label="Minimize window" onClick={() => runWindowCommand("window_minimize")}>
              <Icon name="minimize" />
            </button>
            <button type="button" aria-label="Maximize or restore window" onClick={() => runWindowCommand("window_toggle_maximize")}>
              <Icon name="maximize" />
            </button>
            <button className="window-control-close" type="button" aria-label="Close window" onClick={() => runWindowCommand("window_close")}>
              <Icon name="close" />
            </button>
          </div>

          <div className="mpv-embed-debug" aria-label="MPV embed debug controls">
            <input
              type="text"
              value={mpvEmbedPath}
              placeholder="Local path for mpv embed spike"
              aria-label="Local path for mpv embed spike"
              onChange={(event) => setMpvEmbedPath(event.currentTarget.value)}
            />
            <button type="button" onClick={openMpvEmbedDebug}>MPV embed</button>
            <button type="button" onClick={stopMpvEmbedDebug}>Stop</button>
            {mpvEmbedStatus && <small>{mpvEmbedStatus}</small>}
          </div>

          {playbackError && <div className="playback-error" role="alert">{playbackError}</div>}

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
                style={{ "--progress": `${progress}%` } as CSSProperties}
                onChange={(event) => seekTo(Number(event.currentTarget.value))}
                onPointerUp={(event) => commitSeekTo(Number(event.currentTarget.value))}
                onKeyUp={(event) => commitSeekTo(Number(event.currentTarget.value))}
                onBlur={(event) => commitSeekTo(Number(event.currentTarget.value))}
                disabled={!media || duration <= 0}
              />
              <span className="transport-time">{formatTime(duration)}</span>
            </div>

            <div className="control-strip">
              <button type="button" aria-label="Open media" onClick={openBrowserMediaFiles}>
                <Icon name="folder" />
              </button>
              <button className="control-primary" type="button" aria-label={isPlaying ? "Pause" : media ? "Play" : "Open media"} onClick={togglePlayback}>
                <Icon name={isPlaying ? "pause" : "play"} />
              </button>
              <button type="button" aria-label="Restart" onClick={() => commitSeekTo(0)} disabled={!media}>
                <Icon name="restart" />
              </button>
              <label className="volume-control" aria-label="Volume">
                <Icon name="volume" />
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={volumeLevel}
                  aria-label="Volume"
                  onChange={(event) => setVolume(Number(event.currentTarget.value))}
                  onPointerUp={(event) => commitVolume(Number(event.currentTarget.value))}
                  onKeyUp={(event) => commitVolume(Number(event.currentTarget.value))}
                  onBlur={(event) => commitVolume(Number(event.currentTarget.value))}
                />
              </label>
              <button
                className={`playlist-toggle ${isPlaylistOpen ? "playlist-toggle--open" : ""}`}
                type="button"
                aria-label="Toggle playlist"
                aria-expanded={isPlaylistOpen}
                onClick={togglePlaylist}
              >
                <Icon name="list" />
              </button>
            </div>
          </div>

          {isPlaylistOpen && (
            <aside className="playlist-drawer playlist-drawer--open" aria-label="Playlist">
              <ol>
                {queueItems.map((item, index) => (
                  <li key={item.id}>
                    <button
                      className={`playlist-item ${index === currentIndex ? "playlist-item--active" : ""}`}
                      type="button"
                      aria-current={index === currentIndex ? "true" : undefined}
                      onClick={() => chooseQueueItem(index)}
                    >
                      <span>{item.name}</span>
                    </button>
                  </li>
                ))}
              </ol>
            </aside>
          )}
        </section>
      </section>
    </main>
  );
}

export default App;
