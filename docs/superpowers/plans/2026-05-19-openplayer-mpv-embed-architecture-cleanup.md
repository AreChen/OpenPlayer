# OpenPlayer MPV Embed Architecture Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the failed mpv OpenGL render API spike and make the verified `mpv_embed` overlay architecture the only default desktop playback backend.

**Architecture:** The default desktop feature becomes `mpv-embed`. The Tauri runtime manages one `MpvEmbedState`, creates the transparent overlay window, and routes overlay open/playback commands to the main video window's mpv child HWND player. Static verification rejects the deleted `mpv_render` backend and guards the stable overlay/embed architecture.

**Tech Stack:** Rust, Tauri 2, libmpv2, Win32 HWND via `windows-sys`, React, Vite, Node static verification script.

---

## File Structure

- Modify `apps/desktop/scripts/verify-shell.mjs`: update architecture guards to require `mpv-embed` default and reject the deleted render API spike.
- Modify `apps/desktop/src-tauri/Cargo.toml`: remove `mpv-render`, remove `libmpv2-sys`, remove unused OpenGL/library-loader Windows features, and set `mpv-embed` as default.
- Modify `apps/desktop/src-tauri/build.rs`: remove the obsolete `CARGO_FEATURE_MPV_RENDER` linking trigger.
- Modify `apps/desktop/src-tauri/src/lib.rs`: remove render backend imports/state/commands and keep one production overlay/embed runtime.
- Delete `apps/desktop/src-tauri/src/mpv_render.rs`: failed mpv render API actor.
- Delete `apps/desktop/src-tauri/src/mpv_render/sys.rs`: failed raw render API wrapper.
- Delete `apps/desktop/src-tauri/src/mpv_render/win32_surface.rs`: failed Win32/WGL render surface.
- Do not modify `apps/desktop/src/App.tsx` or `apps/desktop/src/styles.css`; user-visible behavior should stay unchanged.

---

### Task 1: Update Static Verification For The Target Architecture

**Files:**
- Modify: `apps/desktop/scripts/verify-shell.mjs`

- [ ] **Step 1: Replace render-spike file reads with deleted-file guards**

In `apps/desktop/scripts/verify-shell.mjs`, replace the render source setup near the top:

```js
const mpvRenderUrl = new URL("../src-tauri/src/mpv_render.rs", import.meta.url);
const mpvRenderSource = existsSync(mpvRenderUrl) ? await readFile(mpvRenderUrl, "utf8") : "";
const mpvRenderSysUrl = new URL("../src-tauri/src/mpv_render/sys.rs", import.meta.url);
const mpvRenderSysSource = existsSync(mpvRenderSysUrl) ? await readFile(mpvRenderSysUrl, "utf8") : "";
const mpvRenderBackendSource = `${mpvRenderSource}\n${mpvRenderSysSource}`;
```

with this:

```js
const mpvRenderFiles = [
  new URL("../src-tauri/src/mpv_render.rs", import.meta.url),
  new URL("../src-tauri/src/mpv_render/sys.rs", import.meta.url),
  new URL("../src-tauri/src/mpv_render/win32_surface.rs", import.meta.url),
];
```

- [ ] **Step 2: Replace render-run extraction with embed-run extraction**

Replace this block:

```js
const mpvRenderRunMatch = /#\[cfg\(\s*feature\s*=\s*"mpv-render"\s*\)\]\s*pub\s+fn\s+run\s*\(\s*\)/.exec(tauriLibSource);
const mpvRenderRunSource = mpvRenderRunMatch
  ? extractFunctionAt(tauriLibSource, mpvRenderRunMatch.index + mpvRenderRunMatch[0].lastIndexOf("pub"))
  : "";
```

with this:

```js
const mpvEmbedRunMatch = /#\[cfg\(\s*feature\s*=\s*"mpv-embed"\s*\)\]\s*pub\s+fn\s+run\s*\(\s*\)/.exec(tauriLibSource);
const mpvEmbedRunSource = mpvEmbedRunMatch
  ? extractFunctionAt(tauriLibSource, mpvEmbedRunMatch.index + mpvEmbedRunMatch[0].lastIndexOf("pub"))
  : "";
```

- [ ] **Step 3: Replace Cargo/backend render assertions**

Replace the assertion block that currently requires `mpv-render`, `mpvRenderRunSource`, and OpenGL render API symbols:

