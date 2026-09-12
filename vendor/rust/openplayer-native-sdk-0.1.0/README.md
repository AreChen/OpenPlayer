# OpenPlayer native SDK

A Rust helper for trusted out-of-process native modules. This is a development
extension, not part of published OpenPlayer 1.6.3 binaries. Native processes are
**not sandboxed**. They require `native.process` and an explicit host confirmation.

The host's [native module guide](https://github.com/AreChen/OpenPlayer/blob/main/docs/plugins/native-modules.md)
is the complete manifest, protocol, lifecycle, and video-plan contract. Until the
change is published, use the sibling checkout's `docs/plugins/native-modules.md`.
The type surface is in [the JavaScript SDK](../sdk/index.d.ts).

## Implement methods

```rust
use openplayer_native_sdk::{serve, json};

fn main() -> std::io::Result<()> {
    serve(|method, params| match method {
        "echo" => Ok(params),
        "describe" => Ok(json!({ "name": "Example" })),
        _ => Err("unsupported method".into()),
    })
}
```

`serve` owns stdin/stdout, requires the `openplayer-native-v1` handshake, bounds
each newline-delimited JSON message to 64 KiB, flushes replies, and dispatches
sequential requests. `serve_io` accepts custom streams for testing. An error
ends the session; return expected domain failures inside a successful JSON result
to preserve the process. Long work should use worker-owned jobs plus short
status/cancel methods. Do not send frame pixels over control RPC.

This library does not load GPU runtimes, render frames, allocate shared textures,
or implement interpolation/upscaling. It can be reused by separate native
implementations without making the player core vendor-specific.

## Experimental presentation transport

The optional `presentation` feature adds a Windows CPU RGBA shared-memory
transport. It is an SDK building block, **not yet an attachable player video
adapter**. It does not change the current `vapoursynth-rgb-v1` adapter or grant
plugins additional host access. See [transport contract](presentation.md) for
ownership, timing, limits, and shutdown behavior.

```powershell
rtk proxy cargo test --locked --manifest-path packages/native-sdk/Cargo.toml --features presentation
```

These tests include actual child-process transfer and a receiver killed while
holding the frame slot. They require Windows but do not initialize a GPU.

## Build and test

From the plugin repository root:

```powershell
rtk proxy cargo test --locked --manifest-path packages/native-sdk/Cargo.toml
rtk proxy cargo clippy --locked --manifest-path packages/native-sdk/Cargo.toml --all-targets --all-features -- -D warnings
rtk proxy node scripts/build-native-example.mjs
```

The last command creates a validated `.native-example/` directory containing
the echo executable, its SHA256 manifest declaration, and a runtime action.
It is not installed automatically and is not an official catalog plugin.

The fault-injection executable is gated behind `test-fixture` and must not be
distributed:

```powershell
rtk proxy cargo build --locked --manifest-path packages/native-sdk/Cargo.toml --example protocol-fixture --features test-fixture
$env:OPENPLAYER_NATIVE_TEST_EXECUTABLE = (Resolve-Path packages/native-sdk/target/debug/examples/protocol-fixture.exe).Path
Set-Location ../RustPlayer
rtk proxy cargo test -p openplayer-desktop native_process_roundtrip_faults_and_lifecycle --lib -- --ignored --nocapture
```

The Windows integration test covers handshake/echo, startup helpers inheriting
the job, in-flight rejection, idle crash cleanup, timeouts, errors, oversized
output, and explicit lifecycle termination. It deliberately bypasses the GUI
confirmation layer by testing the internal process session, not by adding a
production consent bypass. Confirmation still needs an interactive host test.
