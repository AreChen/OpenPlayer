# OpenPlayer Native File Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a native multi-file picker, real local-path queue state, playlist item selection, and queue auto-advance while keeping the current HTML5 preview renderer.

**Architecture:** React owns picker interaction and queue state. Tauri dialog returns local paths, React converts those paths to preview URLs with `convertFileSrc`, and the existing Tauri playback command mirror receives real local paths through a new `localFilePath` source kind. Rust remains a state mirror for this slice; no native decoding or `libmpv` integration happens here.

**Tech Stack:** Tauri v2, `@tauri-apps/plugin-dialog`, `tauri-plugin-dialog`, React 19, TypeScript, Rust 2024, existing `openplayer-core` and `openplayer-media` playback service.

---

## Scope Check

This plan implements one slice: native multi-file picking plus queue behavior. It does not implement folder picking, persistent playlists, SQLite recent/progress state, real `libmpv` rendering, or native drag/drop path events.

## File Structure

- Modify: `apps/desktop/package.json` adds the Tauri dialog JavaScript plugin.
- Modify: `apps/desktop/package-lock.json` is updated by `npm install`.
- Modify: `apps/desktop/src-tauri/Cargo.toml` adds the Rust dialog plugin.
- Modify: `apps/desktop/src-tauri/src/lib.rs` registers the dialog plugin.
- Modify: `apps/desktop/src-tauri/capabilities/default.json` grants file-open dialog permission.
- Modify: `apps/desktop/src-tauri/tauri.conf.json` enables asset protocol preview URLs.
- Modify: `apps/desktop/src-tauri/src/playback.rs` adds the `localFilePath` playback source variant and tests.
- Modify: `apps/desktop/src/App.tsx` replaces hidden browser file open with native picker queue behavior while keeping drag/drop preview support.
- Modify: `apps/desktop/src/styles.css` makes playlist entries clickable and marks the active item.
- Modify: `apps/desktop/scripts/verify-shell.mjs` checks plugin wiring, permissions, asset protocol, queue state, native picker usage, and auto-advance wiring.

## Task 1: Add Dialog Plugin, Permissions, And Asset Protocol

**Files:**
- Modify: `apps/desktop/scripts/verify-shell.mjs`
- Modify: `apps/desktop/package.json`
- Modify: `apps/desktop/package-lock.json`
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/capabilities/default.json`
- Modify: `apps/desktop/src-tauri/tauri.conf.json`

- [ ] **Step 1: Add failing shell assertions for native picker infrastructure**

In `apps/desktop/scripts/verify-shell.mjs`, add this constant after `const [mainWindow] = config.app.windows;`:

```js
const assetProtocol = config.app.security.assetProtocol;
```

Replace the current local file support assertion:

```js
assert.match(appSource, /type="file"/, "player shell must expose local file open support");
```

with these native picker assertions:

```js
assert.ok(packageJson.dependencies["@tauri-apps/plugin-dialog"], "desktop package must depend on Tauri dialog plugin");
assert.equal(assetProtocol?.enable, true, "Tauri asset protocol must be enabled for local preview URLs");
assert.ok(assetProtocol?.scope?.includes("**"), "asset protocol scope must allow user-selected local media paths");
assert.ok(capability.permissions.includes("dialog:allow-open"), "capability must allow native file-open dialogs");
assert.match(tauriLibSource, /tauri_plugin_dialog::init\(\)/, "desktop app must register the dialog plugin");
assert.match(appSource, /from "@tauri-apps\/plugin-dialog"/, "frontend must import the Tauri dialog plugin");
assert.match(appSource, /convertFileSrc/, "frontend must convert native paths into preview URLs");
assert.match(appSource, /openNativeMediaFiles/, "open control must use the native media picker");
assert.doesNotMatch(appSource, /fileInputRef/, "open control must not route through the hidden browser file input");
```

- [ ] **Step 2: Run shell verification and verify RED**

Run:

```powershell
npm run verify:shell
```

Working directory: `apps/desktop`

Expected: FAIL because the dialog dependency, plugin registration, permission, asset protocol, and frontend native picker usage are not implemented yet.

- [ ] **Step 3: Add the JavaScript dialog dependency**

Run:

```powershell
npm install @tauri-apps/plugin-dialog@^2.0.0
```

Working directory: `apps/desktop`

Expected: `package.json` contains `"@tauri-apps/plugin-dialog": "^2.0.0"` under dependencies, and `package-lock.json` is updated.

- [ ] **Step 4: Add the Rust dialog dependency**

In `apps/desktop/src-tauri/Cargo.toml`, update dependencies to include `tauri-plugin-dialog`:

```toml
[dependencies]
openplayer-core = { path = "../../../crates/core" }
openplayer-media = { path = "../../../crates/media" }
openplayer-shared = { path = "../../../crates/shared" }
serde.workspace = true
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
```

- [ ] **Step 5: Register the dialog plugin in Tauri**

In `apps/desktop/src-tauri/src/lib.rs`, update `run()` so the builder registers the plugin before managed state:

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(DesktopPlaybackState::default())
        .invoke_handler(tauri::generate_handler![
            app_health_command,
            window_minimize,
            window_toggle_maximize,
            window_close,
            playback_snapshot,
            playback_open_preview_source,
            playback_play,
            playback_pause,
            playback_stop,
            playback_seek,
            playback_set_volume
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}
```

