# OpenPlayer on Windows

The Windows executable and NSIS installer use one native GPUI Fast window. libmpv renders video through D3D11 and DirectComposition without a WebView dependency. GPUI Motion respects the Windows animations setting. The current video composition path targets 8-bit SDR.

Playback uses the active Windows audio output when available. If Windows has no usable output device, libmpv keeps playback and its timeline running silently.

## Run and package

Use rustup with the x64 MSVC host and PowerShell. Cargo selects Rust 1.100.0 from the project toolchain file. From the repository root:

~~~powershell
./apps/desktop/scripts/restore-windows-mpv.ps1
$env:PATH = "$(Resolve-Path vendor/native/mpv/windows-x64);$env:PATH"
cargo run --locked -p openplayer-desktop --bin openplayer-desktop -- "C:\Media\sample.mp4"
~~~

Use `--adapter INDEX` to select a DXGI adapter. Set `OPENPLAYER_WINDOWS_SMOKE_PROFILE` to a name such as `regression` for isolated playback and preference data. See [native runtime metadata](native-deps/README.md) for the pinned binary and checksums.

Install NSIS and run the package script from the repository root:

~~~powershell
./apps/desktop/scripts/build-windows.ps1
~~~

The installer is created in `target/release/bundle/nsis/`. The package includes libmpv and uses a statically linked Visual C++ runtime. The finish page offers two independent options. **Launch OpenPlayer now** is selected by default. **Choose default apps in Windows Settings** opens the system settings page. During upgrade and removal, the installer closes the process that is using the files in the selected installation directory.

## Default player

Open **Settings → Playback → Default Player** to check how many supported file types currently open with OpenPlayer. Choose **Open Windows Default Apps**, then select OpenPlayer for the file types you want. The File Explorer preview controls register thumbnails and media metadata for the selected formats.

## File browser

Use **Sort** to order folders and files by name, file type, modified date, or size. Choose ascending or descending in the same menu. **Category** lets you show all items, video, audio, or folders. **Only media** starts off for new preferences. Turn it on to show playable video and audio files. Selecting **Folders** turns it off. These controls apply to search results, and the search still scans subfolders.

The browser remembers its last folder, sort order, category, and **Only media** setting between sessions. Opening a local video sets its folder as the starting location for the next browse.

## Window fitting

In windowed mode, OpenPlayer fits the window to the video's aspect ratio within the available display area. Dragging a window edge pauses automatic fitting for the session. Press **Ctrl+H** or choose **Fit Window to Video** from the right-click menu to resume it. In narrow windows, secondary playback actions are available under **More playback controls**. In fullscreen, **Fit** shows the entire image and **Fill** covers the display by cropping it.

## Updates

OpenPlayer 2.x checks the GitHub release list in the background. To check immediately, open **Settings → About → Check for Updates**. When a newer stable 2.x installer and its SHA-256 checksum are available for Windows x64, choose **Download Installer**. After verification, choose **Install Update** and confirm in OpenPlayer Setup. Setup closes the running player, updates the registered per-user installation and offers to launch OpenPlayer again. Settings, playback history and favorites are stored outside the installation directory.

Published 1.x builds use GitHub's repository-wide `/releases/latest` endpoint, so keep a stable 1.x release marked **Latest**. Publish 2.x with `--latest=false` and verify the legacy endpoint before and after publishing. [Release workflow](../.github/workflows/release.yml)
