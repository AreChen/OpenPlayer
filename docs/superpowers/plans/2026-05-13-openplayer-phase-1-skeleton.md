# OpenPlayer Phase 1 Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current RustPlayer animation demo with a clean OpenPlayer repository skeleton that builds as a Tauri v2 + React/Vite desktop app backed by a Rust workspace.

**Architecture:** This plan establishes the workspace boundaries from the approved spec without implementing real media playback. The desktop UI talks to Rust through a small Tauri health command, proving the React -> Tauri -> Rust core path before adding SQLite, libmpv, and playback services in separate plans.

**Tech Stack:** Rust 2024 workspace, Tauri v2, React, Vite, TypeScript, GitHub Actions, MIT license.

---

## Scope Check

The approved spec covers repository governance, UI, media playback, storage, plugins, themes, native dependency packaging, and release hardening. This plan implements Phase 1 only: skeleton and governance. It deliberately creates compileable boundaries for later phases instead of building SQLite services, libmpv playback, plugin loading, or theme switching in this phase.

## File Structure

- Create: `.gitignore` tracks source while ignoring build output, local media, `.superpowers/`, and native binary downloads.
- Create: `LICENSE` contains the MIT license.
- Replace: `README.md` describes OpenPlayer instead of the RGB565 animation demo.
- Replace: `Cargo.toml` becomes the workspace manifest.
- Delete: `src/main.rs`, `src/lib.rs`, `src/bin/player.rs`, `src/bin/pack_anim.rs` remove the old demo.
- Create: `crates/shared/` contains serializable DTOs shared across Rust services and Tauri IPC.
- Create: `crates/core/` contains initial app-level domain/service entry points.
- Create: `crates/media/` contains the first backend-neutral media trait boundary.
- Create: `crates/mpv/` contains the first mpv backend descriptor crate without linking native mpv in this phase.
- Create: `crates/storage/` contains the storage crate shell and typed storage error.
- Create: `crates/plugin/` contains the plugin manifest model and validation.
- Create: `crates/theme/` contains theme manifest and token models.
- Create: `apps/desktop/src-tauri/` contains the Tauri Rust shell.
- Create: `apps/desktop/` contains the React/Vite frontend.
- Create: `.github/workflows/ci.yml` runs Rust and frontend checks.
- Create: `docs/architecture/`, `docs/plugins/`, `docs/themes/`, and `docs/native-deps/` preserve design context for future work.

## Task 1: Initialize Repository Hygiene And Replace Demo Docs

**Files:**
- Create: `.gitignore`
- Create: `LICENSE`
- Replace: `README.md`
- Delete: `src/main.rs`
- Delete: `src/lib.rs`
- Delete: `src/bin/player.rs`
- Delete: `src/bin/pack_anim.rs`

- [ ] **Step 1: Initialize git on the main branch**

Run:

```powershell
git init -b main
```

Expected: command succeeds and creates `.git/`.

- [ ] **Step 2: Delete the old demo source files**

Use `apply_patch`:

```diff
*** Begin Patch
*** Delete File: src/main.rs
*** Delete File: src/lib.rs
*** Delete File: src/bin/player.rs
*** Delete File: src/bin/pack_anim.rs
*** End Patch
```

- [ ] **Step 3: Add `.gitignore`**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: .gitignore
+# Rust
+/target/
+**/*.rs.bk
+
+# Node / frontend
+/apps/desktop/node_modules/
+/apps/desktop/dist/
+/apps/desktop/.vite/
+
+# Tauri generated output
+/apps/desktop/src-tauri/target/
+/apps/desktop/src-tauri/gen/
+/apps/desktop/src-tauri/target-*/
+
+# Local media and generated assets
+/ANI/
+/media/
+/local-media/
+/fixtures/local/
+*.anim
+
+# Native dependency downloads and bundles
+/vendor/native/
+/dist/
+/bundle/
+*.dll
+*.dylib
+*.so
+*.so.*
+
+# Logs and local environment
+*.log
+.env
+.env.*
+!.env.example
+
+# Editor / OS
+.DS_Store
+Thumbs.db
+.idea/
+.vscode/
+
+# Superpowers brainstorming artifacts
+/.superpowers/
*** End Patch
```

- [ ] **Step 4: Add MIT license**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: LICENSE
+MIT License
+
+Copyright (c) 2026 OpenPlayer contributors
+
+Permission is hereby granted, free of charge, to any person obtaining a copy
+of this software and associated documentation files (the "Software"), to deal
+in the Software without restriction, including without limitation the rights
+to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
+copies of the Software, and to permit persons to whom the Software is
+furnished to do so, subject to the following conditions:
+
+The above copyright notice and this permission notice shall be included in all
+copies or substantial portions of the Software.
+
+THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
+IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
+FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
+AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
+LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
+OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
+SOFTWARE.
*** End Patch
```

- [ ] **Step 5: Replace `README.md`**

Use `apply_patch`:

