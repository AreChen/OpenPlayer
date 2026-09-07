# Regression checks

## Frontend and backend behavior

From `apps/desktop`:

```powershell
rtk npm test
rtk npm run verify:shell
rtk npm run build
```

From the repository root:

```powershell
rtk cargo test -p openplayer-desktop --locked
rtk cargo clippy --workspace --all-targets -- -D warnings
```

The Node tests execute the TypeScript source with controlled hook lifecycles,
timers and IPC responses. They cover slow polling, invalidation, unmount,
atomic task updates, batched store sync and playback clock visibility.
They complement, rather than replace, the structural `verify:shell` checks.
Rust network tests use a local HTTP server and verify that an oversized chunked
response is rejected before the server terminates the body. Store tests cover
schema initialization, prefix boundaries and persistence.

## Native window smoke tests

These tests open temporary native windows on an interactive Windows or macOS
desktop. They create their own video fixture and WebView profile, do not load
user plugins or databases, and have both a process watchdog and bounded
main-thread checks.

With the platform's usual libmpv build dependencies installed, run from the
repository root:

```powershell
rtk cargo build -p openplayer-desktop --locked --features window-smoke --bin window-smoke
rtk proxy node apps/desktop/scripts/run-window-smoke.mjs
```

On Windows, `apps/desktop/scripts/restore-windows-mpv.ps1` can restore the pinned
runtime using the repository's SHA256 manifest; the runner sets the DLL search
path automatically. macOS uses `brew install mpv pkg-config` as in release CI.

Each run loads video into the real native mpv host, continuously resizes the
appropriate window, checks overlay alignment, enters/exits fullscreen and
checks restoration of the original outer bounds. Separate processes close the
overlay and main window, requiring both windows and the mpv player to be gone
before successful exit. Windows sends `SC_CLOSE`, the native system-close path
used by Alt+F4, without injecting global keyboard input. macOS exercises native
close requests through Tauri. Physical keyboard routing, mouse edge dragging,
multi-monitor DPI changes and GPU-specific visual correctness still need manual
platform testing.

The `window-smoke` binary is feature-gated and is not part of normal release
builds. Standard CI now builds/tests Rust on Windows and macOS and runs these
native checks, in addition to the existing Linux checks.

## Playback rendering measurements

`scripts/verify-playback-rendering.mjs` starts and stops an ephemeral Vite server
and uses Playwright to render the actual timeline at desktop and narrow widths.
It checks time/frame labels, seeking anchors, smooth updates, zero parent
renders from clock ticks and zero animation requests from a hidden timeline.
It saves screenshots under `target/playback-rendering/`.

From `apps/desktop`, with Playwright and a Chromium browser available:

```powershell
rtk proxy node scripts/verify-playback-rendering.mjs
```

When using an externally provided Playwright installation, set
`OPENPLAYER_PLAYWRIGHT_MODULE` to its module directory. Set
`OPENPLAYER_BROWSER_EXECUTABLE` to use an already installed Chrome executable.
No additional runtime dependencies are needed by the player itself.

Measurements from this harness isolate clock-driven React work; they are not
whole-player CPU or GPU benchmarks. Store batching reduces database opens per
sync cycle from five to two, but disk-time improvements depend on the machine,
store size and concurrent player processes.
