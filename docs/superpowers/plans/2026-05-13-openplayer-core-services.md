# OpenPlayer Core Services Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the backend-neutral playback contract and a mock-tested core playback service for Phase 2.

**Architecture:** `crates/media` owns source, state, event, error, and backend trait types. `crates/core` owns `PlaybackService<B: MediaBackend>`, maps backend errors into core errors, and preserves last-known playback snapshots for later Tauri IPC. No UI, SQLite, or real `libmpv` integration happens in this plan.

**Tech Stack:** Rust 2024 workspace, `thiserror`, `url`, existing `openplayer-media`, `openplayer-core`, and `openplayer-mpv` crates.

---

## Scope Check

The approved spec is focused on core playback services only. It excludes SQLite, Tauri playback commands, UI rewiring, and real `libmpv`; those will get separate plans.

## File Structure

- Modify: `crates/media/src/lib.rs` defines media sources, playback value types, events, errors, snapshots, and the command-capable `MediaBackend` trait.
- Modify: `crates/mpv/src/lib.rs` keeps `MpvBackendDescriptor` compiling as an identity-only backend descriptor by returning backend-unavailable errors for playback commands.
- Modify: `crates/core/Cargo.toml` adds `openplayer-media` and `thiserror` dependencies.
- Modify: `crates/core/src/lib.rs` adds `CoreError` and `PlaybackService<B>` while preserving `app_info()`.

## Task 1: Add Media Source And Playback Value Types

**Files:**
- Modify: `crates/media/src/lib.rs`

- [ ] **Step 1: Write failing tests for media source and value types**

Use `apply_patch` to append these tests inside the existing `#[cfg(test)] mod tests` in `crates/media/src/lib.rs`:

```diff
*** Begin Patch
*** Update File: crates/media/src/lib.rs
@@
     fn backend_info_is_derived_from_trait() {
         let info = MediaBackendInfo::from_backend(&TestBackend);
 
         assert_eq!(info.backend_id, "test");
         assert_eq!(info.display_name, "Test Backend");
     }
+
+    #[test]
+    fn media_source_accepts_http_and_https_urls() {
+        let http = MediaSource::http_url("http://example.test/movie.mp4").expect("http source");
+        let https = MediaSource::http_url("https://example.test/movie.mp4").expect("https source");
+
+        assert_eq!(http.location(), "http://example.test/movie.mp4");
+        assert_eq!(https.location(), "https://example.test/movie.mp4");
+    }
+
+    #[test]
+    fn media_source_rejects_unsupported_urls() {
+        let error = MediaSource::http_url("ftp://example.test/movie.mp4").expect_err("unsupported scheme");
+
+        assert_eq!(error, MediaError::UnsupportedSource("ftp://example.test/movie.mp4".to_string()));
+    }
+
+    #[test]
+    fn playback_snapshot_starts_idle_with_defaults() {
+        let snapshot = PlaybackSnapshot::idle();
+
+        assert_eq!(snapshot.status, PlaybackStatus::Idle);
+        assert_eq!(snapshot.position, MediaTime::ZERO);
+        assert_eq!(snapshot.duration, None);
+        assert_eq!(snapshot.volume, Volume::DEFAULT);
+        assert_eq!(snapshot.speed, PlaybackSpeed::NORMAL);
+        assert!(!snapshot.muted);
+    }
+
+    #[test]
+    fn volume_rejects_values_above_100_percent() {
+        let error = Volume::from_percent(101).expect_err("invalid volume");
+
+        assert_eq!(error, MediaError::InvalidVolume(101));
+    }
 }
*** End Patch
```

- [ ] **Step 2: Run media tests and verify they fail**

Run:

```powershell
cargo test -p openplayer-media
```

Expected: FAIL with missing types such as `MediaSource`, `PlaybackSnapshot`, `PlaybackStatus`, `MediaTime`, `Volume`, and `PlaybackSpeed`.

- [ ] **Step 3: Implement media value types**

Replace `crates/media/src/lib.rs` with:

