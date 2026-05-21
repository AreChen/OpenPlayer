# Repository Guidelines

## Project Overview

OpenPlayer is a desktop media player built with Tauri v2, Rust, React, and libmpv.
The active app is `apps/desktop`; the Rust workspace currently contains only
`apps/desktop/src-tauri`.

The default playback path is the `mpv-embed` feature. It uses a main Tauri window
as the native mpv video host and a transparent overlay window for React controls.
Do not reintroduce browser `<video>` playback, object URLs, the removed render
spike, or old storage/recent-media plumbing unless a task explicitly asks for it.

## Important Paths

- `apps/desktop/src/App.tsx` - React overlay controls and keyboard shortcuts.
- `apps/desktop/src/styles.css` - overlay UI styling.
- `apps/desktop/src-tauri/src/lib.rs` - Tauri commands, overlay window setup,
  window movement, resize, fullscreen, and command registration.
- `apps/desktop/src-tauri/src/mpv_embed.rs` - libmpv child-window backend.
- `apps/desktop/scripts/verify-shell.mjs` - structural regression checks for
  the shell architecture.
- `vendor/native/mpv/windows-x64` - local Windows mpv import/runtime libraries.

## Development Commands

From the repository root:

```powershell
cargo fmt
cargo test -p openplayer-desktop
```

From `apps/desktop`:

```powershell
npm run verify:shell
npm run build
npm run tauri:dev
```

Build the Windows NSIS installer from `apps/desktop`:

```powershell
npm run tauri:build -- --config src-tauri/tauri.windows.conf.json
```

The installer is emitted under:

```text
target/release/bundle/nsis/
```

## Verification Expectations

For frontend or Tauri shell behavior changes, run:

```powershell
npm run verify:shell
npm run build
cargo test -p openplayer-desktop
```

For Rust-only changes, run `cargo fmt` and the relevant `cargo test` command.
For installer changes, run the Tauri build command above and confirm the NSIS
artifact exists.

## Implementation Notes

- Keep shortcut behavior configurable in `App.tsx` and backed by localStorage.
- Keep fullscreen, drag, resize, and close actions routed through backend
  commands that target the main video window, not the overlay window.
- Keep mpv commands in `mpv_embed.rs` small and snapshot-returning when they
  affect playback state.
- Preserve the overlay/window split: the main window hosts video, the overlay
  hosts controls.
- Be careful in a dirty worktree. Do not revert unrelated user changes.
