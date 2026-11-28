# Rust plugin SDK 2

Native plugins are available starting with OpenPlayer 2.0.1. Open **Settings → Plugins** to run an action or change which plugins are enabled. [Release notes](../releases/v2.0.1.md)

| Bundled plugin | Actions |
| --- | --- |
| Network Stream | Open the player's stream URL dialog and play a network source. |
| Capture Tools | Save a PNG of the current video under Pictures/OpenPlayer. |
| Subtitle Typography | Decrease, reset or increase subtitle size while playing. A Rust guest chooses the next of three sizes from the current subtitle scale. |

## Build and install a plugin

The [official plugin workspace](https://github.com/AreChen/openplayer-plugins/tree/main/native) contains the package generator, the source of the subtitle guest and its build commands. The [SDK crate](../../crates/openplayer-plugin-sdk) defines the package format and guest entry point.

1. Give the package a unique ID and declare its localized name, actions and required permissions in a `PluginManifest`. Generate the `.opplugin.json` file with the SDK. An action can directly invoke a typed player command.
2. For behavior that depends on playback state, build a Rust `cdylib` for `wasm32-unknown-unknown`. Export a callback using `openplayer_plugin_sdk::export_plugin!`. It receives the chosen action index and a snapshot containing loaded/paused state, position, duration and subtitle scale. Return the index of the declared command to invoke, or `None` to skip it.
3. In **Settings → Plugins**, open the plugins folder. Copy `your-plugin.opplugin.json` and, for a programmable plugin, its matching `your-plugin.wasm` into this folder. Choose **Rescan plugins**. Review the listed permissions and enable your plugin. The bundled plugins are enabled on first use. OpenPlayer asks you to enable an external package again after its manifest or code changes.

The native host executes guest code with Wasmi. It limits memory and execution fuel. File access, network playback and subtitle changes are handled by the player through specific commands. The initial SDK exposes the stream dialog, frame capture and subtitle scale. [Package and permission definitions](../../crates/openplayer-plugin-sdk/src/manifest.rs)

For old packages and their JavaScript API, see the [SDK 1.6 reference](README.md).
