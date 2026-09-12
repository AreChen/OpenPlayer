# Native video adapters

**Native video requires OpenPlayer 1.6.4 or later; unavailable in 1.6.3.**
The current development checkout additionally implements the experimental Windows
x64 `present-rgba-v1` adapter. Its presence here is not a claim of availability
in a released installer or a completed installable XeFG plugin.
The main mpv host window and transparent React overlay remain; a presenter owns
a native child output window. Neither adapter makes video-plan scheduling executable.

## Trust and declaration

Declare both `native.process` and `native.video`. Add
`videoAdapter: "vapoursynth-rgb-v1"` or `"present-rgba-v1"` to a native module with only a
`windows-x86_64` target and the methods `frames.open`, `frames.status`, and
`frames.close`. Other method names remain plugin-defined. No vendor-specific
permission is required. The VapourSynth adapter requires the schema-2 inventory
and private runtime layout described in [runtime ownership](native-runtime-ownership.md).

The VapourSynth path runs trusted package code inside the player through
VapourSynth/CPython; a runtime failure can crash the player. The presentation path
keeps the vendor runtime in the native module process. Installing the plugin
grants its declared permissions, including native file/network access; starting
it does not prompt again. Permission descriptions remain visible in plugin
settings. A hash is not a signature. Only install trusted packages.

## Shared API and status

This example uses the existing `vapoursynth-rgb-v1` adapter. Its
`inputConversion` option does not configure the experimental presentation path.

```js
if (openplayer.capabilities.has("native.video") &&
    openplayer.capabilities.hasPermission("native.video")) {
  await openplayer.native.start("enhance"); // Uses installed declared permissions.
  // Configure the module using its declared, plugin-specific control methods.
  const result = await openplayer.native.video.attach("enhance", {
    inputConversion: "sdr-bt709", // Optional compatibility conversion, not HDR passthrough.
    frameRateLimit: 15, // Optional fixed-cadence cap before expensive processing.
    settings: {},
  });
  await openplayer.log.info(JSON.stringify(result));
  const state = await openplayer.native.video.status("enhance");
  await openplayer.native.video.detach("enhance");
  await openplayer.native.stop("enhance");
}
```

| API | Contract |
| --- | --- |
| `attach(moduleId, options?)` | Requires a running, authorized session with a declared adapter. Installs the owned VapourSynth filter or starts the exclusive presentation source, depending on the declaration. Endpoints and window handles are host-owned. |
| `status(moduleId)` | Returns `supported`, `running`, `attached`, `filterEnabled`, and on the new host `presentationActive`. Unsupported platforms return `supported: false`. Read the module's diagnostics separately for generated-frame counts and errors. |
| `refreshPaused(moduleId)` | Queues an exact zero-distance seek for an active attachment; returns `false` while playing. This remains asynchronous. Presenter `refreshPaused` has not been independently tested; the paused OSD redraw test does not cover this method. |
| `detach(moduleId)` | Removes the owned filter or restores the saved output/decode settings and stops the presentation source, then calls `frames.close`. Presenter detach waits for `closed`; the control module can remain running. |
| `stop(moduleId)` | Terminates the entire native job, waits, and cleans its attachment. This is the reference plugin's stop-and-release action. |

`attached` means a cleanup lease exists. `filterEnabled` describes this module's
owned `vapoursynth-rgb-v1` filter, not the whole mpv filter list.
`presentationActive` means this module owns an active host presentation source
whose transport is not closed; it is not proof of generated frames or correct
physical output. The SDK makes it optional for older hosts. A presenter can
report `filterEnabled: false` and `presentationActive: true` even with an upstream
NR filter; query the NR module separately for that filter's status.

## Experimental CPU RGBA presentation

`present-rgba-v1` uses mpv's software render API (`sw`, `rgb0` with opaque alpha)
to produce tightly packed CPU RGBA after the mpv filter chain. The host switches
to `vo=libmpv`, `hwdec=no`, retaining the same mpv core and media/audio clock.
The module receives a bounded shared-memory endpoint using
`openplayer-present-rgba-v1`, not pixels in JSON or shared GPU textures. The host
supplies `endpoint`, `parentWindow`, and `sourceFps` to `frames.open` and waits for
`ready` before installing the presentation source. The XeFG module selects its
GPU by explicit `adapterLuid`; it does not use a default-device fallback.

The host currently requires seekable media with known cadence and SDR BT.709
filter output: `colormatrix=bt.709` and gamma `bt.1886`, `bt.709`, or `srgb`.
HDR/Dolby Vision must already have been converted upstream before attachment.
The old adapter's `inputConversion` option does not perform conversion here.
Software rendering does not support mpv `brightness` and other GPU VO options;
GPU shaders, tone mapping, and complete color equivalence with the normal GPU
output are not supported promises. CPU RGBA transport is not HDR passthrough.

