# OpenPlayer Playback Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make playback progress visually smooth, add a timecode/frame-count display toggle, and remove the transparent startup flash from the main video window.

**Architecture:** mpv snapshots remain the authoritative state, while the React overlay derives a display-only clock with `requestAnimationFrame` between snapshots. The Rust mpv snapshot gains an `fps` field so the UI can compute current and total frames. `index.html` gets tiny pre-React surface classification and critical background CSS so the main video surface paints black immediately while the overlay stays transparent.

**Tech Stack:** Tauri v2, React 19, Vite, TypeScript, Rust, libmpv2, Win32 mpv child HWND.

---

## File Structure

- Modify: `apps/desktop/scripts/verify-shell.mjs` for failing static guards and final regression coverage.
- Modify: `apps/desktop/src-tauri/src/mpv_embed.rs` to add fps metadata to `MpvEmbedSnapshot`.
- Modify: `apps/desktop/src/App.tsx` to add display-clock interpolation, adaptive timecode formatting, and frame-count mode.
- Modify: `apps/desktop/src/styles.css` to make transport time labels button-like without changing layout.
- Modify: `apps/desktop/index.html` to paint the main video surface black before React mounts.

No git commit steps are included because this environment commits only when the user explicitly requests it.

### Task 1: Add Failing Verification Guards

**Files:**
- Modify: `apps/desktop/scripts/verify-shell.mjs`

- [ ] **Step 1: Read `index.html` in the shell verifier**

Add this line after the existing `packageJson` read:

```js
const indexHtml = await readFile(new URL("../index.html", import.meta.url), "utf8");
```

- [ ] **Step 2: Add startup background assertions**

Add these assertions after the existing Vite port assertions:

```js
assert.match(indexHtml, /surface=video[\s\S]*surface-video/, "index.html must classify the main video surface before React mounts");
assert.match(indexHtml, /surface-overlay/, "index.html must classify non-video surfaces as transparent overlays before React mounts");
assert.match(indexHtml, /html\.surface-video[\s\S]*background:\s*#000/, "video surface must paint black before React and mpv finish loading");
assert.match(indexHtml, /html\.surface-overlay[\s\S]*background:\s*transparent/, "overlay surface must remain transparent before React mounts");
```

- [ ] **Step 3: Add frontend smooth progress and frame-mode assertions**

Add these assertions near the existing `appSource` transport assertions:

```js
assert.match(appSource, /type TimeDisplayMode\s*=\s*"timecode"\s*\|\s*"frames"/, "frontend must define a timecode/frame display mode");
assert.match(appSource, /type PlaybackClockAnchor/, "frontend must keep a display-clock anchor for smooth progress interpolation");
assert.match(appSource, /requestAnimationFrame/, "frontend must animate displayed progress with requestAnimationFrame");
assert.match(appSource, /anchorDisplayClock/, "frontend must reset the smooth display clock when mpv state changes");
assert.match(appSource, /formatTimecode/, "frontend must use adaptive timecode formatting");
assert.match(appSource, /formatFrameCount/, "frontend must format frame counts for frame mode");
assert.match(appSource, /toggleTimeDisplayMode/, "transport time labels must toggle timecode and frame display modes");
assert.match(appSource, /const displayTime\s*=\s*snapEndOfMediaPosition\(displayPosition/, "seek slider must use the interpolated display position");
assert.match(appSource, /Math\.floor\(displayTime \* framesPerSecond\)/, "current frame must be derived from smooth display time and fps");
assert.match(appSource, /Math\.floor\(duration \* framesPerSecond\)/, "total frame count must be derived from duration and fps");
assert.match(appSource, /fps:\s*number/, "frontend snapshot type must include fps metadata");
```

- [ ] **Step 4: Add backend fps assertions**

Add these assertions near the existing `mpvEmbedSource` assertions:

```js
assert.match(mpvEmbedSource, /fps:\s*f64/, "mpv snapshots must serialize fps metadata");
assert.match(mpvEmbedSource, /container-fps/, "mpv snapshots must prefer container-fps for frame-count mode");
assert.match(mpvEmbedSource, /estimated-vf-fps/, "mpv snapshots must fall back to estimated-vf-fps when container fps is unavailable");
```