- [ ] **Step 6: Grant dialog open permission**

Replace `apps/desktop/src-tauri/capabilities/default.json` with:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default permissions for OpenPlayer desktop shell",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-is-fullscreen",
    "core:window:allow-set-fullscreen",
    "dialog:allow-open"
  ]
}
```

- [ ] **Step 7: Enable asset protocol for local preview URLs**

Replace `apps/desktop/src-tauri/tauri.conf.json` with:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "OpenPlayer",
  "version": "0.1.0",
  "identifier": "dev.openplayer.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://127.0.0.1:23142",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "OpenPlayer",
        "url": "index.html",
        "width": 1280,
        "height": 720,
        "minWidth": 960,
        "minHeight": 540,
        "resizable": true,
        "center": true,
        "decorations": false,
        "transparent": false,
        "shadow": true
      }
    ],
    "security": {
      "csp": null,
      "assetProtocol": {
        "enable": true,
        "scope": ["**"]
      }
    }
  },
  "bundle": {
    "active": false,
    "targets": "all"
  }
}
```

- [ ] **Step 8: Run infrastructure verification**

Run:

```powershell
npm run verify:shell
cargo check -p openplayer-desktop
```

Working directories:

- `npm run verify:shell`: `apps/desktop`
- `cargo check -p openplayer-desktop`: repository root

Expected: `npm run verify:shell` still FAILS on frontend native picker assertions until Task 3. `cargo check -p openplayer-desktop` PASSES after Cargo downloads/builds `tauri-plugin-dialog`.

- [ ] **Step 9: Commit infrastructure changes**

Run:

```powershell
git add apps/desktop/package.json apps/desktop/package-lock.json apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/src/lib.rs apps/desktop/src-tauri/capabilities/default.json apps/desktop/src-tauri/tauri.conf.json apps/desktop/scripts/verify-shell.mjs Cargo.lock
git commit -m "feat: add native picker infrastructure"
```

Expected: commit succeeds and contains only dialog plugin, capability, asset protocol, lockfile, and shell verification changes.

## Task 2: Add Real Local Path Playback Source Kind

**Files:**
- Modify: `apps/desktop/src-tauri/src/playback.rs`

- [ ] **Step 1: Add failing Rust tests for `localFilePath`**

Append these tests inside the existing `#[cfg(test)] mod tests` in `apps/desktop/src-tauri/src/playback.rs`:

