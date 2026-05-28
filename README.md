<div align="center">

<img src="docs/assets/openplayer-social-preview.png" alt="OpenPlayer desktop media player" width="820" />

# 🎬 OpenPlayer

**A beautiful, extensible, high-performance desktop media player powered by Tauri v2, Rust, React, and libmpv**

<p>
  <a href="README.md"><img alt="English" src="https://img.shields.io/badge/English-Default-111111?style=for-the-badge" /></a>
  <a href="README.zh-CN.md"><img alt="Chinese" src="https://img.shields.io/badge/%E4%B8%AD%E6%96%87-Switch-F6C15B?style=for-the-badge" /></a>
</p>

[![Release](https://img.shields.io/github/v/release/AreChen/OpenPlayer?style=for-the-badge&logo=github&label=Release)](https://github.com/AreChen/OpenPlayer/releases)
[![Windows](https://img.shields.io/badge/Windows-x64-0078D4?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/AreChen/OpenPlayer/releases/latest)
[![Linux](https://img.shields.io/badge/Linux-DEB%20%7C%20AppImage-FCC624?style=for-the-badge&logo=linux&logoColor=111111)](https://github.com/AreChen/OpenPlayer/releases/latest)
[![macOS](https://img.shields.io/badge/macOS-DMG-000000?style=for-the-badge&logo=apple&logoColor=white)](https://github.com/AreChen/OpenPlayer/releases/latest)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-native-CE412B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/github/license/AreChen/OpenPlayer?style=for-the-badge)](LICENSE)

[Download](https://github.com/AreChen/OpenPlayer/releases/latest) · [Release Notes](docs/releases/v1.5.1.md) · [Plugin SDK](docs/plugins/sdk-1.5-developer-guide.md) · [License](LICENSE)

</div>

---

## ✨ Overview

OpenPlayer is a lightweight, visually refined desktop media player built with **Tauri v2, Rust, React, and libmpv**. It keeps playback close to the native mpv surface while using a transparent React overlay for polished controls, menus, shortcuts, and settings.

The default playback path is `mpv-embed`: the main Tauri window hosts the native libmpv video surface, while the overlay window hosts the interactive UI. This split gives OpenPlayer native-level playback behavior without giving up a modern, customizable desktop experience.

<img src="docs/assets/openplayer-feature-banner.png" alt="OpenPlayer playback controls" width="100%" />

## 🚀 Highlights

- ⚡ **Native mpv playback**: Embedded libmpv backend with broad media compatibility and high-performance decoding.
- 🪟 **Video host + transparent overlay**: Native rendering and React controls are separated for stability, responsiveness, and UI flexibility.
- 🌐 **Bilingual interface**: English and Simplified Chinese are supported, with automatic language matching based on the operating system.
- 🎨 **Studio Dark theme**: Built-in dark visual system with configurable accent colors and synchronized appearance across windows.
- ⌨️ **Reliable shortcuts**: Configurable shortcuts plus a native Windows shortcut bridge for cases where the video surface owns focus.
- 🎞️ **Precise playback control**: Fullscreen restore, smooth progress, frame stepping, loop modes, playback speed, track selection, and subtitle controls.
- 🧭 **Smart chrome hiding**: Controls and the title bar hide during playback inactivity and when the mouse leaves the window.
- 🗂️ **Playback memory**: Recent media and resume progress are persisted with a lightweight redb store, with clear-history and private playback options.
- 🧩 **Open plugin SDK**: User-installed plugins can declare permissions, persist runtime data, open custom views, listen for playback events, and call controlled mpv APIs.
- 🧩 **Desktop integration**: Optional Windows media association and Explorer preview registration for selected formats.

<img src="docs/assets/openplayer-feature-grid.png" alt="OpenPlayer playback, themes, shortcuts, and format settings" width="100%" />

## 🧩 Plugins and SDK

OpenPlayer 1.5.1 expands the plugin system into a documented SDK for external developers and AI-assisted plugin authoring. Plugins can use typed manifests, capability checks, runtime events, scoped storage, validated network requests, translucent themed custom views with setting-backed side-panel opacity, native dialogs, and permission-gated mpv controls for playback, filters, OSD, and script messages.

- SDK guide: [docs/plugins/sdk-1.5-developer-guide.md](docs/plugins/sdk-1.5-developer-guide.md)
- Plugin host overview: [docs/plugins/README.md](docs/plugins/README.md)
- Official plugin packages: [AreChen/openplayer-plugins](https://github.com/AreChen/openplayer-plugins)

## 📦 Download

The latest release is available from GitHub Releases:

[![Download OpenPlayer](https://img.shields.io/badge/Download-Latest%20Release-F6C15B?style=for-the-badge&logo=github&logoColor=111111)](https://github.com/AreChen/OpenPlayer/releases/latest)

Current release:

- 🏷️ `v1.5.1`
- 🪟 Windows: `OpenPlayer_1.5.1_x64-setup.exe`
- 🐧 Linux: `OpenPlayer_1.5.1_amd64.deb` and `OpenPlayer_1.5.1_amd64.AppImage`
- 🍎 macOS: `OpenPlayer_1.5.1_arm64.dmg` and `OpenPlayer_1.5.1_x64.dmg`
- 🔐 Checksums: release assets include `.sha256` files

> Windows packages are not code-signed yet, so SmartScreen may show a warning on first install.
> Linux packages are an initial distribution target and still rely on the host desktop media stack, including system libmpv.

## ⌨️ Default Shortcuts

| Action | Shortcut |
| --- | --- |
| Open media | `Ctrl + O` |
| Play / Pause | `Space` |
| Seek backward 5 seconds | `Left` |
| Seek forward 5 seconds | `Right` |
| Previous frame | `D` |
| Next frame | `F` |
| Fullscreen / Restore | `Enter` |
| Volume | Mouse wheel / `Up` / `Down` |

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

## 🤝 Built With

OpenPlayer stands on the work of excellent open source projects:

- [Tauri](https://tauri.app/) - secure, lightweight desktop app shell.
- [Rust](https://www.rust-lang.org/) - native backend, shell integration, and persistence.
- [mpv / libmpv](https://mpv.io/) - high-quality media playback engine.
- [React](https://react.dev/) - overlay controls and settings UI.
- [Vite](https://vite.dev/) and [TypeScript](https://www.typescriptlang.org/) - frontend tooling.
- [redb](https://github.com/cberner/redb) - embedded persistence for history, settings, and playback state.

## ⚖️ Licensing Notes

OpenPlayer's application source code is released under MIT. Release packages also include or link to upstream components under their own licenses.

- Tauri, Rust ecosystem crates, React, Vite, TypeScript, and redb are MIT, Apache-2.0, or similarly permissive according to their package metadata.
- Windows release automation uses the `mpv-dev-lgpl` libmpv artifact from `zhongfly/mpv-winbuild` to keep the bundled media runtime aligned with OpenPlayer's permissive application license.
- Linux packages depend on the distribution's `libmpv2`; macOS packages bundle Homebrew libmpv dylibs. Their notices and source obligations remain governed by the upstream packages used for each platform.

## 📄 License

OpenPlayer is released under the [MIT License](LICENSE).
