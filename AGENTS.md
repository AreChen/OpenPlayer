# OpenPlayer 仓库指南

`main` 用于 OpenPlayer 2.x 开发，当前应用版本为 2.0.0。沿用当前检出，按任务授权创建或切换分支。保留工作区中与任务无关的已有改动。

## 按运行时定位

- Windows x64 的入口在 `apps/desktop/src-tauri/src/windows_gpui/`。GPUI Fast 在一个原生窗口中绘制界面，libmpv 视频经 D3D11 合成。Windows 应用和安装器的依赖树保持无 Tauri、Tao、Wry 和 WebView。
- Windows 的控件样式复用[主题](apps/desktop/src-tauri/src/windows_gpui/theme.rs)和[通用控件](apps/desktop/src-tauri/src/windows_gpui/widgets.rs)。播放器输入、文件浏览器和媒体属性属于[视图模块](apps/desktop/src-tauri/src/windows_gpui/player_view/)，播放状态及偏好属于[控制器](apps/desktop/src-tauri/src/windows_gpui/controller.rs)。
- Linux 和 macOS 仍使用 Tauri 与 React。主窗口承载 libmpv，透明覆盖窗口承载控件。窗口操作经 Rust 窗口命令保持两者同步，播放命令返回后端快照。Linux 嵌入依赖 X11，改动时保留 Wayland 与软件渲染环境的能力判断。
- 播放历史、续播、音量等设置由 Rust 写入 redb。Windows 原生版的主题、快捷键和收藏沿用已有键和迁移路径。保持数据库句柄短生命周期。
- Windows 界面翻译从 `apps/desktop/src/i18n/` 生成。编辑翻译源或 `apps/desktop/scripts/sync-native-locales.mjs`，并重新生成[原生语言包](apps/desktop/src-tauri/src/windows_gpui/locale/)。

新插件 SDK 后续另行设计，当前 Windows 原生版以核心播放为范围。维护旧运行时或 SDK 1.6 时参考 [插件文档](docs/plugins/README.md)。

## 验证

以下在仓库根目录执行。Windows 测试前按 [Windows 开发指南](docs/windows-gpui.md) 恢复 libmpv，并把其目录放入 `PATH`。Linux 和 macOS 的系统依赖见 [CI](.github/workflows/ci.yml)。

~~~powershell
rtk cargo fmt --all -- --check
rtk cargo test --locked -p openplayer-desktop
rtk cargo clippy --locked --workspace --all-targets -- -D warnings
rtk git diff --check
~~~

界面、翻译或前端依赖变更执行对应检查。翻译生成命令去掉 `--check` 即可。

~~~powershell
rtk proxy node apps/desktop/scripts/sync-native-locales.mjs --check
rtk npm --prefix apps/desktop ci
rtk npm --prefix apps/desktop run verify:shell
rtk npm --prefix apps/desktop test
rtk npm --prefix apps/desktop run build
~~~

窗口输入或视频合成的改动按 [回归检查](docs/architecture/regression-checks.md) 验证。Windows UI 使用隔离测试配置。

## 构建与发布

- Windows 使用 [独立 NSIS 脚本](apps/desktop/scripts/build-windows.ps1)。Linux 和 macOS 使用 [发布工作流](.github/workflows/release.yml) 中的平台命令。
- 变更应用版本时同步 `Cargo.toml`、`Cargo.lock`、`apps/desktop/package.json`、`apps/desktop/package-lock.json` 和 `apps/desktop/src-tauri/tauri.conf.json`。在 `docs/releases/` 对应版本记录变更，执行 `rtk npm --prefix apps/desktop run verify:release -- --tag=v2.0.0` 校验当前版本。
- GitHub 的 Latest 标记继续服务 1.x 安装。2.x 检查器读取 Releases 列表，只提供同一主版本的稳定更新。发布细节见 [Windows 更新说明](docs/windows-gpui.md#updates)。
