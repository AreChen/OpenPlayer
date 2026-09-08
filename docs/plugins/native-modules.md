# Native modules (development extension)

This development host can run plugin-packaged native executables through a
bounded control protocol. Published OpenPlayer 1.6.3 binaries do not include this
extension. The application version has not been bumped for a release.

**Native modules are trusted software running with the current user's file,
network, and environment access. They are NOT sandboxed.** The JavaScript worker
remains sandboxed; `native.process` is a separate, high-risk permission with an
explicit host-owned confirmation for each new process launch.

**There is no public live video processing attachment yet.**
`native.video.validatePlan()` checks a proposed format/rate chain and always
returns `executable: false`. It does not enable DLSSNR, interpolation, upscaling,
or rendering. The existing mpv host and transparent control window are unchanged.

## Run the reference module

From the sibling `openplayer-plugins` repository, with Node.js, RTK, and Rust
installed:

```powershell
rtk proxy node scripts/build-native-example.mjs
```

This builds the Rust echo executable, generates a manifest with its actual SHA256,
and validates `.native-example/`. Import that directory into a development host.
The context-menu action requests native launch confirmation, performs an echo
round trip, and stops the module. The script neither installs the example nor
adds it to the official plugin catalog. Do not distribute the fault-injection
`protocol-fixture` executable as a plugin.

Native executables must be imported in a directory or `.opplugin` package, not
as a standalone manifest file. Older hosts reject the new manifest contribution;
capability detection alone cannot make the package installable on those hosts.

## Declare modules

Add `contributes.nativeModules`, an array of at most four declarations:

| Field | Contract |
| --- | --- |
| `id` | Unique module identifier, 1..96 ASCII letters/digits/dot/underscore/hyphen |
| `protocol` | `openplayer-native-v1` |
| `methods` | 1..64 unique identifiers; `host.*` is reserved |
| `targets` | Platform-to-executable map; at least one target |
| `targets[platform].entry` | Relative package path, at most 240 UTF-8 bytes; Windows requires `.exe` |
| `targets[platform].sha256` | 64 hex characters, verified against the executable |
| `targets[platform].args` | Optional fixed arguments, at most 16, each at most 1024 UTF-8 bytes |

Platforms are `windows-x86_64`, `windows-aarch64`, `linux-x86_64`,
`linux-aarch64`, `macos-x86_64`, and `macos-aarch64`. Platform selection is based
on the host process architecture. Only Windows x86_64 was execution-tested in
this implementation batch; other targets still need native CI/runtime validation.

Declare `native.process` in a capability's permissions. `nativeTool` is a broad
capability category, not a separate permission. Existing categories may also
include this permission. No vendor, AI, model, or effect names are host gates.

The installer verifies every declared executable before replacing an installed
package. Launch rechecks the selected executable before and after confirmation.
Each executable is limited to 128 MiB; the existing whole-package size limit
still applies. SHA256 detects mismatch with the manifest, but is not a signature
or proof of publisher identity and does not authenticate dependent DLLs/models.
Trust the entire package. Proprietary runtime distribution remains the plugin
author's responsibility.

## Call from a worker or custom view

Both bridges expose the same API. Calls are scoped to modules declared by the
calling plugin; a plugin cannot use another plugin's identity or module registry.

```js
if (openplayer.capabilities.has("native.process") &&
    openplayer.capabilities.hasPermission("native.process")) {
  const modules = await openplayer.native.list();
  if (modules.some((module) => module.id === "echo" && module.supported)) {
    try {
      await openplayer.native.start("echo");
      const result = await openplayer.native.call("echo", "describe", null, {
        timeoutMs: 5000,
      });
      await openplayer.log.info(JSON.stringify(result));
    } finally {
      await openplayer.native.stop("echo");
    }
  }
}
```

