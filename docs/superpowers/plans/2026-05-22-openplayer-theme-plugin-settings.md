# OpenPlayer Theme Plugin Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add redb-backed Studio Dark appearance settings, accent override, and manifest-only theme plugin support.

**Architecture:** A new Rust `appearance_store` module owns validation, built-in theme catalog, redb persistence, and Tauri commands. React consumes a single `AppearanceState`, applies CSS variables, and extends the existing settings dialog with Appearance, Plugins, and Shortcuts sections.

**Tech Stack:** Tauri v2, Rust, redb, serde, React, TypeScript, CSS custom properties.

---

### Task 1: Backend Store And Validation

**Files:**
- Create: `apps/desktop/src-tauri/src/appearance_store.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/scripts/verify-shell.mjs`

- [ ] Write failing Rust tests for theme state persistence, manifest import, enablement fallback, and invalid colors.
- [ ] Implement built-in theme catalog and manifest validation.
- [ ] Implement `AppearanceStoreState` and redb tables in `openplayer-settings.redb`.
- [ ] Register appearance commands in Tauri.
- [ ] Run `cargo test -p openplayer-desktop appearance_store`.

### Task 2: Frontend Theme Application

**Files:**
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/styles.css`
- Modify: `apps/desktop/scripts/verify-shell.mjs`

- [ ] Add frontend types matching `AppearanceState`.
- [ ] Load `appearance_state()` on startup and after imports/updates.
- [ ] Apply selected theme tokens as CSS variables on the overlay shell.
- [ ] Add Appearance and Plugins sections to settings while keeping Shortcuts intact.
- [ ] Add theme cards, accent swatches, plugin import, plugin enable/disable, and reset actions.
- [ ] Run `npm run verify:shell` and `npm run build`.

### Task 3: Verification

**Files:**
- Modify as needed from Tasks 1-2 only.

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo test -p openplayer-desktop`.
- [ ] Run `cargo clippy -p openplayer-desktop --all-targets -- -D warnings`.
- [ ] Run `npm run verify:shell`.
- [ ] Run `npm run build`.
