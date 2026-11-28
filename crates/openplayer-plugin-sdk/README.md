# OpenPlayer plugin SDK 2

OpenPlayer extensions declare native player actions in a manifest. For context-sensitive actions, compile a Rust function to WebAssembly and ship the module beside the manifest. One package runs on Windows, macOS and Linux.

## Build the example

From the OpenPlayer repository root run:

```sh
rustup target add wasm32-unknown-unknown
cargo build --locked --release -p openplayer-plugin-sdk --no-default-features --target wasm32-unknown-unknown --example smart_capture
```

Copy `examples/smart-capture.opplugin.json` from this crate and the compiled `target/wasm32-unknown-unknown/release/examples/smart_capture.wasm` into the player's `storage/plugins` directory. Rename the module to `smart-capture.wasm` so its basename matches the manifest. Reload the plugin list in Settings and enable **Smart capture**. Its first action captures the current video frame during playback and opens the stream dialog while idle.

## Author a package

For a fixed menu action, use the SDK's default `manifest` feature and serialize a `PluginManifest` with `to_json()`. Define a stable dotted ID, semantic version, English and Simplified Chinese labels, actions and required permissions. Omit `runtime` for these packages.

For a programmable package, set `runtime` to `"wasm"`. Use a Rust `cdylib` with `openplayer-plugin-sdk` as a dependency with `default-features = false`. Write a callback with the signature below and use `export_plugin!` to generate the ABI entry point. The [example](examples/smart_capture.rs) uses the player's loaded state to choose an action.

```rust
use openplayer_plugin_sdk::{PlaybackContext, export_plugin};

fn invoke(action: u32, state: PlaybackContext) -> Option<u32> {
    match action {
        0 if state.loaded => Some(0),
        0 | 1 => Some(1),
        _ => None,
    }
}

export_plugin!(invoke);
```

The input and output are zero-based indices into the same manifest's actions. Returning `None` declines the invocation. The callback receives `loaded`, `paused`, playback position and duration in milliseconds (zero when unknown), and the displayed subtitle scale. The host executes the selected command after checking its declared permission.

| Permission | Host command |
| --- | --- |
| `openStream` | `openStreamDialog` opens the player's native stream dialog |
| `capture` | `captureScreenshot` saves the current video frame |
| `subtitleStyle` | `setSubtitleScale` takes a numeric `scale` from 0.25 to 4 |

SDK 2 guests expose `openplayer_invoke_v2(i32, i32, i64, i64, f64) -> i32`. The flags argument uses bit zero for loaded and bit one for paused. Each invocation gets a fresh interpreter store, 100,000 fuel units and up to 4 MiB of linear memory. Guest modules must be import-free and fit in 256 KiB. Use the package's declared actions for interactions with the player.
