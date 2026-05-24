# Architecture

OpenPlayer is a Tauri v2 desktop media player built around a native libmpv
playback host and a React control overlay.

The active app is `apps/desktop`. The Rust workspace currently contains the
desktop Tauri crate at `apps/desktop/src-tauri`.

## Runtime Split

The default playback path is the `mpv-embed` feature:

```text
main Tauri window     -> native libmpv video host
transparent overlay   -> React controls, menus, settings, shortcuts
Tauri commands        -> playback, persistence, shell integration
redb stores           -> history, resume state, preferences, themes
```

The overlay should not reintroduce browser `<video>` playback, object URLs, or
the removed mpv render API spike. Window movement, fullscreen, always-on-top,
resize, and close commands should target the main video window and keep the
overlay synchronized.

## Main Boundaries

- `apps/desktop/src/App.tsx` owns overlay UI, controls, settings, context menus,
  drag/drop intake, keyboard shortcuts, and i18n wiring.
- `apps/desktop/src-tauri/src/lib.rs` owns Tauri setup, command registration,
  window lifecycle, and native shell bridges.
- `apps/desktop/src-tauri/src/mpv_embed.rs` owns libmpv playback and returns
  snapshots after commands that affect playback state.
- `apps/desktop/src-tauri/src/playback_store.rs` owns playback history, resume
  positions, global playback settings, and per-media settings in redb.
- `apps/desktop/src-tauri/src/appearance_store.rs` owns theme, accent, language,
  and theme plugin state in redb.

## Supporting Notes

- [Native dependencies](../native-deps/README.md) documents bundled runtime
  dependency metadata.
- [Theme plugins](../plugins/README.md) documents the current manifest-only
  theme plugin contract.
- [libmpv smoke test](./libmpv2-smoke.md) documents the optional headless
  libmpv initialization check.
