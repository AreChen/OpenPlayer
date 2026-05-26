# Repository Guidelines

## Project Overview

OpenPlayer is a desktop media player built with Tauri v2, Rust, React, and
libmpv. The active product lives in `apps/desktop`; the Rust workspace currently
contains the Tauri crate at `apps/desktop/src-tauri`.

The default runtime is the `mpv-embed` feature. The main Tauri window owns the
native mpv video host. A separate transparent overlay window renders the React
controls. Preserve this two-window shell. Do not replace it with browser
`<video>` playback, object URLs, the old render spike, or frontend-only recent
media storage unless a task explicitly asks for that architecture change.

Persistent app state is backend-owned. Playback history, resume positions,
global playback settings, per-media settings, appearance, language, plugins, and
plugin runtime storage go through Tauri commands backed by redb. Shortcut
bindings remain frontend-configurable and localStorage-backed because they are a
UI preference and must be sent to the native shortcut bridge at runtime.

## Architecture Contracts

- Keep `apps/desktop/src/App.tsx` and
  `apps/desktop/src/components/player/PlayerOverlayApp.tsx` as composition
  shells. Add behavior in focused hooks, app services, or view components.
- Register Tauri commands in `apps/desktop/src-tauri/src/bootstrap/`. Commands
  available in both runtimes belong in both `embedded.rs` and `fallback.rs`.
  mpv-only commands belong only in the `mpv-embed` path.
- Keep fullscreen, drag, resize, close, focus, always-on-top, file-manager, and
  overlay sync behavior routed through backend window commands that target the
  main video window and keep the overlay aligned.
- Keep mpv playback commands snapshot-returning when they affect playback state.
  The frontend should apply backend snapshots, not guess mpv state.
- When opening media, pass saved resume position and initial volume to
  `mpv_overlay_open_path`. The backend sets volume before `loadfile` and retries
  the initial resume seek until mpv reports a seekable duration.
- Do not move playback history, resume retry loops, or persisted playback
  settings back into React-only state.
- Keep Linux embedding conservative. X11 supports native mpv embedding; Wayland
  must not be assumed to support this path. Avoid unconditionally forcing
  `gpu-context=x11egl` in virtual or software-rendered environments.
- Be careful in a dirty worktree. Never revert unrelated user changes.

## Current Module Map

### Rust Backend

- `apps/desktop/src-tauri/src/lib.rs` declares crate modules and re-exports the
  selected bootstrap `run`.
- `apps/desktop/src-tauri/src/bootstrap/` owns Tauri builder setup, managed
  state, overlay startup, command registration, and feature-specific runtime
  selection.
- `apps/desktop/src-tauri/src/window.rs` plus `window/` owns main-window and
  overlay window actions: chrome, movement, resize, fullscreen, focus, file
  manager bridges, overlay platform details, and mpv-open command bridging.
- `apps/desktop/src-tauri/src/mpv_embed/` owns libmpv embedding, player state,
  snapshots, tracks, subtitles, resume seeking, capture, recording, wall
  playback, platform video output, and video-host handles.
- `apps/desktop/src-tauri/src/mpv_embed/commands/` is the command-facing layer
  for mpv operations. Keep IPC wrappers thin and put behavior in player modules.
- `apps/desktop/src-tauri/src/mpv_embed/types/` contains playback, capture,
  state, wall, video-host, and video-output DTOs.
- `apps/desktop/src-tauri/src/appearance_store/` owns redb-backed appearance,
  themes, plugin manifests, plugin import/uninstall, plugin settings, plugin
  runtime sources/views/storage, and player preferences.
- `apps/desktop/src-tauri/src/playback_store/` owns redb-backed playback
  history, resume positions, network stream history, global playback settings,
  and per-media settings.
- `apps/desktop/src-tauri/src/media_paths/` owns media extension catalogs,
  folder/path expansion, startup media parsing, and natural playlist ordering.
- `apps/desktop/src-tauri/src/native_shortcuts/` owns native shortcut dispatch,
  including the Windows low-level keyboard hook.
- `apps/desktop/src-tauri/src/platform_support/` and `platform_support.rs` own
  platform capability detection and Linux runtime environment preparation.
- `apps/desktop/src-tauri/src/shell_preview/` owns Windows Explorer preview
  format catalogs, selected-format registration, and default-app settings links.
