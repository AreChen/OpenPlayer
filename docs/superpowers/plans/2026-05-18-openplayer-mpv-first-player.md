# OpenPlayer MPV First Player Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current HTML video playback path with the validated Windows mpv child-window path.

**Architecture:** React remains the shell and control surface. Rust owns native path playback through `libmpv2`; the frontend opens files through Tauri dialog and controls mpv through Tauri commands. HTML `<video>`, browser `File`, and object URLs are removed from the primary player.

**Tech Stack:** Tauri v2, React 19, TypeScript, Rust 2024, `libmpv2`, `raw-window-handle`, `windows-sys`, Tauri dialog plugin.

---

## Scope Check

This implements one Windows-first player slice: open one or more local files, play/pause, restart, seek, volume, stop, and simple queue selection through mpv. It does not implement subtitles, track selection, persisted progress, release packaging, or cross-platform rendering.

## File Structure

- Modify: `apps/desktop/src-tauri/src/mpv_embed.rs` adds mpv control/snapshot commands.
- Modify: `apps/desktop/src-tauri/src/lib.rs` registers those commands and keeps mpv state managed.
- Modify: `apps/desktop/src/App.tsx` removes HTML video state and uses native dialog + mpv command snapshots.
- Modify: `apps/desktop/src/styles.css` removes video-specific styling and keeps a player host surface.
- Modify: `apps/desktop/scripts/verify-shell.mjs` guards against HTML playback and requires mpv-first wiring.

## Task 1: Backend Control Commands

- [ ] Add mpv commands for play, pause, stop, seek, volume, and snapshot.
- [ ] Keep `loadfile` and child HWND host creation in `mpv_embed_open_path`.
- [ ] Add tests for path validation and snapshot defaults where possible.

## Task 2: Frontend MPV Control Surface

- [ ] Replace browser file input with native dialog `open({ multiple: true })`.
- [ ] Store queue entries as native paths and names, not browser `File` objects.
- [ ] Wire play/pause/restart/seek/volume/playlist selection to mpv commands.
- [ ] Remove HTML `<video>` and object URL code.

## Task 3: Verification And Runtime Smoke

- [ ] Run shell verification, frontend build, Rust fmt/tests, and `mpv-embed` tests.
- [ ] Launch with `OPENPLAYER_MPV_EMBED_FILE` and verify visible playback and usable React controls.
- [ ] Commit only after verification.

## Plan Self-Review

- Spec coverage: This plan removes HTML playback and makes mpv the primary local playback path.
- Completeness scan: Each task has concrete files and verification commands.
- Type consistency: frontend queue stores native paths; backend snapshots expose mpv state.
