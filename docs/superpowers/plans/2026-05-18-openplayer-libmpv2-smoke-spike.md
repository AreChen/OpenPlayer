# OpenPlayer libmpv2 Smoke Spike Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a feature-gated Rust smoke path proving `libmpv2` can initialize local Windows `libmpv` artifacts without changing the current HTML video player.

**Architecture:** The current React UI and HTML `<video>` renderer remain the default player. The Tauri Rust crate gains an optional `mpv-smoke` feature that links against local ignored Windows `libmpv` artifacts and runs a headless `vo=null`/`ao=null` initialization probe. Documentation records how to run the probe and what it does not prove.

**Tech Stack:** Tauri v2, Rust 2024, `libmpv2` 6.0.0, local Windows `libmpv-2.dll` and `libmpv.dll.a`, Node shell verification.

---

## Scope Check

This plan implements one narrow spike: compile/link/runtime initialization of `libmpv2` from the desktop Rust crate. It does not implement native file picking, `loadfile` playback, mpv window embedding, OpenGL rendering, subtitle APIs, playlist persistence, or replacement of the current HTML `<video>` UI.

## File Structure

- Modify: `apps/desktop/scripts/verify-shell.mjs` adds architecture guards for the feature-gated spike and confirms the frontend still uses HTML video.
- Modify: `apps/desktop/src-tauri/Cargo.toml` adds an optional `mpv-smoke` feature and optional `libmpv2` dependency.
- Modify: `apps/desktop/src-tauri/build.rs` adds Windows `libmpv` link-search output only when `mpv-smoke` is enabled.
- Modify: `apps/desktop/src-tauri/src/lib.rs` includes the feature-gated smoke module without registering UI commands.
- Create: `apps/desktop/src-tauri/src/mpv_smoke.rs` owns the smoke probe and its feature-gated unit test.
- Create: `docs/architecture/libmpv2-smoke.md` documents what was proven, how to run it, and remaining risks.

## Task 1: Add Feature Wiring Guards

