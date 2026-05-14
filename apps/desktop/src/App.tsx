import { useEffect, useRef, useState, type CSSProperties, type DragEvent, type PointerEvent, type SyntheticEvent } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";

type MediaSourceKind = "localFilePath" | "localFileLabel";

type MediaItem = {
  id: string;
  name: string;
  path: string | null;
  type: string;
  size: number | null;
  url: string;
  sourceKind: MediaSourceKind;
  openedAtMs: number;
};

type PlaybackSourceDto = {
  kind: "localFilePath" | "localFileLabel" | "localFolderLabel" | "httpUrl";
  value: string;
};

type PlaybackStatusDto = "idle" | "loading" | "ready" | "playing" | "paused" | "stopped" | "ended" | "error";

type PlaybackSnapshotDto = {
  sourceLabel: string | null;
  status: PlaybackStatusDto;
  positionMs: number;
  durationMs: number | null;
  volumePercent: number;
  muted: boolean;
  speedMilli: number;
  latestError: PlaybackCommandError | null;
};

type PlaybackCommandError = {
  code: string;
  message: string;
};

type RecentMediaDto = {
  path: string;
  name: string;
  lastOpenedAtMs: number;
  openCount: number;
};

type PlaybackProgressDto = {
  path: string;
  positionMs: number;
  durationMs: number | null;
  updatedAtMs: number;
};

type StorageCommandError = {
  code: string;
  message: string;
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

function nextMediaItemId(prefix: string) {
  mediaItemIdCounter += 1;
  return `${prefix}:${mediaItemIdCounter}`;
}

function runWindowCommand(command: WindowCommand) {
  invoke(command).catch((error: unknown) => {
    console.error(`Window command failed: ${command}`, error);
  });
}

function playbackErrorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) {
    return String((error as PlaybackCommandError).message);
  }
  return error instanceof Error ? error.message : String(error);
}

function runPlaybackCommand(command: string, args?: Record<string, unknown>) {
  return invoke<PlaybackSnapshotDto>(command, args).catch((error: unknown) => {
    throw new Error(playbackErrorMessage(error));
  });
}

function storageErrorMessage(error: unknown) {
  if (typeof error === "object" && error && "message" in error) {
    return String((error as StorageCommandError).message);
  }
  return error instanceof Error ? error.message : String(error);
}