```rust
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaSource {
    LocalFile(PathBuf),
    LocalFolder(PathBuf),
    HttpUrl(String),
}

impl MediaSource {
    pub fn local_file(path: impl Into<PathBuf>) -> Self {
        Self::LocalFile(path.into())
    }

    pub fn local_folder(path: impl Into<PathBuf>) -> Self {
        Self::LocalFolder(path.into())
    }

    pub fn http_url(url: impl Into<String>) -> Result<Self, MediaError> {
        let url = url.into();
        if url.starts_with("http://") || url.starts_with("https://") {
            Ok(Self::HttpUrl(url))
        } else {
            Err(MediaError::UnsupportedSource(url))
        }
    }

    pub fn location(&self) -> String {
        match self {
            Self::LocalFile(path) | Self::LocalFolder(path) => path.display().to_string(),
            Self::HttpUrl(url) => url.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MediaTime {
    millis: u64,
}

impl MediaTime {
    pub const ZERO: Self = Self { millis: 0 };

    pub const fn from_millis(millis: u64) -> Self {
        Self { millis }
    }

    pub const fn as_millis(self) -> u64 {
        self.millis
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume {
    percent: u8,
}

impl Volume {
    pub const DEFAULT: Self = Self { percent: 82 };

    pub fn from_percent(percent: u16) -> Result<Self, MediaError> {
        if percent <= 100 {
            Ok(Self {
                percent: percent as u8,
            })
        } else {
            Err(MediaError::InvalidVolume(percent))
        }
    }

    pub const fn percent(self) -> u8 {
        self.percent
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackSpeed {
    milli: u16,
}

impl PlaybackSpeed {
    pub const NORMAL: Self = Self { milli: 1000 };

    pub const fn from_milli(milli: u16) -> Self {
        Self { milli }
    }

    pub const fn as_milli(self) -> u16 {
        self.milli
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackStatus {
    Idle,
    Loading,
    Ready,
    Playing,
    Paused,
    Stopped,
    Ended,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackSnapshot {
    pub source: Option<MediaSource>,
    pub status: PlaybackStatus,
    pub position: MediaTime,
    pub duration: Option<MediaTime>,
    pub volume: Volume,
    pub muted: bool,
    pub speed: PlaybackSpeed,
    pub latest_error: Option<MediaError>,
}

impl PlaybackSnapshot {
    pub fn idle() -> Self {
        Self {
            source: None,
            status: PlaybackStatus::Idle,
            position: MediaTime::ZERO,
            duration: None,
            volume: Volume::DEFAULT,
            muted: false,
            speed: PlaybackSpeed::NORMAL,
            latest_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackEvent {
    StateChanged(PlaybackSnapshot),
    PositionChanged(MediaTime),
    MediaOpened(MediaSource),
    MediaEnded,
    BackendError(MediaError),
}

pub trait MediaBackend: Send + Sync {
    fn backend_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaBackendInfo {
    pub backend_id: String,
    pub display_name: String,
}

impl MediaBackendInfo {
    pub fn from_backend(backend: &dyn MediaBackend) -> Self {
        Self {
            backend_id: backend.backend_id().to_string(),
            display_name: backend.display_name().to_string(),
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MediaError {
    #[error("media backend is unavailable: {0}")]
    BackendUnavailable(String),
    #[error("invalid media source: {0}")]
    InvalidSource(String),
    #[error("failed to open media: {0}")]
    OpenFailed(String),
    #[error("media command failed: {0}")]
    CommandFailed(String),
    #[error("unsupported media source: {0}")]
    UnsupportedSource(String),
    #[error("invalid seek target: {0}ms")]
    InvalidSeekTarget(u64),
    #[error("invalid volume: {0}%")]
    InvalidVolume(u16),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestBackend;

    impl MediaBackend for TestBackend {
        fn backend_id(&self) -> &'static str {
            "test"
        }

        fn display_name(&self) -> &'static str {
            "Test Backend"
        }
    }

    #[test]
    fn backend_info_is_derived_from_trait() {
        let info = MediaBackendInfo::from_backend(&TestBackend);

        assert_eq!(info.backend_id, "test");
        assert_eq!(info.display_name, "Test Backend");
    }

    #[test]
    fn media_source_accepts_http_and_https_urls() {
        let http = MediaSource::http_url("http://example.test/movie.mp4").expect("http source");
        let https = MediaSource::http_url("https://example.test/movie.mp4").expect("https source");

        assert_eq!(http.location(), "http://example.test/movie.mp4");
        assert_eq!(https.location(), "https://example.test/movie.mp4");
    }

    #[test]
    fn media_source_rejects_unsupported_urls() {
        let error = MediaSource::http_url("ftp://example.test/movie.mp4").expect_err("unsupported scheme");

        assert_eq!(error, MediaError::UnsupportedSource("ftp://example.test/movie.mp4".to_string()));
    }

    #[test]
    fn playback_snapshot_starts_idle_with_defaults() {
        let snapshot = PlaybackSnapshot::idle();

        assert_eq!(snapshot.status, PlaybackStatus::Idle);
        assert_eq!(snapshot.position, MediaTime::ZERO);
        assert_eq!(snapshot.duration, None);
        assert_eq!(snapshot.volume, Volume::DEFAULT);
        assert_eq!(snapshot.speed, PlaybackSpeed::NORMAL);
        assert!(!snapshot.muted);
    }

    #[test]
    fn volume_rejects_values_above_100_percent() {
        let error = Volume::from_percent(101).expect_err("invalid volume");

        assert_eq!(error, MediaError::InvalidVolume(101));
    }
}
```

