# OpenPlayer Icon And Logo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the supplied OpenPlayer logo into the app asset tree and use it for app icons, Windows packaging, and the empty player UI.

**Architecture:** `apps/desktop/src/assets/openplayer-logo.png` becomes the canonical source asset. Generated Tauri icon files live under `apps/desktop/src-tauri/icons/`, preserving the existing `build.rs` path for `icons/icon.ico`. React references the app asset through Vite's `new URL(..., import.meta.url).href` pattern so no TypeScript asset declaration is needed.

**Tech Stack:** Tauri v2, React 19, Vite, TypeScript, PowerShell/.NET `System.Drawing` for one-time local icon conversion.

---

## File Structure

- Move: `openplayer_logo_10001000.png` -> `apps/desktop/src/assets/openplayer-logo.png`
- Modify: `apps/desktop/src-tauri/icons/icon.png` generated from the logo source
- Modify: `apps/desktop/src-tauri/icons/icon.ico` generated from the logo source
- Create or modify: `apps/desktop/src-tauri/icons/32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png`
- Modify: `apps/desktop/src/App.tsx` to render the logo in the empty state
- Modify: `apps/desktop/src/styles.css` to size and polish the empty-state logo
- Modify: `apps/desktop/scripts/verify-shell.mjs` to guard asset placement and UI usage

No git commit steps are included because this environment only commits when the user explicitly asks.

### Task 1: Add Asset Placement Guards

**Files:**
- Modify: `apps/desktop/scripts/verify-shell.mjs`

- [ ] **Step 1: Add asset URL constants**

Add these constants after the existing `mpvRenderFiles` declaration:

```js
const rootLogoUrl = new URL("../../../openplayer_logo_10001000.png", import.meta.url);
const uiLogoUrl = new URL("../src/assets/openplayer-logo.png", import.meta.url);
const tauriIconPngUrl = new URL("../src-tauri/icons/icon.png", import.meta.url);
const tauriIconIcoUrl = new URL("../src-tauri/icons/icon.ico", import.meta.url);
```

- [ ] **Step 2: Add placement assertions**

Add these assertions after the existing package/dev URL assertions:

```js
assert.ok(!existsSync(rootLogoUrl), "source logo must live under app assets, not the repository root");
assert.ok(existsSync(uiLogoUrl), "frontend logo asset must exist");
assert.ok(existsSync(tauriIconPngUrl), "Tauri PNG icon must exist");
assert.ok(existsSync(tauriIconIcoUrl), "Windows ICO icon must exist");
```

- [ ] **Step 3: Add UI usage assertions**

Add these assertions near the existing `appSource` and `styles` UI assertions:

```js
assert.match(appSource, /openplayer-logo\.png/, "empty player state must use the OpenPlayer logo asset");
assert.match(styles, /\.empty-open-logo/, "empty player state must style the OpenPlayer logo");
```

- [ ] **Step 4: Run guard and verify it fails before implementation**

Run from `apps/desktop`:

```bash
npm run verify:shell
```

Expected: FAIL because `openplayer_logo_10001000.png` still exists in the repository root and `apps/desktop/src/assets/openplayer-logo.png` does not exist yet.

### Task 2: Move Source Logo And Generate Icons

**Files:**
- Move: `openplayer_logo_10001000.png` -> `apps/desktop/src/assets/openplayer-logo.png`
- Modify: `apps/desktop/src-tauri/icons/icon.png`
- Modify: `apps/desktop/src-tauri/icons/icon.ico`
- Create or modify: `apps/desktop/src-tauri/icons/32x32.png`
- Create or modify: `apps/desktop/src-tauri/icons/128x128.png`
- Create or modify: `apps/desktop/src-tauri/icons/128x128@2x.png`

- [ ] **Step 1: Move the root PNG into the app asset tree**

Run from the repository root:

```powershell
Test-Path -LiteralPath "apps/desktop/src"; New-Item -ItemType Directory -Force -Path "apps/desktop/src/assets"; Move-Item -LiteralPath "openplayer_logo_10001000.png" -Destination "apps/desktop/src/assets/openplayer-logo.png"
```

Expected: `apps/desktop/src/assets/openplayer-logo.png` exists and `openplayer_logo_10001000.png` no longer exists in the root.

- [ ] **Step 2: Generate Tauri PNG and ICO files**

Run this PowerShell from the repository root:

