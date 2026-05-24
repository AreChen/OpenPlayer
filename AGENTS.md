# Repository Guidelines

## Project Overview

OpenPlayer is a desktop media player built with Tauri v2, Rust, React, and
libmpv. The active app is `apps/desktop`; the Rust workspace currently contains
only `apps/desktop/src-tauri`.

The default playback path is the `mpv-embed` feature. The main Tauri window is
the native mpv video host, while a transparent overlay window renders the React
controls. Preserve this split. Do not reintroduce browser `<video>` playback,
object URLs, the removed render spike, or old recent-media storage plumbing
unless a task explicitly asks for it.

Persistent app state now lives in backend redb stores. Shortcut bindings remain
frontend-configurable and localStorage-backed, but playback history, resume
state, appearance, language, plugins, global playback settings, and per-media
settings should go through the Tauri store commands.

## Important Paths

- `apps/desktop/src/App.tsx` - React overlay controls, shortcut handling,
  context menus, playlist, settings UI, drag/drop intake, and i18n wiring.
- `apps/desktop/src/i18n.ts` - English/Chinese translations, language mode
  options, and system language resolution.
- `apps/desktop/src/styles.css` - overlay, settings, menus, theme, and playback
  UI styling.
- `apps/desktop/src-tauri/src/lib.rs` - Tauri setup, overlay window lifecycle,
  command registration, startup media, window movement/resize/fullscreen,
  always-on-top, and native shell bridges.
- `apps/desktop/src-tauri/src/mpv_embed.rs` - libmpv child-window backend,
  snapshot-returning playback commands, resume seek handling, initial volume,
  tracks/subtitles, audio-only visualizer configuration, and platform mpv
  options.
- `apps/desktop/src-tauri/src/playback_store.rs` - redb-backed playback
  history, resume position, global playback settings, and per-media settings.
- `apps/desktop/src-tauri/src/appearance_store.rs` - redb-backed theme,
  accent color, plugin manifest, and language preference state.
- `apps/desktop/src-tauri/src/media_paths.rs` - media extension lists, folder
  expansion, and natural playlist ordering.
- `apps/desktop/src-tauri/src/platform_support.rs` - platform capability
  detection and Linux/X11 runtime defaults.
- `apps/desktop/src-tauri/src/shell_preview.rs` - Windows Explorer preview and
  default-app association registration helpers.
- `apps/desktop/src-tauri/src/macos_mpv_gl_view.m` - macOS AppKit view glue for
  embedded mpv rendering.
- `apps/desktop/scripts/verify-shell.mjs` - structural regression checks for
  the two-window shell architecture.
- `apps/desktop/scripts/verify-release.mjs` - release metadata/version checks.
- `apps/desktop/scripts/bundle-macos-libmpv.mjs` - rewrites and bundles macOS
  libmpv dylib dependencies into the app bundle.
- `.github/workflows/ci.yml` - CI checks.
- `.github/workflows/release.yml` - Windows, Linux, and macOS release builds.
- `vendor/native/mpv/windows-x64` - local Windows mpv import/runtime libraries.
- `docs/native-deps/mpv-windows-x64.json` - CI manifest for restoring Windows
  mpv runtime assets.
- `docs/releases/` - concise release notes used by GitHub Releases.

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

Verify release metadata from `apps/desktop`:

```powershell
npm run verify:release -- --tag=vX.Y.Z
```

Build the Windows NSIS installer from `apps/desktop`:

```powershell
npm run tauri:build -- --config src-tauri/tauri.windows.conf.json
```

The Windows installer is emitted under:

```text
target/release/bundle/nsis/
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
`.deb`/AppImage, and unsigned macOS DMG assets plus SHA256 files.

## Verification Expectations

For frontend, overlay shell, shortcut, menu, settings, or Tauri command changes,
run:

```powershell
npm run verify:shell
npm run build
cargo test -p openplayer-desktop
```

For Rust-only changes, run `cargo fmt`, the relevant `cargo test` command, and
`cargo clippy --workspace --all-targets -- -D warnings` when shared backend
behavior is touched.

For release or installer changes, run `npm run verify:release -- --tag=vX.Y.Z`,
build the relevant Tauri package, and confirm the expected artifact exists.
Before committing, run `git diff --check`.

## Release Checklist

- Keep the version in sync across the workspace package, `Cargo.lock`,
  `apps/desktop/package.json`, `apps/desktop/package-lock.json`, and
  `apps/desktop/src-tauri/tauri.conf.json`.
- Add a concise `docs/releases/vX.Y.Z.md` file. Release notes should summarize
  what changed and list produced package types without long marketing copy.
- Keep README hero assets under `docs/assets/` and use repo-relative image links.
- Use the release workflow for Linux and macOS artifacts unless a task
  explicitly requires local packaging.

## Implementation Notes

- Keep fullscreen, drag, resize, close, and always-on-top actions routed through
  backend commands that target the main video window and keep the overlay synced.
- Keep mpv commands in `mpv_embed.rs` small and snapshot-returning when they
  affect playback state.
- When opening media, pass the saved resume position and saved initial volume to
  the backend. The backend should set volume before `loadfile` and retry initial
  resume seeks until mpv reports a seekable duration; do not move this back to
  frontend retry loops.
- Store playback history, volume, loop mode, decoding mode, playback speed,
  video layout, time display mode, and per-media subtitle selection in
  `playback_store.rs`.
- Store theme, accent color, plugin manifests, and language preference in
  `appearance_store.rs`.
- Keep shortcut behavior configurable in `App.tsx` and backed by localStorage.
  The Windows native shortcut bridge exists because the native mpv host can
  take focus away from the overlay.
- Floating playback menus should close when clicking outside controls while
  preserving interactions inside the controls and menus.
- Keep drag/drop support accepting media files and folders; folders should expand
  to a naturally sorted playlist and start the first playable media.
- Keep Linux embedding conservative: X11 supports native mpv embedding; Wayland
  should not be assumed to support this path. Avoid unconditionally forcing
  `gpu-context=x11egl` in virtual or software-rendered environments.
- Windows default app and Explorer preview support lives in `shell_preview.rs`.
  Registry writes can advertise OpenPlayer, but final default-app ownership is
  still governed by Windows user choice.
- Be careful in a dirty worktree. Do not revert unrelated user changes.