```diff
*** Begin Patch
*** Delete File: README.md
*** Add File: README.md
+# OpenPlayer
+
+OpenPlayer is a cross-platform, high-performance desktop media player built with Rust, Tauri v2, and React.
+
+The project is in its first architecture milestone. Phase 1 establishes the repository, workspace, desktop shell, and core module boundaries. Media playback, SQLite persistence, application plugins, custom themes, and bundled libmpv support are designed in `docs/superpowers/specs/2026-05-13-openplayer-design.md` and will be implemented in follow-up phases.
+
+## Goals
+
+- Cross-platform desktop player foundation.
+- Tauri v2 shell with a polished Studio Dark React UI.
+- Rust workspace with focused crates for core services, media backends, storage, plugins, and themes.
+- Future `libmpv` backend for broad media format, subtitle, and hardware decode support.
+- MIT licensed public GitHub project.
+
+## Development
+
+This Task 1 checkpoint contains repository metadata and planning documents only. The buildable Rust workspace begins in Phase 1 Task 2, and desktop/frontend commands will be documented once `apps/desktop` is added in later Phase 1 tasks.
+
+## License
+
+OpenPlayer is licensed under the MIT license. See `LICENSE`.
*** End Patch
```

- [ ] **Step 6: Verify ignored files are not tracked**

Run:

```powershell
git status --short --ignored
```

Expected: `target/`, `ANI/`, and `.superpowers/` appear as ignored entries if they exist.

- [ ] **Step 7: Commit repository hygiene changes**

Run:

```powershell
git add .gitignore LICENSE README.md docs/superpowers/specs/2026-05-13-openplayer-design.md
git add -u src
git commit -m "chore: initialize OpenPlayer repository hygiene"
```

Expected: commit succeeds with deleted old demo files and new repository metadata.

## Task 2: Create Rust Workspace And Crate Boundaries

**Files:**
- Replace: `Cargo.toml`
- Create: `crates/shared/Cargo.toml`
- Create: `crates/shared/src/lib.rs`
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `crates/media/Cargo.toml`
- Create: `crates/media/src/lib.rs`
- Create: `crates/mpv/Cargo.toml`
- Create: `crates/mpv/src/lib.rs`
- Create: `crates/storage/Cargo.toml`
- Create: `crates/storage/src/lib.rs`
- Create: `crates/plugin/Cargo.toml`
- Create: `crates/plugin/src/lib.rs`
- Create: `crates/theme/Cargo.toml`
- Create: `crates/theme/src/lib.rs`

- [ ] **Step 1: Replace root workspace manifest**

Use `apply_patch`:

```diff
*** Begin Patch
*** Delete File: Cargo.toml
*** Add File: Cargo.toml
+[workspace]
+members = [
+    "crates/core",
+    "crates/media",
+    "crates/mpv",
+    "crates/plugin",
+    "crates/shared",
+    "crates/storage",
+    "crates/theme",
+]
+resolver = "2"
+
+[workspace.package]
+version = "0.1.0"
+edition = "2024"
+license = "MIT"
+authors = ["OpenPlayer contributors"]
+
+[workspace.dependencies]
+async-trait = "0.1"
+serde = { version = "1", features = ["derive"] }
+serde_json = "1"
+thiserror = "2"
*** End Patch
```

- [ ] **Step 2: Add `openplayer-shared` crate**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: crates/shared/Cargo.toml
+[package]
+name = "openplayer-shared"
+version.workspace = true
+edition.workspace = true
+license.workspace = true
+authors.workspace = true
+
+[dependencies]
+serde.workspace = true
*** Add File: crates/shared/src/lib.rs
+use serde::{Deserialize, Serialize};
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub struct AppInfo {
+    pub name: String,
+    pub version: String,
+    pub stage: AppStage,
+}
+
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub enum AppStage {
+    Skeleton,
+}
+
+impl AppInfo {
+    pub fn skeleton(version: impl Into<String>) -> Self {
+        Self {
+            name: "OpenPlayer".to_string(),
+            version: version.into(),
+            stage: AppStage::Skeleton,
+        }
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn serializes_app_info_for_tauri_ipc() {
+        let info = AppInfo::skeleton("0.1.0");
+        let json = serde_json::to_value(info).expect("app info serializes");
+
+        assert_eq!(json["name"], "OpenPlayer");
+        assert_eq!(json["version"], "0.1.0");
+        assert_eq!(json["stage"], "skeleton");
+    }
+}
*** End Patch
```

- [ ] **Step 3: Add `serde_json` as a dev dependency for shared tests**

Use `apply_patch`:

```diff
*** Begin Patch
*** Update File: crates/shared/Cargo.toml
@@
 [dependencies]
 serde.workspace = true
+
+[dev-dependencies]
+serde_json.workspace = true
*** End Patch
```

- [ ] **Step 4: Add `openplayer-core` crate**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: crates/core/Cargo.toml
+[package]
+name = "openplayer-core"
+version.workspace = true
+edition.workspace = true
+license.workspace = true
+authors.workspace = true
+
+[dependencies]
+openplayer-shared = { path = "../shared" }
*** Add File: crates/core/src/lib.rs
+use openplayer_shared::AppInfo;
+
+pub fn app_info() -> AppInfo {
+    AppInfo::skeleton(env!("CARGO_PKG_VERSION"))
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use openplayer_shared::AppStage;
+
+    #[test]
+    fn reports_openplayer_skeleton_info() {
+        let info = app_info();
+
+        assert_eq!(info.name, "OpenPlayer");
+        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
+        assert_eq!(info.stage, AppStage::Skeleton);
+    }
+}
*** End Patch
```

- [ ] **Step 5: Add `openplayer-media` crate**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: crates/media/Cargo.toml
+[package]
+name = "openplayer-media"
+version.workspace = true
+edition.workspace = true
+license.workspace = true
+authors.workspace = true
+
+[dependencies]
+thiserror.workspace = true
*** Add File: crates/media/src/lib.rs
+use thiserror::Error;
+
+pub trait MediaBackend: Send + Sync {
+    fn backend_id(&self) -> &'static str;
+    fn display_name(&self) -> &'static str;
+}
+
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct MediaBackendInfo {
+    pub backend_id: String,
+    pub display_name: String,
+}
+
+impl MediaBackendInfo {
+    pub fn from_backend(backend: &dyn MediaBackend) -> Self {
+        Self {
+            backend_id: backend.backend_id().to_string(),
+            display_name: backend.display_name().to_string(),
+        }
+    }
+}
+
+#[derive(Debug, Error, PartialEq, Eq)]
+pub enum MediaError {
+    #[error("media backend is unavailable: {0}")]
+    BackendUnavailable(String),
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    struct TestBackend;
+
+    impl MediaBackend for TestBackend {
+        fn backend_id(&self) -> &'static str {
+            "test"
+        }
+
+        fn display_name(&self) -> &'static str {
+            "Test Backend"
+        }
+    }
+
+    #[test]
+    fn backend_info_is_derived_from_trait() {
+        let info = MediaBackendInfo::from_backend(&TestBackend);
+
+        assert_eq!(info.backend_id, "test");
+        assert_eq!(info.display_name, "Test Backend");
+    }
+}
*** End Patch
```

- [ ] **Step 6: Add `openplayer-mpv` crate**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: crates/mpv/Cargo.toml
+[package]
+name = "openplayer-mpv"
+version.workspace = true
+edition.workspace = true
+license.workspace = true
+authors.workspace = true
+
+[dependencies]
+openplayer-media = { path = "../media" }
*** Add File: crates/mpv/src/lib.rs
+use openplayer_media::MediaBackend;
+
+#[derive(Debug, Default, Clone, Copy)]
+pub struct MpvBackendDescriptor;
+
+impl MediaBackend for MpvBackendDescriptor {
+    fn backend_id(&self) -> &'static str {
+        "mpv"
+    }
+
+    fn display_name(&self) -> &'static str {
+        "libmpv"
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use openplayer_media::MediaBackendInfo;
+
+    #[test]
+    fn exposes_mpv_backend_identity() {
+        let descriptor = MpvBackendDescriptor;
+        let info = MediaBackendInfo::from_backend(&descriptor);
+
+        assert_eq!(info.backend_id, "mpv");
+        assert_eq!(info.display_name, "libmpv");
+    }
+}
*** End Patch
```