- [ ] **Step 4: Run media tests and verify they pass**

Run:

```powershell
cargo test -p openplayer-media
```

Expected: PASS, including the four new media source/value tests and the existing backend info test.

- [ ] **Step 5: Checkpoint status**

Run:

```powershell
git status --short
```

Expected: `crates/media/src/lib.rs` is modified. Commit only if the user has explicitly requested commits during execution.

## Task 2: Add Backend Playback Command Surface

**Files:**
- Modify: `crates/media/src/lib.rs`
- Modify: `crates/mpv/src/lib.rs`

- [ ] **Step 1: Write failing tests for backend command flow**

Use `apply_patch` to add this test backend and test inside `crates/media/src/lib.rs` under the existing `tests` module:

```diff
*** Begin Patch
*** Update File: crates/media/src/lib.rs
@@
     struct TestBackend;
@@
     impl MediaBackend for TestBackend {
@@
     }
+
+    struct RecordingBackend {
+        calls: Vec<&'static str>,
+        snapshot: PlaybackSnapshot,
+    }
+
+    impl RecordingBackend {
+        fn new() -> Self {
+            Self {
+                calls: Vec::new(),
+                snapshot: PlaybackSnapshot::idle(),
+            }
+        }
+    }
+
+    impl MediaBackend for RecordingBackend {
+        fn backend_id(&self) -> &'static str {
+            "recording"
+        }
+
+        fn display_name(&self) -> &'static str {
+            "Recording Backend"
+        }
+
+        fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
+            self.calls.push("open");
+            self.snapshot.source = Some(source);
+            self.snapshot.status = PlaybackStatus::Ready;
+            Ok(self.snapshot.clone())
+        }
+
+        fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
+            self.calls.push("play");
+            self.snapshot.status = PlaybackStatus::Playing;
+            Ok(self.snapshot.clone())
+        }
+
+        fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
+            self.calls.push("pause");
+            self.snapshot.status = PlaybackStatus::Paused;
+            Ok(self.snapshot.clone())
+        }
+
+        fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
+            self.calls.push("stop");
+            self.snapshot.status = PlaybackStatus::Stopped;
+            self.snapshot.position = MediaTime::ZERO;
+            Ok(self.snapshot.clone())
+        }
+
+        fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
+            self.calls.push("seek");
+            self.snapshot.position = position;
+            Ok(self.snapshot.clone())
+        }
+
+        fn set_volume(&mut self, volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
+            self.calls.push("set_volume");
+            self.snapshot.volume = volume;
+            Ok(self.snapshot.clone())
+        }
+
+        fn snapshot(&self) -> PlaybackSnapshot {
+            self.snapshot.clone()
+        }
+    }
@@
     fn volume_rejects_values_above_100_percent() {
         let error = Volume::from_percent(101).expect_err("invalid volume");
 
         assert_eq!(error, MediaError::InvalidVolume(101));
     }
+
+    #[test]
+    fn backend_commands_update_snapshot_and_record_call_order() {
+        let mut backend = RecordingBackend::new();
+        let source = MediaSource::local_file("movie.mp4");
+
+        backend.open(source.clone()).expect("open");
+        backend.play().expect("play");
+        backend.seek(MediaTime::from_millis(42_000)).expect("seek");
+        backend.set_volume(Volume::from_percent(55).expect("volume")).expect("volume");
+        backend.pause().expect("pause");
+        backend.stop().expect("stop");
+
+        assert_eq!(backend.calls, ["open", "play", "seek", "set_volume", "pause", "stop"]);
+        assert_eq!(backend.snapshot().source, Some(source));
+        assert_eq!(backend.snapshot().status, PlaybackStatus::Stopped);
+        assert_eq!(backend.snapshot().position, MediaTime::ZERO);
+        assert_eq!(backend.snapshot().volume, Volume::from_percent(55).expect("volume"));
+    }
 }
*** End Patch
```

