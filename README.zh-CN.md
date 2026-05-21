<div align="center">

<img src="docs/assets/openplayer.jpg" alt="OpenPlayer 桌面媒体播放器" width="920" />

# 🎬 OpenPlayer

**高性能跨平台桌面视频播放器**

<p>
  <a href="README.md"><img alt="English" src="https://img.shields.io/badge/English-Switch-111111?style=for-the-badge" /></a>
  <a href="README.zh-CN.md"><img alt="中文" src="https://img.shields.io/badge/%E4%B8%AD%E6%96%87-%E5%BD%93%E5%89%8D-0078D4?style=for-the-badge" /></a>
</p>

[![Release](https://img.shields.io/github/v/release/AreChen/OpenPlayer?style=for-the-badge&logo=github&label=Release)](https://github.com/AreChen/OpenPlayer/releases)
[![Windows](https://img.shields.io/badge/Windows-x64-0078D4?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/AreChen/OpenPlayer/releases/latest)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-native-CE412B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/github/license/AreChen/OpenPlayer?style=for-the-badge)](LICENSE)

[下载安装包](https://github.com/AreChen/OpenPlayer/releases/latest) · [发布说明](docs/releases/v1.0.0.md) · [许可证](LICENSE)

</div>

---

## ✨ 项目介绍

OpenPlayer 是一个使用 **Tauri v2 + Rust + React + libmpv** 构建的桌面媒体播放器，目标是在保持轻量体积的同时，提供接近原生播放器的播放性能、窗口体验和交互流畅度。

当前默认播放路径是 `mpv-embed`：主 Tauri 窗口作为原生 libmpv 视频宿主，透明 overlay 窗口承载 React 控件。这样的架构让 OpenPlayer 同时拥有 mpv 的播放能力和现代前端 UI 的可定制性。

## 🚀 功能亮点

- ⚡ **高性能播放内核**：基于 libmpv 嵌入式播放后端，继承 mpv 的格式兼容性和播放能力。
- 🪟 **原生视频 + 透明控件层**：视频渲染与 React 控件分离，兼顾性能、稳定性和 UI 表现。
- ⌨️ **可靠快捷键系统**：支持可配置快捷键，并通过 Windows 原生快捷键桥接解决视频区域聚焦后的按键失效问题。
- 🖥️ **全屏恢复体验**：按 `Enter` 进入全屏，再次按下恢复原窗口尺寸与位置。
- 🎞️ **逐帧播放**：按 `D` 后退一帧，按 `F` 前进一帧，适合精细查看画面细节。
- 🎚️ **平滑进度条**：播放进度和帧数显示使用平滑刷新，减少跳变和回退感。
- 🧭 **智能界面隐藏**：播放时 5 秒无操作自动隐藏控件与标题栏，鼠标离开窗口时也会隐藏。
- 🧩 **播放器常用工作流**：右键菜单、设置面板、自定义快捷键、时间/帧数显示切换、音量控制和播放列表入口。

## 📦 下载

最新 Windows x64 安装包可在 GitHub Release 下载：

[![Download OpenPlayer for Windows](https://img.shields.io/badge/Download-Windows%20Installer-0078D4?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/AreChen/OpenPlayer/releases/latest)

当前版本：

- 🏷️ `v1.0.0`
- 🪟 `OpenPlayer_1.0.0_x64-setup.exe`
- 🔐 SHA256：`173071771C3322A444E514AA190DE9FA9C7C3ACDB237E5C5416E4D7CF8FA536A`

> 如果安装包尚未配置商业代码签名，Windows 首次安装时可能出现 SmartScreen 提示。

## ⌨️ 默认快捷键

| 操作 | 快捷键 |
| --- | --- |
| 打开媒体 | `Ctrl + O` |
| 播放 / 暂停 | `Space` |
| 后退 5 秒 | `←` |
| 前进 5 秒 | `→` |
| 上一帧 | `D` |
| 下一帧 | `F` |
| 全屏 / 恢复 | `Enter` |

## 🛠️ 本地开发

环境要求：

- Rust stable toolchain
- Node.js 20+
- npm 10+
- Tauri v2 对应平台系统依赖
- Windows 构建需要本仓库中的 `vendor/native/mpv/windows-x64`

安装依赖：

```powershell
Set-Location apps/desktop
npm install
```

运行开发版：

```powershell
Set-Location apps/desktop
npm run tauri:dev
```

验证项目：

```powershell
npm run verify:shell
npm run build
cargo test -p openplayer-desktop
```

构建 Windows 安装包：

```powershell
Set-Location apps/desktop
npm run tauri:build -- --config src-tauri/tauri.windows.conf.json
```

构建产物位于：

```text
target/release/bundle/nsis/
```

## 📄 许可证

OpenPlayer 使用 [MIT License](LICENSE) 开源。