```powershell
Add-Type -AssemblyName System.Drawing; $source = "apps/desktop/src/assets/openplayer-logo.png"; $iconDir = "apps/desktop/src-tauri/icons"; $sourceImage = [System.Drawing.Image]::FromFile((Resolve-Path -LiteralPath $source)); function New-PngBytes([System.Drawing.Image] $image, [int] $size) { $bitmap = [System.Drawing.Bitmap]::new($size, $size); $graphics = [System.Drawing.Graphics]::FromImage($bitmap); $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality; $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic; $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality; $graphics.DrawImage($image, 0, 0, $size, $size); $stream = [System.IO.MemoryStream]::new(); $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png); $graphics.Dispose(); $bitmap.Dispose(); $bytes = $stream.ToArray(); $stream.Dispose(); return $bytes }; function Save-Png([string] $path, [int] $size) { [System.IO.File]::WriteAllBytes((Join-Path $PWD $path), (New-PngBytes $sourceImage $size)) }; Save-Png "apps/desktop/src-tauri/icons/32x32.png" 32; Save-Png "apps/desktop/src-tauri/icons/128x128.png" 128; Save-Png "apps/desktop/src-tauri/icons/128x128@2x.png" 256; Save-Png "apps/desktop/src-tauri/icons/icon.png" 512; $icoSizes = @(16, 32, 48, 64, 128, 256); $entries = @($icoSizes | ForEach-Object { [pscustomobject]@{ Size = $_; Bytes = New-PngBytes $sourceImage $_ } }); $writer = [System.IO.BinaryWriter]::new([System.IO.File]::Create((Join-Path $PWD "apps/desktop/src-tauri/icons/icon.ico"))); $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$entries.Count); $offset = 6 + (16 * $entries.Count); foreach ($entry in $entries) { $sizeByte = if ($entry.Size -eq 256) { 0 } else { $entry.Size }; $writer.Write([byte]$sizeByte); $writer.Write([byte]$sizeByte); $writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([uint16]1); $writer.Write([uint16]32); $writer.Write([uint32]$entry.Bytes.Length); $writer.Write([uint32]$offset); $offset += $entry.Bytes.Length }; foreach ($entry in $entries) { $writer.Write([byte[]]$entry.Bytes) }; $writer.Dispose(); $sourceImage.Dispose()
```

Expected: PNG icons are regenerated from the logo and `icon.ico` contains common Windows icon sizes.

- [ ] **Step 3: Verify generated dimensions**

Run from the repository root:

```powershell
Add-Type -AssemblyName System.Drawing; foreach ($p in @("apps/desktop/src/assets/openplayer-logo.png", "apps/desktop/src-tauri/icons/icon.png", "apps/desktop/src-tauri/icons/128x128.png", "apps/desktop/src-tauri/icons/128x128@2x.png", "apps/desktop/src-tauri/icons/32x32.png")) { $img = [System.Drawing.Image]::FromFile((Resolve-Path -LiteralPath $p)); "$p $($img.Width)x$($img.Height)"; $img.Dispose() }
```

Expected output includes `1000x1000`, `512x512`, `128x128`, `256x256`, and `32x32`.

### Task 3: Render Logo In Empty Player State

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/styles.css`

- [ ] **Step 1: Add the logo URL constant**

Add this line after the existing `surface` constant in `apps/desktop/src/App.tsx`:

```ts
const openPlayerLogoUrl = new URL("./assets/openplayer-logo.png", import.meta.url).href;
```

- [ ] **Step 2: Render the logo in the empty state**

Replace the empty-state JSX with this version:

```tsx
{!media && (
  <div className="empty-open">
    <img className="empty-open-logo" src={openPlayerLogoUrl} alt="" draggable={false} />
    <span>Open media</span>
  </div>
)}
```

- [ ] **Step 3: Add logo styling**

Add this CSS after the `.empty-open` rule in `apps/desktop/src/styles.css`:

```css
.empty-open-logo {
  width: clamp(88px, 18vw, 180px);
  height: clamp(88px, 18vw, 180px);
  object-fit: contain;
  border-radius: 28%;
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.34);
  user-select: none;
  -webkit-user-drag: none;
}
```

Also add `justify-items: center;` to the existing `.empty-open` rule so the logo is centered relative to the text column.

- [ ] **Step 4: Run frontend build**

Run from `apps/desktop`:

```bash
npm run build
```

Expected: PASS. Vite should resolve the logo asset through `new URL(..., import.meta.url)`, and the empty state should not render the old `MPV native playback` tagline.

### Task 4: Validate App Icon Wiring

**Files:**
- Read: `apps/desktop/src-tauri/build.rs`
- Read: `apps/desktop/src-tauri/icons/icon.ico`
- Read: `apps/desktop/src-tauri/icons/icon.png`

- [ ] **Step 1: Run shell guard**

Run from `apps/desktop`:

```bash
npm run verify:shell
```

Expected: PASS. The root PNG is gone, app assets exist, and UI references the logo.

- [ ] **Step 2: Run Rust/Tauri validation**

Run from the repository root:

```bash
cargo check -p openplayer-desktop
```

Expected: PASS. The existing `build.rs` still finds `apps/desktop/src-tauri/icons/icon.ico`.

- [ ] **Step 3: Build the Windows bundle if local mpv artifacts are present**

Run from `apps/desktop`:

```bash
npm run tauri:build
```

Expected: PASS and regenerate the NSIS installer using the new Windows icon.

- [ ] **Step 4: Inspect final working tree**

Run from the repository root:

```bash
git status --short
```

Expected: changed files include the moved logo, regenerated icon assets, UI/CSS changes, verification guard changes, and this spec/plan. The root `openplayer_logo_10001000.png` should appear as deleted or absent.