```js
assert.match(tauriCargoToml, /default = \["mpv-render"\]/, "desktop default feature must use the mpv render backend");
assert.match(tauriCargoToml, /mpv-render/, "Cargo features must define mpv-render");
assert.match(tauriBuildScript, /CARGO_FEATURE_MPV_SMOKE[\s\S]*CARGO_FEATURE_MPV_EMBED/, "build script must only add mpv link paths when an mpv feature is enabled");
assert.match(tauriBuildScript, /vendor[\\/]native[\\/]mpv[\\/]windows-x64/, "build script must point at the vendored Windows mpv directory");
assert.match(tauriLibSource, /#\[cfg\(feature = "mpv-smoke"\)\]\s*mod mpv_smoke;/, "desktop crate must keep libmpv2 smoke code feature-gated");
assert.match(tauriLibSource, /mod mpv_embed;/, "overlay fallback must use mpv child HWND in the main video window");
assert.match(mpvRenderRunSource, /pub\s+fn\s+run\s*\(\s*\)/, "desktop default runtime must include an mpv-render run function");
assert.match(mpvRenderRunSource, /WebviewWindowBuilder[\s\S]*surface=overlay/, "desktop runtime must create a separate transparent overlay controls window");
assert.match(mpvRenderRunSource, /mpv_overlay_open_path/, "default runtime must register overlay commands that target the main video window");
assert.doesNotMatch(mpvRenderRunSource, /\.always_on_top\(true\)/, "overlay controls must not be globally topmost over other apps");
assert.doesNotMatch(mpvRenderRunSource, /\.position\(position\.x as f64, position\.y as f64\)|\.inner_size\(size\.width as f64, size\.height as f64\)/, "overlay startup must not pass physical main window pixels to logical builder sizing APIs");
assert.match(mpvRenderRunSource, /\.visible\(false\)[\s\S]*sync_overlay_to_main\(&app_handle\)[\s\S]*overlay\.show\(\)/, "overlay startup must stay hidden until physical-position sync prevents DPI-scale misalignment");
assert.match(tauriLibSource, /GWLP_HWNDPARENT|set_overlay_owner/, "overlay controls should be owned by the main player window instead of global topmost");
assert.doesNotMatch(mpvRenderRunSource, /OPENPLAYER_MPV_EMBED_FILE/, "normal render API runtime must not auto-play the old Abbott embed smoke file");
```

with this block:

```js
assert.match(tauriCargoToml, /default = \["mpv-embed"\]/, "desktop default feature must use the stable mpv embed overlay backend");
assert.match(tauriCargoToml, /mpv-embed/, "Cargo features must define mpv-embed");
assert.doesNotMatch(tauriCargoToml, /mpv-render|libmpv2-sys|Win32_Graphics_OpenGL|Win32_System_LibraryLoader/, "desktop crate must not keep the failed mpv render backend or its render-only dependencies");
assert.doesNotMatch(tauriBuildScript, /CARGO_FEATURE_MPV_RENDER/, "build script must not keep the removed mpv-render feature gate");
assert.match(tauriBuildScript, /CARGO_FEATURE_MPV_SMOKE[\s\S]*CARGO_FEATURE_MPV_EMBED/, "build script must only add mpv link paths when an active mpv feature is enabled");
assert.match(tauriBuildScript, /vendor[\\/]native[\\/]mpv[\\/]windows-x64/, "build script must point at the vendored Windows mpv directory");
assert.match(tauriLibSource, /#\[cfg\(feature = "mpv-smoke"\)\]\s*mod mpv_smoke;/, "desktop crate must keep libmpv2 smoke code feature-gated");
assert.match(tauriLibSource, /#\[cfg\(feature = "mpv-embed"\)\]\s*mod mpv_embed;/, "desktop crate must compile the stable mpv child HWND backend behind mpv-embed");
assert.doesNotMatch(tauriLibSource, /mpv_render|MpvRenderState|mpv_render_/, "desktop runtime must not reference the removed mpv render backend");
for (const fileUrl of mpvRenderFiles) {
  assert.ok(!existsSync(fileUrl), `removed render spike file must not exist: ${fileUrl.pathname}`);
}
assert.match(mpvEmbedRunSource, /pub\s+fn\s+run\s*\(\s*\)/, "desktop default runtime must include an mpv-embed run function");
assert.match(mpvEmbedRunSource, /WebviewWindowBuilder[\s\S]*surface=overlay/, "desktop runtime must create a separate transparent overlay controls window");
assert.match(mpvEmbedRunSource, /mpv_overlay_open_path/, "default runtime must register overlay commands that target the main video window");
assert.match(tauriLibSource, /mpv_overlay_open_path[\s\S]*main_window\(&app\)\?[\s\S]*mpv_embed::open_path_for_window\(&main, state\.inner\(\), path\)/, "overlay open command must target the main video window through mpv_embed");
assert.doesNotMatch(mpvEmbedRunSource, /\.always_on_top\(true\)/, "overlay controls must not be globally topmost over other apps");
assert.doesNotMatch(mpvEmbedRunSource, /\.position\(position\.x as f64, position\.y as f64\)|\.inner_size\(size\.width as f64, size\.height as f64\)/, "overlay startup must not pass physical main window pixels to logical builder sizing APIs");
assert.match(mpvEmbedRunSource, /\.visible\(false\)[\s\S]*sync_overlay_to_main\(&app_handle\)[\s\S]*overlay\.show\(\)/, "overlay startup must stay hidden until physical-position sync prevents DPI-scale misalignment");
assert.match(tauriLibSource, /GWLP_HWNDPARENT|set_overlay_owner/, "overlay controls should be owned by the main player window instead of global topmost");
assert.doesNotMatch(mpvEmbedRunSource, /OPENPLAYER_MPV_EMBED_FILE/, "normal embed overlay runtime must not auto-play the old Abbott embed smoke file");
```

