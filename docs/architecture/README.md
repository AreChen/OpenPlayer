# Architecture

OpenPlayer is organized around a Rust workspace and a Tauri desktop shell.

The stable runtime boundary is:

```text
React UI -> Tauri command -> Rust service -> backend/storage/plugin/theme -> typed event -> React UI
```

The UI must not call media backends or SQLite directly. Rust services own playback state, persistence, plugin validation, and theme validation.

## Native Media Spikes

- [libmpv2 smoke spike](./libmpv2-smoke.md) documents the feature-gated Rust initialization probe for local `libmpv` artifacts.
