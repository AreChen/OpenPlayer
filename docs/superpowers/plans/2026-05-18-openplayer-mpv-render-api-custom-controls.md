# OpenPlayer MPV Render API Custom Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the mpv child HWND playback path with a Windows-first mpv render API spike that lets OpenPlayer draw custom React/CSS controls above mpv video.

**Architecture:** A Rust render actor owns the mpv handle, mpv render context, and OpenGL video surface on a dedicated render thread. Tauri commands send playback commands to the actor; the WebView remains the styled control layer above an OpenPlayer-owned video surface. The primary path must not pass `wid` to mpv and must not use HTML `<video>` playback.

**Tech Stack:** Tauri 2, React 19, TypeScript, Rust, `libmpv2-sys`, Win32 `windows-sys`, WGL/OpenGL, Vite.

---

## Scope Check

This plan covers one subsystem: mpv render API playback with custom controls. It deliberately excludes installer bundling, cross-platform support, advanced subtitles, audio track UI, and persisted playback state.

## File Structure

- Modify `apps/desktop/scripts/verify-shell.mjs`: static guards for render API architecture, no HTML playback, no primary `wid` path, and no dev auto-play requirement.
- Modify `apps/desktop/src-tauri/Cargo.toml`: replace the default `mpv-embed` child-window feature with a new `mpv-render` feature and Win32/OpenGL dependencies.
- Modify `apps/desktop/src-tauri/build.rs`: link mpv artifacts for `mpv-render` as well as the existing smoke feature.
- Create `apps/desktop/src-tauri/src/mpv_render.rs`: Tauri command-facing render backend, actor command API, snapshots, validation, and tests.
- Create `apps/desktop/src-tauri/src/mpv_render/win32_surface.rs`: Windows OpenGL child surface owned by OpenPlayer, not by mpv via `wid`.
- Create `apps/desktop/src-tauri/src/mpv_render/sys.rs`: raw `libmpv2_sys` helper wrappers for mpv command/property/render calls.
- Modify `apps/desktop/src-tauri/src/lib.rs`: register render backend state and commands, wire resize events, remove child HWND command registration from the default path.
- Modify `apps/desktop/src-tauri/tauri.conf.json`: restore custom chrome only after the WebView overlay is the reachable control layer.
- Modify `apps/desktop/src/App.tsx`: keep custom controls, remove stale child HWND assumptions, use render backend commands.
- Modify `apps/desktop/src/styles.css`: make the player shell overlay-friendly, no permanent video control bands.

Commit steps in this plan are checkpoints. Run the commit commands only when the user has explicitly authorized commits in the current session.

---

### Task 1: Add Architecture Guards Before Touching Runtime Code

**Files:**
- Modify: `apps/desktop/scripts/verify-shell.mjs`

- [ ] **Step 1: Write the failing static guards**

Replace the mpv-specific assertions in `apps/desktop/scripts/verify-shell.mjs` with guards that require the render API path and reject the child HWND path as the default implementation:

```js
const appSource = readText("src/App.tsx");
const tauriConfig = JSON.parse(readText("src-tauri/tauri.conf.json"));
const tauriLibSource = readText("src-tauri/src/lib.rs");
const cargoToml = readText("src-tauri/Cargo.toml");
const mpvRenderSource = fs.existsSync(path.join(root, "src-tauri/src/mpv_render.rs"))
  ? readText("src-tauri/src/mpv_render.rs")
  : "";

assert.doesNotMatch(appSource, /<video\b|URL\.createObjectURL|\bFile\b/, "mpv-first player must not use browser video, object URLs, or browser File playback");
assert.match(appSource, /open\(/, "player must keep native Tauri file picker access");
assert.match(appSource, /mpv_render_open_path/, "player must open files through the mpv render backend");
assert.match(cargoToml, /default = \["mpv-render"\]/, "desktop default feature must use the mpv render backend");
assert.match(cargoToml, /mpv-render/, "Cargo features must define mpv-render");
assert.match(mpvRenderSource, /mpv_render_context_create|create_render_context/, "mpv render backend must create an mpv render context");
assert.match(mpvRenderSource, /MPV_RENDER_API_TYPE_OPENGL|RenderParamApiType::OpenGl/, "mpv render backend must use the OpenGL render API");
assert.doesNotMatch(mpvRenderSource, /set_option\("wid"|MPV_RENDER_PARAM_X11_DISPLAY/, "mpv render backend must not use mpv-owned native window embedding");
assert.doesNotMatch(tauriLibSource, /mpv_embed_open_path|mpv_embed_play|mpv_embed_pause|mpv_embed_seek|mpv_embed_set_volume/, "default runtime must not register child HWND mpv embed commands");
assert.doesNotMatch(tauriLibSource, /OPENPLAYER_MPV_EMBED_FILE/, "normal render API runtime must not auto-play the old Abbott embed smoke file");
assert.equal(tauriConfig.app.windows[0].decorations, false, "custom controls require custom window chrome once overlay is restored");
assert.equal(tauriConfig.app.windows[0].transparent, true, "video surface behind WebView requires transparent WebView/window composition");
```

- [ ] **Step 2: Run the guard to verify RED**

Run:

```powershell
npm run verify:shell
```

Expected: FAIL with at least `mpv render backend must create an mpv render context` because `src-tauri/src/mpv_render.rs` does not exist yet.

- [ ] **Step 3: Commit checkpoint if authorized**

Run only if commits are authorized:

```powershell
git add apps/desktop/scripts/verify-shell.mjs
git commit -m "test: guard mpv render api architecture"
```

---

### Task 2: Switch Cargo Features To The Render Backend

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/build.rs`

- [ ] **Step 1: Update `Cargo.toml` features and dependencies**

Change the feature section and dependencies to make `mpv-render` the default path. Keep `mpv-embed` available only as a non-default spike reference.

```toml
[features]
default = ["mpv-render"]
mpv-smoke = ["dep:libmpv2"]
mpv-embed = ["dep:libmpv2", "dep:raw-window-handle", "dep:serde", "dep:windows-sys"]
mpv-render = ["dep:libmpv2-sys", "dep:raw-window-handle", "dep:serde", "dep:windows-sys"]