- [ ] **Step 4: Remove render API backend assertions**

Delete these assertions completely:

```js
assert.match(mpvRenderBackendSource, /mpv_render_context_create|create_render_context/, "mpv render backend must create an mpv render context");
assert.match(mpvRenderBackendSource, /MPV_RENDER_API_TYPE_OPENGL|RenderParamApiType::OpenGl/, "mpv render backend must use the OpenGL render API");
assert.doesNotMatch(mpvRenderBackendSource, /set_option\("wid"|set_option_string\("wid"|MPV_RENDER_PARAM_X11_DISPLAY/, "mpv render backend must not use mpv-owned native window embedding");
```

- [ ] **Step 5: Run static verification and confirm it fails for the current code**

Run from `E:\Project\CodeProject\RustPlayer\apps\desktop`:

```powershell
npm run verify:shell
```

Expected: FAIL. The failure should mention one of these stale architecture facts: `default = ["mpv-render"]`, `CARGO_FEATURE_MPV_RENDER`, `mpv_render`, or an existing `src-tauri/src/mpv_render*` file.

- [ ] **Step 6: Commit the failing verification change**

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
git add apps/desktop/scripts/verify-shell.mjs
git commit -m "test: guard mpv embed architecture cleanup"
```

---

### Task 2: Clean Cargo Features And Build Script

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/build.rs`

- [ ] **Step 1: Update Cargo features and dependencies**

In `apps/desktop/src-tauri/Cargo.toml`, replace the `[features]` section with:

```toml
[features]
default = ["mpv-embed"]
mpv-smoke = ["dep:libmpv2"]
mpv-embed = ["dep:libmpv2", "dep:raw-window-handle", "dep:serde", "dep:windows-sys"]
```

In the `[dependencies]` section, remove this line:

```toml
libmpv2-sys = { version = "4.0.1", optional = true }
```

Replace the `windows-sys` dependency with:

```toml
windows-sys = { version = "0.60", features = [
  "Win32_Foundation",
  "Win32_UI_WindowsAndMessaging",
], optional = true }
```

- [ ] **Step 2: Remove the obsolete build-script feature gate**

In `apps/desktop/src-tauri/build.rs`, replace:

```rust
if std::env::var_os("CARGO_FEATURE_MPV_SMOKE").is_none()
    && std::env::var_os("CARGO_FEATURE_MPV_EMBED").is_none()
    && std::env::var_os("CARGO_FEATURE_MPV_RENDER").is_none()
{
    return;
}
```

with:

```rust
if std::env::var_os("CARGO_FEATURE_MPV_SMOKE").is_none()
    && std::env::var_os("CARGO_FEATURE_MPV_EMBED").is_none()
{
    return;
}
```

- [ ] **Step 3: Run static verification and confirm remaining failures are runtime/file cleanup failures**

Run from `E:\Project\CodeProject\RustPlayer\apps\desktop`:

```powershell
npm run verify:shell
```

Expected: FAIL. Cargo feature assertions should pass. Remaining failures should reference `mpv_render`, `MpvRenderState`, `mpv_render_*`, or existing render spike files.