| API | Behavior |
| --- | --- |
| `list()` | Module id, protocol, methods, platform support, and running state |
| `start(moduleId)` | Verify permission/enablement/integrity, prompt, spawn, negotiate; reuse an already-running session |
| `call(moduleId, method, params?, options?)` | One bounded request to an explicitly declared method |
| `stop(moduleId)` | Terminate the module process group/job; on Windows, wait for job processes to exit |
| `stopAll()` | Stop this plugin's modules, not other plugins |
| `video.validatePlan(plan)` | Validate and normalize a proposed chain; never executes it |

There are at most 16 registered sessions across the application. Each session
allows one in-flight request. Concurrent requests fail with `busy`, rather than
forming an unbounded queue. `timeoutMs` is 1..5000, default 5000. Starting a new
process prompts again; there is no persisted blanket grant. Only one launch
confirmation can be pending. Confirmation may take longer than command callback
deadlines; the reference runtime starts that interaction outside its timed
command callback and reports completion/failure separately.

Timeout, transport failure, worker error, malformed output, or protocol mismatch
terminates the session. The host does not restart it silently. Undeclared methods,
invalid timeouts, oversized input, and `busy` fail without dispatching work.
Expected domain failures should be represented in the worker's `result` object
if the worker is to remain usable. Long operations should return a job id quickly
and expose short status/cancel methods, composed with `openplayer.tasks` in JS.

## Implement a worker

The reusable Rust server is `openplayer-plugins/packages/native-sdk`. Other
languages can implement the same UTF-8 newline-delimited JSON protocol over
stdin/stdout. Reserve stdout for protocol messages. The current host discards
stderr; expose actionable diagnostics in replies and forward them to
`openplayer.log` from JavaScript.

Initial host request and worker reply:

```json
{"id":1,"method":"host.initialize","params":{"protocol":"openplayer-native-v1","pluginId":"dev.openplayer.example.native","moduleId":"echo","maxMessageBytes":65536}}
{"id":1,"result":{"protocol":"openplayer-native-v1"}}
```

Subsequent requests use the same `id`, `method`, and `params` fields. Each reply
has the matching unsigned integer `id` and exactly one of `result` (any JSON,
including null) or `error` (a string). Unknown reply fields and missing newlines
are rejected. The 65536-byte response limit includes the newline. Request params
are limited to 64512 serialized bytes, reserving space for the envelope. Flush
each reply. There are no unsolicited messages or worker-to-host RPCs in v1.

The executable's parent directory is its working directory. JavaScript cannot
choose an executable, append command-line arguments, inject environment variables,
or call `host.*`. These are API restrictions, not OS isolation: the native process
inherits the host environment and can access the user's resources directly.
Keep persistent plugin state in host-owned redb via JS `openplayer.storage`,
not by opening the host database from native code.

## Lifecycle and cleanup

- Disable, upgrade, and uninstall stop that plugin's registered native modules.
- Pending starts are rejected if a plugin lifecycle operation invalidates their
  snapshot while confirmation is open. Retry explicitly after the operation.
- On Windows, workers start suspended, join a kill-on-close Job Object, and only
  then execute, so startup helpers inherit the job. Stop waits up to two seconds
  per job; failed cleanup preserves tracking and fails the operation for retry.
- An idle worker-exit monitor checks every 250 ms and stops its remaining helpers.
  An in-flight request has its own timeout/EOF handling.
- On Unix, workers use a separate process group and stop sends SIGKILL to that
  group. Detached sessions and host crashes do not have Windows Job Object
  guarantees. Cross-platform execution and cleanup remain unverified here.
- Application exit stops registered jobs/groups. Closing a custom view alone
  does not stop its plugin's processes; call `stop` when that workflow is done.
- Uninstall removes managed package files and existing host-owned plugin data.
  Arbitrary files created elsewhere by trusted native code are not automatically
  discovered or deleted. Native shutdown callbacks are not guaranteed.

## Validate a resolution/rate chain

The validator accepts 1..8 ordered stages referencing modules in the same plugin.
Each stage specifies `input`, `output`, `lookaheadFrames`, and `maxOutputFrames`.
Formats describe width/height, rational `frameRate`, `pixelFormat`, and
`colorSpace`. Equivalent rates are normalized, and adjacent formats must match.
There is no automatic format conversion or cross-plugin module sharing.

