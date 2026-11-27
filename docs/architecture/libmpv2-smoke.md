# libmpv Smoke Test

The optional `mpv-smoke` feature verifies that the desktop Rust crate can
compile, link, and initialize libmpv in a headless mode.

For video rendering, window lifecycle and user input, use the platform [regression checks](regression-checks.md).

## Local Requirements

Windows builds use ignored native artifacts under:

```text
vendor/native/mpv/windows-x64
```

You can also set `OPENPLAYER_MPV_DIR` to a directory containing the required
libmpv import/runtime libraries.

## Run

From the repository root:

```powershell
$env:PATH = "$PWD\vendor\native\mpv\windows-x64;$env:PATH"
cargo test -p openplayer-desktop --features mpv-smoke mpv_smoke -- --nocapture
```

This proves:

- The Rust crate can enable the optional libmpv smoke feature.
- libmpv can initialize with `vo=null` and `ao=null`.
- The local native runtime can be found by the process.

This does not prove:

- Embedded video playback.
- Resume seek behavior.
- Overlay input handling.
- Release package dependency layout.
