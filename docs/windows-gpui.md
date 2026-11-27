# OpenPlayer on Windows

The Windows executable and NSIS installer use one native GPUI Fast window. libmpv renders video through D3D11 and DirectComposition without a WebView dependency. GPUI Motion respects the Windows animations setting. The current video composition path targets 8-bit SDR.

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

The installer is created in `target/release/bundle/nsis/`. The finish page offers two independent options. **Launch OpenPlayer now** is selected by default. **Choose default apps in Windows Settings** opens the system settings page. During upgrade and removal, the installer closes the process that is using the files in the selected installation directory.

## Default player

Open **Settings → Playback → Default Player** to check how many supported file types currently open with OpenPlayer. Choose **Open Windows Default Apps**, then select OpenPlayer for the file types you want. The File Explorer preview controls register thumbnails and media metadata for the selected formats.

## File browser

Use **Sort** to order folders and files by name, file type, modified date, or size. Choose ascending or descending in the same menu. **Category** lets you show all items, video, audio, or folders. **Only media** is on by default and limits the list to playable video and audio files. Use the address bar or sidebar to navigate, or turn off **Only media** to show folders in the list. Selecting **Folders** also turns off **Only media**. These controls apply to search results, and the search still scans subfolders.

## Window fitting

In windowed mode, OpenPlayer fits the window to the video's aspect ratio within the available display area. Dragging a window edge pauses automatic fitting for the session. Press **Ctrl+H** or choose **Fit Window to Video** from the right-click menu to resume it. In narrow windows, secondary playback actions are available under **More playback controls**. In fullscreen, **Fit** shows the entire image and **Fill** covers the display by cropping it.

## Updates

From **Settings → About → Check for Updates**, OpenPlayer 2.x checks the GitHub release list. It offers a newer stable Windows installer for the installed major version when the asset is available.

Published 1.x builds use GitHub's repository-wide `/releases/latest` endpoint, so keep a stable 1.x release marked **Latest**. The release workflow publishes 2.x with `--latest=false` and checks the 1.x endpoint before uploading the new installer. This setting applies to the GitHub release. `main` is the 2.x development branch. [Release workflow](../.github/workflows/release.yml)
