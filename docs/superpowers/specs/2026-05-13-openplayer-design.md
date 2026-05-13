# OpenPlayer Design Spec

Date: 2026-05-13

## Purpose

OpenPlayer is a cross-platform, high-performance desktop media player. The current RGB565 animation demo will be replaced by a long-term product skeleton that can grow into a normal, user-facing player with strong format compatibility, subtitle support, playlist/history features, application-level plugins, and customizable themes.

The first milestone prioritizes architecture and product foundation over exhaustive feature depth. It must still produce a usable player slice instead of only abstract crates.

## Confirmed Decisions

- Product name: `OpenPlayer`.
- Target: cross-platform desktop design from the start.
- GUI: Tauri v2 with React + Vite.
- Rust: latest stable toolchain available to the project.
- Media backend: define a `MediaBackend` abstraction and implement the first backend with bundled `libmpv`.
- Storage: SQLite.
- Default visual direction: `Studio Dark`.
- Themes: expose theme tokens and theme manifests so skins can be extended later.
- Plugins: application-level plugins in V0, not decoder/filter/render-pipeline plugins.
- Network playback: HTTP(S) direct media URLs in V0.
- Repository: public GitHub repository under MIT license.
- Existing RGB565 animation demo: replace directly; do not keep as a legacy module.

## Architecture

OpenPlayer will use a Cargo workspace with a Tauri desktop app and focused Rust crates.

```text
apps/desktop/          Tauri v2 app and React + Vite UI
crates/core/           domain models, playback state, commands, events, services
crates/media/          MediaBackend trait and backend-neutral playback types
crates/mpv/            libmpv MediaBackend implementation
crates/storage/        SQLite migrations and repositories
crates/plugin/         application plugin manifests, command registration, extension points
crates/theme/          theme tokens, theme manifests, validation
crates/shared/         shared DTOs/types used across Tauri IPC and crates
docs/architecture/     architecture notes
docs/plugins/          plugin authoring docs
docs/themes/           theme authoring docs
.github/workflows/     CI and release workflows
assets/                tracked static assets
scripts/               project scripts
```

Runtime data flow:

```text
React UI -> Tauri command -> openplayer-core service -> MediaBackend / SQLite / Plugin / Theme service -> typed event stream -> React UI
```

The UI must not call `libmpv` or SQLite directly. Tauri commands form the stable boundary between frontend and Rust services.

## Media Backend

`crates/media` defines backend-neutral playback operations and state events. The V0 interface must cover:

- Open local file.
- Open local folder as a queue source.
- Open HTTP(S) direct URL.
- Play, pause, stop, seek.
- Set volume, mute, playback speed, fullscreen state where applicable.
- Report position, duration, buffering/loading, pause/play state, and errors.
- List audio, video, subtitle tracks, and chapters when available.
- Switch audio/subtitle/video tracks.
- Load external subtitles.
- Adjust subtitle delay.
- Report media metadata and hardware-decode/backend status where available.

`crates/mpv` implements this interface using `libmpv`. Official release artifacts should bundle `libmpv` per platform. The repository should document the source, version, license, and checksum for native dependencies. Large native binaries should not be committed to git unless there is a specific small file that must be tracked, such as a license or manifest.

Future FFmpeg or GStreamer backends can be added by implementing `MediaBackend`; they are not part of V0.

## Product Scope For V0

V0 must work:

- Open local files and folders.
- Drag and drop media into the player.
- Open HTTP(S) direct media URL.
- Playback controls: play, pause, seek, volume, mute, speed, fullscreen.
- Keyboard shortcuts for primary playback controls.
- Common and professional media format support through `libmpv`.
- Subtitle auto-discovery for sidecar subtitle files.
- Manual subtitle load for `.srt`, `.ass`, and `.ssa`.
- Subtitle track switching and subtitle delay adjustment.
- Audio/video/subtitle track display.
- Chapter display when available.
- Media information and backend/hardware-decode status display when available.
- Persistent playlist/queue.
- Recent media list.
- Same-file playback progress memory.
- Settings persisted in SQLite.
- Application plugin manifest loading and validation.
- Theme manifest loading and validation.
- Default `Studio Dark` GUI.

Deferred from V0:

- Full media library scanning.
- Cover wall and poster management.
- Online metadata scraping.
- Accounts and cloud sync.
- YouTube/Bilibili/webpage parsing through tools such as `yt-dlp`.
- RTSP/RTMP.
- Decoder/filter/renderer pipeline plugins.
- Theme marketplace.

## UI Design

The default UI uses the `Studio Dark` direction: professional, restrained, high-contrast, and durable for long viewing sessions.

Primary UI areas:

- Player surface with minimal overlay controls.
- Timeline with current position and duration.
- Playback control cluster.
- Playlist/queue panel.
- Track and subtitle panel.
- Media information panel.
- Settings page.
- Plugin page.
- Theme page.
- Open URL dialog.
- Error and diagnostic surfaces.

The first implementation should establish theme tokens instead of hard-coding colors throughout the UI. Theme manifests can override tokens later while the default skin remains `Studio Dark`.

## Storage

SQLite stores local application data. V0 storage must include:

