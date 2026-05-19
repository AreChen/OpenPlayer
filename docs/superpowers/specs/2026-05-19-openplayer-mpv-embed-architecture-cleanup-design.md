# OpenPlayer MPV Embed Architecture Cleanup Design

Date: 2026-05-19

## Purpose

OpenPlayer now has a verified Windows playback architecture: mpv renders into a native child HWND hosted by the main video window, while a separate transparent Tauri overlay window provides the fully custom React controls. Manual testing confirms playback, transport controls, seek behavior, auto-hidden chrome, drag, resize, fullscreen, and installer packaging work correctly.

The codebase still carries a failed OpenGL mpv render API spike as the default `mpv-render` feature. That name and implementation no longer match the working architecture. This cleanup makes the stable mpv child-window overlay path the only production mpv backend and removes the failed render API spike from runtime code.

## Scope

Included:

- Delete the failed OpenGL render API spike files.
- Remove the misleading `mpv-render` feature from the desktop crate.
- Make `mpv-embed` the default desktop playback feature.
- Simplify `lib.rs` so production runtime setup has one mpv overlay/embed path.
- Update static verification so it protects the current stable architecture instead of requiring render API symbols.
- Keep mpv DLL bundling and NSIS installer hooks unchanged.
- Keep the `mpv-smoke` feature for libmpv sanity checks.

Not included:

- Changing the player UI behavior or styling.
- Reworking the playlist, recent files, or persistence model.
- Adding Linux/macOS playback support.
- Reintroducing the failed render API as an experimental feature.
- Changing installer naming or release process.

## Architecture

Target runtime shape:

```text
main window: index.html?surface=video
  -> owns the native mpv child HWND video host

overlay window: index.html?surface=overlay
  -> transparent, owned by the main window, renders React controls
  -> calls Tauri commands that operate on the main window mpv player
```

The main window is the video host. The overlay window follows the main window using physical position and size sync. The overlay is owned by the main window with `GWLP_HWNDPARENT`, so it stays above the player without becoming globally topmost over other applications.

`mpv_embed.rs` remains the backend module that owns the mpv handle, creates the native video host, passes the host HWND to mpv through `wid`, and exposes playback commands and snapshots.

## Backend Cleanup

Remove these files:

- `apps/desktop/src-tauri/src/mpv_render.rs`
- `apps/desktop/src-tauri/src/mpv_render/sys.rs`
- `apps/desktop/src-tauri/src/mpv_render/win32_surface.rs`

Update `apps/desktop/src-tauri/Cargo.toml`:

- Change `default = ["mpv-render"]` to `default = ["mpv-embed"]`.
- Remove the `mpv-render` feature.
- Remove `libmpv2-sys`, because only the failed render API spike uses it.
- Remove Windows OpenGL and library loader feature flags, because the stable embed path does not use them.
- Keep `mpv-smoke` and `mpv-embed`.

Update `apps/desktop/src-tauri/src/lib.rs`:

- Keep the no-mpv baseline `run()` only as the compile path for builds with neither `mpv-embed` nor `mpv-smoke` enabled; it is not the default runtime.
- Keep the `mpv-smoke` module feature gate.
- Keep only the `mpv-embed` production runtime path.
- Remove `MpvRenderState` and all `mpv_render_*` command registrations.
- Keep `mpv_overlay_open_path` as the overlay command that opens media against the main video window through `mpv_embed::open_path_for_window`.
- Keep overlay creation, owner relationship, hidden-then-sync-then-show startup, and main-window drag/resize/fullscreen commands.

## Frontend Behavior

The frontend should not change visually or behaviorally in this cleanup. It should continue to:

- Render `surface=video` as the video-only main window content.
- Render `surface=overlay` as custom controls.
- Use the native Tauri file picker for local paths.
- Call `mpv_overlay_open_path` for open and `mpv_embed_*` for playback controls.
- Avoid HTML `<video>`, browser `File`, and object URL playback paths.
- Keep seek snapping, `step="any"`, and auto-hide controls behavior.

## Verification

Update `apps/desktop/scripts/verify-shell.mjs` so static checks match the cleaned architecture:

- Require `default = ["mpv-embed"]`.
- Reject `mpv-render`, `mpv_render`, `MpvRenderState`, and `mpv_render_*` references in production runtime files.
- Require overlay window creation through `WebviewWindowBuilder` with `surface=overlay`.
- Require `mpv_overlay_open_path` to call `mpv_embed::open_path_for_window` against the main window.
- Keep guards for no HTML video, no object URLs, native dialog usage, overlay capability scope, drag/resize/fullscreen routing, auto-hide controls, EOF snapping, and `step="any"`.
- Keep guards for mpv DLL bundling and NSIS copy/delete hooks.

Run these verification commands after implementation:

- `npm run verify:shell` from `apps/desktop`.
- `npm run build` from `apps/desktop`.
- `cargo fmt --all -- --check` from the workspace root.
- `cargo check -p openplayer-desktop` from the workspace root.
- `cargo check -p openplayer-desktop --features mpv-smoke` from the workspace root.

Optionally rebuild the installer with `npm run tauri:build` from `apps/desktop` after setting the mpv DLL/link environment used by prior builds.

## Acceptance Criteria

- The default desktop build uses `mpv-embed`, not `mpv-render`.
- No production Rust module references the failed `mpv_render` backend.
- The deleted render API spike is not required by static verification or Cargo features.
- The app keeps the same tested user-visible behavior: playback, controls, seek end snapping, auto-hide chrome, drag, resize, fullscreen, and installer mpv DLL loading.
- `verify-shell`, TypeScript build, Rust formatting, and Rust checks pass.