- [ ] **Step 2: Run media tests and verify they fail**

Run:

```powershell
cargo test -p openplayer-media
```

Expected: FAIL because `MediaBackend` does not yet define `open`, `play`, `pause`, `stop`, `seek`, `set_volume`, or `snapshot`.

- [ ] **Step 3: Add command methods to `MediaBackend`**

Update the trait in `crates/media/src/lib.rs` to:

```rust
pub trait MediaBackend: Send + Sync {
    fn backend_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, MediaError>;
    fn play(&mut self) -> Result<PlaybackSnapshot, MediaError>;
    fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError>;
    fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError>;
    fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, MediaError>;
    fn set_volume(&mut self, volume: Volume) -> Result<PlaybackSnapshot, MediaError>;
    fn snapshot(&self) -> PlaybackSnapshot;
}
```

Also update `TestBackend` in `crates/media/src/lib.rs` tests to implement these methods:

```rust
        fn open(&mut self, _source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn seek(&mut self, _position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn set_volume(&mut self, _volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
            Ok(PlaybackSnapshot::idle())
        }

        fn snapshot(&self) -> PlaybackSnapshot {
            PlaybackSnapshot::idle()
        }
```

- [ ] **Step 4: Run media tests and verify they pass**

Run:

```powershell
cargo test -p openplayer-media
```

Expected: PASS for all media tests.

- [ ] **Step 5: Run workspace tests and verify mpv fails to compile**

Run:

```powershell
cargo test --workspace
```

Expected: FAIL because `MpvBackendDescriptor` does not yet implement the new `MediaBackend` command methods.

- [ ] **Step 6: Update mpv descriptor for the expanded trait**

Replace `crates/mpv/src/lib.rs` with:

```rust
use openplayer_media::{MediaBackend, MediaError, MediaSource, MediaTime, PlaybackSnapshot, Volume};

#[derive(Debug, Default, Clone, Copy)]
pub struct MpvBackendDescriptor;

impl MpvBackendDescriptor {
    fn unavailable() -> MediaError {
        MediaError::BackendUnavailable("libmpv playback is not wired yet".to_string())
    }
}

impl MediaBackend for MpvBackendDescriptor {
    fn backend_id(&self) -> &'static str {
        "mpv"
    }

    fn display_name(&self) -> &'static str {
        "libmpv"
    }

    fn open(&mut self, _source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn seek(&mut self, _position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn set_volume(&mut self, _volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
        Err(Self::unavailable())
    }

    fn snapshot(&self) -> PlaybackSnapshot {
        PlaybackSnapshot::idle()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openplayer_media::MediaBackendInfo;

    #[test]
    fn exposes_mpv_backend_identity() {
        let descriptor = MpvBackendDescriptor;
        let info = MediaBackendInfo::from_backend(&descriptor);

        assert_eq!(info.backend_id, "mpv");
        assert_eq!(info.display_name, "libmpv");
    }

    #[test]
    fn playback_commands_report_unavailable_until_mpv_is_wired() {
        let mut descriptor = MpvBackendDescriptor;
        let error = descriptor.play().expect_err("mpv is not wired yet");

        assert_eq!(error, MediaError::BackendUnavailable("libmpv playback is not wired yet".to_string()));
    }
}
```