Only one presenter may own the output. An existing `vapoursynth-rgb-v1` NR filter
can remain upstream and its output can feed the presenter; a short NR plus XeFG
integration run is recorded below. This is not an executable multi-stage video plan. The native
module owns its presentation child; the host continues to own media and the
transparent control overlay.

Development host options `width` and `height` specify opening limits (defaults
1920x1080; integer ranges 128..3840 and 128..2160), with endpoint capacity fixed
at `width * height * 4` bytes. Actual initial/frame dimensions follow the window
viewport, fit within these limits, and change after a 150 ms stable-size debounce.
The host scales down larger viewports; each dimension has a 128-pixel minimum,
so exact aspect preservation is not promised for very small/extreme windows.
Resize invalidates the epoch and requests `REDRAW`/`RESET`, including while paused.
The module resizes its swapchain/resources on its render thread without recreating
the XeFG context or changing GPU. Increasing the opening capacity requires a new
attachment. These are implemented mechanics, not a passed host paused-resize test.

`frameRateLimit` is optional, finite, and 1..120. The presenter applies a drop-only
cap against mpv presentation timestamps, without inserting an `fps` filter or
duplicating slow upstream frames. Skipped frames still acknowledge mpv's render
contract; audio speed is unchanged. Frame durations follow actual selected
timestamps rather than an initially estimated filter FPS. Redraws bypass the cap.
Omitting the limit preserves upstream cadence. This does not reduce work already
done by NR and is not a generated-output FPS guarantee. Upstream filters remain
owned by their original adapters.

The TypeScript `NativeVideoAttachOptions` interface types `inputConversion`,
`frameRateLimit`, `settings`, `width`, and `height`. Its string index signature
accepts module-specific top-level options such as `adapterLuid` as `unknown`,
without weakening known field types or adding vendor-specific SDK fields.
The host and module still validate options at runtime; host-owned endpoint,
window and cadence fields cannot be supplied by plugins.

### Host verification (2026-09-12)

The real installed-package host harness verified generated frames, paused
`show-text` OSD redraw with a changed pixel hash and no position/generated-count
advance, seek/resume, resize and maximized-to-fullscreen transitions, detach and
re-attach, and native-process crash recovery retaining the same mpv core and
restoring actual `current-vo` and `hwdec`. The normal close path uses `WM_CLOSE`.
Evidence: [`presentation-smoke.log`](../../target/presentation-smoke.log).

Real Alt+F4 completed process exit in
[`presentation-smoke-alt-f4.log`](../../target/presentation-smoke-alt-f4.log).
Closing while `frames.open` initialization held the media guard also completed
normal exit in [`presentation-smoke-init-close.log`](../../target/presentation-smoke-init-close.log).
These local build logs record the tested harness paths, not a shipped XeFG product.

The real XeFG `.opplugin` and accepted DLSSNR 0.4.0 package were also tested on
GPU 1, with NR capped at 15 FPS and the presenter capped at 30 FPS. In 12.224
seconds, NR processed 184 frames, the presenter received 183 source frames and
generated 182 frames. Media advanced 12.200 seconds; maximum sampled mpv A/V
error was 0.032 ms. No input upsampling was used. A paused NR intensity update
changed downstream pixels without advancing playback. Detach/crash recovery
retained the NR worker, and application close stopped both process trees.
Evidence: [`nr-xefg-drop-only.log`](../../target/nr-xefg-drop-only.log).
The current drop-only source also passed actual Alt+F4 and initialization-close
in `target/xefg-drop-only-alt-f4.log` and `target/xefg-drop-only-init-close.log`.

An installable XeFG 0.1.0 candidate and theme-aware UI are available in the sibling
`openplayer-xefg` repository. Its browser tests use simulated native replies;
real React UI integration, long-duration A/V behavior, paused resize and user
image-quality acceptance remain pending. Counters and mpv A/V diagnostics do not
establish physical display FPS, perceived lip-sync, subtitle fidelity or color
equivalence. No desktop pixel readback is required for this contract.

## VapourSynth filter adapter

The following behavior and historical evidence apply to `vapoursynth-rgb-v1`.
Duplicate attachment is rejected. Installation failures stop the attempted
session and execute rollback; cleanup failures retain tracking for retry.
Its frame endpoint is accepted only from the authorized module's reply, with
protocol `openplayer-frame-experimental-v1`, a nonzero `u16` port and a 64-hex
token. Transport uses loopback and bounded shared buffers, not pixel JSON RPC.

### Media and lifecycle