- [ ] **Step 5: Run guard and verify it fails before implementation**

Run from `apps/desktop`:

```bash
npm run verify:shell
```

Expected: FAIL on the first new missing requirement, such as `index.html must classify the main video surface before React mounts`.

### Task 2: Add FPS Metadata To mpv Snapshots

**Files:**
- Modify: `apps/desktop/src-tauri/src/mpv_embed.rs`

- [ ] **Step 1: Add fps to the serialized snapshot**

Change `MpvEmbedSnapshot` to include `fps` after `duration`:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvEmbedSnapshot {
    path: String,
    hwnd: i64,
    status: String,
    ended: bool,
    paused: bool,
    position: f64,
    duration: f64,
    fps: f64,
    volume: f64,
}
```

- [ ] **Step 2: Add fps helper functions**

Add these helper functions before `impl MpvEmbedPlayer`:

```rust
fn valid_fps(value: f64) -> Option<f64> {
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        None
    }
}

fn read_player_fps(mpv: &libmpv2::Mpv) -> f64 {
    mpv.get_property::<f64>("container-fps")
        .ok()
        .and_then(valid_fps)
        .or_else(|| {
            mpv.get_property::<f64>("estimated-vf-fps")
                .ok()
                .and_then(valid_fps)
        })
        .unwrap_or(0.0)
}
```

- [ ] **Step 3: Include fps in snapshots**

Inside `MpvEmbedPlayer::snapshot`, add this line after reading `duration`:

```rust
let fps = read_player_fps(&self.mpv);
```

Then include the field in `MpvEmbedSnapshot` after `duration`:

```rust
duration,
fps,
volume: self.volume,
```

- [ ] **Step 4: Run Rust check**

Run from repository root:

```bash
cargo check -p openplayer-desktop
```

Expected: PASS. The Rust snapshot compiles with the new `fps` field.

### Task 3: Implement Smooth Progress And Timecode/Frame Toggle

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/styles.css`

- [ ] **Step 1: Extend frontend types and constants**

In `apps/desktop/src/App.tsx`, add `fps` to `MpvSnapshot` after `duration`:

```ts
  fps: number;
```

Add these types after `PendingSeek`:

```ts
type PlaybackClockAnchor = {
  position: number;
  startedAt: number;
  playing: boolean;
};

type TimeDisplayMode = "timecode" | "frames";
```

- [ ] **Step 2: Replace `formatTime` with adaptive timecode and frame helpers**

Replace the existing `formatTime` function with:

```ts
function formatTimecode(value: number, totalDuration: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return totalDuration > 3600 ? "0:00:00" : "00:00";
  }

  const totalSeconds = Math.floor(value);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (totalDuration > 3600) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
  }

  return `${Math.floor(totalSeconds / 60).toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`;
}

function formatFrameCount(value: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return "0";
  }

  return Math.floor(value).toLocaleString("en-US");
}

function canDisplayFrames(fps: number, duration: number) {
  return Number.isFinite(fps) && fps > 0 && Number.isFinite(duration) && duration > 0;
}
```

- [ ] **Step 3: Add smooth-display state and refs**

After the existing `currentTime` state, add:

```ts
  const [displayPosition, setDisplayPosition] = useState(0);
```

After `volumeLevel`, add:

```ts
  const [framesPerSecond, setFramesPerSecond] = useState(0);
  const [timeDisplayMode, setTimeDisplayMode] = useState<TimeDisplayMode>("timecode");
```

After `pendingSeekRef`, add:

```ts
  const playbackClockAnchorRef = useRef<PlaybackClockAnchor>({ position: 0, startedAt: performance.now(), playing: false });
```

- [ ] **Step 4: Add display-clock helper functions**

Add these functions after `seekTarget`:

```ts
  function clampPlaybackPosition(value: number, upperDuration = duration) {
    if (!Number.isFinite(value)) {
      return 0;
    }

    const upperBound = upperDuration > 0 ? upperDuration : value;
    return Math.min(upperBound, Math.max(0, value));
  }

  function anchorDisplayClock(position: number, playing: boolean, upperDuration = duration) {
    const clampedPosition = clampPlaybackPosition(position, upperDuration);
    playbackClockAnchorRef.current = {
      position: clampedPosition,
      startedAt: performance.now(),
      playing,
    };
    setDisplayPosition(clampedPosition);
  }

  function toggleTimeDisplayMode() {
    if (!canDisplayFrames(framesPerSecond, duration)) {
      setTimeDisplayMode("timecode");
      return;
    }

    setTimeDisplayMode((mode) => (mode === "timecode" ? "frames" : "timecode"));
  }
```

- [ ] **Step 5: Add the animation frame effect**

Add this effect after the snapshot polling effect:

```ts
  useEffect(() => {
    if (!media || !isPlaying || duration <= 0) {
      return;
    }

    let frameId = 0;
    const tick = () => {
      const anchor = playbackClockAnchorRef.current;
      const elapsedSeconds = anchor.playing ? (performance.now() - anchor.startedAt) / 1000 : 0;
      setDisplayPosition(clampPlaybackPosition(anchor.position + elapsedSeconds, duration));
      frameId = window.requestAnimationFrame(tick);
    };

    frameId = window.requestAnimationFrame(tick);
    return () => window.cancelAnimationFrame(frameId);
  }, [media?.id, isPlaying, duration]);
```

Add this effect after it so unavailable fps returns to timecode mode:

```ts
  useEffect(() => {
    if (!canDisplayFrames(framesPerSecond, duration)) {
      setTimeDisplayMode("timecode");
    }
  }, [framesPerSecond, duration]);
```

- [ ] **Step 6: Anchor display state from snapshots, seeks, and opens**

In `applySnapshot`, replace the first status updates with this sequence:

```ts
    const nextIsPlaying = !snapshot.paused && snapshot.status !== "idle" && snapshot.status !== "ended";

    setDuration(snapshotDuration);
    setIsPlaying(nextIsPlaying);
    setFramesPerSecond(Number.isFinite(snapshot.fps) && snapshot.fps > 0 ? snapshot.fps : 0);
    setVolumeLevel(Math.min(1, Math.max(0, snapshot.volume / 100)));
```

At the end of `applySnapshot`, immediately after `setCurrentTime(snapshotPosition);`, add:

```ts
    anchorDisplayClock(snapshotPosition, nextIsPlaying, snapshotDuration);
```

In `seekTo`, replace `setCurrentTime(target);` with:

```ts
    setCurrentTime(target);
    anchorDisplayClock(target, false);
```

In `commitSeekTo`, replace `setCurrentTime(target);` with:

```ts
    setCurrentTime(target);
    anchorDisplayClock(target, false);
```

- [ ] **Step 7: Derive labels and frame counts from display time**

Replace the existing `displayTime`, `progress`, `queueItems`, and `isChromeHidden` derived constants with:

```ts
  const displayTime = snapEndOfMediaPosition(displayPosition, duration, isPlaying);
  const progress = duration > 0 ? Math.min(100, Math.max(0, (displayTime / duration) * 100)) : 0;
  const queueItems = queue.length ? queue : media ? [media] : [];
  const canShowFrames = canDisplayFrames(framesPerSecond, duration);
  const effectiveTimeDisplayMode: TimeDisplayMode = timeDisplayMode === "frames" && canShowFrames ? "frames" : "timecode";
  const totalFrames = canShowFrames ? Math.max(0, Math.floor(duration * framesPerSecond)) : 0;
  const currentFrame = canShowFrames ? Math.min(totalFrames, Math.max(0, Math.floor(displayTime * framesPerSecond))) : 0;
  const currentTransportLabel = effectiveTimeDisplayMode === "frames" ? formatFrameCount(currentFrame) : formatTimecode(displayTime, duration);
  const durationTransportLabel = effectiveTimeDisplayMode === "frames" ? formatFrameCount(totalFrames) : formatTimecode(duration, duration);
  const timeToggleLabel = canShowFrames ? "Toggle timecode and frame display" : "Frame display unavailable for this media";
  const isChromeHidden = Boolean(media) && !isChromeVisible && !isChromePinned;
```

- [ ] **Step 8: Make both transport labels toggle buttons**

Replace the left transport time span:

```tsx
<span className="transport-time">{formatTime(displayTime)}</span>
```

with:

```tsx
<button className="transport-time transport-time--toggle" type="button" aria-label={timeToggleLabel} onClick={toggleTimeDisplayMode} disabled={!canShowFrames}>
  {currentTransportLabel}
</button>
```

Replace the right transport time span:

```tsx
<span className="transport-time">{formatTime(duration)}</span>
```

with:

```tsx
<button className="transport-time transport-time--toggle" type="button" aria-label={timeToggleLabel} onClick={toggleTimeDisplayMode} disabled={!canShowFrames}>
  {durationTransportLabel}
</button>
```

- [ ] **Step 9: Style the toggle labels without changing layout**

In `apps/desktop/src/styles.css`, add this block after the existing `.transport-time` rule:

```css
.transport-time--toggle {
  border: 0;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  font-variant-numeric: tabular-nums;
  padding: 0;
}

.transport-time--toggle:disabled {
  cursor: default;
}

.transport-time--toggle:not(:disabled):hover {
  color: var(--text);
}
```

- [ ] **Step 10: Run frontend build**

Run from `apps/desktop`:

```bash
npm run build
```

Expected: PASS. TypeScript accepts the new snapshot shape and Vite builds the overlay.

### Task 4: Add Pre-React Video Surface Background

**Files:**
- Modify: `apps/desktop/index.html`

- [ ] **Step 1: Add pre-React surface CSS and classifier**

Replace the `<head>` contents of `apps/desktop/index.html` with:

```html
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <script>
      document.documentElement.classList.add(new URLSearchParams(window.location.search).get("surface") === "video" ? "surface-video" : "surface-overlay");
    </script>
    <style>
      html.surface-video,
      html.surface-video body,
      html.surface-video #root {
        background: #000;
      }

      html.surface-overlay,
      html.surface-overlay body,
      html.surface-overlay #root {
        background: transparent;
      }
    </style>
    <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
    <title>OpenPlayer</title>
  </head>
```

- [ ] **Step 2: Run frontend build**

Run from `apps/desktop`:

```bash
npm run build
```

Expected: PASS. The inline script and style are accepted by Vite.

### Task 5: Final Verification

**Files:**
- Read: `apps/desktop/scripts/verify-shell.mjs`
- Read: `apps/desktop/src/App.tsx`
- Read: `apps/desktop/src-tauri/src/mpv_embed.rs`
- Read: `apps/desktop/index.html`

- [ ] **Step 1: Run shell guards**

Run from `apps/desktop`:

```bash
npm run verify:shell
```

Expected: PASS. Static guards confirm smooth clock, frame mode, fps metadata, and startup background requirements.

- [ ] **Step 2: Run frontend build**

Run from `apps/desktop`:

```bash
npm run build
```

Expected: PASS. TypeScript and Vite compile the new UI behavior.

- [ ] **Step 3: Run Rust check**

Run from repository root:

```bash
cargo check -p openplayer-desktop
```

Expected: PASS. The desktop crate compiles with `fps` in the snapshot payload.

- [ ] **Step 4: Verify startup surface classes in a browser**

Start Vite from `apps/desktop`:

```bash
npm run dev
```

Open `http://127.0.0.1:23142/?surface=video` and evaluate:

```js
JSON.stringify({
  classes: document.documentElement.className,
  background: getComputedStyle(document.documentElement).backgroundColor,
})
```

Expected: classes include `surface-video` and background is `rgb(0, 0, 0)`.

Open `http://127.0.0.1:23142/?surface=overlay` and evaluate:

```js
JSON.stringify({
  classes: document.documentElement.className,
  background: getComputedStyle(document.documentElement).backgroundColor,
})
```

Expected: classes include `surface-overlay` and background is transparent.

- [ ] **Step 5: Stop the dev server and inspect status**

Stop the Vite process that was started in Step 4. Then run from repository root:

```bash
git status --short -uall
```

Expected: changed files are limited to `apps/desktop/scripts/verify-shell.mjs`, `apps/desktop/src-tauri/src/mpv_embed.rs`, `apps/desktop/src/App.tsx`, `apps/desktop/src/styles.css`, `apps/desktop/index.html`, and this plan file.
