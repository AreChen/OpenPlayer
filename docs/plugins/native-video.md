# Attach a native video processor

**Development builds only; not included in published OpenPlayer 1.6.3.** This is
a single-stage Windows x64 attachment, not executable video-plan scheduling.
The existing mpv window and transparent React overlay remain unchanged.

## Trust and declaration

Declare both `native.process` and `native.video`. Add
`videoAdapter: "vapoursynth-rgb-v1"` to a native module with only a
`windows-x86_64` target and the methods `frames.open`, `frames.status`, and
`frames.close`. Other method names remain plugin-defined. The package root must
contain the schema-2 inventory and private runtime layout described in
[runtime ownership](native-runtime-ownership.md).

This permission allows trusted package code to run inside the player through
VapourSynth/CPython. A runtime failure can crash the player. Installing the plugin
grants its declared permissions, including native file/network access; starting
it does not prompt again. Permission descriptions remain visible in plugin
settings. A hash is not a signature. Only install trusted packages.

## Compose the APIs

```js
if (openplayer.capabilities.has("native.video") &&
    openplayer.capabilities.hasPermission("native.video")) {
  await openplayer.native.start("enhance"); // Uses installed declared permissions.
  // Configure the module using its declared, plugin-specific control methods.
  const result = await openplayer.native.video.attach("enhance", {
    inputConversion: "sdr-bt709", // Optional compatibility conversion, not HDR passthrough.
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
| `attach(moduleId, options?)` | Requires a running, authorized session with a declared adapter. Resolves the installed package, pins its verified host-owned runtime, calls `frames.open(options)`, and installs a host-generated filter. No script, executable, endpoint or filter expression can be supplied by JavaScript. |
| `status(moduleId)` | Returns `supported`, `running`, `attached`, `filterEnabled`. A cleanup lease is not proof of successful inference: also inspect the module's diagnostics. Unsupported platforms return `supported: false`. |
| `refreshPaused(moduleId)` | After a settings update, queue a zero-distance exact seek to re-filter the paused position. Returns `true` when queued, `false` while playing. Requires an active owned attachment and seekable media. Completion is asynchronous; preserves pause and reuses the worker. |
| `detach(moduleId)` | Removes the owned filter, calls `frames.close` if the module is running, and retains the control process. Repeating detach is harmless for the reference module. |
| `stop(moduleId)` | Terminates the entire native job, waits, and cleans its attachment. This is the reference plugin's stop-and-release action. |

Only one native attachment can own the player. Duplicate attachment is rejected
without stopping the existing attachment. Installation failures stop the attempted
session and execute rollback; cleanup failures retain tracking for retry.
The frame endpoint is accepted only from the authorized module's reply, with
protocol `openplayer-frame-experimental-v1`, a nonzero `u16` port and a 64-hex
token. Transport uses loopback and bounded shared buffers, not pixel JSON RPC.

## Media and lifecycle

The adapter processes constant 8-bit, limited-range BT.709 SDR YUV up to
3840x2160. `inputConversion` defaults to `"none"`; `"sdr-bt709"` permits the host
to normalize HDR, Dolby Vision, other ranges/matrices and 10-bit input before the
adapter. Already-compatible SDR bypasses conversion. This preserves resolution
and frame rate, but **outputs SDR, not HDR or Dolby Vision**.

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

## Verification

2026-09-10: sibling `.local/paused-preview-first.json` verifies two parameter
updates change displayed pixels while paused, retain position (within 20ms) and
reuse the same GPU-1 worker. Refresh during playback is a no-op. Exact refresh
can be slower for long-GOP media; coalesce slider updates before requesting it.

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
