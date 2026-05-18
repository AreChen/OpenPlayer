# libmpv2 Smoke Spike

The `mpv-smoke` feature proves that the desktop Rust crate can compile, link, and initialize `libmpv2` against local Windows `libmpv` artifacts.

The smoke feature requires local ignored native artifacts under `vendor/native/mpv/windows-x64`, or an `OPENPLAYER_MPV_DIR` override pointing at a directory containing `libmpv.dll.a` and `libmpv-2.dll`. These native artifacts are not tracked by git: a clean checkout can run default builds, but cannot run this smoke test until the artifacts are restored locally.

Run the smoke test from the repository root:

```powershell
$env:PATH = "E:\Project\CodeProject\RustPlayer\vendor\native\mpv\windows-x64;$env:PATH"; cargo test -p openplayer-desktop --features mpv-smoke mpv_smoke -- --nocapture
```

The default application path remains the browser `File` + `URL.createObjectURL` + HTML `<video>` renderer. The smoke feature does not register a frontend command, replace the renderer, or enable mpv window embedding.

This spike proves:

- The Rust crate can depend on `libmpv2` without affecting default builds.
- The local Windows import library can satisfy `libmpv2-sys` linking with the MSVC Rust toolchain.
- `libmpv-2.dll` can initialize at runtime when its directory is on `PATH`.

This spike does not prove:

- Local media path playback through `loadfile`.
- Tauri/WebView video-surface embedding.
- OpenGL render context integration.
- Packaging of `libmpv-2.dll` into release installers.
- Clean-checkout reproducibility of the smoke test without restoring local native artifacts.
- Better playback performance than the current HTML video path.
