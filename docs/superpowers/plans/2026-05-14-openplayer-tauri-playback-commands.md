# OpenPlayer Tauri Playback Commands Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Tauri playback commands that exercise the Rust `PlaybackService` while keeping the current HTML5 preview renderer.

**Architecture:** The desktop crate owns a private `PreviewPlaybackBackend` that implements `MediaBackend` and mirrors state transitions only. Tauri commands mutate a managed `DesktopPlaybackState` wrapping `PlaybackService<PreviewPlaybackBackend>`, then return serializable DTO snapshots to React. The frontend keeps playing through `<video>` but mirrors open/play/pause/seek/volume actions into Rust.

**Tech Stack:** Rust 2024, Tauri v2 managed state and commands, Serde DTOs, React/TypeScript, existing `openplayer-core` and `openplayer-media` crates.

---

## Scope Check

This plan implements the Tauri command boundary only. It does not integrate `libmpv`, SQLite, native file picker paths, or persistent playlists. The current HTML5 preview remains the visible player.

## File Structure

- Create: `apps/desktop/src-tauri/src/playback.rs` contains preview backend, DTOs, command error mapping, Tauri command functions, and Rust tests.
- Modify: `apps/desktop/src-tauri/Cargo.toml` adds `openplayer-media` because the desktop preview backend needs media DTO/trait types directly.
- Modify: `apps/desktop/src-tauri/src/lib.rs` registers playback state and Tauri commands.
- Modify: `apps/desktop/src/App.tsx` mirrors existing HTML5 playback actions into Tauri playback commands.
- Modify: `apps/desktop/scripts/verify-shell.mjs` asserts frontend command wiring and Tauri command registration.

## Task 1: Add Desktop Playback State And Commands

**Files:**
- Create: `apps/desktop/src-tauri/src/playback.rs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`

- [ ] **Step 1: Add desktop dependency for media types**

Update `apps/desktop/src-tauri/Cargo.toml` dependencies to:

```toml
[dependencies]
openplayer-core = { path = "../../../crates/core" }
openplayer-media = { path = "../../../crates/media" }
openplayer-shared = { path = "../../../crates/shared" }
serde.workspace = true
tauri = { version = "2", features = [] }
```

- [ ] **Step 2: Write failing playback module tests**

Create `apps/desktop/src-tauri/src/playback.rs` with test-first content:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_state_starts_idle() {
        let state = DesktopPlaybackState::default();

        let snapshot = state.snapshot().expect("snapshot");

        assert_eq!(snapshot.status, PlaybackStatusDto::Idle);
        assert_eq!(snapshot.source_label, None);
        assert_eq!(snapshot.position_ms, 0);
        assert_eq!(snapshot.volume_percent, 82);
    }

    #[test]
    fn opening_preview_file_sets_ready_snapshot() {
        let state = DesktopPlaybackState::default();

        let snapshot = state
            .open_preview_source(PlaybackSourceDto {
                kind: PlaybackSourceKindDto::LocalFileLabel,
                value: "movie.mp4".to_string(),
            })
            .expect("open source");

        assert_eq!(snapshot.status, PlaybackStatusDto::Ready);
        assert_eq!(snapshot.source_label, Some("movie.mp4".to_string()));
    }

    #[test]
    fn play_pause_stop_updates_snapshot_status() {
        let state = DesktopPlaybackState::default();
        state
            .open_preview_source(PlaybackSourceDto {
                kind: PlaybackSourceKindDto::LocalFileLabel,
                value: "movie.mp4".to_string(),
            })
            .expect("open source");

        assert_eq!(state.play().expect("play").status, PlaybackStatusDto::Playing);
        assert_eq!(state.pause().expect("pause").status, PlaybackStatusDto::Paused);
        let stopped = state.stop().expect("stop");
        assert_eq!(stopped.status, PlaybackStatusDto::Stopped);
        assert_eq!(stopped.position_ms, 0);
    }

    #[test]
    fn seek_and_volume_update_snapshot_values() {
        let state = DesktopPlaybackState::default();

        let seek_snapshot = state.seek(12_500).expect("seek");
        let volume_snapshot = state.set_volume(40).expect("volume");

        assert_eq!(seek_snapshot.position_ms, 12_500);
        assert_eq!(volume_snapshot.volume_percent, 40);
    }

    #[test]
    fn invalid_volume_returns_stable_error_code() {
        let state = DesktopPlaybackState::default();

        let error = state.set_volume(101).expect_err("invalid volume");

        assert_eq!(error.code, "media.invalidVolume");
    }

    #[test]
    fn play_before_open_returns_invalid_source() {
        let state = DesktopPlaybackState::default();

        let error = state.play().expect_err("no source");

        assert_eq!(error.code, "media.invalidSource");
    }
}
```

- [ ] **Step 3: Wire the module for RED compile**

Add this line near the top of `apps/desktop/src-tauri/src/lib.rs`:

```rust
mod playback;
```

- [ ] **Step 4: Run tests and verify RED**

Run:

```powershell
cargo test -p openplayer-desktop playback
```

Expected: FAIL with missing `DesktopPlaybackState`, DTO, and status types.

- [ ] **Step 5: Implement playback module**

Replace `apps/desktop/src-tauri/src/playback.rs` with:

```rust
use std::sync::Mutex;