[dependencies]
tauri = { version = "2", features = [] }
libmpv2 = { version = "6.0.0", optional = true, default-features = false }
libmpv2-sys = { version = "4.0.1", optional = true }
raw-window-handle = { version = "0.6", optional = true }
serde = { version = "1", features = ["derive"], optional = true }
tauri-plugin-dialog = "2"
windows-sys = { version = "0.60", features = [
  "Win32_Foundation",
  "Win32_Graphics_Gdi",
  "Win32_Graphics_OpenGL",
  "Win32_System_LibraryLoader",
  "Win32_UI_WindowsAndMessaging"
], optional = true }
```

- [ ] **Step 2: Update build script feature detection**

In `apps/desktop/src-tauri/build.rs`, rename `configure_mpv_smoke_linking` to `configure_mpv_linking` and include `CARGO_FEATURE_MPV_RENDER` in the early-return check:

```rust
fn configure_mpv_linking() {
    if std::env::var_os("CARGO_FEATURE_MPV_SMOKE").is_none()
        && std::env::var_os("CARGO_FEATURE_MPV_EMBED").is_none()
        && std::env::var_os("CARGO_FEATURE_MPV_RENDER").is_none()
    {
        return;
    }

    println!("cargo:rerun-if-env-changed=OPENPLAYER_MPV_DIR");

    #[cfg(windows)]
    {
        let mpv_dir = std::env::var_os("OPENPLAYER_MPV_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                let manifest_dir = std::path::PathBuf::from(
                    std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"),
                );
                manifest_dir.join("../../../vendor/native/mpv/windows-x64")
            });
        let import_library = mpv_dir.join("libmpv.dll.a");
        let runtime_library = mpv_dir.join("libmpv-2.dll");

        if !import_library.exists() || !runtime_library.exists() {
            let missing = [
                (!import_library.exists()).then_some("libmpv.dll.a"),
                (!runtime_library.exists()).then_some("libmpv-2.dll"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");

            panic!(
                "mpv integration requires local ignored mpv artifacts at {} or OPENPLAYER_MPV_DIR; missing {}",
                mpv_dir.display(),
                missing
            );
        }

        println!("cargo:rustc-link-search=native={}", mpv_dir.display());
        println!("cargo:rerun-if-changed={}", import_library.display());
        println!("cargo:rerun-if-changed={}", runtime_library.display());
    }
}

fn main() {
    configure_mpv_linking();
    // Keep the existing Tauri build code below this call.
}
```

- [ ] **Step 3: Run the guard to verify progress**

Run:

```powershell
npm run verify:shell
```

Expected: still FAIL because `mpv_render.rs` is not implemented, but the `default = ["mpv-render"]` assertion now passes.

- [ ] **Step 4: Run Cargo metadata check**

Run:

```powershell
cargo check -p openplayer-desktop --no-default-features
```

Expected: PASS. This confirms the non-mpv default-disabled crate still parses without local mpv artifacts.

- [ ] **Step 5: Commit checkpoint if authorized**

Run only if commits are authorized:

```powershell
git add apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/build.rs Cargo.lock
git commit -m "build: add mpv render backend feature"
```

---

### Task 3: Add Pure Render Backend Types And Tests

**Files:**
- Create: `apps/desktop/src-tauri/src/mpv_render.rs`

- [ ] **Step 1: Write tests for validation and viewport behavior**

Create `apps/desktop/src-tauri/src/mpv_render.rs` with pure tests first:

```rust
use std::{path::PathBuf, sync::Mutex};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RenderViewport {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MpvRenderSnapshot {
    path: String,
    status: String,
    paused: bool,
    position: f64,
    duration: f64,
    volume: f64,
}

#[derive(Default)]
pub struct MpvRenderState {
    actor: Mutex<Option<MpvRenderActor>>,
}

struct MpvRenderActor;

fn validate_media_path(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("enter a local media path".to_string());
    }

    let path = PathBuf::from(trimmed);
    if !path.is_file() {
        return Err(format!("media path does not exist: {}", path.display()));
    }

    Ok(path)
}

fn clamp_volume(volume: f64) -> Result<f64, String> {
    if !volume.is_finite() {
        return Err("invalid mpv volume".to_string());
    }

    Ok(volume.clamp(0.0, 100.0))
}

