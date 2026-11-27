# OpenPlayer

[简体中文](README.zh-CN.md) · [GitHub Releases](https://github.com/AreChen/OpenPlayer/releases) · [2.0.0 release notes](docs/releases/v2.0.0.md)

<img src="apps/desktop/src-tauri/icons/icon.png" alt="OpenPlayer app icon" width="96" />

OpenPlayer is a desktop media player built with Rust and libmpv. The `main` branch is the 2.x development line.

| Platform | Runtime | Release package |
| --- | --- | --- |
| Windows x64 | Native GPUI Fast window, D3D11 video composition and GPUI Motion | NSIS installer with libmpv |
| Linux | Tauri and React with a native libmpv video host | `.deb` and AppImage |
| macOS Apple Silicon and Intel | Tauri and React with a native libmpv video host | DMG with bundled libmpv |

## Windows player

- Play local media and network streams with speed, loop, frame stepping, subtitle and audio-track controls. Resume from playback history or use incognito mode.
- Double-click the empty player to open media. The file browser remembers the last folder and provides an editable path, favorites, and video thumbnails with silent hover previews. Use `Ctrl+F` to search filenames in the current folder and its subfolders.
- View container, video and audio details from the file browser or the playback menu. Copy the details from the properties panel.
- Adjust the accent color and the opacity of the title bar and transport. The controls hide on pointer exit or after three seconds of inactivity.
- Configure keyboard shortcuts and Windows Explorer media previews. Use the installer finish page to open OpenPlayer or go to Windows Default Apps to select it as your player.

Common shortcuts are `Ctrl+O` to open media, `Space` to play or pause, `←` / `→` to seek, `D` / `F` to step frames, and `Enter` to toggle fullscreen. Use the mouse wheel or `↑` / `↓` for volume.

## Install and update

Choose your platform and architecture in the release list and use the corresponding SHA-256 file to verify the download. OpenPlayer 2.x checks for stable updates in the background. **Settings → About** also lets you check, download and start installing the update for your package type. Existing 1.x installations keep using GitHub's **Latest** release. [GitHub Releases](https://github.com/AreChen/OpenPlayer/releases)

The Windows installer is unsigned, so SmartScreen may show a prompt. The Linux `.deb` uses the distribution's libmpv runtime. The AppImage bundles libmpv. [Native runtime and licenses](docs/native-deps/README.md)

## Development

Use rustup to select Rust 1.100.0 from the project toolchain file and install the platform prerequisites in CI. Windows builds use Cargo, PowerShell and NSIS. [Windows development](docs/windows-gpui.md) · [CI prerequisites](.github/workflows/ci.yml)

On Linux or macOS, use Node.js 26 as in CI. From `apps/desktop`:

~~~sh
npm ci
npm run tauri:dev
~~~

[Runtime architecture](docs/architecture/README.md) · [Verification](docs/architecture/regression-checks.md)

The native Windows player focuses on core playback. The plugin documentation covers SDK 1.6 and the Tauri plugin host. [Plugin documentation](docs/plugins/README.md)

OpenPlayer source code is available under the [MIT License](LICENSE). Bundled dependencies retain their upstream licenses.