function runStorageCommand<T>(command: string, args?: Record<string, unknown>) {
  return invoke<T>(command, args).catch((error: unknown) => {
    throw new Error(storageErrorMessage(error));
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

function fileNameFromPath(path: string) {
  return path.replace(/\\/g, "/").split("/").filter(Boolean).pop() ?? path;
}

function mediaItemFromNativePath(path: string, index: number, displayName = fileNameFromPath(path), openedAtMs = Date.now()): MediaItem {
  return {
    id: nextMediaItemId("native"),
    name: displayName,
    path,
    type: "media file",
    size: null,
    url: convertFileSrc(path),
    sourceKind: "localFilePath",
    openedAtMs,
  };
}

function mediaItemFromBrowserFile(file: File, index: number): MediaItem {
  return {
    id: nextMediaItemId("preview"),
    name: file.name,
    path: null,
    type: file.type || "media file",
    size: file.size,
    url: URL.createObjectURL(file),
    sourceKind: "localFileLabel",
    openedAtMs: Date.now(),
  };
}

function revokePreviewUrls(items: MediaItem[]) {
  for (const item of items) {
    if (item.sourceKind === "localFileLabel") {
      URL.revokeObjectURL(item.url);
    }
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
  const [playbackSnapshot, setPlaybackSnapshot] = useState<PlaybackSnapshotDto | null>(null);
  const [recentMedia, setRecentMedia] = useState<RecentMediaDto[]>([]);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const dragIntentRef = useRef<DragIntent | null>(null);
  const playbackCommandIdRef = useRef(0);
  const nativeOpenRequestIdRef = useRef(0);
  const recentMediaRequestIdRef = useRef(0);
  const resumeRequestIdRef = useRef(0);
  const pendingAutoplayRef = useRef(false);
  const currentMediaIdRef = useRef<string | null>(null);
  const resumedMediaIdRef = useRef<string | null>(null);
  const resumeLookupCompletedMediaIdRef = useRef<string | null>(null);
  const lastProgressSaveRef = useRef<{ mediaId: string; positionMs: number } | null>(null);
  const media = currentIndex === null ? null : (queue[currentIndex] ?? null);

  useEffect(() => {
    currentMediaIdRef.current = media?.id ?? null;
  }, [media?.id]);

  useEffect(() => {
    void refreshRecentMedia();
  }, []);

  useEffect(() => {
    return () => revokePreviewUrls(queue);
  }, [queue]);

  useEffect(() => {
    if (!media) {
      return;
    }

    resumedMediaIdRef.current = null;
    resumeLookupCompletedMediaIdRef.current = null;
    lastProgressSaveRef.current = null;
    setCurrentTime(0);
    setDuration(0);
    setIsPlaying(false);
    setPlaybackError(null);
    mirrorOpenMedia(media);
    recordRecentMedia(media);
  }, [media?.id]);

  function refreshRecentMedia() {
    const requestId = recentMediaRequestIdRef.current + 1;
    recentMediaRequestIdRef.current = requestId;

    return runStorageCommand<RecentMediaDto[]>("storage_recent_media_list", { limit: 12 })
      .then((items) => {
        if (requestId === recentMediaRequestIdRef.current) {
          setRecentMedia(items);
        }
      })
      .catch((error: unknown) => {
        if (requestId === recentMediaRequestIdRef.current) {
          console.error("Recent media load failed", error);
        }
      });
  }

  function recordRecentMedia(item: MediaItem) {
    if (item.sourceKind !== "localFilePath" || !item.path) {
      return;
    }

    runStorageCommand<RecentMediaDto>("storage_recent_media_record", { path: item.path, name: item.name, openedAtMs: item.openedAtMs })
      .then(() => refreshRecentMedia())
      .catch((error: unknown) => {
        console.error("Recent media record failed", error);
      });
  }

  function mirrorPlaybackCommand(command: string, args?: Record<string, unknown>) {
    const commandId = playbackCommandIdRef.current + 1;
    playbackCommandIdRef.current = commandId;
    runPlaybackCommand(command, args)
      .then((snapshot) => {
        if (commandId === playbackCommandIdRef.current) {
          setPlaybackSnapshot(snapshot);
        }
      })
      .catch((error: unknown) => {
        if (commandId === playbackCommandIdRef.current) {
          setPlaybackError(error instanceof Error ? error.message : String(error));
        }
      });
  }

  function playbackSourceFromMedia(item: MediaItem): PlaybackSourceDto {
    return {
      kind: item.sourceKind,
      value: item.path ?? item.name,
    };
  }

  function mirrorOpenMedia(item: MediaItem) {
    mirrorPlaybackCommand("playback_open_preview_source", {
      source: playbackSourceFromMedia(item),
    });
  }

  function replaceQueue(nextQueue: MediaItem[]) {
    pendingAutoplayRef.current = false;
    setQueue(nextQueue);
    setCurrentIndex(nextQueue.length ? 0 : null);
    setIsPlaylistOpen(nextQueue.length > 1);
  }

  async function openNativeMediaFiles() {
    const requestId = nativeOpenRequestIdRef.current + 1;
    nativeOpenRequestIdRef.current = requestId;

    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: "Media", extensions: playableExtensions }],
      });
      if (requestId !== nativeOpenRequestIdRef.current) {
        return;
      }

      const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
      if (!paths.length) {
        return;
      }

      const nextQueue = paths.filter(isSupportedMediaName).map((path, index) => mediaItemFromNativePath(path, index));
      if (!nextQueue.length) {
        setPlaybackError("No supported media file was found in that selection.");
        return;
      }

      setPlaybackError(null);
      replaceQueue(nextQueue);
    } catch (error: unknown) {
      if (requestId !== nativeOpenRequestIdRef.current) {
        return;
      }

      setPlaybackError(error instanceof Error ? error.message : String(error));
    }
  }

  function openRecentMedia(item: RecentMediaDto) {
    setPlaybackError(null);
    replaceQueue([mediaItemFromNativePath(item.path, 0, item.name, Date.now())]);
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

  function isResumePositionValid(progress: PlaybackProgressDto, durationMs: number | null) {
    if (progress.positionMs <= 10_000) return false;
    if (durationMs !== null && progress.positionMs >= durationMs - 10_000) return false;
    if (durationMs !== null && progress.positionMs >= durationMs) return false;
    return true;
  }

  function maybeResumePlayback(item: MediaItem, video: HTMLVideoElement, durationSeconds: number) {
    if (item.sourceKind !== "localFilePath" || !item.path || resumedMediaIdRef.current === item.id) return;

    resumedMediaIdRef.current = item.id;
    const requestId = resumeRequestIdRef.current + 1;
    resumeRequestIdRef.current = requestId;
    const durationMs = Number.isFinite(durationSeconds) && durationSeconds > 0 ? Math.round(durationSeconds * 1000) : null;
    const markResumeLookupCompleted = () => {
      if (currentMediaIdRef.current === item.id) {
        resumeLookupCompletedMediaIdRef.current = item.id;
      }
    };

    runStorageCommand<PlaybackProgressDto | null>("storage_progress_get", { path: item.path })
      .then((savedProgress) => {
        if (currentMediaIdRef.current !== item.id || videoRef.current !== video) return;
        markResumeLookupCompleted();
        if (requestId !== resumeRequestIdRef.current) return;
        if (!savedProgress) return;
        if (!isResumePositionValid(savedProgress, durationMs)) return;

        const resumeSeconds = savedProgress.positionMs / 1000;
        video.currentTime = resumeSeconds;
        setCurrentTime(resumeSeconds);
        mirrorPlaybackCommand("playback_seek", { positionMs: savedProgress.positionMs });
      })
      .catch((error: unknown) => {
        markResumeLookupCompleted();
        console.error("Playback progress load failed", error);
      });
  }

  function maybeSavePlaybackProgress(positionSeconds: number, durationSeconds: number, force = false) {
    if (!media?.path || media.sourceKind !== "localFilePath" || !Number.isFinite(positionSeconds)) return;
    if (!force && resumeLookupCompletedMediaIdRef.current !== media.id) return;

    const positionMs = Math.max(0, Math.round(positionSeconds * 1000));
    const durationMs = Number.isFinite(durationSeconds) && durationSeconds > 0 ? Math.round(durationSeconds * 1000) : null;
    const lastSave = lastProgressSaveRef.current;
    if (!force && lastSave?.mediaId === media.id && Math.abs(positionMs - lastSave.positionMs) < 5_000) return;

    lastProgressSaveRef.current = { mediaId: media.id, positionMs };
    runStorageCommand<void>("storage_progress_save", { path: media.path, positionMs, durationMs }).catch((error: unknown) => {
      console.error("Playback progress save failed", error);
    });
  }

  function clearSavedPlaybackProgress(item: MediaItem | null) {
    if (!item?.path || item.sourceKind !== "localFilePath") return;

    runStorageCommand<void>("storage_progress_clear", { path: item.path }).catch((error: unknown) => {
      console.error("Playback progress clear failed", error);
    });
  }

  function handleLoadedMetadata(event: SyntheticEvent<HTMLVideoElement>) {
    event.currentTarget.volume = volumeLevel;
    setDuration(event.currentTarget.duration);
    if (media) {
      maybeResumePlayback(media, event.currentTarget, event.currentTarget.duration);
    }
  }

  function handleTimeUpdate(event: SyntheticEvent<HTMLVideoElement>) {
    const nextTime = event.currentTarget.currentTime;
    setCurrentTime(nextTime);
    maybeSavePlaybackProgress(nextTime, event.currentTarget.duration);
  }

  function handleCanPlay(event: SyntheticEvent<HTMLVideoElement>) {
    event.currentTarget.volume = volumeLevel;
    if (!pendingAutoplayRef.current) {
      return;
    }

    pendingAutoplayRef.current = false;
    event.currentTarget
      .play()
      .then(() => mirrorPlaybackCommand("playback_play"))
      .catch((error: unknown) => {
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
      void openNativeMediaFiles();
      return;
    }

    if (video.paused) {
      video
        .play()
        .then(() => mirrorPlaybackCommand("playback_play"))
        .catch((error: unknown) => {
          setPlaybackError(error instanceof Error ? error.message : String(error));
        });
    } else {
      video.pause();
      mirrorPlaybackCommand("playback_pause");
    }
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
    if (Number.isFinite(value)) {
      resumeRequestIdRef.current += 1;
      if (media) {
        resumeLookupCompletedMediaIdRef.current = media.id;
      }
      mirrorPlaybackCommand("playback_seek", { positionMs: Math.round(value * 1000) });
      maybeSavePlaybackProgress(value, videoRef.current?.duration ?? duration, true);
    }
  }

  function setVolume(value: number) {
    const nextVolume = Math.min(1, Math.max(0, value));
    setVolumeLevel(nextVolume);
    if (videoRef.current) {
      videoRef.current.volume = nextVolume;
    }
  }

  function commitVolume(value: number) {
    const nextVolume = Math.min(1, Math.max(0, value));
    setVolume(nextVolume);
    mirrorPlaybackCommand("playback_set_volume", { percent: Math.round(nextVolume * 100) });
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
          {media ? (
            <video
              key={media.id}
              ref={videoRef}
              className="media-view"
              src={media.url}
              onCanPlay={handleCanPlay}
              onLoadedMetadata={handleLoadedMetadata}
              onTimeUpdate={handleTimeUpdate}
              onPlay={() => setIsPlaying(true)}
              onPause={() => setIsPlaying(false)}
              onEnded={() => {
                setIsPlaying(false);
                clearSavedPlaybackProgress(media);
                if (!advanceToNextQueueItem()) {
                  mirrorPlaybackCommand("playback_stop");
                }
              }}
              onError={() => setPlaybackError("This file could not be decoded by the current preview renderer.")}
            />
          ) : (
            <>
              <div className="empty-open">
                <span>Open media</span>
                <small>or drop a file anywhere</small>
              </div>
              {recentMedia.length > 0 && (
                <div className="recent-shortcuts" aria-label="Recent media">
                  {recentMedia.slice(0, 4).map((item) => (
                    <button key={item.path} type="button" onClick={() => openRecentMedia(item)}>
                      {item.name}
                    </button>
                  ))}
                </div>
              )}
            </>
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
              <button type="button" aria-label="Open media" onClick={() => void openNativeMediaFiles()}>
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
                      <span>{playbackSnapshot?.sourceLabel === item.path ? item.path : item.name}</span>
                    </button>
                  </li>
                ))}
              </ol>
              {recentMedia.length > 0 && (
                <section className="recent-drawer-section" aria-label="Recent media">
                  <h2>Recent</h2>
                  {recentMedia.map((item) => (
                    <button key={item.path} type="button" onClick={() => openRecentMedia(item)}>
                      <span>{item.name}</span>
                    </button>
                  ))}
                </section>
              )}
            </aside>
          )}
        </section>
      </section>
    </main>
  );
}

export default App;