- [ ] **Step 7: Add `openplayer-storage` crate**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: crates/storage/Cargo.toml
+[package]
+name = "openplayer-storage"
+version.workspace = true
+edition.workspace = true
+license.workspace = true
+authors.workspace = true
+
+[dependencies]
+thiserror.workspace = true
*** Add File: crates/storage/src/lib.rs
+use thiserror::Error;
+
+#[derive(Debug, Error, PartialEq, Eq)]
+pub enum StorageError {
+    #[error("storage is not configured")]
+    NotConfigured,
+}
+
+pub fn storage_crate_ready() -> bool {
+    true
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn storage_crate_reports_ready() {
+        assert!(storage_crate_ready());
+    }
+}
*** End Patch
```

- [ ] **Step 8: Add `openplayer-plugin` crate**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: crates/plugin/Cargo.toml
+[package]
+name = "openplayer-plugin"
+version.workspace = true
+edition.workspace = true
+license.workspace = true
+authors.workspace = true
+
+[dependencies]
+serde.workspace = true
+thiserror.workspace = true
*** Add File: crates/plugin/src/lib.rs
+use serde::{Deserialize, Serialize};
+use thiserror::Error;
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub struct PluginManifest {
+    pub id: String,
+    pub name: String,
+    pub version: String,
+    pub entry: PluginEntry,
+}
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub enum PluginEntry {
+    BuiltIn,
+}
+
+#[derive(Debug, Error, PartialEq, Eq)]
+pub enum PluginManifestError {
+    #[error("plugin id must not be empty")]
+    EmptyId,
+    #[error("plugin name must not be empty")]
+    EmptyName,
+    #[error("plugin version must not be empty")]
+    EmptyVersion,
+}
+
+pub fn validate_plugin_manifest(manifest: &PluginManifest) -> Result<(), PluginManifestError> {
+    if manifest.id.trim().is_empty() {
+        return Err(PluginManifestError::EmptyId);
+    }
+    if manifest.name.trim().is_empty() {
+        return Err(PluginManifestError::EmptyName);
+    }
+    if manifest.version.trim().is_empty() {
+        return Err(PluginManifestError::EmptyVersion);
+    }
+
+    Ok(())
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn accepts_valid_builtin_plugin_manifest() {
+        let manifest = PluginManifest {
+            id: "openplayer.core".to_string(),
+            name: "OpenPlayer Core".to_string(),
+            version: "0.1.0".to_string(),
+            entry: PluginEntry::BuiltIn,
+        };
+
+        assert_eq!(validate_plugin_manifest(&manifest), Ok(()));
+    }
+
+    #[test]
+    fn rejects_empty_plugin_id() {
+        let manifest = PluginManifest {
+            id: " ".to_string(),
+            name: "OpenPlayer Core".to_string(),
+            version: "0.1.0".to_string(),
+            entry: PluginEntry::BuiltIn,
+        };
+
+        assert_eq!(
+            validate_plugin_manifest(&manifest),
+            Err(PluginManifestError::EmptyId)
+        );
+    }
+}
*** End Patch
```