Bounds are 16..8192 pixels per dimension, positive rate numerator/denominator up
to 1000000, at most 480 fps, 0..8 lookahead frames, and 1..8 output frames per
input frame. The output/input rate ratio must fit `maxOutputFrames`; accumulated
lookahead is at most two seconds. Pixel formats are `rgba8` and `rgba16f`; color
spaces are `srgb`, `linear-srgb`, and `bt2020-pq`. Non-sRGB requires `rgba16f`.
These are validation rules, not claims of supported GPU effects or color fidelity.

A denoiser can preserve the format, an upscaler can change dimensions, and an
interpolator can raise the rate with a larger `maxOutputFrames`. The tests cover
720p/24 to 1080p/24 to 1080p/48. The result includes the normalized plan, output
format, accumulated `lookaheadMs`, and `executable: false`.

### Remaining playback work (not implemented APIs)

Live playback still needs an mpv frame adapter, bounded shared-memory/GPU texture
transport, GPU synchronization, and a scheduler carrying frame PTS/duration.
Seek/media/format/device changes must reset temporal history and invalidate stale
outputs. Interpolation additionally needs multiple outputs per input, lookahead,
backpressure, A/V synchronization, EOS draining, and original-frame fallback.
Do not move frame pixels through this JSON control channel. No `attach`,
`processFrame`, timestamp transport, or history-reset API is currently exposed.

The separate `openplayer-dlssnr` prototype now also exercises real decoded frames
inside this repository's `window-smoke` harness. Its opt-in `.vpy` fixture is
compiled behind the `window-smoke` feature, never registered as an IPC command,
and does not change the safe plugin filter allowlist. The two native windows are
retained with inert WebViews; real React/plugin UI interaction was not tested.

The 720p path passed pause/seek, detach/reattach, resize, maximized fullscreen and
single Alt+F4 close with processed nonblack output and clean worker exits. It uses
CPU RGB copies and an isolated NGX process, not shared GPU textures. Tested 1080p
callbacks averaged about 69 ms after warm-up, too slow for 24 FPS. See the sibling
repository's `docs/native-playback-probe.md` for reproducible evidence and limits.

This test adapter is not owned by the native-module registry: private runtime
configuration and worker containment are provided by the developer launcher.
Production lifecycle/consent, packaging, A/V sync and color handling remain gates.
These tests do not establish interpolation or super-resolution engine support.

A subsequent prototype-only step separates engine ownership from short-lived
filter instances. Its opt-in persistent owner reused one worker across three
720p window-test leases (117 processed frames, no fallback), reducing reconnect
to about 1-11 ms rather than repeating engine initialization. This is not a full
seek-latency measurement. The owner's two buffers and worker are released on
owner close; failed workers are not silently restarted on same-format reconnect.
The host SDK registry still does not own this service. See the sibling prototype's
`docs/persistent-frame-owner.md`; no additional host command or permission was
introduced for this test.

## Verification snapshot (2026-09-08)

Windows x86_64 development checkout verification:

- Host Rust suite: 194 passed. The separately gated native process test also
  passed with the SDK's compiled `protocol-fixture`; no fixture processes remained.
- Host JavaScript suite: 14 passed; shell/native architecture checks and frontend
  production build passed.
- Official plugin repository: 47 tests passed; existing catalog packages built.
- Native Rust SDK: two tests and all-target/all-feature strict Clippy passed.
- Host strict workspace Clippy, Rust formatting, SDK TypeScript/example checks,
  and both repositories' diff whitespace checks passed.
- Non-mpv fallback compile check passed after fixing existing feature-gated
  imports/command registration mismatches. It retains unrelated dead-code warnings.
- The echo development directory built and passed manifest/executable validation;
  it was not installed into the user's player.

The process integration test exercises internal sessions, not GUI consent.
Interactive authorization and non-Windows execution were not tested. The framework
was committed locally as host `ee157b0` and official SDK `ffbe061`, without a push,
installer, version bump or release. The follow-up window smoke above does not
constitute a shipped live-filter SDK capability.