fn render_viewport(width: i32, height: i32) -> RenderViewport {
    RenderViewport {
        x: 0,
        y: 0,
        width: width.max(1),
        height: height.max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_media_path() {
        let error = validate_media_path("   ").expect_err("empty paths should be rejected");

        assert_eq!(error, "enter a local media path");
    }

    #[test]
    fn clamps_valid_volume_to_mpv_percent_range() {
        assert_eq!(clamp_volume(-12.0).expect("finite volume is valid"), 0.0);
        assert_eq!(clamp_volume(42.5).expect("finite volume is valid"), 42.5);
        assert_eq!(clamp_volume(182.0).expect("finite volume is valid"), 100.0);
    }

    #[test]
    fn rejects_non_finite_volume() {
        let error = clamp_volume(f64::NAN).expect_err("NaN volume should be rejected");

        assert_eq!(error, "invalid mpv volume");
    }

    #[test]
    fn render_viewport_fills_available_area_without_control_reserves() {
        assert_eq!(
            render_viewport(1280, 720),
            RenderViewport {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            }
        );
    }
}
```

- [ ] **Step 2: Include the module in `lib.rs` just enough to compile**

In `apps/desktop/src-tauri/src/lib.rs`, add this module declaration near the existing mpv modules:

```rust
#[cfg(feature = "mpv-render")]
mod mpv_render;
```

- [ ] **Step 3: Run tests to verify RED or compile gaps**

Run:

```powershell
cargo test -p openplayer-desktop --features mpv-render mpv_render::tests
```

Expected: PASS for the pure tests after the module is included. If the command fails because `mpv-render` and default features conflict with the old child embed module, fix only the feature gating in `lib.rs` so `mpv-embed` and `mpv-render` are separate paths.

- [ ] **Step 4: Commit checkpoint if authorized**

Run only if commits are authorized:

```powershell
git add apps/desktop/src-tauri/src/mpv_render.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "test: add mpv render backend core types"
```

---

### Task 4: Add The Windows OpenGL Surface Wrapper

**Files:**
- Create: `apps/desktop/src-tauri/src/mpv_render/win32_surface.rs`
- Modify: `apps/desktop/src-tauri/src/mpv_render.rs`

- [ ] **Step 1: Add focused pure tests for Win32 helper functions**

Create `apps/desktop/src-tauri/src/mpv_render/win32_surface.rs` with helper tests before adding Win32 calls:

```rust
use super::RenderViewport;

pub struct Win32RenderSurface {
    parent_hwnd: isize,
    hwnd: isize,
    hdc: isize,
    hglrc: isize,
    viewport: RenderViewport,
}

pub fn wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn viewport_from_client_size(width: i32, height: i32) -> RenderViewport {
    RenderViewport {
        x: 0,
        y: 0,
        width: width.max(1),
        height: height.max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_null_appends_single_terminator() {
        let encoded = wide_null("OpenPlayerRenderSurface");

        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(encoded.iter().filter(|value| **value == 0).count(), 1);
    }

    #[test]
    fn viewport_from_client_size_never_returns_zero_dimensions() {
        assert_eq!(
            viewport_from_client_size(0, -20),
            RenderViewport {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            }
        );
    }
}
```

In `apps/desktop/src-tauri/src/mpv_render.rs`, expose the submodule:

```rust
#[cfg(windows)]
mod win32_surface;
```

- [ ] **Step 2: Run tests to verify helper behavior**

Run:

```powershell
cargo test -p openplayer-desktop --features mpv-render win32_surface::tests
```

Expected: PASS.

- [ ] **Step 3: Implement Win32 surface creation**

Extend `win32_surface.rs` with Win32 imports and a `new(parent_hwnd)` constructor. The created HWND is OpenPlayer-owned and must be placed behind the WebView, not passed to mpv as `wid`.

```rust
use std::{ffi::c_void, ptr::null_mut};

use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, RECT},
    Graphics::{
        Gdi::{ChoosePixelFormat, GetDC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW, PFD_MAIN_PLANE, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA, PIXELFORMATDESCRIPTOR, ReleaseDC, SetPixelFormat, SwapBuffers},
        OpenGL::{HGLRC, wglCreateContext, wglDeleteContext, wglGetProcAddress, wglMakeCurrent},
    },
    System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
    UI::WindowsAndMessaging::{
        CS_OWNDC, CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, HWND_BOTTOM, MoveWindow, RegisterClassW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_VISIBLE,
    },
};

const SURFACE_CLASS_NAME: &str = "OpenPlayerRenderSurface";