- [ ] **Step 7: Run workspace tests and verify they pass**

Run:

```powershell
cargo test --workspace
```

Expected: PASS for all workspace tests.

- [ ] **Step 8: Checkpoint status**

Run:

```powershell
git status --short
```

Expected: `crates/media/src/lib.rs` and `crates/mpv/src/lib.rs` are modified. Commit only if the user has explicitly requested commits during execution.

## Task 3: Add Core PlaybackService

**Files:**
- Modify: `crates/core/Cargo.toml`
- Modify: `crates/core/src/lib.rs`

- [ ] **Step 1: Add core dependencies needed by tests**

Update `crates/core/Cargo.toml` to:

```toml
[package]
name = "openplayer-core"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
openplayer-media = { path = "../media" }
openplayer-shared = { path = "../shared" }
thiserror.workspace = true
```

- [ ] **Step 2: Write failing core service tests**

Replace `crates/core/src/lib.rs` with this test-first version:

```rust
use openplayer_shared::AppInfo;

pub fn app_info() -> AppInfo {
    AppInfo::skeleton(env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openplayer_media::{MediaBackend, MediaError, MediaSource, MediaTime, PlaybackSnapshot, PlaybackStatus, Volume};
    use openplayer_shared::AppStage;

    #[derive(Debug)]
    struct MockBackend {
        calls: Vec<&'static str>,
        snapshot: PlaybackSnapshot,
        next_error: Option<MediaError>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                calls: Vec::new(),
                snapshot: PlaybackSnapshot::idle(),
                next_error: None,
            }
        }

        fn fail_next(&mut self, error: MediaError) {
            self.next_error = Some(error);
        }

        fn maybe_fail(&mut self) -> Result<(), MediaError> {
            if let Some(error) = self.next_error.take() {
                Err(error)
            } else {
                Ok(())
            }
        }
    }

    impl MediaBackend for MockBackend {
        fn backend_id(&self) -> &'static str {
            "mock"
        }

        fn display_name(&self) -> &'static str {
            "Mock Backend"
        }

        fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("open");
            self.maybe_fail()?;
            self.snapshot.source = Some(source);
            self.snapshot.status = PlaybackStatus::Ready;
            Ok(self.snapshot.clone())
        }

        fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("play");
            self.maybe_fail()?;
            self.snapshot.status = PlaybackStatus::Playing;
            Ok(self.snapshot.clone())
        }

        fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("pause");
            self.maybe_fail()?;
            self.snapshot.status = PlaybackStatus::Paused;
            Ok(self.snapshot.clone())
        }

        fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("stop");
            self.maybe_fail()?;
            self.snapshot.status = PlaybackStatus::Stopped;
            self.snapshot.position = MediaTime::ZERO;
            Ok(self.snapshot.clone())
        }

        fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("seek");
            self.maybe_fail()?;
            self.snapshot.position = position;
            Ok(self.snapshot.clone())
        }

        fn set_volume(&mut self, volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("set_volume");
            self.maybe_fail()?;
            self.snapshot.volume = volume;
            Ok(self.snapshot.clone())
        }

        fn snapshot(&self) -> PlaybackSnapshot {
            self.snapshot.clone()
        }
    }

    #[test]
    fn reports_openplayer_skeleton_info() {
        let info = app_info();

        assert_eq!(info.name, "OpenPlayer");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.stage, AppStage::Skeleton);
    }

    #[test]
    fn playback_service_opens_media_and_stores_ready_snapshot() {
        let source = MediaSource::local_file("movie.mp4");
        let mut service = PlaybackService::new(MockBackend::new());

        let snapshot = service.open(source.clone()).expect("open media");

        assert_eq!(snapshot.status, PlaybackStatus::Ready);
        assert_eq!(snapshot.source, Some(source));
        assert_eq!(service.snapshot(), &snapshot);
    }

    #[test]
    fn playback_service_tracks_play_pause_stop_flow() {
        let mut service = PlaybackService::new(MockBackend::new());

        service.open(MediaSource::local_file("movie.mp4")).expect("open");
        service.play().expect("play");
        assert_eq!(service.snapshot().status, PlaybackStatus::Playing);

        service.pause().expect("pause");
        assert_eq!(service.snapshot().status, PlaybackStatus::Paused);

        service.stop().expect("stop");
        assert_eq!(service.snapshot().status, PlaybackStatus::Stopped);
        assert_eq!(service.snapshot().position, MediaTime::ZERO);
    }

    #[test]
    fn playback_service_updates_seek_and_volume() {
        let mut service = PlaybackService::new(MockBackend::new());

        service.seek(MediaTime::from_millis(90_000)).expect("seek");
        service.set_volume_percent(40).expect("volume");

        assert_eq!(service.snapshot().position, MediaTime::from_millis(90_000));
        assert_eq!(service.snapshot().volume, Volume::from_percent(40).expect("volume"));
    }

    #[test]
    fn playback_service_maps_backend_errors_to_core_errors() {
        let mut backend = MockBackend::new();
        backend.fail_next(MediaError::CommandFailed("play failed".to_string()));
        let mut service = PlaybackService::new(backend);

        let error = service.play().expect_err("backend error");

        assert_eq!(error, CoreError::Media(MediaError::CommandFailed("play failed".to_string())));
    }

    #[test]
    fn playback_service_rejects_invalid_volume_before_backend_call() {
        let mut service = PlaybackService::new(MockBackend::new());

        let error = service.set_volume_percent(101).expect_err("invalid volume");

        assert_eq!(error, CoreError::Media(MediaError::InvalidVolume(101)));
        assert_eq!(service.backend().calls, Vec::<&'static str>::new());
    }
}
```