- `apps/desktop/src-tauri/src/plugin_network/` owns validated plugin network
  requests.
- `apps/desktop/src-tauri/src/system_fonts.rs`,
  `apps/desktop/src-tauri/src/app_info.rs`, and
  `apps/desktop/src-tauri/src/external_open.rs` expose small backend services.
- `apps/desktop/src-tauri/src/macos_mpv_gl_view.m` is macOS AppKit glue for
  embedded libmpv rendering.

### React Frontend

- `apps/desktop/src/app/` contains app-level services, constants, types,
  playback helpers, mpv session orchestration, media utilities, update checks,
  theme helpers, shortcut helpers, and window command wrappers.
- `apps/desktop/src/hooks/` contains domain hooks for backend sync, playback,
  media intake, queue actions, shortcuts, settings, plugin runtime, window
  frame interactions, feedback, lifecycle, and overlay composition.
- `apps/desktop/src/hooks/windowFrameInteractions/` owns non-control surface
  drag, double-click, and resize behavior.
- `apps/desktop/src/components/player/` contains the player shell, stage
  overlays, transport controls, playlist drawer, media panels, and view-prop
  assembly.
- `apps/desktop/src/components/settings/`,
  `apps/desktop/src/components/plugins/`, and `apps/desktop/src/components/media/`
  contain focused UI surfaces outside the core player shell.
- `apps/desktop/src/i18n.ts` and `apps/desktop/src/i18n/` contain translations,
  language mode options, and system language resolution.
- `apps/desktop/src/styles.css` imports modular CSS from `apps/desktop/src/styles/`.
  Put new CSS in the domain folder (`shell`, `transport`, `settings`, etc.)
  instead of growing the root stylesheet.

## Window Interaction Rules

The player surface is a transparent overlay over the native mpv host, so Windows
hit-testing and native dragging are fragile. Preserve these details:

- `.drag-region` must paint a near-invisible hit-test surface over the video
  (`rgba(0, 0, 0, 0.004)`) and disable browser text/image dragging.
- Do not add `data-tauri-drag-region` to the full video surface. It swallows
  double-click playback toggles.
- Left pointerdown on the non-control surface must immediately call
  `window_start_drag` through `startMainWindowDrag()`. Do not wait for a
  pointermove threshold; mpv playback can swallow the follow-up move event.
- Double-click play/pause must use app-level time and distance detection
  (`WINDOW_DOUBLE_CLICK_MAX_MS` and `WINDOW_DOUBLE_CLICK_MAX_DISTANCE_PX`), not
  WebView `event.detail`.
- The browser `dblclick` fallback must be suppressed after pointerdown already
  handled the double-click, otherwise playback toggles twice.
- Middle click on the non-control surface toggles fullscreen through the backend
  command.

## Persistence Rules

- Use `appearance_store` for theme, accent override, plugin manifests, plugin
  settings, plugin runtime storage, incognito mode, quiet keyboard controls, and
  language preference.
- Use `playback_store` for media history, network stream history, resume
  positions, loop mode, decoding mode, speed, volume, video layout, time display
  mode, and per-media subtitle selection.
- Stores should open redb per command through their state helpers so multiple
  OpenPlayer processes do not hold long-lived database locks.
- Validate external input before persisting it. Plugin manifests and theme
  manifests should reject unknown or invalid fields.
- Keep store DTOs in the domain `types/` module and record encoding/decoding in
  `records/` or `helpers/` modules.

## Extension Patterns

### Adding A Tauri Command

1. Add a thin command wrapper in the relevant domain `commands.rs` or small
   service file.
2. Put business logic in a domain module, not in `bootstrap` or `lib.rs`.
3. Register the command in `bootstrap/embedded.rs`; also register it in
   `bootstrap/fallback.rs` if it is not mpv-specific.
4. Add or update the frontend wrapper/hook that invokes the command.
5. Add regression coverage in Rust tests or `scripts/verify-shell.mjs` when the
   command protects architecture-sensitive behavior.

### Adding Playback Capability

1. Put mpv IPC wrappers in `mpv_embed/commands/`.
2. Put state mutation and normalization in `player*`, `media/`, `types/`, or
   capture/recording modules as appropriate.
3. Return an `MpvSnapshot` when the command changes playback state.
4. Update frontend session orchestration in `src/app/mpvSession.ts`,
   `src/app/mpvSessionCommands/`, or focused playback hooks.