unsafe extern "system" fn surface_window_proc(hwnd: HWND, message: u32, wparam: usize, lparam: isize) -> isize {
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

impl Win32RenderSurface {
    pub fn new(parent_hwnd: isize) -> Result<Self, String> {
        let parent = parent_hwnd as HWND;
        let viewport = parent_viewport(parent)?;
        register_surface_class()?;

        let class_name = wide_null(SURFACE_CLASS_NAME);
        let window_name = wide_null("OpenPlayer MPV Render Surface");
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                window_name.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
                viewport.x,
                viewport.y,
                viewport.width,
                viewport.height,
                parent,
                null_mut(),
                null_mut(),
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            return Err("failed to create OpenPlayer render surface".to_string());
        }

        unsafe {
            SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }

        let hdc = unsafe { GetDC(hwnd) };
        if hdc.is_null() {
            unsafe { DestroyWindow(hwnd) };
            return Err("failed to get render surface device context".to_string());
        }

        let mut pfd = PIXELFORMATDESCRIPTOR {
            nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cRedBits: 0,
            cRedShift: 0,
            cGreenBits: 0,
            cGreenShift: 0,
            cBlueBits: 0,
            cBlueShift: 0,
            cAlphaBits: 8,
            cAlphaShift: 0,
            cAccumBits: 0,
            cAccumRedBits: 0,
            cAccumGreenBits: 0,
            cAccumBlueBits: 0,
            cAccumAlphaBits: 0,
            cDepthBits: 24,
            cStencilBits: 8,
            cAuxBuffers: 0,
            iLayerType: PFD_MAIN_PLANE,
            bReserved: 0,
            dwLayerMask: 0,
            dwVisibleMask: 0,
            dwDamageMask: 0,
        };
        let pixel_format = unsafe { ChoosePixelFormat(hdc, &pfd) };
        if pixel_format == 0 || unsafe { SetPixelFormat(hdc, pixel_format, &pfd) } == 0 {
            unsafe {
                ReleaseDC(hwnd, hdc);
                DestroyWindow(hwnd);
            }
            return Err("failed to configure OpenGL pixel format".to_string());
        }

        let hglrc = unsafe { wglCreateContext(hdc) };
        if hglrc.is_null() {
            unsafe {
                ReleaseDC(hwnd, hdc);
                DestroyWindow(hwnd);
            }
            return Err("failed to create OpenGL context".to_string());
        }

        Ok(Self {
            parent_hwnd,
            hwnd: hwnd as isize,
            hdc: hdc as isize,
            hglrc: hglrc as isize,
            viewport,
        })
    }

    pub fn make_current(&self) -> Result<(), String> {
        let ok = unsafe { wglMakeCurrent(self.hdc as _, self.hglrc as _) };
        if ok == 0 {
            return Err("failed to make OpenGL render context current".to_string());
        }
        Ok(())
    }

    pub fn swap_buffers(&self) {
        unsafe { SwapBuffers(self.hdc as _) };
    }

    pub fn resize_to_parent(&mut self) -> Result<RenderViewport, String> {
        let viewport = parent_viewport(self.parent_hwnd as HWND)?;
        unsafe {
            MoveWindow(self.hwnd as HWND, viewport.x, viewport.y, viewport.width, viewport.height, 1);
            SetWindowPos(self.hwnd as HWND, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
        self.viewport = viewport;
        Ok(viewport)
    }

    pub fn viewport(&self) -> RenderViewport {
        self.viewport
    }
}

impl Drop for Win32RenderSurface {
    fn drop(&mut self) {
        unsafe {
            wglMakeCurrent(null_mut(), null_mut());
            wglDeleteContext(self.hglrc as HGLRC);
            ReleaseDC(self.hwnd as HWND, self.hdc as _);
            DestroyWindow(self.hwnd as HWND);
        }
    }
}

fn parent_viewport(parent: HWND) -> Result<RenderViewport, String> {
    let mut rect = RECT::default();
    if unsafe { GetClientRect(parent, &mut rect) } == 0 {
        return Err("failed to read Tauri client size".to_string());
    }
    Ok(viewport_from_client_size(rect.right - rect.left, rect.bottom - rect.top))
}

fn register_surface_class() -> Result<(), String> {
    let class_name = wide_null(SURFACE_CLASS_NAME);
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) } as HINSTANCE;
    let class = WNDCLASSW {
        style: CS_OWNDC,
        lpfnWndProc: Some(surface_window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance,
        hIcon: null_mut(),
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
    };

    let atom = unsafe { RegisterClassW(&class) };
    if atom == 0 {
        return Err("failed to register OpenPlayer render surface class".to_string());
    }

    Ok(())
}

pub unsafe extern "C" fn get_proc_address(_ctx: *mut c_void, name: *const i8) -> *mut c_void {
    let address = unsafe { wglGetProcAddress(name.cast()) };
    if !address.is_null() {
        return address.cast();
    }

    let module = unsafe { LoadLibraryW(wide_null("opengl32.dll").as_ptr()) };
    if module.is_null() {
        return std::ptr::null_mut();
    }

    unsafe { GetProcAddress(module, name.cast()) }
}
```

- [ ] **Step 4: Run formatting and compile check**

Run:

```powershell
cargo fmt --all -- --check
cargo check -p openplayer-desktop --features mpv-render
```

Expected: PASS. If Windows API signatures differ, fix imports or pointer casts only in `win32_surface.rs`.

- [ ] **Step 5: Commit checkpoint if authorized**

Run only if commits are authorized:

```powershell
git add apps/desktop/src-tauri/src/mpv_render.rs apps/desktop/src-tauri/src/mpv_render/win32_surface.rs
git commit -m "feat: add OpenPlayer-owned OpenGL video surface"
```

---

### Task 5: Add Raw mpv Render API Wrapper

**Files:**
- Create: `apps/desktop/src-tauri/src/mpv_render/sys.rs`
- Modify: `apps/desktop/src-tauri/src/mpv_render.rs`

- [ ] **Step 1: Add raw helper module with error conversion**

Create `apps/desktop/src-tauri/src/mpv_render/sys.rs`:

```rust
use std::{ffi::{CStr, CString}, os::raw::{c_char, c_double, c_int, c_void}, ptr::null_mut};

use libmpv2_sys as mpv;

pub struct RawMpv {
    handle: *mut mpv::mpv_handle,
}

pub struct RawRenderContext {
    ctx: *mut mpv::mpv_render_context,
}

unsafe impl Send for RawMpv {}
unsafe impl Send for RawRenderContext {}

impl RawMpv {
    pub fn new() -> Result<Self, String> {
        let handle = unsafe { mpv::mpv_create() };
        if handle.is_null() {
            return Err("mpv_create returned null".to_string());
        }

        let player = Self { handle };
        player.set_option_string("vo", "libmpv")?;
        player.set_option_string("hwdec", "auto-safe")?;
        player.set_option_string("keep-open", "yes")?;
        mpv_result(unsafe { mpv::mpv_initialize(player.handle) }, "mpv_initialize")?;
        Ok(player)
    }

    pub fn command(&self, args: &[&str]) -> Result<(), String> {
        let cstrings = args
            .iter()
            .map(|arg| CString::new(*arg).map_err(|_| format!("mpv command contains null byte: {arg}")))
            .collect::<Result<Vec<_>, _>>()?;
        let mut pointers = cstrings.iter().map(|arg| arg.as_ptr()).collect::<Vec<_>>();
        pointers.push(std::ptr::null());

        mpv_result(unsafe { mpv::mpv_command(self.handle, pointers.as_ptr()) }, "mpv_command")
    }

    pub fn set_option_string(&self, name: &str, value: &str) -> Result<(), String> {
        let name = CString::new(name).map_err(|_| "mpv option name contains null byte".to_string())?;
        let value = CString::new(value).map_err(|_| "mpv option value contains null byte".to_string())?;
        mpv_result(unsafe { mpv::mpv_set_option_string(self.handle, name.as_ptr(), value.as_ptr()) }, "mpv_set_option_string")
    }

    pub fn set_flag_property(&self, name: &str, value: bool) -> Result<(), String> {
        let name = CString::new(name).map_err(|_| "mpv property name contains null byte".to_string())?;
        let mut flag: c_int = i32::from(value);
        mpv_result(
            unsafe { mpv::mpv_set_property(self.handle, name.as_ptr(), mpv::mpv_format_MPV_FORMAT_FLAG, (&mut flag as *mut c_int).cast()) },
            "mpv_set_property flag",
        )
    }

    pub fn set_double_property(&self, name: &str, value: f64) -> Result<(), String> {
        let name = CString::new(name).map_err(|_| "mpv property name contains null byte".to_string())?;
        let mut value = value as c_double;
        mpv_result(
            unsafe { mpv::mpv_set_property(self.handle, name.as_ptr(), mpv::mpv_format_MPV_FORMAT_DOUBLE, (&mut value as *mut c_double).cast()) },
            "mpv_set_property double",
        )
    }

    pub fn get_double_property(&self, name: &str) -> f64 {
        let Ok(name) = CString::new(name) else { return 0.0; };
        let mut value: c_double = 0.0;
        let result = unsafe { mpv::mpv_get_property(self.handle, name.as_ptr(), mpv::mpv_format_MPV_FORMAT_DOUBLE, (&mut value as *mut c_double).cast()) };
        if result < 0 { 0.0 } else { value }
    }

    pub fn get_flag_property(&self, name: &str) -> bool {
        let Ok(name) = CString::new(name) else { return false; };
        let mut value: c_int = 0;
        let result = unsafe { mpv::mpv_get_property(self.handle, name.as_ptr(), mpv::mpv_format_MPV_FORMAT_FLAG, (&mut value as *mut c_int).cast()) };
        result >= 0 && value != 0
    }

    pub fn create_render_context(&self, get_proc_address: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void) -> Result<RawRenderContext, String> {
        let mut init_params = mpv::mpv_opengl_init_params {
            get_proc_address: Some(get_proc_address),
            get_proc_address_ctx: null_mut(),
        };
        let mut params = [
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
                data: mpv::MPV_RENDER_API_TYPE_OPENGL.as_ptr().cast::<c_void>() as *mut c_void,
            },
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                data: (&mut init_params as *mut mpv::mpv_opengl_init_params).cast(),
            },
            mpv::mpv_render_param { type_: 0, data: null_mut() },
        ];
        let mut ctx = null_mut();
        mpv_result(unsafe { mpv::mpv_render_context_create(&mut ctx, self.handle, params.as_mut_ptr()) }, "mpv_render_context_create")?;
        Ok(RawRenderContext { ctx })
    }
}

