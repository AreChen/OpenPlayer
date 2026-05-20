import { useEffect, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

type MediaItem = {
  id: string;
  name: string;
  path: string;
};

type MpvSnapshot = {
  path: string;
  status: string;
  ended: boolean;
  paused: boolean;
  position: number;
  duration: number;
  fps: number;
  volume: number;
};

type PendingSeek = {
  target: number;
  startedAt: number;
};

type PlaybackClockAnchor = {
  position: number;
  startedAt: number;
  playing: boolean;
};

type TimeDisplayMode = "timecode" | "frames";

type ResizeDirection = "East" | "North" | "NorthEast" | "NorthWest" | "South" | "SouthEast" | "SouthWest" | "West";

type WindowCommand = "window_minimize" | "window_toggle_maximize" | "window_toggle_fullscreen" | "window_close";
type IconName = "close" | "folder" | "list" | "maximize" | "minimize" | "pause" | "play" | "restart" | "volume";

const playableExtensions = ["3gp", "aac", "avi", "flac", "m4a", "m4v", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "oga", "ogg", "ogv", "opus", "wav", "webm"];
const SEEK_CONFIRM_TOLERANCE_SECONDS = 0.75;
const SEEK_SNAPSHOT_SUPPRESS_MS = 1600;
const AUTO_HIDE_CONTROLS_MS = 5000;
const END_OF_MEDIA_SNAP_TOLERANCE_SECONDS = 0.5;
const resizeRegions: Array<{ className: string; direction: ResizeDirection }> = [
  { className: "resize-region--north", direction: "North" },
  { className: "resize-region--south", direction: "South" },
  { className: "resize-region--east", direction: "East" },
  { className: "resize-region--west", direction: "West" },
  { className: "resize-region--north-east", direction: "NorthEast" },
  { className: "resize-region--north-west", direction: "NorthWest" },
  { className: "resize-region--south-east", direction: "SouthEast" },
  { className: "resize-region--south-west", direction: "SouthWest" },
];
const surface = new URLSearchParams(window.location.search).get("surface");
const openPlayerLogoUrl = new URL("./assets/openplayer-logo.png", import.meta.url).href;
let mediaItemIdCounter = 0;

function nextMediaItemId() {
  mediaItemIdCounter += 1;
  return `path:${mediaItemIdCounter}`;
}

function runWindowCommand(command: WindowCommand) {
  invoke(command).catch((error: unknown) => {
    console.error(`Window command failed: ${command}`, error);
  });
}

function startMainWindowDrag() {
  invoke("window_start_drag").catch((error: unknown) => {
    console.error("Window drag failed", error);
  });
}

function startMainWindowResize(direction: ResizeDirection) {
  invoke("window_start_resize", { direction }).catch((error: unknown) => {
    console.error(`Window resize failed: ${direction}`, error);
  });
}

function formatTimecode(value: number, totalDuration: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return totalDuration > 3600 ? "0:00:00" : "00:00";
  }

  const totalSeconds = Math.floor(value);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (totalDuration > 3600) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
  }

  return `${Math.floor(totalSeconds / 60).toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

function formatFrameCount(value: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return "0";
  }

  return Math.floor(value).toLocaleString("en-US");
}

function canDisplayFrames(fps: number, duration: number) {
  return Number.isFinite(fps) && fps > 0 && Number.isFinite(duration) && duration > 0;
}

function snapEndOfMediaPosition(position: number, duration: number, isPlaying: boolean) {
  if (!Number.isFinite(position) || !Number.isFinite(duration) || duration <= 0) {
    return Number.isFinite(position) ? Math.max(0, position) : 0;
  }

  const clamped = Math.min(duration, Math.max(0, position));
  if (!isPlaying && duration - clamped <= END_OF_MEDIA_SNAP_TOLERANCE_SECONDS) {
    return duration;
  }

  return clamped;
}

function mediaItemFromPath(path: string): MediaItem {
  const normalized = path.replace(/\\/g, "/");
  return {
    id: nextMediaItemId(),
    name: normalized.split("/").pop() || path,
    path,
  };
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
  if (surface === "video") {
    return <main className="video-host-surface" aria-label="OpenPlayer video surface" />;
  }

  const [queue, setQueue] = useState<MediaItem[]>([]);
  const [currentIndex, setCurrentIndex] = useState<number | null>(null);
  const [isPlaylistOpen, setIsPlaylistOpen] = useState(false);
  const [duration, setDuration] = useState(0);
  const [currentTime, setCurrentTime] = useState(0);
  const [displayPosition, setDisplayPosition] = useState(0);
  const [volumeLevel, setVolumeLevel] = useState(0.82);
  const [framesPerSecond, setFramesPerSecond] = useState(0);
  const [timeDisplayMode, setTimeDisplayMode] = useState<TimeDisplayMode>("timecode");
  const [isPlaying, setIsPlaying] = useState(false);
  const [isChromeVisible, setIsChromeVisible] = useState(true);
  const [isPickerOpen, setIsPickerOpen] = useState(false);
  const [playbackError, setPlaybackError] = useState<string | null>(null);
  const pendingSeekRef = useRef<PendingSeek | null>(null);
  const playbackClockAnchorRef = useRef<PlaybackClockAnchor>({ position: 0, startedAt: performance.now(), playing: false });
  const snapshotRequestIdRef = useRef(0);
  const chromeHideTimerRef = useRef<number | null>(null);
  const media = currentIndex === null ? null : (queue[currentIndex] ?? null);
  const isChromePinned = !media || isPlaylistOpen || isPickerOpen || playbackError !== null;

  useEffect(() => {
    if (!media) {
      return;
    }

    const timer = window.setInterval(() => {
      const requestId = ++snapshotRequestIdRef.current;
      invoke<MpvSnapshot | null>("mpv_embed_snapshot")
        .then((snapshot) => {
          if (snapshot && requestId === snapshotRequestIdRef.current) {
            applySnapshot(snapshot);
          }
        })
        .catch(() => undefined);
    }, 500);

    return () => {
      window.clearInterval(timer);
      invalidatePendingSnapshots();
    };
  }, [media?.id]);

  useEffect(() => {
    if (!media || !isPlaying || duration <= 0) {
      return;
    }

    let frameId = 0;
    const tick = () => {
      const anchor = playbackClockAnchorRef.current;
      const elapsedSeconds = anchor.playing ? (performance.now() - anchor.startedAt) / 1000 : 0;
      setDisplayPosition(clampPlaybackPosition(anchor.position + elapsedSeconds, duration));
      frameId = window.requestAnimationFrame(tick);
    };

    frameId = window.requestAnimationFrame(tick);
    return () => window.cancelAnimationFrame(frameId);
  }, [media?.id, isPlaying, duration]);

  useEffect(() => {
    if (!canDisplayFrames(framesPerSecond, duration)) {
      setTimeDisplayMode("timecode");
    }
  }, [framesPerSecond, duration]);

  useEffect(() => {
    setIsChromeVisible(true);
    scheduleChromeHide();
    return clearChromeHideTimer;
  }, [media?.id, isChromePinned]);

  function clearChromeHideTimer() {
    if (chromeHideTimerRef.current !== null) {
      window.clearTimeout(chromeHideTimerRef.current);
      chromeHideTimerRef.current = null;
    }
  }

  function scheduleChromeHide() {
    clearChromeHideTimer();
    if (isChromePinned) {
      return;
    }

    chromeHideTimerRef.current = window.setTimeout(() => {
      setIsChromeVisible(false);
      chromeHideTimerRef.current = null;
    }, AUTO_HIDE_CONTROLS_MS);
  }

  function recordUserActivity() {
    setIsChromeVisible(true);
    scheduleChromeHide();
  }

  function invalidatePendingSnapshots() {
    snapshotRequestIdRef.current += 1;
  }

  function applyCommandSnapshot(snapshot: MpvSnapshot) {
    invalidatePendingSnapshots();
    applySnapshot(snapshot);
  }

  function applySnapshot(snapshot: MpvSnapshot) {
    const snapshotPosition = Number.isFinite(snapshot.position) ? snapshot.position : 0;
    const snapshotDuration = Number.isFinite(snapshot.duration) ? snapshot.duration : 0;
    const pendingSeek = pendingSeekRef.current;
    const nextIsPlaying = !snapshot.paused && snapshot.status !== "idle" && snapshot.status !== "ended";

    setDuration(snapshotDuration);
    setIsPlaying(nextIsPlaying);
    setFramesPerSecond(Number.isFinite(snapshot.fps) && snapshot.fps > 0 ? snapshot.fps : 0);
    setVolumeLevel(Math.min(1, Math.max(0, snapshot.volume / 100)));

    if (pendingSeek) {
      const isConfirmed = Math.abs(snapshotPosition - pendingSeek.target) <= SEEK_CONFIRM_TOLERANCE_SECONDS;
      const isExpired = performance.now() - pendingSeek.startedAt > SEEK_SNAPSHOT_SUPPRESS_MS;
      if (!isConfirmed && !isExpired) {
        return;
      }

      pendingSeekRef.current = null;
    }

    setCurrentTime(snapshotPosition);
    anchorDisplayClock(snapshotPosition, nextIsPlaying, snapshotDuration);
  }

  function reportPlaybackError(error: unknown) {
    setPlaybackError(error instanceof Error ? error.message : String(error));
  }

  function openMpvPath(path: string) {
    invalidatePendingSnapshots();
    return invoke<MpvSnapshot>("mpv_overlay_open_path", { path }).then((snapshot) => {
      pendingSeekRef.current = null;
      setPlaybackError(null);
      applyCommandSnapshot(snapshot);
    });
  }

  function seekTarget(value: number) {
    if (!Number.isFinite(value)) {
      return 0;
    }

    const upperBound = duration > 0 ? duration : value;
    return Math.min(upperBound, Math.max(0, value));
  }

  function clampPlaybackPosition(value: number, upperDuration = duration) {
    if (!Number.isFinite(value)) {
      return 0;
    }

    const upperBound = upperDuration > 0 ? upperDuration : value;
    return Math.min(upperBound, Math.max(0, value));
  }

  function anchorDisplayClock(position: number, playing: boolean, upperDuration = duration) {
    const clampedPosition = clampPlaybackPosition(position, upperDuration);
    playbackClockAnchorRef.current = {
      position: clampedPosition,
      startedAt: performance.now(),
      playing,
    };
    setDisplayPosition(clampedPosition);
  }

  function toggleTimeDisplayMode() {
    if (!canDisplayFrames(framesPerSecond, duration)) {
      setTimeDisplayMode("timecode");
      return;
    }

    setTimeDisplayMode((mode) => (mode === "timecode" ? "frames" : "timecode"));
  }

  async function openNativeMediaFiles() {
    if (isPickerOpen) {
      return;
    }

    setIsPickerOpen(true);
    setPlaybackError(null);
    try {
      const selection = await open({
        multiple: true,
        filters: [{ name: "Media", extensions: playableExtensions }],
      });
      const paths = typeof selection === "string" ? [selection] : Array.isArray(selection) ? selection : [];
      const nextQueue = paths.map(mediaItemFromPath);
      if (!nextQueue.length) {
        return;
      }

      setQueue(nextQueue);
      setCurrentIndex(0);
      setIsPlaylistOpen(nextQueue.length > 1);
      await openMpvPath(nextQueue[0].path);
    } catch (error) {
      reportPlaybackError(error);
    } finally {
      setIsPickerOpen(false);
    }
  }

  function chooseQueueItem(index: number) {
    const item = queue[index];
    if (!item || index === currentIndex) {
      return;
    }

    setCurrentIndex(index);
    openMpvPath(item.path).catch(reportPlaybackError);
  }

  function toggleFullscreen() {
    runWindowCommand("window_toggle_fullscreen");
  }

  function handleDragRegionPointerDown(event: ReactPointerEvent<HTMLDivElement>) {
    if (event.button === 1) {
      event.preventDefault();
      runWindowCommand("window_toggle_fullscreen");
      return;
    }

    if (event.button === 0) {
      startMainWindowDrag();
    }
  }

  function handleResizePointerDown(event: ReactPointerEvent<HTMLDivElement>, direction: ResizeDirection) {
    if (event.button !== 0) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    recordUserActivity();
    startMainWindowResize(direction);
  }

  function togglePlayback() {
    if (!media) {
      openNativeMediaFiles();
      return;
    }

    invalidatePendingSnapshots();
    invoke<MpvSnapshot>(isPlaying ? "mpv_embed_pause" : "mpv_embed_play")
      .then(applyCommandSnapshot)
      .catch(reportPlaybackError);
  }

  function togglePlaylist() {
    setIsPlaylistOpen((isOpen) => !isOpen);
  }

  function seekTo(value: number) {
    const target = seekTarget(value);
    pendingSeekRef.current = { target, startedAt: performance.now() };
    setCurrentTime(target);
    anchorDisplayClock(target, false);
  }

  function commitSeekTo(value: number) {
    const target = seekTarget(value);
    pendingSeekRef.current = { target, startedAt: performance.now() };
    setCurrentTime(target);
    anchorDisplayClock(target, false);
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_seek", { position: target })
      .then(applyCommandSnapshot)
      .catch((error: unknown) => {
        pendingSeekRef.current = null;
        reportPlaybackError(error);
      });
  }

  function setVolume(value: number) {
    const nextVolume = Math.min(1, Math.max(0, value));
    setVolumeLevel(nextVolume);
    invalidatePendingSnapshots();
    invoke<MpvSnapshot>("mpv_embed_set_volume", { volume: nextVolume * 100 })
      .then(applyCommandSnapshot)
      .catch(reportPlaybackError);
  }

  const displayTime = snapEndOfMediaPosition(displayPosition, duration, isPlaying);
  const progress = duration > 0 ? Math.min(100, Math.max(0, (displayTime / duration) * 100)) : 0;
  const queueItems = queue.length ? queue : media ? [media] : [];
  const canShowFrames = canDisplayFrames(framesPerSecond, duration);
  const effectiveTimeDisplayMode: TimeDisplayMode = timeDisplayMode === "frames" && canShowFrames ? "frames" : "timecode";
  const totalFrames = canShowFrames ? Math.max(0, Math.floor(duration * framesPerSecond)) : 0;
  const currentFrame = canShowFrames ? Math.min(totalFrames, Math.max(0, Math.floor(displayTime * framesPerSecond))) : 0;
  const currentTransportLabel = effectiveTimeDisplayMode === "frames" ? formatFrameCount(currentFrame) : formatTimecode(displayTime, duration);
  const durationTransportLabel = effectiveTimeDisplayMode === "frames" ? formatFrameCount(totalFrames) : formatTimecode(duration, duration);
  const currentTimeToggleLabel = canShowFrames ? "Toggle current playback time and frame display" : "Current playback time; frame display unavailable for this media";
  const durationTimeToggleLabel = canShowFrames ? "Toggle total duration and frame display" : "Total duration; frame display unavailable for this media";
  const isChromeHidden = Boolean(media) && !isChromeVisible && !isChromePinned;

  return (
    <main className="app-shell" onKeyDown={recordUserActivity} onPointerDown={recordUserActivity} onPointerMove={recordUserActivity}>
      <section className={`window-shell ${media ? "window-shell--loaded" : ""}`} aria-label="OpenPlayer">
        <section className={`stage ${media ? "stage--loaded" : ""} ${isChromeHidden ? "stage--chrome-hidden" : ""}`} aria-label="Player surface">
          {!media && (
            <div className="empty-open">
              <img className="empty-open-logo" src={openPlayerLogoUrl} alt="" draggable={false} />
              <span>Open media</span>
            </div>
          )}

          <div className="drag-region" data-tauri-drag-region aria-hidden="true" onAuxClick={(event) => event.preventDefault()} onPointerDown={handleDragRegionPointerDown} />

          {resizeRegions.map((region) => (
            <div
              key={region.direction}
              aria-hidden="true"
              className={`resize-region ${region.className}`}
              onPointerDown={(event) => handleResizePointerDown(event, region.direction)}
            />
          ))}

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
              <button
                className="transport-time transport-time--toggle"
                type="button"
                aria-label={currentTimeToggleLabel}
                aria-pressed={effectiveTimeDisplayMode === "frames"}
                onClick={toggleTimeDisplayMode}
                disabled={!canShowFrames}
              >
                {currentTransportLabel}
              </button>
              <input
                className="seek-slider"
                type="range"
                min="0"
                max={duration || 0}
                step="any"
                value={displayTime}
                aria-label="Seek playback position"
                style={{ "--progress": `${progress}%` } as CSSProperties}
                onChange={(event) => seekTo(Number(event.currentTarget.value))}
                onPointerUp={(event) => commitSeekTo(Number(event.currentTarget.value))}
                onKeyUp={(event) => commitSeekTo(Number(event.currentTarget.value))}
                onBlur={(event) => commitSeekTo(Number(event.currentTarget.value))}
                disabled={!media || duration <= 0}
              />
              <button
                className="transport-time transport-time--toggle"
                type="button"
                aria-label={durationTimeToggleLabel}
                aria-pressed={effectiveTimeDisplayMode === "frames"}
                onClick={toggleTimeDisplayMode}
                disabled={!canShowFrames}
              >
                {durationTransportLabel}
              </button>
            </div>

            <div className="control-strip">
              <button type="button" aria-label="Open media" onClick={openNativeMediaFiles} disabled={isPickerOpen}>
                <Icon name="folder" />
              </button>
              <button className="control-primary" type="button" aria-label={isPlaying ? "Pause" : media ? "Play" : "Open media"} onClick={togglePlayback} disabled={!media && isPickerOpen}>
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