### Adding Persisted Settings

1. Decide whether the setting belongs in `appearance_store` or `playback_store`.
2. Add a typed DTO, redb key/table handling, validation, and command wrapper.
3. Sync it through `useBackendStateSync` or a focused settings hook.
4. Keep frontend localStorage only for shortcut bindings unless the task
   explicitly changes persistence ownership.

### Adding UI

1. Put stateful behavior in a hook under `src/hooks/`.
2. Put app-level pure helpers, constants, and DTOs under `src/app/`.
3. Put view assembly in `components/player/viewProps/` when it feeds the player
   shell, and keep presentational JSX in focused components.
4. Put styles in the matching file under `src/styles/`; avoid growing
   `styles.css` directly.
5. Keep text in `i18n.ts` or `src/i18n/`; do not hard-code user-facing strings.

## Development Commands

From the repository root:

```powershell
cargo fmt
cargo test -p openplayer-desktop
cargo clippy --workspace --all-targets -- -D warnings
```

From `apps/desktop`:

```powershell
npm ci
npm run verify:shell
npm run build
npm run tauri:dev
```

Build the Windows NSIS installer from `apps/desktop`:

```powershell
npm run tauri:build -- --config src-tauri/tauri.windows.conf.json
```

The Windows installer is emitted under:

```text
target/release/bundle/nsis/
```

Verify release metadata from `apps/desktop`:

```powershell
npm run verify:release -- --tag=vX.Y.Z
```

Linux packages are built on Linux with:

```bash
npm run tauri:build -- --config src-tauri/tauri.linux.conf.json
```

Linux needs the native Tauri/mpv packages installed, including `pkg-config`,
`libdbus-1-dev`, `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`,
`libayatana-appindicator3-dev`, `libmpv-dev`, `librsvg2-dev`, and `patchelf`.

macOS packages are built on macOS with:

```bash
npm run tauri:build -- --config src-tauri/tauri.macos.conf.json --bundles app
node scripts/bundle-macos-libmpv.mjs
```

GitHub Actions is the normal path for release packages. Pushing a `v*` tag or
running the release workflow manually builds and uploads Windows NSIS, Linux
`.deb`/AppImage, unsigned macOS DMG assets, and SHA256 files.

## Verification Matrix

- Documentation-only changes: run `git diff --check`.
- Frontend, overlay shell, controls, shortcuts, settings, i18n, or CSS changes:
  run `npm run verify:shell`, `npm run build`, and `git diff --check`.
- Window interaction or Tauri command changes: run `npm run verify:shell`,
  `npm run build`, `cargo test -p openplayer-desktop`, and `git diff --check`.
- Rust-only domain changes: run `cargo fmt`, the relevant
  `cargo test -p openplayer-desktop` target, and
  `cargo clippy --workspace --all-targets -- -D warnings` when shared backend
  behavior is touched.
- Release or installer changes: run `npm run verify:release -- --tag=vX.Y.Z`,
  build the relevant Tauri package, and confirm the expected artifact exists.
- Before committing, run `git diff --check`.

## Release Checklist

- Keep versions in sync across the workspace package, `Cargo.lock`,
  `apps/desktop/package.json`, `apps/desktop/package-lock.json`, and
  `apps/desktop/src-tauri/tauri.conf.json`.
- Add a concise `docs/releases/vX.Y.Z.md` file. Release notes should summarize
  what changed and list produced package types without long marketing copy.
- Keep README hero assets under `docs/assets/` and use repo-relative image
  links.
- Use the release workflow for Linux and macOS artifacts unless a task
  explicitly requires local packaging.

## Maintenance Guidance

- Prefer many small modules with clear ownership over another large catch-all
  file. If a file starts mixing command wrappers, persistence, parsing, and UI
  policy, split by responsibility.
- Use structured APIs and parsers for manifests, paths, JSON, and registry
  data. Avoid ad hoc string manipulation when a typed helper already exists.
- Add tests near the module being changed. Existing domain tests live in
  `tests.rs` or a `tests/` submodule.
- Update `apps/desktop/scripts/verify-shell.mjs` when a structural invariant is
  important enough that future refactors must not accidentally remove it.
- Use `rg` or `rg --files` for repository searches.
- Do not reformat or move unrelated files while working in the dirty worktree.