impl RawRenderContext {
    pub fn set_update_callback(&self, sender: std::sync::mpsc::Sender<()>) {
        let boxed = Box::into_raw(Box::new(sender));
        unsafe { mpv::mpv_render_context_set_update_callback(self.ctx, Some(render_update_callback), boxed.cast()) };
    }

    pub fn update(&self) -> bool {
        let flags = unsafe { mpv::mpv_render_context_update(self.ctx) };
        flags & mpv::mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME as u64 != 0
    }

    pub fn render(&self, width: i32, height: i32) -> Result<(), String> {
        let mut fbo = mpv::mpv_opengl_fbo { fbo: 0, w: width, h: height, internal_format: 0 };
        let mut flip_y: c_int = 1;
        let mut params = [
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_FBO,
                data: (&mut fbo as *mut mpv::mpv_opengl_fbo).cast(),
            },
            mpv::mpv_render_param {
                type_: mpv::mpv_render_param_type_MPV_RENDER_PARAM_FLIP_Y,
                data: (&mut flip_y as *mut c_int).cast(),
            },
            mpv::mpv_render_param { type_: 0, data: null_mut() },
        ];
        mpv_result(unsafe { mpv::mpv_render_context_render(self.ctx, params.as_mut_ptr()) }, "mpv_render_context_render")
    }

    pub fn report_swap(&self) {
        unsafe { mpv::mpv_render_context_report_swap(self.ctx) };
    }
}

impl Drop for RawRenderContext {
    fn drop(&mut self) {
        unsafe { mpv::mpv_render_context_free(self.ctx) };
    }
}

impl Drop for RawMpv {
    fn drop(&mut self) {
        unsafe { mpv::mpv_terminate_destroy(self.handle) };
    }
}

unsafe extern "C" fn render_update_callback(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    let sender = unsafe { &*(ctx as *const std::sync::mpsc::Sender<()>) };
    let _ = sender.send(());
}

fn mpv_result(code: i32, operation: &str) -> Result<(), String> {
    if code >= 0 {
        return Ok(());
    }

    let message = unsafe {
        let raw = mpv::mpv_error_string(code);
        if raw.is_null() {
            "unknown mpv error".to_string()
        } else {
            CStr::from_ptr(raw).to_string_lossy().into_owned()
        }
    };
    Err(format!("{operation} failed: {message}"))
}
```

In `mpv_render.rs`, add:

```rust
mod sys;
```

- [ ] **Step 2: Run compile check**

Run:

```powershell
cargo check -p openplayer-desktop --features mpv-render
```

Expected: PASS. If `libmpv2_sys` constant names differ, use the exact names from `C:\Users\cmx27\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\libmpv2-sys-4.0.1\pregenerated_bindings.rs`.

- [ ] **Step 3: Commit checkpoint if authorized**

Run only if commits are authorized:

```powershell
git add apps/desktop/src-tauri/src/mpv_render.rs apps/desktop/src-tauri/src/mpv_render/sys.rs
git commit -m "feat: add raw mpv render api wrapper"
```

---

### Task 6: Implement The Render Actor And Tauri Commands

**Files:**
- Modify: `apps/desktop/src-tauri/src/mpv_render.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`

- [ ] **Step 1: Add actor command shape**

In `mpv_render.rs`, add render actor commands. Keep command responses synchronous for the first spike so React command behavior remains straightforward.

```rust
use std::sync::mpsc;

enum RenderCommand {
    Open { path: String, response: mpsc::Sender<Result<MpvRenderSnapshot, String>> },
    Play { response: mpsc::Sender<Result<MpvRenderSnapshot, String>> },
    Pause { response: mpsc::Sender<Result<MpvRenderSnapshot, String>> },
    Seek { position: f64, response: mpsc::Sender<Result<MpvRenderSnapshot, String>> },
    SetVolume { volume: f64, response: mpsc::Sender<Result<MpvRenderSnapshot, String>> },
    Snapshot { response: mpsc::Sender<Result<Option<MpvRenderSnapshot>, String>> },
    Resize,
    RenderFrame,
    Stop { response: mpsc::Sender<Result<(), String>> },
    Shutdown,
}
```

- [ ] **Step 2: Implement state startup and command dispatch**

Add methods on `MpvRenderState`:

```rust
impl MpvRenderState {
    pub fn start(&self, parent_hwnd: isize) -> Result<(), String> {
        let mut actor = self.actor.lock().map_err(|_| "mpv render state lock failed".to_string())?;
        if actor.is_some() {
            return Ok(());
        }

        let (sender, receiver) = mpsc::channel::<RenderCommand>();
        std::thread::Builder::new()
            .name("openplayer-mpv-render".to_string())
            .spawn(move || render_thread(parent_hwnd, receiver))
            .map_err(|error| format!("failed to start mpv render thread: {error}"))?;

        *actor = Some(MpvRenderActor { sender });
        Ok(())
    }

    pub fn resize(&self) -> Result<(), String> {
        self.send_without_response(RenderCommand::Resize)
    }

    fn actor_sender(&self) -> Result<mpsc::Sender<RenderCommand>, String> {
        let actor = self.actor.lock().map_err(|_| "mpv render state lock failed".to_string())?;
        actor.as_ref().map(|actor| actor.sender.clone()).ok_or_else(|| "mpv render backend is not started".to_string())
    }

    fn send_without_response(&self, command: RenderCommand) -> Result<(), String> {
        self.actor_sender()?.send(command).map_err(|_| "mpv render thread is unavailable".to_string())
    }
}

