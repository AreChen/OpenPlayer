# OpenPlayer MPV Child Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Windows-only `mpv-embed` spike render visible video by giving mpv its own native child HWND inside the Tauri window.

**Architecture:** Keep the React shell and current HTML video fallback. Under `mpv-embed`, Rust creates a Win32 child window owned by the Tauri WebView/window handle and passes that child HWND to mpv through `wid`; frontend picker-driven open calls still use `mpv_embed_open_path`.

**Tech Stack:** Tauri v2, Rust 2024, `libmpv2`, `raw-window-handle`, `windows-sys`, Tauri dialog plugin, React 19.

---

## Scope Check

This is a Windows-only rendering spike. It does not implement final player controls, subtitle controls, playlist persistence, cross-platform rendering, OpenGL rendering, or release packaging of mpv DLLs.

## File Structure

- Modify: `apps/desktop/src-tauri/Cargo.toml` adds optional `windows-sys` to `mpv-embed`.
- Modify: `apps/desktop/src-tauri/src/mpv_embed.rs` owns child HWND creation, sizing, cleanup, and mpv `wid` binding.
- Modify: `apps/desktop/src-tauri/src/lib.rs` keeps feature-gated command registration.
- Modify: `apps/desktop/scripts/verify-shell.mjs` guards child HWND dependency and APIs.
- Modify: `apps/desktop/src/App.tsx` keeps picker-driven open and debug status.

## Task 1: Child HWND Host

- [ ] Add RED shell assertions for `windows-sys`, `CreateWindowExW`, `SetParent`, `MoveWindow`, and `DestroyWindow` in the feature-gated embed module.
- [ ] Add optional `windows-sys` dependency with Win32 UI feature flags and include it in `mpv-embed`.
- [ ] Implement a small `MpvVideoHost` that creates a child `STATIC` HWND, sizes it to the parent client rect, stores it with the player, and destroys it on stop/drop.
- [ ] Pass the child HWND, not the WebView HWND, to mpv `wid`.
- [ ] Run `npm run verify:shell`, `npm run build`, `cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo test -p openplayer-desktop --features mpv-embed`.
- [ ] Launch `tauri dev --features mpv-embed` with `OPENPLAYER_MPV_EMBED_FILE` and inspect whether video appears.

## Plan Self-Review

- Spec coverage: The plan targets only the approved Windows child HWND spike and keeps HTML fallback.
- Completeness scan: Each task has concrete files, APIs, commands, and expected verification.
- Type consistency: `MpvVideoHost`, `MpvEmbedPlayer`, and `mpv_embed_open_path` remain Rust-side embed concepts only.
