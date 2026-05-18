# OpenPlayer Overlay Drag Fullscreen Seek Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the mpv overlay player draggable from non-control space, toggle true main-window fullscreen, and keep seek UI stable after user seeks.

**Architecture:** Keep the current two-window architecture: main owns mpv video, overlay owns React controls. Route window movement/fullscreen commands through Rust so the overlay controls operate on the main player window. Treat seek as a short optimistic UI transaction so stale mpv snapshots cannot overwrite the target while mpv catches up.

**Tech Stack:** Tauri 2, React, TypeScript, Windows HWND ownership, libmpv2 embed backend.

---

### Task 1: Add Shell Guards

**Files:**
- Modify: `apps/desktop/scripts/verify-shell.mjs`

- [ ] **Step 1: Write failing guards**

Add assertions that require `window_toggle_fullscreen`, non-control drag wiring, and seek-pending snapshot suppression.

- [ ] **Step 2: Run guards and verify RED**

Run: `npm run verify:shell`

Expected: FAIL because fullscreen and seek-pending behavior are not implemented yet.

### Task 2: Main Window Fullscreen Command

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src/App.tsx`

- [ ] **Step 1: Implement backend command**

Add `window_toggle_fullscreen(app: AppHandle)` that calls `main_window(&app)?.set_fullscreen(!is_fullscreen)` and then `sync_overlay_to_main(&app)`.

- [ ] **Step 2: Route frontend fullscreen through backend**

Replace overlay-local `getCurrentWindow().setFullscreen(...)` with `invoke("window_toggle_fullscreen")`.

- [ ] **Step 3: Run guards**

Run: `npm run verify:shell`

Expected: fullscreen guards pass; remaining seek guards may still fail.

### Task 3: Non-Control Drag Surface

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/styles.css`

- [ ] **Step 1: Expand drag surface**

Make `.drag-region` cover the full overlay viewport behind controls, excluding controls by z-index and pointer targets.

- [ ] **Step 2: Keep controls interactive**

Ensure `.window-controls`, `.transport`, `.playlist-drawer`, and alert surfaces remain above the drag layer with `pointer-events: auto`.

- [ ] **Step 3: Run frontend build**

Run: `npm run build`

Expected: PASS.

### Task 4: Seek Stability

**Files:**
- Modify: `apps/desktop/src/App.tsx`

- [ ] **Step 1: Add pending seek state**

Track `{ target, startedAt } | null` while a seek is in flight.

- [ ] **Step 2: Suppress stale snapshots**

When a pending seek exists, ignore snapshots whose position is not close to the target until mpv catches up or a short timeout expires.

- [ ] **Step 3: Clear pending state on seek confirmation**

Clear pending seek when the seek response or later snapshot is close enough to the target.

- [ ] **Step 4: Run frontend build**

Run: `npm run build`

Expected: PASS.

### Task 5: Full Verification

**Files:**
- Verify only.

- [ ] **Step 1: Run shell guards**

Run: `npm run verify:shell`

Expected: PASS.

- [ ] **Step 2: Run frontend build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 3: Run Rust checks**

Run: `cargo fmt --all -- --check` and `cargo check -p openplayer-desktop --features mpv-render`

Expected: PASS.

- [ ] **Step 4: Runtime smoke**

Run: `npm run tauri:dev`, open a video manually, verify non-control drag, double-click fullscreen, controls still clickable, and single seek does not visibly jump back.
