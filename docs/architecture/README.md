# Runtime architecture

OpenPlayer 2.x has two desktop runtimes and a shared Rust workspace. The platform entry points are selected in [the Rust crate](../../apps/desktop/src-tauri/src/lib.rs).

| Platform | Window and playback path |
| --- | --- |
| Windows x64 | GPUI Fast draws a single native window. libmpv renders through a D3D11 swap chain placed in the window's DirectComposition tree. GPUI Motion drives control transitions according to the Windows animation preference. |
| Linux and macOS | A Tauri main window hosts libmpv. A transparent React overlay draws the controls, menus and settings. Rust window commands keep both windows synchronized. |

## Windows

[`windows_gpui`](../../apps/desktop/src-tauri/src/windows_gpui/) owns the view, controller, video renderer and native integration. The controller owns the libmpv session, handles commands and exposes snapshots. The player predicts timeline position between authoritative snapshots. Startup defers initialization of the playback core until it is needed, and Stop releases it. The file browser has its own preview lifecycle.

Windows uses redb for playback history, resume positions, preferences, favorites and shortcut bindings. The library owns the supported media catalog, subtitle matching and storage paths. The UI reads theme and localization data from Rust. Edit locale source data or the [locale generator](../../apps/desktop/scripts/sync-native-locales.mjs) when changing visible text.

## Linux and macOS

The [React app](../../apps/desktop/src/) owns control presentation and dispatches playback through Tauri. The [Tauri bootstrap](../../apps/desktop/src-tauri/src/bootstrap/) registers thin command wrappers. Backend snapshots anchor React's playback clock. Backend redb stores own shared playback state, themes and plugin data.

The embedded video path is platform-specific. Linux embeds with X11 and retains a capability check for Wayland. macOS uses its AppKit video view. The Tauri plugin bridge supports SDK 1.6. A new SDK for the Windows runtime is future work. [Plugin host](../plugins/README.md)

## Further reading

- [Windows development and packaging](../windows-gpui.md)
- [Regression checks](regression-checks.md)
- [Native runtime and build metadata](../native-deps/README.md)
- [Headless libmpv smoke test](libmpv2-smoke.md)