The adapter processes constant 8-bit, limited-range BT.709 SDR YUV up to
3840x2160. `inputConversion` defaults to `"none"`; `"sdr-bt709"` permits the host
to normalize HDR, Dolby Vision, other ranges/matrices and 10-bit input before the
adapter. Already-compatible SDR bypasses conversion. This preserves resolution
and frame rate, but **outputs SDR, not HDR or Dolby Vision**.

`frameRateLimit` is an optional finite number from 1 to 120. It is consumed by
the host, not forwarded to `frames.open`. The host inserts an owned FFmpeg `fps`
filter before color normalization and native processing, using the lower of the
requested limit and known container FPS (the requested rate when unknown).
It changes video cadence, not media duration or audio speed. Variable-rate input
is normalized to that cadence, which can also repeat frames. Omitting the option
preserves the previous input-cadence behavior. Detach/stop removes the rate filter
and restores original playback. Change the limit by detaching and attaching again.
This is a capacity control, not automatic performance detection or a guarantee
that any GPU can sustain the selected rate.

The host batches a fixed hardware-download/10-bit format, libplacebo conversion,
and VapourSynth attachment into one filter-list update. libplacebo applies DV RPU
metadata and removes it from the converted output. Each filter has an owned label;
detach/stop/exit remove the entire owned chain while preserving unrelated filters.
If conversion fails, status must not report a healthy enabled attachment. The
plugin receives only its settings in `frames.open`; the host consumes the reserved
`inputConversion` option. Plugins cannot supply arbitrary conversion filter text.

This requires the bundled libmpv's FFmpeg/libplacebo filter support. It is not a
zero-copy path. At 4K, seeks can take several seconds and CPU conversion plus
NR processing can fall behind audio. Compatibility is not a claim of real-time
4K playback, HDR preservation, interpolation, upscaling or shared GPU textures.
The processing GPU selected by the plugin is the NR GPU; mpv owns decode, color
conversion and presentation device selection independently.
`native.video.validatePlan()` remains validation-only (`executable: false`).

Opening another media item or stopping playback stops declared video sessions
before replacing/destroying the mpv player. Re-enable explicitly on the next
media item. Disable, upgrade, uninstall, worker exit and app exit use session-owned
cleanup. Seek/filter rebuild resets are provided by the frame adapter. Closing
the plugin panel alone leaves processing active; stopping is an explicit action.
The cached embedded runtime stays resident until app exit; changing its inventory
requires restarting the player, not merely toggling the plugin.

### Verification

2026-09-10: sibling `.local/paused-preview-first.json` verifies two parameter
updates change displayed pixels while paused, retain position (within 20ms) and
reuse the same GPU-1 worker. Refresh during playback is a no-op. Exact refresh
can be slower for long-GOP media; coalesce slider updates before requesting it.

`.local/realtime-1080p60-final.json` uses the user's 1920x1080 60000/1001 FPS
video on GPU 1 with a 15 FPS cap: 303 processed frames in 20.21 seconds,
20.20 seconds of media progression and peak mpv-reported A/V error below 1ms.
`.local/paused-1080p60-final.json` verifies paused pixel changes with that cap.
These are short functional runs, not perceptual lip-sync or long-duration guarantees.

2026-09-09 evidence in sibling `openplayer-dlssnr/.local/`:

- `public-lifecycle-matrix.json`: stop, disable, upgrade, uninstall, worker crash
  and active-enhancement app exit passed with real GPU-1 processing.
- `public-media-detach.json`: public detach and actual 720p-to-1080p media
  replacement passed, with zero native job processes afterward.
- `public-final-cleanup.json`: the final plugin package passed cold runtime
  loading through the public command, duplicate-attach rejection, stop/app exit
  and checks of the actual host-cache script directory.
- `plugin-ui-final-check/report.json`: the actual generated view bridge/styles
  passed browser tests for explicit GPU choice, settings restore, directory
  picker, enable/stop, failure cleanup and narrow layouts, using simulated replies.

These window tests exercise production commands but register an internal test
session. On 2026-09-10 the user confirmed GUI installation and enhancement work.
Launch confirmations were subsequently removed at the user's request.
Long-duration A/V synchronization, color
fidelity, mid-stream format changes and sustained performance remain unverified.

2026-09-10: `.local/gpu1-live-update.json` in the sibling plugin repository proves
parameter updates change pixels without replacing the GPU-1 worker.
`.local/dv-profile5-batched.json` uses the user's 3840x1606 Profile-5 file, seeks
to 180 seconds, waits for `seeking=no`, actual BT.709 SDR output and advancing
playback before taking a scene screenshot. It passes processing, worker reuse,
owned-chain removal and original Dolby Vision playback recovery. Earlier tests
that captured immediately after seeking were insufficient to establish readiness.