- [ ] **Step 3: Run core tests and verify they fail**

Run:

```powershell
cargo test -p openplayer-core
```

Expected: FAIL because `PlaybackService` and `CoreError` are not defined.

- [ ] **Step 4: Implement core playback service**

Replace `crates/core/src/lib.rs` with:

```rust
use openplayer_media::{MediaBackend, MediaBackendInfo, MediaError, MediaSource, MediaTime, PlaybackSnapshot, Volume};
use openplayer_shared::AppInfo;
use thiserror::Error;

pub fn app_info() -> AppInfo {
    AppInfo::skeleton(env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error(transparent)]
    Media(#[from] MediaError),
}

pub struct PlaybackService<B: MediaBackend> {
    backend: B,
    snapshot: PlaybackSnapshot,
}

impl<B: MediaBackend> PlaybackService<B> {
    pub fn new(backend: B) -> Self {
        let snapshot = backend.snapshot();
        Self { backend, snapshot }
    }

    pub fn backend_info(&self) -> MediaBackendInfo {
        MediaBackendInfo::from_backend(&self.backend)
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn snapshot(&self) -> &PlaybackSnapshot {
        &self.snapshot
    }

    pub fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, CoreError> {
        self.apply_backend_result(|backend| backend.open(source))
    }

    pub fn play(&mut self) -> Result<PlaybackSnapshot, CoreError> {
        self.apply_backend_result(MediaBackend::play)
    }

    pub fn pause(&mut self) -> Result<PlaybackSnapshot, CoreError> {
        self.apply_backend_result(MediaBackend::pause)
    }

    pub fn stop(&mut self) -> Result<PlaybackSnapshot, CoreError> {
        self.apply_backend_result(MediaBackend::stop)
    }

    pub fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, CoreError> {
        self.apply_backend_result(|backend| backend.seek(position))
    }

    pub fn set_volume_percent(&mut self, percent: u16) -> Result<PlaybackSnapshot, CoreError> {
        let volume = Volume::from_percent(percent)?;
        self.apply_backend_result(|backend| backend.set_volume(volume))
    }

    fn apply_backend_result(
        &mut self,
        command: impl FnOnce(&mut B) -> Result<PlaybackSnapshot, MediaError>,
    ) -> Result<PlaybackSnapshot, CoreError> {
        let snapshot = command(&mut self.backend)?;
        self.snapshot = snapshot.clone();
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openplayer_media::{PlaybackStatus, Volume};
    use openplayer_shared::AppStage;

    #[derive(Debug)]
    struct MockBackend {
        calls: Vec<&'static str>,
        snapshot: PlaybackSnapshot,
        next_error: Option<MediaError>,
    }

    impl MockBackend {
        fn new() -> Self {
            Self {
                calls: Vec::new(),
                snapshot: PlaybackSnapshot::idle(),
                next_error: None,
            }
        }

        fn fail_next(&mut self, error: MediaError) {
            self.next_error = Some(error);
        }

        fn maybe_fail(&mut self) -> Result<(), MediaError> {
            if let Some(error) = self.next_error.take() {
                Err(error)
            } else {
                Ok(())
            }
        }
    }

    impl MediaBackend for MockBackend {
        fn backend_id(&self) -> &'static str {
            "mock"
        }

        fn display_name(&self) -> &'static str {
            "Mock Backend"
        }

        fn open(&mut self, source: MediaSource) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("open");
            self.maybe_fail()?;
            self.snapshot.source = Some(source);
            self.snapshot.status = PlaybackStatus::Ready;
            Ok(self.snapshot.clone())
        }

        fn play(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("play");
            self.maybe_fail()?;
            self.snapshot.status = PlaybackStatus::Playing;
            Ok(self.snapshot.clone())
        }

        fn pause(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("pause");
            self.maybe_fail()?;
            self.snapshot.status = PlaybackStatus::Paused;
            Ok(self.snapshot.clone())
        }

        fn stop(&mut self) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("stop");
            self.maybe_fail()?;
            self.snapshot.status = PlaybackStatus::Stopped;
            self.snapshot.position = MediaTime::ZERO;
            Ok(self.snapshot.clone())
        }

        fn seek(&mut self, position: MediaTime) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("seek");
            self.maybe_fail()?;
            self.snapshot.position = position;
            Ok(self.snapshot.clone())
        }

        fn set_volume(&mut self, volume: Volume) -> Result<PlaybackSnapshot, MediaError> {
            self.calls.push("set_volume");
            self.maybe_fail()?;
            self.snapshot.volume = volume;
            Ok(self.snapshot.clone())
        }

        fn snapshot(&self) -> PlaybackSnapshot {
            self.snapshot.clone()
        }
    }

    #[test]
    fn reports_openplayer_skeleton_info() {
        let info = app_info();

        assert_eq!(info.name, "OpenPlayer");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.stage, AppStage::Skeleton);
    }

    #[test]
    fn playback_service_opens_media_and_stores_ready_snapshot() {
        let source = MediaSource::local_file("movie.mp4");
        let mut service = PlaybackService::new(MockBackend::new());

        let snapshot = service.open(source.clone()).expect("open media");

        assert_eq!(snapshot.status, PlaybackStatus::Ready);
        assert_eq!(snapshot.source, Some(source));
        assert_eq!(service.snapshot(), &snapshot);
    }

    #[test]
    fn playback_service_tracks_play_pause_stop_flow() {
        let mut service = PlaybackService::new(MockBackend::new());

        service.open(MediaSource::local_file("movie.mp4")).expect("open");
        service.play().expect("play");
        assert_eq!(service.snapshot().status, PlaybackStatus::Playing);

        service.pause().expect("pause");
        assert_eq!(service.snapshot().status, PlaybackStatus::Paused);

        service.stop().expect("stop");
        assert_eq!(service.snapshot().status, PlaybackStatus::Stopped);
        assert_eq!(service.snapshot().position, MediaTime::ZERO);
    }

    #[test]
    fn playback_service_updates_seek_and_volume() {
        let mut service = PlaybackService::new(MockBackend::new());

        service.seek(MediaTime::from_millis(90_000)).expect("seek");
        service.set_volume_percent(40).expect("volume");

        assert_eq!(service.snapshot().position, MediaTime::from_millis(90_000));
        assert_eq!(service.snapshot().volume, Volume::from_percent(40).expect("volume"));
    }

    #[test]
    fn playback_service_maps_backend_errors_to_core_errors() {
        let mut backend = MockBackend::new();
        backend.fail_next(MediaError::CommandFailed("play failed".to_string()));
        let mut service = PlaybackService::new(backend);

        let error = service.play().expect_err("backend error");

        assert_eq!(error, CoreError::Media(MediaError::CommandFailed("play failed".to_string())));
    }

    #[test]
    fn playback_service_rejects_invalid_volume_before_backend_call() {
        let mut service = PlaybackService::new(MockBackend::new());

        let error = service.set_volume_percent(101).expect_err("invalid volume");

        assert_eq!(error, CoreError::Media(MediaError::InvalidVolume(101)));
        assert_eq!(service.backend().calls, Vec::<&'static str>::new());
    }
}
```

