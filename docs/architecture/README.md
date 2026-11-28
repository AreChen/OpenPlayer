# Desktop architecture

The desktop binary has one GPUI Fast window. Playback controls, settings, the playlist, the file browser and its previews use the same Rust views on each platform. libmpv starts on first playback. Stopping releases the main playback core, and closing the browser releases the preview decoder. [Player sources](../../apps/desktop/native/src/player/)

| Platform | Video presentation | Installed update |
| --- | --- | --- |
| Windows | D3D11 swap chain composed with the GPUI window | The verified NSIS installer closes the matching installation and replaces it. |
| macOS | libmpv software render buffer uploaded into the GPUI scene | Finder opens the verified DMG. Quit the player and drag the new app to Applications. |
| Linux | The same frame upload in a GPUI X11 or Wayland window | The Debian package uses apt with PolicyKit. The AppImage is replaced atomically. The About panel can restart the upgraded player. |

The release checker filters the stable GitHub releases by installed major version, architecture and package type. The download is verified against the matching SHA-256 before installation. The Linux and macOS packages ship the unified native interface starting with 2.0.1. [Updater](../../apps/desktop/native/src/updater/) · [2.0.1 release notes](../releases/v2.0.1.md)

## Build

Use Rust 1.100.0 from the repository toolchain file. Commands run from the repository root.

| Platform | Prerequisites | Build an installer |
| --- | --- | --- |
| Windows x64 | Restore the pinned libmpv runtime, set PATH, install NSIS. [Windows instructions](../windows-gpui.md) | `./apps/desktop/scripts/build-windows.ps1` |
| macOS | Xcode tools, Homebrew mpv and pkg-config | The [packaging action](../../.github/actions/build-desktop-packages/action.yml) creates an app bundle, embeds its dylibs, signs it and builds a DMG. |
| Linux x64 | [Distribution development packages](../../apps/desktop/scripts/install-linux-build-deps.sh) and a Vulkan driver | `bash apps/desktop/scripts/build-linux.sh` creates a .deb and an AppImage. |

~~~sh
cargo run --locked -p openplayer-desktop -- "/path/to/video.mp4"
cargo test --locked --workspace
~~~

## GitHub releases

Pushing a stable `v2.x.y` tag builds five packages and stages a draft. The repository enables `OPENPLAYER_AUTO_PUBLISH`, so the draft publishes automatically once the four installation and upgrade jobs and the matching repository CI pass. For a single release, run the Release workflow on `main` with the matching tag and select `auto_publish`. The workflow compares the uploaded files to the validated artifacts before marking the release Latest. The version in Cargo and the corresponding release notes must match the tag. [Release workflow](../../.github/workflows/release.yml)

For a verification-only build, leave the dispatch options clear. For a separately approved release, stage a draft and then run the existing `publish` option with its candidate run ID and native acceptance URL.

## User data

Windows uses the roaming application data directory and Linux uses XDG_DATA_HOME (normally ~/.local/share). Both store data under dev.openplayer.app/storage. macOS retains ~/Library/Application Support/dev.openplayer.desktop/storage. History and playback settings live in playback-history.redb. The native settings database holds theme, language, keybindings and browser preferences. [Playback storage](../../apps/desktop/native/src/playback_store/) · [Preferences](../../apps/desktop/native/src/player/controller/preferences.rs)
