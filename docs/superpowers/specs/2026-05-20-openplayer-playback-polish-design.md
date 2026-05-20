# OpenPlayer Playback Polish Design

## Goal

Improve playback polish before new feature development by making the progress control feel continuously animated, adding a professional timecode/frame display toggle, and removing the transparent startup flash from the main video window.

## Current State

- The overlay asks `mpv_embed_snapshot` every 500 ms and stores `snapshot.position` in `currentTime`.
- The seek slider uses `displayTime` derived directly from that snapshot-backed state, so short videos visibly jump between samples.
- The left time label shows the current time and the right label shows duration using `formatTime`.
- `MpvEmbedSnapshot` contains position, duration, status, pause state, and volume, but no frame-rate metadata.
- The shared `index.html` starts transparent until React and CSS render the video surface. The main video window then becomes black, causing a visible transparent-to-black flash.
- The app still uses the verified two-window architecture: main window hosts mpv video, overlay window hosts custom controls.

## Design

### Smooth Progress Display

The frontend will separate mpv's authoritative playback state from the displayed playback clock.

- Keep mpv snapshots as the source of truth.
- Store a display clock anchor whenever a snapshot, play, pause, seek, or open command updates playback state.
- While media is playing, use `requestAnimationFrame` to derive a smooth display position from `anchorPosition + elapsedSeconds`.
- Clamp the derived display position to `[0, duration]`.
- Continue using the existing pending seek suppression so stale mpv snapshots do not pull the slider backward during seeks.
- Continue using the existing end-of-media snap behavior so short clips still land exactly at duration.
- Keep the snapshot polling interval conservative; the smooth UI should not require high-frequency calls into mpv.

The result is that the slider thumb and filled progress track move at a consistent visual velocity. Short videos move faster because the same real seconds cover a larger percentage of the duration, not because the UI jumps between sparse samples.

### Timecode And Frame Mode

The transport time labels become a toggleable display mode.

- Default mode is timecode.
- Clicking either transport time label toggles between timecode mode and frame mode.
- Timecode mode uses adaptive formatting:
  - For media with duration up to one hour, display `MM:SS`.
  - For media longer than one hour, display `H:MM:SS`.
- Frame mode displays current frame on the left and total frames on the right.
- Frame counts are derived from the same smooth display position so the current frame advances smoothly and consistently with the slider.
- Frame mode needs an fps value from mpv. The backend should add `fps` to `MpvEmbedSnapshot` by reading mpv metadata, preferring `container-fps` and falling back to `estimated-vf-fps`.
- If fps is missing, zero, or not finite, the UI should remain in timecode mode and avoid showing misleading frame counts.

The frame count formula is:

```text
currentFrame = clamp(floor(displayPosition * fps), 0, totalFrames)
totalFrames = max(0, floor(duration * fps))
```

### Startup Background

The video surface should be visually black before React mounts, while the overlay surface must remain transparent.

- Add a tiny inline surface classifier in `index.html` before loading the React bundle.
- If `location.search` contains `surface=video`, add a `surface-video` class to the root element.
- Otherwise add `surface-overlay`.
- Add critical inline CSS so `html.surface-video`, `html.surface-video body`, and `html.surface-video #root` paint black immediately.
- Keep overlay surfaces transparent so the controls window does not become an opaque black sheet.
- Keep the existing `.video-host-surface` black background as the post-mount steady state.

This avoids a transparent-to-black flash in the main window without changing the mpv child HWND architecture or making the overlay opaque.

## Validation

- Add or update shell guards so the codebase requires the smooth-display clock, timecode/frame toggle, backend fps field, and surface-specific startup background.
- Run `npm run verify:shell` from `apps/desktop`.
- Run `npm run build` from `apps/desktop`.
- Run `cargo check -p openplayer-desktop` from the repository root.
- If practical, run the app or browser dev surface to verify:
  - The video surface starts black.
  - The overlay surface remains transparent.
  - The transport labels default to timecode.
  - Clicking a time label toggles frame mode only when fps is available.
  - The seek slider advances smoothly during playback.

## Out Of Scope

- Changing the mpv playback architecture.
- Replacing native mpv playback with HTML video.
- Adding timeline thumbnails, chapter markers, or waveform previews.
- Changing the existing 5-second chrome auto-hide behavior.
