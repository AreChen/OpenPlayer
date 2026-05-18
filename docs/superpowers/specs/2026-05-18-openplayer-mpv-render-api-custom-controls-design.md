# OpenPlayer MPV Render API Custom Controls Design

Date: 2026-05-18

## Purpose

OpenPlayer needs mpv-backed playback while keeping a fully custom player UI. The current mpv child HWND spike proves that `libmpv` can decode and display local HEVC/MKV content with low CPU and memory, but it cannot support React controls overlaying the video. A Win32 child window has native z-order behavior outside the WebView's DOM tree, so DOM titlebar and transport controls either disappear behind video or force reserved black bands.

This design replaces the child HWND playback path with an mpv render API path. mpv remains the media engine, but video output is rendered into a surface that OpenPlayer controls, allowing the UI layer to stay custom and visually independent from mpv's native window ownership.

## Scope

Included:

- A Windows-first mpv render API backend using `libmpv`/`libmpv2` native artifacts.
- A rendering host owned by OpenPlayer rather than a child window owned by mpv via `wid`.
- Custom OpenPlayer controls for titlebar, transport, playlist, progress, volume, and errors.
- Tauri commands for open, play, pause, seek, volume, stop, and snapshot.
- Immediate resize handling tied to window resize/render surface resize events, not snapshot polling.
- A small runtime spike that verifies custom controls can visually overlay and receive pointer input above video.

Not included:

- Linux or macOS render API support in the first implementation slice.
- Shipping installer/bundling of mpv native artifacts.
- Advanced subtitle styling, audio track selection, chapter UI, or shader/video filters.
- Persisted playlist/recent progress changes beyond existing app state.
- Using mpv's built-in OSC as the main UI.
- Automatically playing the Abbott test file during normal development runs.

## Architecture

Target runtime shape:

```text
React custom UI -> Tauri commands -> Rust MpvRenderPlayer -> libmpv render context -> OpenPlayer-owned render surface
                         ^                                                     |
                         |                                                     v
                 snapshot/events <---------------------------------- render/update callbacks
```

The key change is ownership of the video output. The old child HWND path gave mpv a native window through `wid`; mpv then painted directly into that window. The new path creates an mpv render context and asks mpv to render into an OpenPlayer-owned graphics target.

The first Windows implementation should be treated as a spike until it proves two conditions:

- Video displays through mpv render API without the `wid` child-window path.
- Custom OpenPlayer controls can be visible and clickable above the video while the video fills the player area.

## Rendering Strategy

The preferred first attempt is:

- Create a dedicated native graphics surface for video rendering.
- Initialize mpv render context for that surface using `mpv_render_context_create` and the Windows-supported OpenGL path.
- Keep the custom control UI in Tauri/WebView above the video surface.
- Make the WebView/player shell transparent only where video must show through.
- Keep all interactive controls in the WebView layer so styling remains React/CSS-driven.

This is different from the current child HWND spike. The video surface is owned by OpenPlayer and driven by mpv render callbacks. mpv does not receive `wid` and does not own the window that determines z-order.

If Windows WebView transparency or z-order prevents reliable overlay, the fallback is a separate transparent control overlay window that follows the video window. That fallback should be explicitly chosen only after the render API spike proves that a single-window WebView overlay cannot work reliably.

## Backend Components

`MpvRenderPlayer` will replace the child HWND player as the primary backend:

- Holds the mpv handle/render context.
- Owns playback state needed for snapshots.
- Exposes command methods for open/play/pause/seek/volume/stop.
- Receives render update callbacks from mpv.
- Invalidates or schedules redraws when mpv reports new frames.
- Recreates or resizes the render target when the window size changes.

The existing `mpv_embed.rs` child HWND module should not remain the default path. It may be kept temporarily as a spike reference, but runtime guards must prevent the app from mixing child HWND video with custom DOM overlays.

## Frontend Components

The React app remains the custom UI layer:

- Native file selection uses the Tauri dialog plugin and passes real local paths to Rust.
- The player controls are OpenPlayer-styled React/CSS, not mpv OSC controls.
- The titlebar can stay custom only if the overlay layer is reliably clickable; otherwise use native decorations during the render spike and restore custom chrome after overlay input is proven.
- The video itself is not an HTML `<video>` element and not a browser object URL.

The UI must not reserve permanent bottom/top bands just to keep controls visible. Controls should overlay the video and appear/disappear based on app interaction state.

## Error Handling

Backend errors should be reported as concise user-facing messages with stable internal causes:

- mpv native library unavailable.
- render context initialization failed.
- graphics surface unavailable or unsupported.
- media path invalid or missing.
- mpv command failed.
- render target resize failed.

If render API initialization fails, the app should show an error instead of silently falling back to child HWND. Silent fallback would hide the exact overlay problem this design is meant to solve.

## Testing And Verification

Static verification:

- `npm run verify:shell` must reject HTML `<video>`, browser `File`, and object URL playback paths.
- `npm run verify:shell` must reject `wid`/child HWND as the primary mpv path once render API implementation begins.
- `npm run verify:shell` must require render API symbols or wrapper calls.
- `npm run build` must type-check the React command integration.

Rust verification:

- `cargo fmt --all -- --check`.
- `cargo test --workspace`.
- Render API wrapper tests for path validation and state transitions where possible without a graphics device.

Manual runtime verification:

- Launch dev app without `OPENPLAYER_MPV_EMBED_FILE`.
- Select a local video manually with the native picker.
- Confirm video fills the player area with no reserved control bands.
- Confirm custom controls render above video and receive clicks.
- Confirm window resize updates the video immediately, not after polling delay.
- Confirm native or custom window controls remain reachable.
- Confirm CPU/memory stay in the same low range as the child HWND mpv spike for the same media.

## Acceptance Criteria

- Playback uses mpv as the media engine and does not use HTML `<video>`.
- The primary runtime path does not pass `wid` to mpv.
- Video fills the intended player surface without permanent top/bottom control reservations.
- OpenPlayer-styled controls are visible and clickable above video.
- Resize is event-driven and visually immediate.
- Development startup does not auto-play the Abbott sample unless explicitly requested through an environment variable.
- If the render API overlay spike fails, the failure is documented before choosing the transparent overlay-window fallback.

## Implementation Notes

The first implementation plan should be a spike, not a broad rewrite. It should remove or disable the child HWND default path, introduce the minimum render API wrapper, and prove one local file can render with custom controls above it. Only after that proof should the app polish transport state, playlist behavior, and custom chrome.