- Settings.
- Recent media.
- Playback progress keyed by stable media identity.
- Playlists and queue state.
- Plugin enablement and plugin settings.
- Active theme and theme settings.

Migrations must be versioned and tested. Storage access should be hidden behind repository/service APIs rather than used directly from UI commands.

## Plugin System

V0 supports application-level plugins only. A plugin can provide:

- Manifest metadata.
- Command registrations.
- Menu or command-palette entries.
- Settings page entry.
- Metadata extension point.
- Subtitle-source extension point.

V0 must not allow plugins to inject arbitrary decoder, renderer, or filter pipeline code. Those plugin types require separate stability, safety, ABI, and sandboxing design and are deferred.

## Theme System

V0 supports theme manifests and token validation. Themes can customize visual tokens such as:

- Surface colors.
- Accent colors.
- Text colors.
- Border colors.
- Radius scale.
- Shadow/elevation scale.
- Control density.

The system should validate manifests before applying them. Broken themes should fail safely and fall back to the default theme.

## Repository Governance

The project should be initialized as a git repository and published as a public GitHub repository.

Tracked content:

- Source code.
- Cargo and frontend lockfiles.
- Configuration files.
- SQLite migrations.
- Documentation.
- CI/release workflow files.
- Small static assets and icons.
- MIT `LICENSE`.

Ignored content:

- `target/`.
- `node_modules/`.
- Frontend build output.
- Tauri generated bundles.
- Local logs.
- OS/editor files.
- `.superpowers/` brainstorming artifacts.
- Local media/test assets unless deliberately added under a small fixtures directory.
- Native dependency downloads and large binary artifacts.

CI should run:

- `cargo fmt --check`.
- `cargo clippy`.
- `cargo test`.
- Frontend install/build/lint checks once frontend tooling is in place.

Release workflow should later build Tauri installers for Windows, macOS, and Linux with bundled `libmpv` dependencies.

## Error Handling

Rust services should return categorized errors. UI-facing commands should map these into stable error codes and user-safe messages.

Required error categories:

- Media open failure.
- Backend unavailable or misconfigured.
- Unsupported or unreadable media.
- Subtitle load failure.
- Network URL failure.
- Storage migration or query failure.
- Plugin manifest validation failure.
- Theme manifest validation failure.
- Permission or filesystem failure.

The UI should show actionable messages and avoid raw internal error dumps. Structured tracing should capture enough details for diagnostics.

## Testing Strategy

Unit tests:

- Domain models.
- Command and event state transitions.
- Playlist ordering.
- Resume rules.
- Plugin manifest validation.
- Theme manifest validation.

Storage tests:

- SQLite migrations.
- Settings roundtrip.
- Playback progress persistence.
- Recent media persistence.
- Playlist persistence.

Backend tests:

- Mock backend contract tests for `crates/media` and `crates/core`.
- `libmpv` smoke tests gated by native dependency availability.

Desktop tests:

- Frontend build.
- Core UI component tests where practical.
- Tauri command boundary tests where practical.

## Implementation Phases

### Phase 1: Skeleton And Governance

- Initialize git and public GitHub repository.
- Replace current demo with the OpenPlayer workspace.
- Add MIT license.
- Add `.gitignore`.
- Add workspace crates and Tauri v2 desktop app.
- Add React + Vite frontend skeleton.
- Add baseline README and docs folders.
- Add CI skeleton.

### Phase 2: Core Services

- Define `MediaBackend` and backend-neutral types.
- Define core command and event model.
- Add SQLite migrations and repository layer.
- Implement playlist, recent media, settings, and resume-progress services.
- Add plugin manifest validation.
- Add theme manifest validation.
- Add tests for core/storage/plugin/theme boundaries.

### Phase 3: libmpv And UI Slice

- Integrate bundled `libmpv` backend.
- Implement local file playback.
- Implement HTTP(S) direct URL playback.
- Implement playback controls.
- Implement subtitle loading and switching.
- Implement track and media-info display.
- Implement Studio Dark player UI.

### Phase 4: Packaging Hardening

- Add platform-specific native dependency documentation and scripts.
- Add Tauri release workflow.
- Add smoke tests where native dependencies are available.
- Polish error states.
- Add plugin and theme authoring docs.
- Update README with usage and development setup.

## Open Risks

- Tauri + `libmpv` video-surface integration must be validated early. If direct embedding proves fragile, the implementation plan must choose a fallback rendering approach before building complex UI around it.
- Bundling `libmpv` across platforms adds packaging and licensing work in V0.
- A full professional media-library experience is intentionally outside V0 and should not creep into the first milestone.

## Acceptance Criteria

- The repository is a clean OpenPlayer project, not an animation demo.
- The project builds from a fresh checkout with documented prerequisites.
- Git tracking ignores generated and local-only artifacts.
- Public GitHub repository exists with MIT license.
- Workspace boundaries match this spec.
- Tauri desktop shell launches.
- Core services and storage have tests.
- `MediaBackend` abstraction exists and has mock-backed tests.
- `libmpv` backend can play at least one local media file in a smoke path once native dependencies are present.
- UI can exercise the command/event path rather than directly touching backend internals.