use openplayer_core::{CoreError, PlaybackService};
use openplayer_media::{
    MediaBackend, MediaError, MediaSource, MediaTime, PlaybackSnapshot, PlaybackStatus, Volume,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug)]
pub struct DesktopPlaybackState {
    service: Mutex<PlaybackService<PreviewPlaybackBackend>>,
}

impl Default for DesktopPlaybackState {
    fn default() -> Self {
        Self {
            service: Mutex::new(PlaybackService::new(PreviewPlaybackBackend::default())),
        }
    }
}

impl DesktopPlaybackState {
    pub fn snapshot(&self) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        let service = self.lock_service()?;
        Ok(PlaybackSnapshotDto::from(service.snapshot()))
    }

    pub fn open_preview_source(
        &self,
        source: PlaybackSourceDto,
    ) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        let source = source.try_into()?;
        self.with_service(|service| service.open(source))
    }

    pub fn play(&self) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        self.with_service(PlaybackService::play)
    }

    pub fn pause(&self) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        self.with_service(PlaybackService::pause)
    }

    pub fn stop(&self) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        self.with_service(PlaybackService::stop)
    }

    pub fn seek(&self, position_ms: u64) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        self.with_service(|service| service.seek(MediaTime::from_millis(position_ms)))
    }

    pub fn set_volume(&self, percent: u16) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        self.with_service(|service| service.set_volume_percent(percent))
    }

    fn with_service(
        &self,
        command: impl FnOnce(
            &mut PlaybackService<PreviewPlaybackBackend>,
        ) -> Result<PlaybackSnapshot, CoreError>,
    ) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
        let mut service = self.lock_service()?;
        command(&mut service)
            .map(|snapshot| PlaybackSnapshotDto::from(&snapshot))
            .map_err(PlaybackCommandError::from)
    }

    fn lock_service(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, PlaybackService<PreviewPlaybackBackend>>, PlaybackCommandError>
    {
        self.service
            .lock()
            .map_err(|_| PlaybackCommandError::new("state.lockFailed", "Playback state is unavailable"))
    }
}

#[derive(Debug)]
struct PreviewPlaybackBackend {
    snapshot: PlaybackSnapshot,
}

impl Default for PreviewPlaybackBackend {
    fn default() -> Self {
        Self {
            snapshot: PlaybackSnapshot::idle(),
        }
    }
}

impl PreviewPlaybackBackend {
    fn require_source(&self) -> Result<(), MediaError> {
        if self.snapshot.source.is_some() {
            Ok(())
        } else {
            Err(MediaError::InvalidSource("no media opened".to_string()))
        }
    }
}