```rust
#[test]
fn source_dto_converts_local_file_path() {
    let source = MediaSource::try_from(PlaybackSourceDto {
        kind: PlaybackSourceKindDto::LocalFilePath,
        value: r"C:\media\movie.mp4".to_string(),
    })
    .expect("source");

    assert_eq!(source.location(), r"C:\media\movie.mp4");
}

#[test]
fn opening_local_file_path_sets_real_path_snapshot_label() {
    let state = DesktopPlaybackState::default();

    let snapshot = state
        .open_preview_source(PlaybackSourceDto {
            kind: PlaybackSourceKindDto::LocalFilePath,
            value: r"C:\media\movie.mp4".to_string(),
        })
        .expect("open source");

    assert_eq!(snapshot.status, PlaybackStatusDto::Ready);
    assert_eq!(snapshot.source_label, Some(r"C:\media\movie.mp4".to_string()));
}
```

- [ ] **Step 2: Run playback tests and verify RED**

Run:

```powershell
cargo test -p openplayer-desktop playback::tests::source_dto_converts_local_file_path playback::tests::opening_local_file_path_sets_real_path_snapshot_label
```

Working directory: repository root

Expected: FAIL to compile because `PlaybackSourceKindDto::LocalFilePath` does not exist yet.

- [ ] **Step 3: Add the Rust DTO variant and conversion**

In `apps/desktop/src-tauri/src/playback.rs`, update `PlaybackSourceKindDto` to:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackSourceKindDto {
    LocalFilePath,
    LocalFileLabel,
    LocalFolderLabel,
    HttpUrl,
}
```

Update the `TryFrom<PlaybackSourceDto> for MediaSource` match to:

```rust
impl TryFrom<PlaybackSourceDto> for MediaSource {
    type Error = PlaybackCommandError;

    fn try_from(source: PlaybackSourceDto) -> Result<Self, Self::Error> {
        match source.kind {
            PlaybackSourceKindDto::LocalFilePath | PlaybackSourceKindDto::LocalFileLabel => {
                Ok(MediaSource::local_file(source.value))
            }
            PlaybackSourceKindDto::LocalFolderLabel => Ok(MediaSource::local_folder(source.value)),
            PlaybackSourceKindDto::HttpUrl => MediaSource::http_url(source.value)
                .map_err(CoreError::from)
                .map_err(PlaybackCommandError::from),
        }
    }
}
```

- [ ] **Step 4: Run playback tests and verify GREEN**

Run:

```powershell
cargo test -p openplayer-desktop playback
```

Working directory: repository root

Expected: PASS for all desktop playback tests, including the new `localFilePath` tests.

- [ ] **Step 5: Commit playback source DTO changes**

Run:

```powershell
git add apps/desktop/src-tauri/src/playback.rs
git commit -m "feat: support native file path playback sources"
```

Expected: commit succeeds and contains only `playback.rs` changes.

## Task 3: Add Native Picker Queue, Playlist Selection, And Auto-Advance

**Files:**
- Modify: `apps/desktop/scripts/verify-shell.mjs`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/styles.css`

- [ ] **Step 1: Add failing shell assertions for queue behavior**

In `apps/desktop/scripts/verify-shell.mjs`, add these assertions near the existing playback command assertions:

