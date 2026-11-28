# OpenPlayer 仓库指南

在现有 `main` 上开发 OpenPlayer 2.x。平台入口与共用界面位于同一个 [Cargo 包](apps/desktop/native/Cargo.toml)。窗口、菜单和设置复用 GPUI Fast 的[主题和控件](apps/desktop/native/src/player/)。界面文字直接维护在[英文与中文语言包](apps/desktop/native/src/player/locale/)。

改动视频呈现时同时检查 Windows D3D11 和 Unix libmpv 软件渲染路径。Linux 覆盖 X11 和 Wayland。偏好、快捷键、浏览器收藏及续播历史使用已有 redb 键，升级时保持原数据目录和短生命周期的数据库句柄。[平台构建与数据路径](docs/architecture/README.md)

从仓库根目录执行以下检查。Windows 先恢复固定版本的 libmpv 并加入 PATH。[Windows 环境](docs/windows-gpui.md)

~~~sh
rtk cargo fmt --all -- --check
rtk cargo clippy --locked --workspace --all-targets -- -D warnings
rtk cargo test --locked --workspace
rtk proxy node apps/desktop/scripts/verify-release.mjs
rtk git diff --check
~~~

改动窗口、视频、快捷键或更新器时还要走对应的[实机检查](docs/architecture/regression-checks.md)。版本定义在工作区 Cargo.toml。同步 Cargo.lock 中的本包版本，并在 docs/releases/ 下记录对应版本。发布工作流验证四种平台架构、五种安装包和校验文件。稳定 2.x 作为 GitHub Latest，应用内更新只选择同一主版本。[发布流程](.github/workflows/release.yml)

插件开发使用 [Rust SDK 2](docs/plugins/native-sdk.md)。新增宿主动作时同步维护 SDK 权限、原生分发和测试。内置清单及 Wasm 从[官方插件源码](https://github.com/AreChen/openplayer-plugins/tree/main/native)生成。历史实现见 [SDK 1.6 文档](docs/plugins/README.md)。