- [ ] **Step 9: Add `openplayer-theme` crate**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: crates/theme/Cargo.toml
+[package]
+name = "openplayer-theme"
+version.workspace = true
+edition.workspace = true
+license.workspace = true
+authors.workspace = true
+
+[dependencies]
+serde.workspace = true
+thiserror.workspace = true
*** Add File: crates/theme/src/lib.rs
+use serde::{Deserialize, Serialize};
+use thiserror::Error;
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub struct ThemeManifest {
+    pub id: String,
+    pub name: String,
+    pub version: String,
+    pub tokens: ThemeTokens,
+}
+
+#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
+#[serde(rename_all = "camelCase")]
+pub struct ThemeTokens {
+    pub surface: String,
+    pub surface_elevated: String,
+    pub text_primary: String,
+    pub text_muted: String,
+    pub accent: String,
+    pub border: String,
+}
+
+#[derive(Debug, Error, PartialEq, Eq)]
+pub enum ThemeManifestError {
+    #[error("theme id must not be empty")]
+    EmptyId,
+    #[error("theme name must not be empty")]
+    EmptyName,
+}
+
+pub fn studio_dark_manifest() -> ThemeManifest {
+    ThemeManifest {
+        id: "studio-dark".to_string(),
+        name: "Studio Dark".to_string(),
+        version: "0.1.0".to_string(),
+        tokens: ThemeTokens {
+            surface: "#080A0F".to_string(),
+            surface_elevated: "#111722".to_string(),
+            text_primary: "#E9EEF8".to_string(),
+            text_muted: "#AEB9CC".to_string(),
+            accent: "#5B8CFF".to_string(),
+            border: "rgba(255,255,255,0.10)".to_string(),
+        },
+    }
+}
+
+pub fn validate_theme_manifest(manifest: &ThemeManifest) -> Result<(), ThemeManifestError> {
+    if manifest.id.trim().is_empty() {
+        return Err(ThemeManifestError::EmptyId);
+    }
+    if manifest.name.trim().is_empty() {
+        return Err(ThemeManifestError::EmptyName);
+    }
+
+    Ok(())
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn studio_dark_manifest_is_valid() {
+        let manifest = studio_dark_manifest();
+
+        assert_eq!(manifest.id, "studio-dark");
+        assert_eq!(manifest.name, "Studio Dark");
+        assert_eq!(validate_theme_manifest(&manifest), Ok(()));
+    }
+}
*** End Patch
```

- [ ] **Step 10: Run Rust tests for workspace crates**

Run:

```powershell
cargo test --workspace
```

Expected: all crate tests pass.

- [ ] **Step 11: Commit workspace crates**

Run:

```powershell
git add Cargo.toml Cargo.lock crates
git commit -m "feat: add OpenPlayer Rust workspace skeleton"
```

Expected: commit succeeds with the workspace manifest, lockfile, and crate skeletons.

## Task 3: Add Project Documentation Skeletons

**Files:**
- Create: `docs/architecture/README.md`
- Create: `docs/plugins/README.md`
- Create: `docs/themes/README.md`
- Create: `docs/native-deps/README.md`

- [ ] **Step 1: Add architecture docs index**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: docs/architecture/README.md
+# Architecture
+
+OpenPlayer is organized around a Rust workspace and a Tauri desktop shell.
+
+The stable runtime boundary is:
+
+```text
+React UI -> Tauri command -> Rust service -> backend/storage/plugin/theme -> typed event -> React UI
+```
+
+The UI must not call media backends or SQLite directly. Rust services own playback state, persistence, plugin validation, and theme validation.
*** End Patch
```

- [ ] **Step 2: Add plugin docs index**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: docs/plugins/README.md
+# Plugins
+
+OpenPlayer V0 supports application-level plugin concepts only.
+
+Supported extension categories for the first plugin design are:
+
+- Manifest metadata.
+- Command registrations.
+- Menu or command-palette entries.
+- Settings page entries.
+- Metadata extension points.
+- Subtitle-source extension points.
+
+Decoder, filter, renderer, and media-pipeline plugins are outside the V0 plugin boundary.
*** End Patch
```

- [ ] **Step 3: Add theme docs index**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: docs/themes/README.md
+# Themes
+
+OpenPlayer uses Studio Dark as the default theme direction.
+
+The theme system is token-based. Theme manifests can override stable tokens such as surface colors, text colors, accent colors, borders, radius scale, and density. Invalid themes must fail safely and fall back to Studio Dark.
*** End Patch
```