impl MediaBackend for PreviewPlaybackBackend {
    fn backend_id(&self) -> &'static str {
        "preview"
    }

    fn display_name(&self) -> &'static str {
        "HTML5 Preview Mirror"
    }

    fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
        self.snapshot.source = Some(source);
        self.snapshot.status = PlaybackStatus::Ready;
        self.snapshot.position = MediaTime::ZERO;
        self.snapshot.latest_error = None;
        Ok(self.snapshot.clone())
    }

    fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        self.require_source()?;
        self.snapshot.status = PlaybackStatus::Playing;
        Ok(self.snapshot.clone())
    }

    fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        self.require_source()?;
        self.snapshot.status = PlaybackStatus::Paused;
        Ok(self.snapshot.clone())
    }

    fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        self.require_source()?;
        self.snapshot.status = PlaybackStatus::Stopped;
        self.snapshot.position = MediaTime::ZERO;
        Ok(self.snapshot.clone())
    }

    fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
        self.snapshot.position = position;
        Ok(self.snapshot.clone())
    }

    fn set_volume(&mut self, volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
        self.snapshot.volume = volume;
        Ok(self.snapshot.clone())
    }

    fn snapshot(&self) -> PlaybackSnapshot {
        self.snapshot.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSourceDto {
    pub kind: PlaybackSourceKindDto,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackSourceKindDto {
    LocalFileLabel,
    LocalFolderLabel,
    HttpUrl,
}

impl TryFrom<PlaybackSourceDto> for MediaSource {
    type Error = PlaybackCommandError;

    fn try_from(source: PlaybackSourceDto) -> Result<Self, Self::Error> {
        match source.kind {
            PlaybackSourceKindDto::LocalFileLabel => Ok(MediaSource::local_file(source.value)),
            PlaybackSourceKindDto::LocalFolderLabel => Ok(MediaSource::local_folder(source.value)),
            PlaybackSourceKindDto::HttpUrl => MediaSource::http_url(source.value).map_err(CoreError::from).map_err(PlaybackCommandError::from),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSnapshotDto {
    pub source_label: Option<String>,
    pub status: PlaybackStatusDto,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub volume_percent: u16,
    pub muted: bool,
    pub speed_milli: u32,
    pub latest_error: Option<PlaybackCommandError>,
}

impl From<&PlaybackSnapshot> for PlaybackSnapshotDto {
    fn from(snapshot: &PlaybackSnapshot) -> Self {
        Self {
            source_label: snapshot.source.as_ref().map(MediaSource::location),
            status: PlaybackStatusDto::from(snapshot.status),
            position_ms: snapshot.position.as_millis(),
            duration_ms: snapshot.duration.map(MediaTime::as_millis),
            volume_percent: snapshot.volume.percent(),
            muted: snapshot.muted,
            speed_milli: snapshot.speed.as_milli(),
            latest_error: snapshot
                .latest_error
                .clone()
                .map(CoreError::from)
                .map(PlaybackCommandError::from),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatusDto {
    Idle,
    Loading,
    Ready,
    Playing,
    Paused,
    Stopped,
    Ended,
    Error,
}

impl From<PlaybackStatus> for PlaybackStatusDto {
    fn from(status: PlaybackStatus) -> Self {
        match status {
            PlaybackStatus::Idle => Self::Idle,
            PlaybackStatus::Loading => Self::Loading,
            PlaybackStatus::Ready => Self::Ready,
            PlaybackStatus::Playing => Self::Playing,
            PlaybackStatus::Paused => Self::Paused,
            PlaybackStatus::Stopped => Self::Stopped,
            PlaybackStatus::Ended => Self::Ended,
            PlaybackStatus::Error => Self::Error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackCommandError {
    pub code: String,
    pub message: String,
}

impl PlaybackCommandError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<CoreError> for PlaybackCommandError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::Media(media_error) => Self::from(media_error),
        }
    }
}

impl From<MediaError> for PlaybackCommandError {
    fn from(error: MediaError) -> Self {
        match error {
            MediaError::BackendUnavailable(_) => Self::new("media.backendUnavailable", "Playback backend is unavailable"),
            MediaError::InvalidSource(_) => Self::new("media.invalidSource", "This media source cannot be opened"),
            MediaError::OpenFailed(_) => Self::new("media.openFailed", "The media source could not be opened"),
            MediaError::CommandFailed(_) => Self::new("media.commandFailed", "Playback command failed"),
            MediaError::UnsupportedSource(_) => Self::new("media.unsupportedSource", "This media source is not supported"),
            MediaError::InvalidSeekTarget(_) => Self::new("media.invalidSeekTarget", "The requested position is invalid"),
            MediaError::InvalidVolume(_) => Self::new("media.invalidVolume", "The requested volume is invalid"),
        }
    }
}

#[tauri::command]
pub fn playback_snapshot(
    state: State<'_, DesktopPlaybackState>,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.snapshot()
}

#[tauri::command]
pub fn playback_open_preview_source(
    state: State<'_, DesktopPlaybackState>,
    source: PlaybackSourceDto,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.open_preview_source(source)
}

#[tauri::command]
pub fn playback_play(
    state: State<'_, DesktopPlaybackState>,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.play()
}

#[tauri::command]
pub fn playback_pause(
    state: State<'_, DesktopPlaybackState>,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.pause()
}

#[tauri::command]
pub fn playback_stop(
    state: State<'_, DesktopPlaybackState>,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.stop()
}

#[tauri::command]
pub fn playback_seek(
    state: State<'_, DesktopPlaybackState>,
    position_ms: u64,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.seek(position_ms)
}

#[tauri::command]
pub fn playback_set_volume(
    state: State<'_, DesktopPlaybackState>,
    percent: u16,
) -> Result<PlaybackSnapshotDto, PlaybackCommandError> {
    state.set_volume(percent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_state_starts_idle() {
        let state = DesktopPlaybackState::default();

        let snapshot = state.snapshot().expect("snapshot");

        assert_eq!(snapshot.status, PlaybackStatusDto::Idle);
        assert_eq!(snapshot.source_label, None);
        assert_eq!(snapshot.position_ms, 0);
        assert_eq!(snapshot.volume_percent, 82);
    }

    #[test]
    fn opening_preview_file_sets_ready_snapshot() {
        let state = DesktopPlaybackState::default();

        let snapshot = state
            .open_preview_source(PlaybackSourceDto {
                kind: PlaybackSourceKindDto::LocalFileLabel,
                value: "movie.mp4".to_string(),
            })
            .expect("open source");

        assert_eq!(snapshot.status, PlaybackStatusDto::Ready);
        assert_eq!(snapshot.source_label, Some("movie.mp4".to_string()));
    }

    #[test]
    fn play_pause_stop_updates_snapshot_status() {
        let state = DesktopPlaybackState::default();
        state
            .open_preview_source(PlaybackSourceDto {
                kind: PlaybackSourceKindDto::LocalFileLabel,
                value: "movie.mp4".to_string(),
            })
            .expect("open source");

        assert_eq!(state.play().expect("play").status, PlaybackStatusDto::Playing);
        assert_eq!(state.pause().expect("pause").status, PlaybackStatusDto::Paused);
        let stopped = state.stop().expect("stop");
        assert_eq!(stopped.status, PlaybackStatusDto::Stopped);
        assert_eq!(stopped.position_ms, 0);
    }

    #[test]
    fn seek_and_volume_update_snapshot_values() {
        let state = DesktopPlaybackState::default();

        let seek_snapshot = state.seek(12_500).expect("seek");
        let volume_snapshot = state.set_volume(40).expect("volume");

        assert_eq!(seek_snapshot.position_ms, 12_500);
        assert_eq!(volume_snapshot.volume_percent, 40);
    }

    #[test]
    fn invalid_volume_returns_stable_error_code() {
        let state = DesktopPlaybackState::default();

        let error = state.set_volume(101).expect_err("invalid volume");

        assert_eq!(error.code, "media.invalidVolume");
    }

    #[test]
    fn play_before_open_returns_invalid_source() {
        let state = DesktopPlaybackState::default();

        let error = state.play().expect_err("no source");

        assert_eq!(error.code, "media.invalidSource");
    }
}
```

- [ ] **Step 6: Run playback tests and verify GREEN**

Run:

```powershell
cargo test -p openplayer-desktop playback
```

Expected: PASS for the six playback module tests.

## Task 2: Register Tauri Playback State And Commands

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing registration assertions**

Update `apps/desktop/scripts/verify-shell.mjs` to read `src-tauri/src/lib.rs` and assert the command names are registered. Add these assertions near the existing command assertions:

```js
assert.match(tauriLibSource, /DesktopPlaybackState::default\(\)/, "desktop app must manage playback state");
assert.match(tauriLibSource, /playback_snapshot/, "Tauri must register playback snapshot command");
assert.match(tauriLibSource, /playback_open_preview_source/, "Tauri must register preview open command");
assert.match(tauriLibSource, /playback_play/, "Tauri must register playback play command");
assert.match(tauriLibSource, /playback_pause/, "Tauri must register playback pause command");
assert.match(tauriLibSource, /playback_stop/, "Tauri must register playback stop command");
assert.match(tauriLibSource, /playback_seek/, "Tauri must register playback seek command");
assert.match(tauriLibSource, /playback_set_volume/, "Tauri must register playback volume command");
```

Also add this file read near the existing `mainSource` read:

```js
const tauriLibSource = await readFile(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");
```

- [ ] **Step 2: Run shell verification and verify RED**

Run:

```powershell
npm run verify:shell
```

Working directory: `apps/desktop`

Expected: FAIL because playback state is not managed and playback commands are not registered.

- [ ] **Step 3: Register playback module, state, and commands**

Update `apps/desktop/src-tauri/src/lib.rs` imports to include:

```rust
mod playback;

use openplayer_shared::AppInfo;
use playback::{
    playback_open_preview_source, playback_pause, playback_play, playback_seek,
    playback_set_volume, playback_snapshot, playback_stop, DesktopPlaybackState,
};
use tauri::Window;
```

Update `run()` to:

```rust
pub fn run() {
    tauri::Builder::default()
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

- [ ] **Step 4: Run shell verification and Rust desktop tests**

Run:

```powershell
npm run verify:shell
cargo test -p openplayer-desktop
```

Working directories:

- `npm run verify:shell`: `apps/desktop`
- `cargo test -p openplayer-desktop`: repository root

Expected: both PASS.

## Task 3: Mirror Frontend Preview Actions Into Tauri Commands

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/scripts/verify-shell.mjs`

- [ ] **Step 1: Add failing frontend wiring assertions**

Add these assertions to `apps/desktop/scripts/verify-shell.mjs`:

```js
assert.match(appSource, /type PlaybackSourceDto/, "frontend must define playback source DTO");
assert.match(appSource, /type PlaybackSnapshotDto/, "frontend must define playback snapshot DTO");
assert.match(appSource, /runPlaybackCommand/, "frontend must use a playback command helper");
assert.match(appSource, /playback_open_preview_source/, "opening media must mirror to Rust playback state");
assert.match(appSource, /playback_play/, "play action must mirror to Rust playback state");
assert.match(appSource, /playback_pause/, "pause action must mirror to Rust playback state");
assert.match(appSource, /playback_seek/, "seek action must mirror to Rust playback state");
assert.match(appSource, /playback_set_volume/, "volume action must mirror to Rust playback state");
```

- [ ] **Step 2: Run shell verification and verify RED**

Run:

```powershell
npm run verify:shell
```

Working directory: `apps/desktop`

Expected: FAIL because frontend does not call playback commands yet.

- [ ] **Step 3: Add TypeScript DTOs and command helpers**

In `apps/desktop/src/App.tsx`, add these types below the existing `MediaItem` type:

```ts
type PlaybackSourceDto = {
  kind: "localFileLabel" | "localFolderLabel" | "httpUrl";
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
```

Add these helpers below `runWindowCommand`:

```ts
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
```

- [ ] **Step 4: Add React state for Rust playback snapshot**

Inside `App()`, add state after existing `playbackError` state:

```ts
const [playbackSnapshot, setPlaybackSnapshot] = useState<PlaybackSnapshotDto | null>(null);
```

Add this helper inside `App()` before `openFiles`:

```ts
function mirrorPlaybackCommand(command: string, args?: Record<string, unknown>) {
  runPlaybackCommand(command, args)
    .then(setPlaybackSnapshot)
    .catch((error: unknown) => {
      setPlaybackError(error instanceof Error ? error.message : String(error));
    });
}
```

- [ ] **Step 5: Mirror open/play/pause/seek/volume actions**

In `openFiles`, after `setPlaybackError(null);`, add:

```ts
mirrorPlaybackCommand("playback_open_preview_source", {
  source: { kind: "localFileLabel", value: file.name } satisfies PlaybackSourceDto,
});
```

In `togglePlayback`, replace the play branch with:

```ts
video
  .play()
  .then(() => mirrorPlaybackCommand("playback_play"))
  .catch((error: unknown) => {
    setPlaybackError(error instanceof Error ? error.message : String(error));
  });
```

In the pause branch, after `video.pause();`, add:

```ts
mirrorPlaybackCommand("playback_pause");
```

In `seekTo`, after `setCurrentTime(value);`, add:

```ts
mirrorPlaybackCommand("playback_seek", { positionMs: Math.round(value * 1000) });
```

In `setVolume`, after updating the video volume, add:

```ts
mirrorPlaybackCommand("playback_set_volume", { percent: Math.round(nextVolume * 100) });
```

In the video `onEnded` handler, replace `onEnded={() => setIsPlaying(false)}` with:

```tsx
onEnded={() => {
  setIsPlaying(false);
  mirrorPlaybackCommand("playback_stop");
}}
```

To avoid an unused state warning, use `playbackSnapshot` when computing `queueItems`:

```ts
const queueItems = media ? [playbackSnapshot?.sourceLabel ?? media.name] : ["No media loaded"];
```

- [ ] **Step 6: Run frontend verification and build**

Run:

```powershell
npm run verify:shell
npm run build
```

Working directory: `apps/desktop`

Expected: both PASS.

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

- [ ] **Step 3: Inspect final diff**

Run:

```powershell
git diff -- apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/src/lib.rs apps/desktop/src-tauri/src/playback.rs apps/desktop/src/App.tsx apps/desktop/scripts/verify-shell.mjs docs/superpowers/specs/2026-05-14-openplayer-tauri-playback-commands-design.md docs/superpowers/plans/2026-05-14-openplayer-tauri-playback-commands.md
```

Expected: diff only contains the Tauri playback command boundary, frontend command mirroring, verification script updates, and the spec/plan docs.

- [ ] **Step 4: Optional commit if requested by the user**

If the user explicitly requests a commit for this execution session, run:

```powershell
git add apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/src/lib.rs apps/desktop/src-tauri/src/playback.rs apps/desktop/src/App.tsx apps/desktop/scripts/verify-shell.mjs docs/superpowers/specs/2026-05-14-openplayer-tauri-playback-commands-design.md docs/superpowers/plans/2026-05-14-openplayer-tauri-playback-commands.md
git commit -m "feat: add Tauri playback commands"
```

Expected: commit succeeds. If the user did not request commits, do not commit.

## Self-Review

- Spec coverage: preview backend, managed playback state, Tauri commands, DTOs, frontend mirroring, error codes, and verification are covered.
- Out of scope: real `libmpv`, SQLite, native file picker, and replacing HTML5 playback are not implemented.
- Placeholder scan: no `TBD`, `TODO`, or unspecified implementation steps remain.
- Type consistency: `PlaybackSourceDto`, `PlaybackSourceKindDto`, `PlaybackSnapshotDto`, `PlaybackStatusDto`, `PlaybackCommandError`, `DesktopPlaybackState`, and command names are consistent across tasks.