```js
assert.match(appSource, /type MediaSourceKind = "localFilePath" \| "localFileLabel"/, "frontend must model native and preview-only media sources");
assert.match(appSource, /kind: "localFilePath" \| "localFileLabel" \| "localFolderLabel" \| "httpUrl"/, "frontend playback DTO must include localFilePath");
assert.match(appSource, /const \[queue, setQueue\]/, "frontend must keep queue state");
assert.match(appSource, /const \[currentIndex, setCurrentIndex\]/, "frontend must track the current queue index");
assert.match(appSource, /mediaItemFromNativePath/, "frontend must build queue items from native paths");
assert.match(appSource, /mediaItemFromBrowserFile/, "frontend must keep drag-and-drop preview file support");
assert.match(appSource, /open\(\{[\s\S]*multiple:\s*true/, "native picker must allow selecting multiple files");
assert.match(appSource, /chooseQueueItem/, "playlist drawer must allow choosing queued files");
assert.match(appSource, /advanceToNextQueueItem/, "player must advance to the next queued file on media end");
assert.match(appSource, /pendingAutoplayRef/, "auto-advance must remember when to start the next item");
assert.match(appSource, /onCanPlay=\{handleCanPlay\}/, "auto-advance playback must wait until the next preview can play");
assert.match(styles, /playlist-item--active/, "playlist styles must mark the active queue item");
```

Remove this assertion if it still exists:

```js
assert.match(appSource, /type="file"/, "player shell must expose local file open support");
```

- [ ] **Step 2: Run shell verification and verify RED**

Run:

```powershell
npm run verify:shell
```

Working directory: `apps/desktop`

Expected: FAIL because native picker queue behavior is not implemented yet.

- [ ] **Step 3: Replace frontend app logic with native picker queue behavior**

Replace `apps/desktop/src/App.tsx` with:

```tsx
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

type DragIntent = {
  pointerId: number;
  startX: number;
  startY: number;
};

type WindowCommand = "window_minimize" | "window_toggle_maximize" | "window_close";
type IconName = "close" | "folder" | "list" | "maximize" | "minimize" | "pause" | "play" | "restart" | "volume";

const playableExtensions = ["3gp", "aac", "avi", "flac", "m4a", "m4v", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "oga", "ogg", "ogv", "opus", "wav", "webm"];
const playableNamePattern = new RegExp(`\\.(${playableExtensions.join("|")})$`, "i");

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
  return path.replaceAll("\\", "/").split("/").filter(Boolean).pop() ?? path;
}

function mediaItemFromNativePath(path: string, index: number): MediaItem {
  const name = fileNameFromPath(path);
  return {
    id: `native:${path}:${index}`,
    name,
    path,
    type: "media file",
    size: null,
    url: convertFileSrc(path),
    sourceKind: "localFilePath",
  };
}

function mediaItemFromBrowserFile(file: File, index: number): MediaItem {
  return {
    id: `preview:${file.name}:${file.size}:${file.lastModified}:${index}`,
    name: file.name,
    path: null,
    type: file.type || "media file",
    size: file.size,
    url: URL.createObjectURL(file),
    sourceKind: "localFileLabel",
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
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const dragIntentRef = useRef<DragIntent | null>(null);
  const playbackCommandIdRef = useRef(0);
  const pendingAutoplayRef = useRef(false);
  const media = currentIndex === null ? null : (queue[currentIndex] ?? null);

  useEffect(() => {
    return () => revokePreviewUrls(queue);
  }, [queue]);

  useEffect(() => {
    if (!media) {
      return;
    }

    setCurrentTime(0);
    setDuration(0);
    setIsPlaying(false);
    setPlaybackError(null);
    mirrorOpenMedia(media);
  }, [media?.id]);

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
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: "Media", extensions: playableExtensions }],
      });
      const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
      if (!paths.length) {
        return;
      }

      const nextQueue = paths.filter(isSupportedMediaName).map(mediaItemFromNativePath);
      if (!nextQueue.length) {
        setPlaybackError("No supported media file was found in that selection.");
        return;
      }

      setPlaybackError(null);
      replaceQueue(nextQueue);
    } catch (error: unknown) {
      setPlaybackError(error instanceof Error ? error.message : String(error));
    }
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
      mirrorPlaybackCommand("playback_seek", { positionMs: Math.round(value * 1000) });
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
              ref={videoRef}
              className="media-view"
              src={media.url}
              onCanPlay={handleCanPlay}
              onLoadedMetadata={(event) => {
                event.currentTarget.volume = volumeLevel;
                setDuration(event.currentTarget.duration);
              }}
              onTimeUpdate={(event) => setCurrentTime(event.currentTarget.currentTime)}
              onPlay={() => setIsPlaying(true)}
              onPause={() => setIsPlaying(false)}
              onEnded={() => {
                setIsPlaying(false);
                if (!advanceToNextQueueItem()) {
                  mirrorPlaybackCommand("playback_stop");
                }
              }}
              onError={() => setPlaybackError("This file could not be decoded by the current preview renderer.")}
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
            </aside>
          )}
        </section>
      </section>
    </main>
  );
}

export default App;
```

