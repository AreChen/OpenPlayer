# Regression checks

## Shared CI checks

Install the host dependencies declared in [CI](../../.github/workflows/ci.yml). From the repository root:

~~~sh
cargo fmt --all -- --check
cargo test --locked -p openplayer-desktop
cargo clippy --locked --workspace --all-targets -- -D warnings
git diff --check
~~~

From `apps/desktop` with Node.js 26:

~~~sh
npm ci
npm run verify:shell
npm test
npm run build
npm run verify:release -- --tag=v2.0.0
~~~

The frontend tests exercise hooks, timer-driven updates and IPC responses. The shell check covers runtime and build-time structure. For native locale changes, run `node apps/desktop/scripts/sync-native-locales.mjs --check` from the repository root.

## Windows GPUI smoke

Prepare [the native runtime](../windows-gpui.md#run-and-package). Use a temporary profile and a known local video. From the repository root in PowerShell:

~~~powershell
$env:OPENPLAYER_WINDOWS_SMOKE_PROFILE = 'regression'
cargo tree -p openplayer-desktop --locked --target x86_64-pc-windows-msvc -e normal,build --prefix none
cargo run --locked -p openplayer-desktop --bin openplayer-desktop -- "C:\Media\sample.mp4"
~~~

The Windows dependency tree should be free of Tauri, Tao, Wry and WebView2. Check video through window resize, maximize and fullscreen, and check playback independently of the controls animation. Use an interactive idle desktop to check blank-stage double-click, stage drag, mouse wheel, volume slider, hover, timeout and pointer exit. Check media properties and clipboard output with a file that has video, audio and subtitles. Remove the smoke profile variable from the session before a normal launch.

For installer changes, build the NSIS bundle and test finish, upgrade and uninstall on an isolated Windows profile. Exercise both final-page actions separately and together, including the required selection in Windows Default Apps.

## Tauri window smoke on macOS

Install the macOS native runtime and build the React frontend first. From the repository root:

~~~sh
cargo build -p openplayer-desktop --locked --features window-smoke --bin window-smoke
node apps/desktop/scripts/run-window-smoke.mjs
~~~

The harness opens temporary windows in an interactive desktop, loads a video into the embedded libmpv host and checks resizing, overlay alignment, fullscreen restoration and coordinated closure. CI runs the harness on macOS. The standard Windows player has its own single-window pipeline.

## React timeline check

Run from `apps/desktop` with Playwright and Chromium available:

~~~sh
node scripts/verify-playback-rendering.mjs
~~~

The script tests the real React timeline at desktop and narrow widths and saves screenshots in `target/playback-rendering/`. Set `OPENPLAYER_PLAYWRIGHT_MODULE` or `OPENPLAYER_BROWSER_EXECUTABLE` to point to existing installations. The measurements cover the React timeline.