- [ ] **Step 5: Run core tests and verify they pass**

Run:

```powershell
cargo test -p openplayer-core
```

Expected: PASS for app info and all playback service tests.

- [ ] **Step 6: Run workspace tests and verify they pass**

Run:

```powershell
cargo test --workspace
```

Expected: PASS for every crate.

- [ ] **Step 7: Checkpoint status**

Run:

```powershell
git status --short
```

Expected: `crates/core/Cargo.toml` and `crates/core/src/lib.rs` are modified in addition to prior task files. Commit only if the user has explicitly requested commits during execution.

## Task 4: Final Verification

**Files:**
- No code changes expected.

- [ ] **Step 1: Run formatting check**

Run:

```powershell
cargo fmt --all -- --check
```

Expected: PASS with no formatting diff.

- [ ] **Step 2: Run clippy with warnings denied**

Run:

```powershell
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: PASS with no warnings.

- [ ] **Step 3: Run workspace tests**

Run:

```powershell
cargo test --workspace
```

Expected: PASS for all unit tests and doc tests.

- [ ] **Step 4: Run desktop shell verification to catch unrelated regressions**

Run:

```powershell
npm run verify:shell
```

Working directory: `apps/desktop`

Expected: PASS.

- [ ] **Step 5: Inspect final diff**

Run:

```powershell
git diff -- crates/media/src/lib.rs crates/mpv/src/lib.rs crates/core/Cargo.toml crates/core/src/lib.rs
```

Expected: diff only contains the playback contract, core service, tests, and dependency changes described in this plan.

- [ ] **Step 6: Optional commit if requested by the user**

If the user explicitly requested commits for this execution session, run:

```powershell
git add crates/media/src/lib.rs crates/mpv/src/lib.rs crates/core/Cargo.toml crates/core/src/lib.rs
git commit -m "feat: add core playback service"
```

Expected: commit succeeds. If the user did not request commits, do not commit.

## Self-Review

- Spec coverage: media source/types, backend command surface, core `PlaybackService`, mock-backed tests, and error mapping are covered by Tasks 1-3.
- Out of scope: SQLite, Tauri IPC, UI rewiring, and real `libmpv` are not implemented.
- Placeholder scan: no `TBD`, `TODO`, or unspecified implementation steps remain.
- Type consistency: `MediaSource`, `MediaTime`, `Volume`, `PlaybackSpeed`, `PlaybackStatus`, `PlaybackSnapshot`, `PlaybackEvent`, `MediaError`, `MediaBackend`, `CoreError`, and `PlaybackService` names are consistent across tasks.