- [ ] **Step 4: Add native dependency docs index**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: docs/native-deps/README.md
+# Native Dependencies
+
+Official OpenPlayer releases will bundle native media dependencies per platform.
+
+The first native dependency target is `libmpv`. Each bundled native dependency must document:
+
+- Dependency name.
+- Upstream source.
+- Version.
+- License.
+- Platform artifact name.
+- Checksum.
+
+Large native binaries are not committed to git. Packaging scripts and metadata are tracked instead.
*** End Patch
```

- [ ] **Step 5: Commit documentation skeletons**

Run:

```powershell
git add docs/architecture docs/plugins docs/themes docs/native-deps
git commit -m "docs: add OpenPlayer architecture indexes"
```

Expected: commit succeeds with documentation indexes.

## Task 4: Add Tauri Rust Desktop Shell

**Files:**
- Modify: `Cargo.toml`
- Create: `apps/desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/src-tauri/build.rs`
- Create: `apps/desktop/src-tauri/tauri.conf.json`
- Create: `apps/desktop/src-tauri/capabilities/default.json`
- Create: `apps/desktop/src-tauri/src/lib.rs`
- Create: `apps/desktop/src-tauri/src/main.rs`

- [ ] **Step 1: Add desktop crate to the workspace**

Use `apply_patch`:

```diff
*** Begin Patch
*** Update File: Cargo.toml
@@
 [workspace]
 members = [
+    "apps/desktop/src-tauri",
     "crates/core",
     "crates/media",
*** End Patch
```

- [ ] **Step 2: Add Tauri crate manifest**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: apps/desktop/src-tauri/Cargo.toml
+[package]
+name = "openplayer-desktop"
+version.workspace = true
+edition.workspace = true
+license.workspace = true
+authors.workspace = true
+
+[lib]
+name = "openplayer_desktop_lib"
+crate-type = ["staticlib", "cdylib", "rlib"]
+
+[dependencies]
+openplayer-core = { path = "../../../crates/core" }
+openplayer-shared = { path = "../../../crates/shared" }
+serde.workspace = true
+tauri = { version = "2", features = [] }
+
+[build-dependencies]
+tauri-build = { version = "2", features = [] }
*** End Patch
```

- [ ] **Step 3: Add Tauri build script**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: apps/desktop/src-tauri/build.rs
+fn main() {
+    tauri_build::build();
+}
*** End Patch
```

- [ ] **Step 4: Add Tauri configuration**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: apps/desktop/src-tauri/tauri.conf.json
+{
+  "$schema": "https://schema.tauri.app/config/2",
+  "productName": "OpenPlayer",
+  "version": "0.1.0",
+  "identifier": "dev.openplayer.app",
+  "build": {
+    "beforeDevCommand": "npm run dev",
+    "devUrl": "http://localhost:1420",
+    "beforeBuildCommand": "npm run build",
+    "frontendDist": "../dist"
+  },
+  "app": {
+    "windows": [
+      {
+        "title": "OpenPlayer",
+        "width": 1280,
+        "height": 720,
+        "minWidth": 960,
+        "minHeight": 540,
+        "resizable": true,
+        "center": true
+      }
+    ],
+    "security": {
+      "csp": null
+    }
+  },
+  "bundle": {
+    "active": false,
+    "targets": "all"
+  }
+}
*** Add File: apps/desktop/src-tauri/capabilities/default.json
+{
+  "$schema": "../gen/schemas/desktop-schema.json",
+  "identifier": "default",
+  "description": "Default permissions for OpenPlayer desktop shell",
+  "windows": ["main"],
+  "permissions": ["core:default"]
+}
*** End Patch
```

- [ ] **Step 5: Add Tauri command library**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: apps/desktop/src-tauri/src/lib.rs
+use openplayer_shared::AppInfo;
+
+#[tauri::command]
+pub fn app_health() -> AppInfo {
+    openplayer_core::app_info()
+}
+
+pub fn run() {
+    tauri::Builder::default()
+        .invoke_handler(tauri::generate_handler![app_health])
+        .run(tauri::generate_context!())
+        .expect("failed to run OpenPlayer desktop app");
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use openplayer_shared::AppStage;
+
+    #[test]
+    fn app_health_reports_core_info() {
+        let info = app_health();
+
+        assert_eq!(info.name, "OpenPlayer");
+        assert_eq!(info.stage, AppStage::Skeleton);
+    }
+}
*** Add File: apps/desktop/src-tauri/src/main.rs
+fn main() {
+    openplayer_desktop_lib::run();
+}
*** End Patch
```

- [ ] **Step 6: Run Rust tests including desktop crate**

Run:

```powershell
cargo test --workspace
```

Expected: all Rust tests pass, including `app_health_reports_core_info`.

- [ ] **Step 7: Commit Tauri Rust shell**

Run:

```powershell
git add Cargo.toml Cargo.lock apps/desktop/src-tauri
git commit -m "feat: add Tauri desktop shell"
```

Expected: commit succeeds with the Tauri Rust shell.

## Task 5: Add React + Vite Frontend Shell

**Files:**
- Create: `apps/desktop/package.json`
- Create: `apps/desktop/index.html`
- Create: `apps/desktop/tsconfig.json`
- Create: `apps/desktop/tsconfig.node.json`
- Create: `apps/desktop/vite.config.ts`
- Create: `apps/desktop/src/main.tsx`
- Create: `apps/desktop/src/App.tsx`
- Create: `apps/desktop/src/styles.css`
- Generate: `apps/desktop/package-lock.json`

- [ ] **Step 1: Add frontend package manifest**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: apps/desktop/package.json
+{
+  "name": "openplayer-desktop",
+  "version": "0.1.0",
+  "private": true,
+  "type": "module",
+  "scripts": {
+    "dev": "vite --host 127.0.0.1 --port 1420",
+    "build": "tsc && vite build",
+    "preview": "vite preview --host 127.0.0.1 --port 1420",
+    "tauri:dev": "tauri dev",
+    "tauri:build": "tauri build"
+  },
+  "dependencies": {
+    "@tauri-apps/api": "^2.0.0",
+    "@vitejs/plugin-react": "^5.0.0",
+    "react": "^19.0.0",
+    "react-dom": "^19.0.0",
+    "vite": "^7.0.0"
+  },
+  "devDependencies": {
+    "@tauri-apps/cli": "^2.0.0",
+    "@types/react": "^19.0.0",
+    "@types/react-dom": "^19.0.0",
+    "typescript": "^5.0.0"
+  }
+}
*** End Patch
```

- [ ] **Step 2: Add Vite and TypeScript configuration**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: apps/desktop/index.html
+<!doctype html>
+<html lang="en">
+  <head>
+    <meta charset="UTF-8" />
+    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
+    <title>OpenPlayer</title>
+  </head>
+  <body>
+    <div id="root"></div>
+    <script type="module" src="/src/main.tsx"></script>
+  </body>
+</html>
*** Add File: apps/desktop/tsconfig.json
+{
+  "compilerOptions": {
+    "target": "ES2020",
+    "useDefineForClassFields": true,
+    "lib": ["DOM", "DOM.Iterable", "ES2020"],
+    "allowJs": false,
+    "skipLibCheck": true,
+    "esModuleInterop": true,
+    "allowSyntheticDefaultImports": true,
+    "strict": true,
+    "forceConsistentCasingInFileNames": true,
+    "module": "ESNext",
+    "moduleResolution": "Node",
+    "resolveJsonModule": true,
+    "isolatedModules": true,
+    "noEmit": true,
+    "jsx": "react-jsx"
+  },
+  "include": ["src"],
+  "references": [{ "path": "./tsconfig.node.json" }]
+}
*** Add File: apps/desktop/tsconfig.node.json
+{
+  "compilerOptions": {
+    "composite": true,
+    "module": "ESNext",
+    "moduleResolution": "Node",
+    "allowSyntheticDefaultImports": true
+  },
+  "include": ["vite.config.ts"]
+}
*** Add File: apps/desktop/vite.config.ts
+import { defineConfig } from "vite";
+import react from "@vitejs/plugin-react";
+
+export default defineConfig({
+  plugins: [react()],
+  clearScreen: false,
+  server: {
+    host: "127.0.0.1",
+    port: 1420,
+    strictPort: true,
+    watch: {
+      ignored: ["**/src-tauri/**"],
+    },
+  },
+});
*** End Patch
```

- [ ] **Step 3: Add React entrypoint**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: apps/desktop/src/main.tsx
+import React from "react";
+import ReactDOM from "react-dom/client";
+import App from "./App";
+import "./styles.css";
+
+ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
+  <React.StrictMode>
+    <App />
+  </React.StrictMode>,
+);
*** End Patch
```

- [ ] **Step 4: Add Studio Dark app shell**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: apps/desktop/src/App.tsx
+import { useEffect, useState } from "react";
+import { invoke } from "@tauri-apps/api/core";
+
+type AppInfo = {
+  name: string;
+  version: string;
+  stage: "skeleton";
+};
+
+type HealthState =
+  | { status: "loading" }
+  | { status: "ready"; info: AppInfo }
+  | { status: "error"; message: string };
+
+function App() {
+  const [health, setHealth] = useState<HealthState>({ status: "loading" });
+
+  useEffect(() => {
+    let isMounted = true;
+
+    invoke<AppInfo>("app_health")
+      .then((info) => {
+        if (isMounted) {
+          setHealth({ status: "ready", info });
+        }
+      })
+      .catch((error: unknown) => {
+        if (isMounted) {
+          setHealth({
+            status: "error",
+            message: error instanceof Error ? error.message : String(error),
+          });
+        }
+      });
+
+    return () => {
+      isMounted = false;
+    };
+  }, []);
+
+  return (
+    <main className="app-shell">
+      <section className="hero-panel">
+        <header className="top-bar">
+          <div>
+            <p className="eyebrow">OpenPlayer</p>
+            <h1>Studio Dark player foundation</h1>
+          </div>
+          <div className="status-pill">
+            {health.status === "ready" ? `v${health.info.version}` : health.status}
+          </div>
+        </header>
+
+        <div className="player-grid">
+          <section className="video-surface" aria-label="Player surface">
+            <div className="play-orb">Play</div>
+            <div className="transport">
+              <div className="timeline">
+                <span />
+              </div>
+              <div className="transport-meta">
+                <span>00:00 / 00:00</span>
+                <span>1.0x | 76% | Subtitles ready</span>
+              </div>
+            </div>
+          </section>
+
+          <aside className="side-panel" aria-label="Player panels">
+            <div className="panel-card">
+              <p className="panel-label">Queue</p>
+              <strong>Playlist service boundary</strong>
+              <span>Persistent queues arrive in the storage phase.</span>
+            </div>
+            <div className="panel-card">
+              <p className="panel-label">Tracks</p>
+              <strong>MediaBackend contract</strong>
+              <span>Audio, subtitle, chapter, and media info APIs arrive with libmpv.</span>
+            </div>
+            <div className="panel-card">
+              <p className="panel-label">Themes</p>
+              <strong>Studio Dark tokens</strong>
+              <span>Theme manifests are represented in the Rust workspace.</span>
+            </div>
+          </aside>
+        </div>
+
+        <footer className="health-row">
+          {health.status === "ready" && (
+            <span>
+              Rust core connected: {health.info.name} is in {health.info.stage} stage.
+            </span>
+          )}
+          {health.status === "loading" && <span>Connecting to Rust core...</span>}
+          {health.status === "error" && <span>Rust core error: {health.message}</span>}
+        </footer>
+      </section>
+    </main>
+  );
+}
+
+export default App;
*** End Patch
```

- [ ] **Step 5: Add Studio Dark CSS**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: apps/desktop/src/styles.css
+:root {
+  color: #e9eef8;
+  background: #080a0f;
+  font-family:
+    Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
+    sans-serif;
+  font-synthesis: none;
+  text-rendering: optimizeLegibility;
+  -webkit-font-smoothing: antialiased;
+  --surface: #080a0f;
+  --surface-elevated: #111722;
+  --surface-soft: #161d2b;
+  --text-primary: #e9eef8;
+  --text-muted: #aeb9cc;
+  --accent: #5b8cff;
+  --border: rgba(255, 255, 255, 0.1);
+}
+
+* {
+  box-sizing: border-box;
+}
+
+body {
+  margin: 0;
+  min-width: 320px;
+  min-height: 100vh;
+  background:
+    radial-gradient(circle at top left, rgba(91, 140, 255, 0.22), transparent 34rem),
+    linear-gradient(135deg, #080a0f 0%, #0c111a 56%, #050609 100%);
+}
+
+button,
+input {
+  font: inherit;
+}
+
+.app-shell {
+  min-height: 100vh;
+  padding: 32px;
+}
+
+.hero-panel {
+  width: min(1280px, 100%);
+  margin: 0 auto;
+  border: 1px solid var(--border);
+  border-radius: 28px;
+  background: rgba(8, 10, 15, 0.84);
+  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.38);
+  padding: 24px;
+  backdrop-filter: blur(20px);
+}
+
+.top-bar,
+.transport-meta,
+.health-row {
+  display: flex;
+  align-items: center;
+  justify-content: space-between;
+  gap: 16px;
+}
+
+.eyebrow,
+.panel-label {
+  margin: 0 0 6px;
+  color: var(--accent);
+  font-size: 0.72rem;
+  font-weight: 700;
+  letter-spacing: 0.14em;
+  text-transform: uppercase;
+}
+
+h1 {
+  margin: 0;
+  font-size: clamp(2rem, 4vw, 4.3rem);
+  letter-spacing: -0.06em;
+}
+
+.status-pill {
+  border: 1px solid var(--border);
+  border-radius: 999px;
+  background: var(--surface-soft);
+  color: var(--text-muted);
+  padding: 10px 14px;
+}
+
+.player-grid {
+  display: grid;
+  grid-template-columns: minmax(0, 1fr) 320px;
+  gap: 20px;
+  margin-top: 28px;
+}
+
+.video-surface {
+  position: relative;
+  min-height: 520px;
+  overflow: hidden;
+  border: 1px solid var(--border);
+  border-radius: 22px;
+  background:
+    radial-gradient(circle at 50% 42%, rgba(91, 140, 255, 0.34), transparent 22rem),
+    linear-gradient(135deg, #141b27, #050609);
+}
+
+.play-orb {
+  position: absolute;
+  top: 50%;
+  left: 50%;
+  display: grid;
+  width: 76px;
+  height: 76px;
+  place-items: center;
+  border-radius: 50%;
+  background: var(--text-primary);
+  color: var(--surface);
+  font-weight: 800;
+  transform: translate(-50%, -50%);
+}
+
+.transport {
+  position: absolute;
+  right: 20px;
+  bottom: 20px;
+  left: 20px;
+  border: 1px solid var(--border);
+  border-radius: 16px;
+  background: rgba(8, 10, 15, 0.78);
+  padding: 14px;
+}
+
+.timeline {
+  height: 6px;
+  overflow: hidden;
+  border-radius: 999px;
+  background: #273246;
+}
+
+.timeline span {
+  display: block;
+  width: 38%;
+  height: 100%;
+  border-radius: inherit;
+  background: var(--accent);
+}
+
+.transport-meta {
+  margin-top: 12px;
+  color: var(--text-muted);
+  font-size: 0.9rem;
+}
+
+.side-panel {
+  display: grid;
+  gap: 14px;
+}
+
+.panel-card {
+  min-height: 156px;
+  border: 1px solid var(--border);
+  border-radius: 18px;
+  background: var(--surface-elevated);
+  padding: 18px;
+}
+
+.panel-card strong,
+.panel-card span {
+  display: block;
+}
+
+.panel-card span,
+.health-row {
+  color: var(--text-muted);
+}
+
+.panel-card span {
+  margin-top: 10px;
+  line-height: 1.5;
+}
+
+.health-row {
+  margin-top: 20px;
+  font-size: 0.95rem;
+}
+
+@media (max-width: 920px) {
+  .app-shell {
+    padding: 16px;
+  }
+
+  .player-grid {
+    grid-template-columns: 1fr;
+  }
+
+  .video-surface {
+    min-height: 360px;
+  }
+}
*** End Patch
```

- [ ] **Step 6: Install frontend dependencies and generate lockfile**

Run:

```powershell
npm install
```

Working directory: `apps/desktop`

Expected: `package-lock.json` is created and npm reports installed packages without vulnerabilities that block install.

- [ ] **Step 7: Build frontend**

Run:

```powershell
npm run build
```

Working directory: `apps/desktop`

Expected: TypeScript compilation and Vite production build succeed.

- [ ] **Step 8: Commit frontend shell**

Run:

```powershell
git add apps/desktop/package.json apps/desktop/package-lock.json apps/desktop/index.html apps/desktop/tsconfig.json apps/desktop/tsconfig.node.json apps/desktop/vite.config.ts apps/desktop/src
git commit -m "feat: add Studio Dark desktop frontend shell"
```

Expected: commit succeeds with React/Vite frontend files and lockfile.

## Task 6: Add Continuous Integration

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Add CI workflow**

Use `apply_patch`:

```diff
*** Begin Patch
*** Add File: .github/workflows/ci.yml
+name: CI
+
+on:
+  push:
+    branches: [main]
+  pull_request:
+    branches: [main]
+
+jobs:
+  rust:
+    name: Rust
+    runs-on: ubuntu-latest
+    steps:
+      - name: Checkout
+        uses: actions/checkout@v4
+
+      - name: Install Tauri Linux dependencies
+        run: |
+          sudo apt-get update
+          sudo apt-get install -y \
+            libwebkit2gtk-4.1-dev \
+            libgtk-3-dev \
+            libayatana-appindicator3-dev \
+            librsvg2-dev \
+            patchelf
+
+      - name: Install Rust
+        uses: dtolnay/rust-toolchain@stable
+        with:
+          components: rustfmt, clippy
+
+      - name: Check formatting
+        run: cargo fmt --all -- --check
+
+      - name: Run clippy
+        run: cargo clippy --workspace --all-targets -- -D warnings
+
+      - name: Run tests
+        run: cargo test --workspace
+
+  frontend:
+    name: Frontend
+    runs-on: ubuntu-latest
+    defaults:
+      run:
+        working-directory: apps/desktop
+    steps:
+      - name: Checkout
+        uses: actions/checkout@v4
+
+      - name: Install Node
+        uses: actions/setup-node@v4
+        with:
+          node-version: 20
+          cache: npm
+          cache-dependency-path: apps/desktop/package-lock.json
+
+      - name: Install dependencies
+        run: npm ci
+
+      - name: Build frontend
+        run: npm run build
*** End Patch
```

- [ ] **Step 2: Run local formatting**

Run:

```powershell
cargo fmt --all
```

Expected: command succeeds and formats Rust files.

- [ ] **Step 3: Run local Rust CI commands**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected: all commands pass.

- [ ] **Step 4: Run local frontend CI command**

Run:

```powershell
npm ci
npm run build
```

Working directory: `apps/desktop`

Expected: dependencies install from `package-lock.json` and the frontend build passes.

- [ ] **Step 5: Commit CI workflow**

Run:

```powershell
git add .github/workflows/ci.yml
git commit -m "ci: add Rust and frontend checks"
```

Expected: commit succeeds with the CI workflow.

## Task 7: Publish Public GitHub Repository

**Files:**
- No source files changed by this task.

- [ ] **Step 1: Verify GitHub CLI authentication**

Run:

```powershell
gh auth status
```

Expected: GitHub CLI reports an authenticated account with permission to create repositories.

- [ ] **Step 2: Verify working tree is clean before publishing**

Run:

```powershell
git status --short
```

Expected: no output.

- [ ] **Step 3: Create public GitHub repository and push main**

Run:

```powershell
gh repo create OpenPlayer --public --source . --remote origin --push --description "Cross-platform high-performance media player built with Rust and Tauri"
```

Expected: GitHub CLI creates the public repository under the authenticated account, adds `origin`, and pushes `main`.

- [ ] **Step 4: Verify remote configuration**

Run:

```powershell
git remote -v
git status --short --branch
```

Expected: `origin` points to the new GitHub repository and `main` tracks `origin/main` with a clean working tree.

## Task 8: Final Phase 1 Verification

**Files:**
- No source files changed by this task unless verification reveals a defect.

- [ ] **Step 1: Verify repository ignores generated artifacts**

Run:

```powershell
git status --short --ignored
```

Expected: generated directories such as `target/`, `apps/desktop/node_modules/`, `apps/desktop/dist/`, and `.superpowers/` are ignored when present.

- [ ] **Step 2: Verify Rust checks**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected: all Rust checks pass.

- [ ] **Step 3: Verify frontend build**

Run:

```powershell
npm ci
npm run build
```

Working directory: `apps/desktop`

Expected: frontend dependency installation and production build pass.

- [ ] **Step 4: Verify Tauri development launch reaches the app shell**

Run:

```powershell
npm run tauri:dev
```

Working directory: `apps/desktop`

Expected: OpenPlayer desktop window opens, displays the Studio Dark shell, and shows `Rust core connected: OpenPlayer is in skeleton stage.`

- [ ] **Step 5: Record verification result in the final response**

Report the exact commands run and their pass/fail result. If `npm run tauri:dev` cannot be completed in the automation environment because it opens a GUI window, report that limitation and include the last successful non-GUI command.

## Self-Review

- Spec coverage: this plan covers Phase 1 skeleton and governance from the approved spec: repository hygiene, MIT license, `.gitignore`, workspace crates, Tauri shell, React/Vite shell, Studio Dark visual foundation, docs indexes, CI, and public GitHub publication.
- Planned gaps: SQLite persistence, real playlist/history services, real plugin loading, theme application, libmpv playback, subtitles, track info, HTTP(S) playback, native dependency bundling, and release packaging are intentionally outside this Phase 1 plan and require follow-up implementation plans.
- Type consistency: `AppInfo`, `AppStage`, `app_info`, and `app_health` names are consistent across `openplayer-shared`, `openplayer-core`, Tauri Rust, and React.
- Verification coverage: Rust tests, frontend build, git ignore behavior, GitHub remote setup, and Tauri launch are all verified by explicit commands.
