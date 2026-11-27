# OpenPlayer

[English](README.md) · [GitHub 发布列表](https://github.com/AreChen/OpenPlayer/releases) · [2.0.0 发行说明](docs/releases/v2.0.0.md)

<img src="apps/desktop/src-tauri/icons/icon.png" alt="OpenPlayer 应用图标" width="96" />

OpenPlayer 是基于 Rust 和 libmpv 的桌面媒体播放器。`main` 是 2.x 开发主线。

| 平台 | 运行时 | 发行包 |
| --- | --- | --- |
| Windows x64 | GPUI Fast 原生窗口、D3D11 视频合成与 GPUI Motion | 自带 libmpv 的 NSIS 安装包 |
| Linux | Tauri 和 React 界面，libmpv 原生播放 | `.deb` 和 AppImage |
| macOS Apple Silicon 和 Intel | Tauri 和 React 界面，libmpv 原生播放 | 自带 libmpv 的 DMG |

## Windows 播放器

- 支持本地媒体和网络流，可调倍速、循环、逐帧播放、切换音轨和字幕。可从历史记录继续播放，也可使用无痕模式。
- 双击空白播放器打开媒体浏览器。浏览器会记住上次目录，支持编辑路径、收藏文件夹和媒体、查看缩略图与悬停静音预览。按 `Ctrl+F` 可搜索当前目录及其子目录里的文件名。
- 在文件浏览器或播放菜单查看容器、视频和音频参数，并从媒体属性面板复制。
- 可调主题强调色、标题栏和播放控件透明度。鼠标离开窗口或连续三秒无操作时，控件自动隐藏。
- 可配置快捷键和 Windows 资源管理器预览格式。安装完成时，可选择立即打开 OpenPlayer 或前往 Windows 默认应用设置。

常用快捷键为 `Ctrl+O` 打开媒体、`空格` 播放或暂停、`←` / `→` 快退或快进、`D` / `F` 上一帧或下一帧、`Enter` 切换全屏。使用滚轮或 `↑` / `↓` 调节音量。

## 安装与更新

在发布列表中选择平台和架构，并用配套的 SHA-256 文件验证安装包。OpenPlayer 2.x 会在后台检查稳定版更新，也可以在「设置 → 关于」检查、下载并发起当前安装包类型的升级。现有 1.x 安装继续使用 GitHub 的「Latest」发行版。[GitHub 发布列表](https://github.com/AreChen/OpenPlayer/releases)

Windows 安装包未签名，SmartScreen 可能提示。Linux 包使用系统提供的 libmpv。[原生依赖与许可证](docs/native-deps/README.md)

## 本地开发

通过 rustup 使用仓库指定的 Rust 1.100.0，按 CI 准备系统依赖。Windows 使用 Cargo、PowerShell 和 NSIS 构建。[Windows 开发指南](docs/windows-gpui.md) · [CI 环境](.github/workflows/ci.yml)

Linux 和 macOS 使用与 CI 一致的 Node.js 26。在 `apps/desktop` 中运行：

~~~sh
npm ci
npm run tauri:dev
~~~

[运行时架构](docs/architecture/README.md) · [回归检查](docs/architecture/regression-checks.md)

Windows 原生版目前聚焦核心播放。插件文档适用于 SDK 1.6 和 Tauri 插件宿主。[插件文档](docs/plugins/README.md)

OpenPlayer 源码使用 [MIT 许可证](LICENSE)。发行包中各依赖使用对应的上游许可证。