**Files:**
- Modify: `apps/desktop/scripts/verify-shell.mjs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/build.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Create: `apps/desktop/src-tauri/src/mpv_smoke.rs`

- [ ] **Step 1: Write failing shell assertions for the spike boundary**

In `apps/desktop/scripts/verify-shell.mjs`, add these reads after the existing `workspaceToml` read:

```js
const tauriCargoToml = await readFile(new URL("../src-tauri/Cargo.toml", import.meta.url), "utf8");
const tauriBuildScript = await readFile(new URL("../src-tauri/build.rs", import.meta.url), "utf8");
```

Add these assertions after the existing dependency assertions for removed Movi/native dialog dependencies:

```js
assert.match(tauriCargoToml, /\[features\][\s\S]*mpv-smoke = \["dep:libmpv2"\]/, "libmpv2 spike must be hidden behind the mpv-smoke feature");
assert.match(tauriCargoToml, /libmpv2 = \{ version = "6\.0\.0", optional = true, default-features = false \}/, "libmpv2 must be optional and control-only for the first smoke spike");
assert.match(tauriBuildScript, /CARGO_FEATURE_MPV_SMOKE/, "build script must only add mpv link paths when mpv-smoke is enabled");
assert.match(tauriBuildScript, /vendor[\\/]native[\\/]mpv[\\/]windows-x64/, "build script must point at the vendored Windows mpv directory");
assert.match(tauriLibSource, /#\[cfg\(feature = "mpv-smoke"\)\]\s*mod mpv_smoke;/, "desktop crate must keep libmpv2 smoke code feature-gated");
assert.doesNotMatch(appSource, /mpvSmoke|libmpv|libmpv2|mpv_smoke/, "libmpv2 smoke spike must not change the HTML video frontend path");
```

- [ ] **Step 2: Run shell verification and verify RED**

Run:

```powershell
npm run verify:shell
```

Working directory: `apps/desktop`

Expected: FAIL with the message `libmpv2 spike must be hidden behind the mpv-smoke feature` because the feature, dependency, build script hook, and module are not wired yet.

- [ ] **Step 3: Add the optional feature and dependency**

In `apps/desktop/src-tauri/Cargo.toml`, replace the dependency section with:

```toml
[features]
default = []
mpv-smoke = ["dep:libmpv2"]

[dependencies]
tauri = { version = "2", features = [] }
libmpv2 = { version = "6.0.0", optional = true, default-features = false }
```

Leave the existing build dependency unchanged:

```toml
[build-dependencies]
tauri-build = { version = "2", features = [] }
```

- [ ] **Step 4: Add feature-gated mpv link-search output**

In `apps/desktop/src-tauri/build.rs`, add this helper above `fn main()`:

```rust
fn configure_mpv_smoke_linking() {
    if std::env::var_os("CARGO_FEATURE_MPV_SMOKE").is_none() {
        return;
    }

    println!("cargo:rerun-if-env-changed=OPENPLAYER_MPV_DIR");

    #[cfg(windows)]
    {
        let mpv_dir = std::env::var_os("OPENPLAYER_MPV_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                let manifest_dir = std::path::PathBuf::from(
                    std::env::var("CARGO_MANIFEST_DIR")
                        .expect("CARGO_MANIFEST_DIR is set by Cargo"),
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
                "mpv-smoke requires local ignored mpv artifacts at {} or OPENPLAYER_MPV_DIR; missing {}",
                mpv_dir.display(),
                missing
            );
        }

        println!("cargo:rustc-link-search=native={}", mpv_dir.display());
        println!("cargo:rerun-if-changed={}", import_library.display());
        println!("cargo:rerun-if-changed={}", runtime_library.display());
    }
}
```

Then call it as the first statement inside `fn main()`:

```rust
fn main() {
    configure_mpv_smoke_linking();

    #[cfg(windows)]
    {
        let icon_path = std::path::PathBuf::from("icons").join("icon.ico");
        let windows = tauri_build::WindowsAttributes::new().window_icon_path(icon_path);
        let attributes = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attributes).expect("failed to run Tauri build script");
    }

    #[cfg(not(windows))]
    tauri_build::build();
}
```

- [ ] **Step 5: Add the feature-gated module shell**

In `apps/desktop/src-tauri/src/lib.rs`, add this after the `use tauri::Window;` line:

```rust
#[cfg(feature = "mpv-smoke")]
mod mpv_smoke;
```

Create `apps/desktop/src-tauri/src/mpv_smoke.rs` with this temporary content:

```rust
pub fn create_headless_probe() -> Result<MpvSmokeReport, String> {
    Ok(MpvSmokeReport {
        video_output: "null".to_string(),
        audio_output: "null".to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpvSmokeReport {
    pub video_output: String,
    pub audio_output: String,
}
```

- [ ] **Step 6: Run shell verification and verify GREEN**

Run:

```powershell
npm run verify:shell
```

Working directory: `apps/desktop`

Expected: PASS. This proves the architecture guard can see the optional feature wiring and the frontend still has no mpv path.

## Task 2: Add libmpv2 Runtime Initialization Probe

**Files:**
- Modify: `apps/desktop/src-tauri/src/mpv_smoke.rs`
- Modify: `Cargo.lock`

- [ ] **Step 1: Replace the temporary module with a failing libmpv2 test**

Replace `apps/desktop/src-tauri/src/mpv_smoke.rs` with:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpvSmokeReport {
    pub video_output: String,
    pub audio_output: String,
}

pub fn create_headless_probe() -> Result<MpvSmokeReport, String> {
    Err("libmpv2 smoke probe is not wired yet".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_libmpv_with_null_outputs() {
        let report = create_headless_probe().expect("libmpv should initialize with null outputs");

        assert_eq!(report.video_output, "null");
        assert_eq!(report.audio_output, "null");
    }
}
```

- [ ] **Step 2: Run the feature-gated smoke test and verify RED**

Run:

```powershell
$env:PATH = "E:\Project\CodeProject\RustPlayer\vendor\native\mpv\windows-x64;$env:PATH"; cargo test -p openplayer-desktop --features mpv-smoke mpv_smoke -- --nocapture
```

Working directory: repository root.

Expected: FAIL with `libmpv2 smoke probe is not wired yet` from `initializes_libmpv_with_null_outputs`.

- [ ] **Step 3: Implement the minimal libmpv2 initialization**

Replace `create_headless_probe` in `apps/desktop/src-tauri/src/mpv_smoke.rs` with:

```rust
pub fn create_headless_probe() -> Result<MpvSmokeReport, String> {
    let _mpv = libmpv2::Mpv::with_initializer(|initializer| {
        initializer.set_property("vo", "null")?;
        initializer.set_property("ao", "null")?;
        Ok(())
    })
    .map_err(|error| error.to_string())?;

    Ok(MpvSmokeReport {
        video_output: "null".to_string(),
        audio_output: "null".to_string(),
    })
}
```

- [ ] **Step 4: Run the feature-gated smoke test and verify GREEN**

Run:

```powershell
$env:PATH = "E:\Project\CodeProject\RustPlayer\vendor\native\mpv\windows-x64;$env:PATH"; cargo test -p openplayer-desktop --features mpv-smoke mpv_smoke -- --nocapture
```

Working directory: repository root.

Expected: PASS with `initializes_libmpv_with_null_outputs ... ok`.

- [ ] **Step 5: Verify the default workspace still avoids libmpv2 runtime requirements**

Run:

```powershell
cargo test --workspace
```

Working directory: repository root.

Expected: PASS without needing `libmpv-2.dll` on `PATH`, because `mpv-smoke` is disabled by default.

## Task 3: Document The Spike Result And Run Full Verification

**Files:**
- Create: `docs/architecture/libmpv2-smoke.md`
- Modify: `docs/architecture/README.md`

- [ ] **Step 1: Add the architecture note**

Create `docs/architecture/libmpv2-smoke.md` with:

```markdown
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
```

- [ ] **Step 2: Link the note from the architecture README**

Append this to `docs/architecture/README.md`:

```markdown

## Native Media Spikes

- [libmpv2 smoke spike](./libmpv2-smoke.md) documents the feature-gated Rust initialization probe for local `libmpv` artifacts.
```

- [ ] **Step 3: Run shell verification**

Run:

```powershell
npm run verify:shell
```

Working directory: `apps/desktop`

Expected: PASS.

- [ ] **Step 4: Run frontend build**

Run:

```powershell
npm run build
```

Working directory: `apps/desktop`

Expected: PASS.

- [ ] **Step 5: Run default Rust tests**

Run:

```powershell
cargo test --workspace
```

Working directory: repository root.

Expected: PASS.

- [ ] **Step 6: Run feature-gated libmpv2 smoke test**

Run:

```powershell
$env:PATH = "E:\Project\CodeProject\RustPlayer\vendor\native\mpv\windows-x64;$env:PATH"; cargo test -p openplayer-desktop --features mpv-smoke mpv_smoke -- --nocapture
```

Working directory: repository root.

Expected: PASS.

- [ ] **Step 7: Inspect final diff without committing**

Run:

```powershell
git status --short
git diff -- apps/desktop/scripts/verify-shell.mjs apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/build.rs apps/desktop/src-tauri/src/lib.rs apps/desktop/src-tauri/src/mpv_smoke.rs docs/architecture/README.md docs/architecture/libmpv2-smoke.md Cargo.lock
```

Working directory: repository root.

Expected: Only the files listed in this plan are modified or created. Do not commit unless the user explicitly asks for a commit.

## Plan Self-Review

- Spec coverage: The plan proves compile/link/runtime initialization only, keeps HTML video as default, and documents remaining rendering risks.
- Completeness scan: Each task lists exact files, code, commands, and expected outcomes.
- Type consistency: `MpvSmokeReport` and `create_headless_probe` are defined before use and used consistently in tests and implementation.