- [ ] **Step 4: Run a Rust check to catch feature/dependency mistakes early**

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
cargo check -p openplayer-desktop
```

Expected: PASS. If it fails for a missing dependency, correct the dependency list in `apps/desktop/src-tauri/Cargo.toml` before continuing.

- [ ] **Step 5: Commit Cargo/build-script cleanup**

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
git add apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/build.rs Cargo.lock
git commit -m "chore: make mpv embed the default backend"
```

---

### Task 3: Simplify The Tauri Runtime To One Production MPV Path

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Replace top-level mpv cfg/import block**

In `apps/desktop/src-tauri/src/lib.rs`, replace the feature-gated import/module block from the first `#[cfg(any(` through the existing `pub use mpv_smoke::{MpvSmokeReport, create_headless_probe};` line with:

```rust
#[cfg(feature = "mpv-embed")]
use tauri::WindowEvent;

#[cfg(feature = "mpv-smoke")]
mod mpv_smoke;

#[cfg(feature = "mpv-embed")]
mod mpv_embed;

#[cfg(feature = "mpv-embed")]
use mpv_embed::{
    MpvEmbedSnapshot, MpvEmbedState, mpv_embed_pause, mpv_embed_play, mpv_embed_seek,
    mpv_embed_set_volume, mpv_embed_snapshot, mpv_embed_stop,
};

#[cfg(feature = "mpv-smoke")]
pub use mpv_smoke::{MpvSmokeReport, create_headless_probe};
```

- [ ] **Step 2: Gate Windows HWND helpers to mpv-embed builds**

Move these imports behind `mpv-embed` because no-default builds do not need Win32 owner handling:

```rust
#[cfg(feature = "mpv-embed")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(feature = "mpv-embed")]
use windows_sys::Win32::UI::WindowsAndMessaging::{GWLP_HWNDPARENT, SetWindowLongPtrW};
```

Apply `#[cfg(feature = "mpv-embed")]` to these helper functions:

```rust
#[cfg(feature = "mpv-embed")]
fn set_overlay_owner(main: &WebviewWindow, overlay: &WebviewWindow) {
    let Ok(main_hwnd) = window_hwnd(main) else {
        return;
    };
    let Ok(overlay_hwnd) = window_hwnd(overlay) else {
        return;
    };
    unsafe {
        SetWindowLongPtrW(overlay_hwnd as _, GWLP_HWNDPARENT, main_hwnd);
    }
}

#[cfg(feature = "mpv-embed")]
fn window_hwnd(window: &impl HasWindowHandle) -> Result<isize, String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("failed to read Tauri window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as isize),
        _ => Err("window operation is only wired for Windows HWND targets".to_string()),
    }
}
```

- [ ] **Step 3: Replace `mpv_overlay_open_path` cfg**

Replace the current `#[cfg(feature = "mpv-render")]` command definition with:

```rust
#[cfg(feature = "mpv-embed")]
#[tauri::command]
fn mpv_overlay_open_path(
    app: AppHandle,
    state: tauri::State<'_, MpvEmbedState>,
    path: String,
) -> Result<MpvEmbedSnapshot, String> {
    let main = main_window(&app)?;
    sync_overlay_to_main(&app);
    mpv_embed::open_path_for_window(&main, state.inner(), path)
}
```

- [ ] **Step 4: Replace all `run()` variants with baseline plus mpv-embed runtime**

Replace the three current runtime functions starting at `#[cfg(all(not(feature = "mpv-render"), not(feature = "mpv-embed")))] pub fn run()` through the end of the file with:

```rust
#[cfg(not(feature = "mpv-embed"))]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            window_minimize,
            window_toggle_maximize,
            window_toggle_fullscreen,
            window_start_resize,
            window_close
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}

#[cfg(feature = "mpv-embed")]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(MpvEmbedState::default())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let overlay = WebviewWindowBuilder::new(
                    app,
                    "overlay",
                    WebviewUrl::App("index.html?surface=overlay".into()),
                )
                .title("OpenPlayer Controls")
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .resizable(false)
                .skip_taskbar(true)
                .visible(false)
                .build()
                .map_err(|error| format!("failed to create overlay controls window: {error}"))?;
                set_overlay_owner(&window, &overlay);

                let app_handle = app.handle().clone();
                sync_overlay_to_main(&app_handle);
                let _ = overlay.show();
                window.on_window_event(move |event| {
                    if matches!(
                        event,
                        WindowEvent::Moved(_)
                            | WindowEvent::Resized(_)
                            | WindowEvent::ScaleFactorChanged { .. }
                    ) {
                        sync_overlay_to_main(&app_handle);
                        let state = app_handle.state::<MpvEmbedState>();
                        let _ = state.resize_video_host();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window_minimize,
            window_toggle_maximize,
            window_toggle_fullscreen,
            window_close,
            window_start_drag,
            window_start_resize,
            mpv_overlay_open_path,
            mpv_embed_play,
            mpv_embed_pause,
            mpv_embed_seek,
            mpv_embed_set_volume,
            mpv_embed_snapshot,
            mpv_embed_stop
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}
```

