<div align="center">

<img src="docs/assets/openplayer.jpg" alt="OpenPlayer desktop media player" width="920" />

# 🎬 OpenPlayer

**A high-performance cross-platform desktop media player**

<p>
  <a href="README.md"><img alt="English" src="https://img.shields.io/badge/English-Default-111111?style=for-the-badge" /></a>
  <a href="README.zh-CN.md"><img alt="Chinese" src="https://img.shields.io/badge/%E4%B8%AD%E6%96%87-%E5%88%87%E6%8D%A2-0078D4?style=for-the-badge" /></a>
</p>

[![Release](https://img.shields.io/github/v/release/AreChen/OpenPlayer?style=for-the-badge&logo=github&label=Release)](https://github.com/AreChen/OpenPlayer/releases)
[![Windows](https://img.shields.io/badge/Windows-x64-0078D4?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/AreChen/OpenPlayer/releases/latest)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-native-CE412B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/github/license/AreChen/OpenPlayer?style=for-the-badge)](LICENSE)

[Download](https://github.com/AreChen/OpenPlayer/releases/latest) · [Release Notes](docs/releases/v1.0.0.md) · [License](LICENSE)

</div>

---

## ✨ Overview

OpenPlayer is a desktop media player built with **Tauri v2, Rust, React, and libmpv**. It is designed to deliver native-level playback performance, smooth window behavior, and a polished desktop control surface while keeping the application lightweight.

The default playback path is `mpv-embed`: the main Tauri window hosts the native libmpv video surface, while a transparent overlay window hosts the React controls. This split keeps playback close to native performance while preserving a flexible modern UI layer.

## 🚀 Highlights

- ⚡ **High-performance playback core**: Embedded libmpv backend with mpv-powered media compatibility and playback behavior.
- 🪟 **Native video + transparent overlay**: Video rendering and React controls are separated for performance, stability, and UI flexibility.
- ⌨️ **Reliable shortcuts**: Configurable shortcuts with a native Windows shortcut bridge for cases where the video surface owns focus.
- 🖥️ **Fullscreen restore**: Press `Enter` to enter fullscreen, then press it again to restore the previous window size and position.
- 🎞️ **Frame stepping**: Press `D` to step backward one frame and `F` to step forward one frame.
- 🎚️ **Smooth progress feedback**: Playback progress and frame labels update smoothly without abrupt jumps.
- 🧭 **Smart chrome hiding**: Controls and the title bar auto-hide after 5 seconds of inactivity while playing, and also hide when the mouse leaves the window.
- 🧩 **Practical player workflows**: Context menu, settings panel, customizable shortcuts, time/frame display switching, volume control, and playlist entry point.

## 📦 Download

The latest Windows x64 installer is available from GitHub Releases:

[![Download OpenPlayer for Windows](https://img.shields.io/badge/Download-Windows%20Installer-0078D4?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/AreChen/OpenPlayer/releases/latest)

Current release:

- 🏷️ `v1.0.0`
- 🪟 `OpenPlayer_1.0.0_x64-setup.exe`
- 🔐 SHA256: `173071771C3322A444E514AA190DE9FA9C7C3ACDB237E5C5416E4D7CF8FA536A`

> If the installer is not code-signed yet, Windows SmartScreen may show a warning on first install.

## ⌨️ Default Shortcuts

| Action | Shortcut |
| --- | --- |
| Open media | `Ctrl + O` |
| Play / Pause | `Space` |
| Seek backward 5 seconds | `←` |
| Seek forward 5 seconds | `→` |
| Previous frame | `D` |
| Next frame | `F` |
| Fullscreen / Restore | `Enter` |

## 🛠️ Development

Prerequisites:

- Rust stable toolchain
- Node.js 20+
- npm 10+
- Tauri v2 system dependencies for your platform
- Windows builds use the local mpv runtime under `vendor/native/mpv/windows-x64`

Install dependencies:

```powershell
Set-Location apps/desktop
npm install
```

Run the desktop app:

```powershell
Set-Location apps/desktop
npm run tauri:dev
```

Verify the project:

```powershell
npm run verify:shell
npm run build
cargo test -p openplayer-desktop
```

Build the Windows installer:

```powershell
Set-Location apps/desktop
npm run tauri:build -- --config src-tauri/tauri.windows.conf.json
```

The installer is emitted under:

```text
target/release/bundle/nsis/
```

## 📄 License

OpenPlayer is released under the [MIT License](LICENSE).