- [ ] **Step 4: Update playlist styles for clickable items**

In `apps/desktop/src/styles.css`, delete the `.media-file-input` block:

```css
.media-file-input {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  clip-path: inset(50%);
}
```

Replace the playlist item styles:

```css
.playlist-drawer li {
  overflow: hidden;
  border-radius: 8px;
  color: var(--muted);
  padding: 9px 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.playlist-drawer li:first-child {
  background: rgba(236, 231, 221, 0.08);
  color: var(--text);
}
```

with:

```css
.playlist-drawer li {
  min-width: 0;
}

.playlist-item {
  display: block;
  width: 100%;
  overflow: hidden;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  padding: 9px 10px;
  text-align: left;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.playlist-item span {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.playlist-item:hover,
.playlist-item--active {
  background: rgba(236, 231, 221, 0.08);
  color: var(--text);
}
```

- [ ] **Step 5: Run frontend verification and build**

Run:

```powershell
npm run verify:shell
npm run build
```

Working directory: `apps/desktop`

Expected: both PASS.

- [ ] **Step 6: Commit frontend queue changes**

Run:

```powershell
git add apps/desktop/src/App.tsx apps/desktop/src/styles.css apps/desktop/scripts/verify-shell.mjs
git commit -m "feat: add native file queue"
```

Expected: commit succeeds and contains only frontend queue, playlist, auto-advance, and shell verification changes.

## Task 4: Final Verification

**Files:**
- No code changes expected.

- [ ] **Step 1: Run Rust formatting, linting, and tests**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Working directory: repository root

Expected: all PASS.

- [ ] **Step 2: Run frontend verification and build**

Run:

```powershell
npm run verify:shell
npm run build
```

Working directory: `apps/desktop`

Expected: both PASS.

- [ ] **Step 3: Inspect final status and diff**

Run:

```powershell
git status --short --branch
git diff --stat HEAD~3..HEAD
```

Working directory: repository root

Expected: working tree is clean if task commits were created. The three implementation commits should cover infrastructure, playback DTO, and frontend queue behavior.

- [ ] **Step 4: Optional manual smoke test**

Run:

```powershell
npm run tauri:dev
```

Working directory: `apps/desktop`

Expected manual behavior:

- Click the open-media icon.
- Native file picker opens.
- Select at least two local media files.
- First selected file appears in the player.
- Playlist drawer lists every selected file.
- Clicking a playlist item loads that item.
- When a playable file ends, the next queued file loads and attempts to play.
- Dragging a file onto the stage still loads a preview item.

## Self-Review

- Spec coverage: native multi-file picker, real local paths, queue state, first-file open, playlist selection, auto-advance, `localFilePath`, drag/drop preservation, dialog permission, asset protocol, and final verification are covered.
- Placeholder scan: no `TBD`, `TODO`, unspecified tests, or vague implementation steps remain.
- Type consistency: `MediaSourceKind`, `MediaItem`, `PlaybackSourceDto.kind`, `PlaybackSourceKindDto::LocalFilePath`, `openNativeMediaFiles`, `chooseQueueItem`, `advanceToNextQueueItem`, and `pendingAutoplayRef` are used consistently across tasks.
- Scope check: SQLite, `libmpv`, folder picking, persistent playlists, and native file-drop events remain out of scope.