struct MpvRenderActor {
    sender: mpsc::Sender<RenderCommand>,
}
```

- [ ] **Step 3: Implement command-facing helpers**

Add a generic helper for request/response command routing:

```rust
fn request_snapshot(
    state: &MpvRenderState,
    build: impl FnOnce(mpsc::Sender<Result<MpvRenderSnapshot, String>>) -> RenderCommand,
) -> Result<MpvRenderSnapshot, String> {
    let sender = state.actor_sender()?;
    let (response_tx, response_rx) = mpsc::channel();
    sender.send(build(response_tx)).map_err(|_| "mpv render thread is unavailable".to_string())?;
    response_rx.recv().map_err(|_| "mpv render thread dropped command response".to_string())?
}
```

- [ ] **Step 4: Implement Tauri command functions**

Add command functions matching the frontend names:

```rust
#[tauri::command]
pub fn mpv_render_open_path(state: tauri::State<'_, MpvRenderState>, path: String) -> Result<MpvRenderSnapshot, String> {
    let path = validate_media_path(&path)?.to_string_lossy().to_string();
    request_snapshot(&state, |response| RenderCommand::Open { path, response })
}

#[tauri::command]
pub fn mpv_render_play(state: tauri::State<'_, MpvRenderState>) -> Result<MpvRenderSnapshot, String> {
    request_snapshot(&state, |response| RenderCommand::Play { response })
}

#[tauri::command]
pub fn mpv_render_pause(state: tauri::State<'_, MpvRenderState>) -> Result<MpvRenderSnapshot, String> {
    request_snapshot(&state, |response| RenderCommand::Pause { response })
}

#[tauri::command]
pub fn mpv_render_seek(state: tauri::State<'_, MpvRenderState>, position: f64) -> Result<MpvRenderSnapshot, String> {
    if !position.is_finite() || position < 0.0 {
        return Err("invalid mpv seek target".to_string());
    }
    request_snapshot(&state, |response| RenderCommand::Seek { position, response })
}

#[tauri::command]
pub fn mpv_render_set_volume(state: tauri::State<'_, MpvRenderState>, volume: f64) -> Result<MpvRenderSnapshot, String> {
    let volume = clamp_volume(volume)?;
    request_snapshot(&state, |response| RenderCommand::SetVolume { volume, response })
}