- [ ] **Step 5: Run formatter**

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
cargo fmt --all -- --check
```

Expected: FAIL if formatting changed. If it fails, run:

```powershell
cargo fmt --all
```

- [ ] **Step 6: Run Rust checks for default, smoke, and no-default feature sets**

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
cargo check -p openplayer-desktop
cargo check -p openplayer-desktop --features mpv-smoke
cargo check -p openplayer-desktop --no-default-features
```

Expected: PASS for all three commands.

- [ ] **Step 7: Commit runtime cleanup**

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
git add apps/desktop/src-tauri/src/lib.rs
git commit -m "refactor: use mpv embed as the desktop runtime"
```

---

### Task 4: Delete The Failed Render API Spike

**Files:**
- Delete: `apps/desktop/src-tauri/src/mpv_render.rs`
- Delete: `apps/desktop/src-tauri/src/mpv_render/sys.rs`
- Delete: `apps/desktop/src-tauri/src/mpv_render/win32_surface.rs`

- [ ] **Step 1: Delete render spike files**

Delete these files:

```text
apps/desktop/src-tauri/src/mpv_render.rs
apps/desktop/src-tauri/src/mpv_render/sys.rs
apps/desktop/src-tauri/src/mpv_render/win32_surface.rs
```

- [ ] **Step 2: Confirm no production references remain**

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
rg "mpv-render|mpv_render|MpvRenderState|mpv_render_" apps/desktop/src-tauri
```

Expected: No matches in production Rust files. The verification script is allowed to contain these rejected strings because it guards against their return.

- [ ] **Step 3: Run static verification**

Run from `E:\Project\CodeProject\RustPlayer\apps\desktop`:

```powershell
npm run verify:shell
```

Expected: PASS.

- [ ] **Step 4: Commit deleted spike files**

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
git add apps/desktop/src-tauri/src/mpv_render.rs apps/desktop/src-tauri/src/mpv_render/sys.rs apps/desktop/src-tauri/src/mpv_render/win32_surface.rs apps/desktop/scripts/verify-shell.mjs
git commit -m "refactor: remove failed mpv render spike"
```

---

### Task 5: Final Build Verification And Optional Installer Rebuild

**Files:**
- Verify: no required source edits.

- [ ] **Step 1: Run full static and frontend verification**

Run from `E:\Project\CodeProject\RustPlayer\apps\desktop`:

```powershell
npm run verify:shell
npm run build
```

Expected: both commands PASS.

- [ ] **Step 2: Run full Rust verification**

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
cargo fmt --all -- --check
cargo check -p openplayer-desktop
cargo check -p openplayer-desktop --features mpv-smoke
cargo check -p openplayer-desktop --no-default-features
```

Expected: all commands PASS.

- [ ] **Step 3: Rebuild installer with the existing local mpv artifacts**

Run from `E:\Project\CodeProject\RustPlayer\apps\desktop`:

```powershell
$env:PATH = "E:\Project\CodeProject\RustPlayer\vendor\native\mpv\windows-x64;$env:PATH"
$env:OPENPLAYER_MPV_DIR = "E:\Project\CodeProject\RustPlayer\vendor\native\mpv\windows-x64"
npm run tauri:build
```

Expected: PASS and produce:

```text
E:\Project\CodeProject\RustPlayer\target\release\bundle\nsis\OpenPlayer_0.1.0_x64-setup.exe
```

- [ ] **Step 4: Confirm clean worktree**

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
git status --short
```

Expected: no output.

---

## Self-Review Checklist

- Spec coverage: Tasks 1 and 4 update verification and remove render spike files. Task 2 makes `mpv-embed` default and removes render-only dependencies. Task 3 simplifies runtime to one production overlay/embed path. Task 5 verifies the unchanged user-visible behavior by build and installer checks.
- Placeholder scan: This plan contains exact paths, code snippets, commands, expected outcomes, and commit messages.
- Type consistency: The plan consistently uses `MpvEmbedState`, `MpvEmbedSnapshot`, `mpv_overlay_open_path`, and `mpv_embed::*`; it removes `MpvRenderState` and `mpv_render_*` everywhere.
