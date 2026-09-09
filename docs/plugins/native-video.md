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
VapourSynth/CPython. A runtime failure can crash the player. Native launch
confirmation explicitly discloses this, in addition to the native process's
file/network access. A hash is not a signature. Only load trusted packages.

## Compose the APIs

```js
if (openplayer.capabilities.has("native.video") &&
    openplayer.capabilities.hasPermission("native.video")) {
  await openplayer.native.start("enhance"); // Host-owned launch confirmation.
  // Configure the module using its declared, plugin-specific control methods.
  const result = await openplayer.native.video.attach("enhance", { settings: {} });
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
| `detach(moduleId)` | Removes the owned filter, calls `frames.close` if the module is running, and retains the control process. Repeating detach is harmless for the reference module. |
| `stop(moduleId)` | Terminates the entire native job, waits, and cleans its attachment. This is the reference plugin's stop-and-release action. |

Only one native attachment can own the player. Duplicate attachment is rejected
without stopping the existing attachment. Installation failures stop the attempted
session and execute rollback; cleanup failures retain tracking for retry.
The frame endpoint is accepted only from the authorized module's reply, with
protocol `openplayer-frame-experimental-v1`, a nonzero `u16` port and a 64-hex
token. Transport uses loopback and bounded shared buffers, not pixel JSON RPC.

## Media and lifecycle

The initial adapter accepts constant 8-bit, limited-range BT.709 SDR YUV up to
1920x1080. It preserves dimensions and frame rate; it does not implement HDR,
interpolation, upscaling, multi-stage execution or shared GPU textures.
`native.video.validatePlan()` remains validation-only (`executable: false`).

Opening another media item or stopping playback stops declared video sessions
before replacing/destroying the mpv player. Re-enable explicitly on the next
media item. Disable, upgrade, uninstall, worker exit and app exit use session-owned
cleanup. Seek/filter rebuild resets are provided by the frame adapter. Closing
the plugin panel alone leaves processing active; stopping is an explicit action.
The cached embedded runtime stays resident until app exit; changing its inventory
requires restarting the player, not merely toggling the plugin.

## Verification

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
session instead of clicking launch consent. Full GUI consent plus enhancement
remains a manual acceptance step. Long-duration A/V synchronization, color
fidelity, mid-stream format changes and sustained performance remain unverified.