#[tauri::command]
pub fn mpv_render_snapshot(state: tauri::State<'_, MpvRenderState>) -> Result<Option<MpvRenderSnapshot>, String> {
    let sender = state.actor_sender()?;
    let (response_tx, response_rx) = mpsc::channel();
    sender.send(RenderCommand::Snapshot { response: response_tx }).map_err(|_| "mpv render thread is unavailable".to_string())?;
    response_rx.recv().map_err(|_| "mpv render thread dropped command response".to_string())?
}
```

- [ ] **Step 5: Implement render thread loop**

Add the first version of `render_thread`. It owns Win32/WGL and mpv resources on one thread.

```rust
#[cfg(windows)]
fn render_thread(parent_hwnd: isize, receiver: mpsc::Receiver<RenderCommand>) {
    let mut surface = match win32_surface::Win32RenderSurface::new(parent_hwnd) {
        Ok(surface) => surface,
        Err(error) => {
            eprintln!("mpv render surface init failed: {error}");
            return;
        }
    };
    if let Err(error) = surface.make_current() {
        eprintln!("mpv render make-current failed: {error}");
        return;
    }

    let mpv = match sys::RawMpv::new() {
        Ok(mpv) => mpv,
        Err(error) => {
            eprintln!("mpv init failed: {error}");
            return;
        }
    };
    let render = match mpv.create_render_context(win32_surface::get_proc_address) {
        Ok(render) => render,
        Err(error) => {
            eprintln!("mpv render context init failed: {error}");
            return;
        }
    };

    let (redraw_tx, redraw_rx) = mpsc::channel();
    render.set_update_callback(redraw_tx);
    let mut current_path = String::new();
    let mut current_volume = 82.0;

    loop {
        while redraw_rx.try_recv().is_ok() {
            draw_frame(&render, &mut surface);
        }

        match receiver.recv_timeout(std::time::Duration::from_millis(16)) {
            Ok(RenderCommand::Open { path, response }) => {
                current_path = path.clone();
                let result = mpv.command(&["loadfile", &path, "replace"]).map(|_| snapshot(&mpv, &current_path, current_volume, "playing"));
                let _ = response.send(result);
                draw_frame(&render, &mut surface);
            }
            Ok(RenderCommand::Play { response }) => {
                let result = mpv.set_flag_property("pause", false).map(|_| snapshot(&mpv, &current_path, current_volume, "playing"));
                let _ = response.send(result);
            }
            Ok(RenderCommand::Pause { response }) => {
                let result = mpv.set_flag_property("pause", true).map(|_| snapshot(&mpv, &current_path, current_volume, "paused"));
                let _ = response.send(result);
            }
            Ok(RenderCommand::Seek { position, response }) => {
                let result = mpv.command(&["seek", &position.to_string(), "absolute"]).map(|_| snapshot(&mpv, &current_path, current_volume, "playing"));
                let _ = response.send(result);
            }
            Ok(RenderCommand::SetVolume { volume, response }) => {
                current_volume = volume;
                let result = mpv.set_double_property("volume", volume).map(|_| snapshot(&mpv, &current_path, current_volume, "playing"));
                let _ = response.send(result);
            }
            Ok(RenderCommand::Snapshot { response }) => {
                let _ = response.send(if current_path.is_empty() { Ok(None) } else { Ok(Some(snapshot(&mpv, &current_path, current_volume, "ready"))) });
            }
            Ok(RenderCommand::Resize) => {
                let _ = surface.resize_to_parent();
                draw_frame(&render, &mut surface);
            }
            Ok(RenderCommand::RenderFrame) => draw_frame(&render, &mut surface),
            Ok(RenderCommand::Stop { response }) => {
                let result = mpv.command(&["stop"]).map(|_| ());
                let _ = response.send(result);
            }
            Ok(RenderCommand::Shutdown) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn draw_frame(render: &sys::RawRenderContext, surface: &mut win32_surface::Win32RenderSurface) {
    let _ = render.update();
    let viewport = surface.viewport();
    if render.render(viewport.width, viewport.height).is_ok() {
        surface.swap_buffers();
        render.report_swap();
    }
}

fn snapshot(mpv: &sys::RawMpv, path: &str, volume: f64, fallback_status: &str) -> MpvRenderSnapshot {
    let paused = mpv.get_flag_property("pause");
    MpvRenderSnapshot {
        path: path.to_string(),
        status: if paused { "paused" } else { fallback_status }.to_string(),
        paused,
        position: mpv.get_double_property("time-pos"),
        duration: mpv.get_double_property("duration"),
        volume,
    }
}
```

- [ ] **Step 6: Wire `lib.rs` for the render feature**

In `apps/desktop/src-tauri/src/lib.rs`, add render imports under `#[cfg(feature = "mpv-render")]`:

```rust
#[cfg(feature = "mpv-render")]
use mpv_render::{
    MpvRenderState, mpv_render_open_path, mpv_render_pause, mpv_render_play, mpv_render_seek,
    mpv_render_set_volume, mpv_render_snapshot,
};
```

Change the `#[cfg(feature = "mpv-embed")] pub fn run()` block to a render block for the default feature. Keep the old embed run block behind `#[cfg(all(feature = "mpv-embed", not(feature = "mpv-render")))]`.

```rust
#[cfg(feature = "mpv-render")]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(MpvRenderState::default())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let state = app.state::<MpvRenderState>();
                let hwnd = window_hwnd(&window)?;
                state.start(hwnd as isize)?;
                let resize_state = state.clone();
                window.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::Resized(_) | tauri::WindowEvent::ScaleFactorChanged { .. }) {
                        let _ = resize_state.resize();
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            window_minimize,
            window_toggle_maximize,
            window_close,
            mpv_render_open_path,
            mpv_render_play,
            mpv_render_pause,
            mpv_render_seek,
            mpv_render_set_volume,
            mpv_render_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("failed to run OpenPlayer desktop app");
}
```

Add `window_hwnd` helper under the render feature if no shared helper exists:

```rust
#[cfg(feature = "mpv-render")]
fn window_hwnd(window: &impl raw_window_handle::HasWindowHandle) -> Result<i64, String> {
    let handle = window.window_handle().map_err(|error| format!("failed to read Tauri window handle: {error}"))?;
    match handle.as_raw() {
        raw_window_handle::RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as i64),
        _ => Err("mpv render backend is only wired for Windows HWND targets".to_string()),
    }
}
```

- [ ] **Step 7: Run backend verification**

Run:

```powershell
cargo fmt --all -- --check
cargo test -p openplayer-desktop --features mpv-render
npm run verify:shell
```

Expected: Rust tests PASS. `npm run verify:shell` may still FAIL until frontend command names and Tauri config are updated in later tasks.

- [ ] **Step 8: Commit checkpoint if authorized**

Run only if commits are authorized:

```powershell
git add apps/desktop/src-tauri/src/mpv_render.rs apps/desktop/src-tauri/src/mpv_render/sys.rs apps/desktop/src-tauri/src/lib.rs
git commit -m "feat: wire mpv render actor commands"
```

---

### Task 7: Restore Transparent Custom-Control Window Configuration

**Files:**
- Modify: `apps/desktop/src-tauri/tauri.conf.json`

- [ ] **Step 1: Update Tauri window config**

Set the main window to custom chrome with transparent composition:

```json
{
  "title": "OpenPlayer",
  "url": "index.html",
  "width": 1280,
  "height": 720,
  "minWidth": 960,
  "minHeight": 540,
  "resizable": true,
  "center": true,
  "decorations": false,
  "transparent": true,
  "shadow": true
}
```

- [ ] **Step 2: Run static guard**

Run:

```powershell
npm run verify:shell
```

Expected: config assertions PASS. Frontend assertions may still FAIL until Task 8.

- [ ] **Step 3: Commit checkpoint if authorized**

Run only if commits are authorized:

```powershell
git add apps/desktop/src-tauri/tauri.conf.json
git commit -m "chore: enable transparent custom chrome for render overlay"
```

---

### Task 8: Rewire React To Render Commands And Keep Custom Controls

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/styles.css`

- [ ] **Step 1: Update command names and snapshot type**

In `App.tsx`, rename `MpvSnapshot` to `MpvRenderSnapshot` or keep the current type name and route every command to `mpv_render_*`:

```ts
type MpvSnapshot = {
  path: string;
  status: string;
  paused: boolean;
  position: number;
  duration: number;
  volume: number;
};

function openMpvPath(path: string) {
  return invoke<MpvSnapshot>("mpv_render_open_path", { path }).then((snapshot) => {
    setPlaybackError(null);
    applySnapshot(snapshot);
  });
}
```

Replace command calls:

```ts
invoke<MpvSnapshot | null>("mpv_render_snapshot")
invoke<MpvSnapshot>(isPlaying ? "mpv_render_pause" : "mpv_render_play")
invoke<MpvSnapshot>("mpv_render_seek", { position: value })
invoke<MpvSnapshot>("mpv_render_set_volume", { volume: nextVolume * 100 })
```

- [ ] **Step 2: Keep custom titlebar and drag controls**

Keep `getCurrentWindow`, `startDragging`, `window_minimize`, `window_toggle_maximize`, and `window_close` because the WebView overlay should now be above the render surface. The earlier guard that rejected these must be removed in Task 1's final version.

Ensure these controls remain in the returned JSX:

```tsx
<div className="window-controls" aria-label="Window controls">
  <button type="button" aria-label="Minimize window" onClick={() => runWindowCommand("window_minimize")}>
    <Icon name="minimize" />
  </button>
  <button type="button" aria-label="Maximize or restore window" onClick={() => runWindowCommand("window_toggle_maximize")}>
    <Icon name="maximize" />
  </button>
  <button className="window-control-close" type="button" aria-label="Close window" onClick={() => runWindowCommand("window_close")}>
    <Icon name="close" />
  </button>
</div>
```

- [ ] **Step 3: Remove permanent black-band layout assumptions**

In `styles.css`, keep controls absolutely positioned over the stage and make the shell transparent where video should show through:

```css
html,
body,
#root {
  width: 100%;
  height: 100%;
  margin: 0;
  background: transparent;
}

.app-shell,
.window-shell,
.stage {
  background: transparent;
}

.stage {
  position: relative;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
}

.transport {
  position: absolute;
  left: 24px;
  right: 24px;
  bottom: 24px;
  z-index: 30;
  pointer-events: auto;
}

.window-controls {
  position: absolute;
  top: 10px;
  right: 10px;
  z-index: 40;
  pointer-events: auto;
}

.drag-surface {
  position: absolute;
  inset: 0;
  z-index: 5;
  pointer-events: auto;
}
```

The exact visual styling can remain the existing OpenPlayer styling. The required behavior is no permanent top or bottom reservation for the video.

- [ ] **Step 4: Run frontend verification**

Run:

```powershell
npm run verify:shell
npm run build
```

Expected: both PASS after static guards are aligned with render commands and custom controls.

- [ ] **Step 5: Commit checkpoint if authorized**

Run only if commits are authorized:

```powershell
git add apps/desktop/src/App.tsx apps/desktop/src/styles.css apps/desktop/scripts/verify-shell.mjs
git commit -m "feat: route custom controls to mpv render backend"
```

---

### Task 9: Retire The Child HWND Default Path

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/mpv_embed.rs` or leave unchanged behind non-default feature
- Modify: `apps/desktop/scripts/verify-shell.mjs`

- [ ] **Step 1: Ensure child HWND is not in default runtime**

Keep `mpv_embed.rs` only behind this condition:

```rust
#[cfg(all(feature = "mpv-embed", not(feature = "mpv-render")))]
mod mpv_embed;
```

Ensure the old embed command imports are also behind the same condition:

```rust
#[cfg(all(feature = "mpv-embed", not(feature = "mpv-render")))]
use mpv_embed::{
    MpvEmbedState, mpv_embed_open_path, mpv_embed_pause, mpv_embed_play, mpv_embed_seek,
    mpv_embed_set_volume, mpv_embed_snapshot, mpv_embed_stop,
};
```

- [ ] **Step 2: Keep the static guard focused on default runtime**

In `verify-shell.mjs`, reject embed command registration from `lib.rs` but do not fail solely because `mpv_embed.rs` exists as a non-default spike file:

```js
assert.doesNotMatch(tauriLibSource, /mpv_embed_open_path|mpv_embed_play|mpv_embed_pause|mpv_embed_seek|mpv_embed_set_volume/, "default runtime must not register child HWND mpv embed commands");
```

- [ ] **Step 3: Run default and spike checks**

Run:

```powershell
cargo check -p openplayer-desktop --features mpv-render
cargo check -p openplayer-desktop --no-default-features --features mpv-embed
npm run verify:shell
```

Expected: all PASS. The non-default embed spike may compile, but the default app uses render API.

- [ ] **Step 4: Commit checkpoint if authorized**

Run only if commits are authorized:

```powershell
git add apps/desktop/src-tauri/src/lib.rs apps/desktop/scripts/verify-shell.mjs
git commit -m "refactor: retire child hwnd from default mpv path"
```

---

### Task 10: Runtime Verification Without Auto-Playing Abbott

**Files:**
- No source files unless verification reveals a bug

- [ ] **Step 1: Stop stale development processes**

Run:

```powershell
$connections = Get-NetTCPConnection -LocalPort 23142 -ErrorAction SilentlyContinue
$ids = $connections | Select-Object -ExpandProperty OwningProcess -Unique
foreach ($id in $ids) { if (Get-Process -Id $id -ErrorAction SilentlyContinue) { Stop-Process -Id $id -Force } }
```

Expected: no process remains listening on port `23142`.

- [ ] **Step 2: Run full static/build verification**

Run from `E:\Project\CodeProject\RustPlayer\apps\desktop`:

```powershell
npm run verify:shell
npm run build
```

Run from `E:\Project\CodeProject\RustPlayer`:

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo test -p openplayer-desktop --features mpv-render
```

Expected: all PASS.

- [ ] **Step 3: Launch without `OPENPLAYER_MPV_EMBED_FILE`**

Run from `E:\Project\CodeProject\RustPlayer\apps\desktop`:

```powershell
$env:PATH = "E:\Project\CodeProject\RustPlayer\vendor\native\mpv\windows-x64;$env:PATH"
Remove-Item Env:\OPENPLAYER_MPV_EMBED_FILE -ErrorAction SilentlyContinue
npm run tauri:dev
```

Expected: app opens without auto-playing the Abbott sample.

- [ ] **Step 4: Manual runtime checks**

Use the app UI:

```text
1. Click Open media.
2. Select a local test file manually.
3. Confirm mpv video fills the player surface.
4. Confirm custom controls are visible above video.
5. Click play/pause, seek, volume, playlist, minimize, maximize, and close.
6. Resize the window quickly and confirm video follows immediately.
7. Confirm there is no permanent bottom black band reserved for controls.
```

Expected: custom controls remain clickable and video stays behind them.

- [ ] **Step 5: If overlay fails, document exact failure before fallback**

If the video renders but WebView transparency or z-order prevents clickable controls, create `docs/superpowers/specs/2026-05-18-openplayer-render-api-overlay-failure.md` with:

```markdown
# OpenPlayer Render API Overlay Failure Notes

Date: 2026-05-18

## Observed Failure

During the render API runtime check, video rendered into the OpenPlayer-owned surface, but the WebView overlay did not receive pointer input above the video surface. The custom transport and window controls were visible only intermittently or were not clickable after playback started.

## Verified Architecture

- mpv render API was used without `wid`.
- Video rendered into an OpenPlayer-owned surface.
- WebView controls were expected above the video surface.

## Decision

The single-window transparent WebView overlay is not reliable on this Windows/WebView2 setup. The next fallback is a separate transparent controls window that follows the video window.
```

Replace the bracketed sentence with the observed behavior before saving. Do not choose the fallback without this note.

- [ ] **Step 6: Commit checkpoint if authorized**

Run only if commits are authorized and runtime verification passes:

```powershell
git status --short
git diff --check
git add apps/desktop docs/superpowers
git commit -m "feat: spike mpv render api custom controls"
```

---

## Final Verification Checklist

- [ ] `npm run verify:shell` passes.
- [ ] `npm run build` passes.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo test --workspace` passes.
- [ ] `cargo test -p openplayer-desktop --features mpv-render` passes.
- [ ] `npm run tauri:dev` starts without auto-playing Abbott when `OPENPLAYER_MPV_EMBED_FILE` is unset.
- [ ] A manually selected local video plays through mpv render API.
- [ ] Custom controls overlay the video and receive clicks.
- [ ] Resize is event-driven and visually immediate.
- [ ] Default runtime does not register `mpv_embed_*` child HWND commands.
