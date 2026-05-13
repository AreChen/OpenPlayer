# Architecture

OpenPlayer is organized around a Rust workspace and a Tauri desktop shell.

The stable runtime boundary is:

```text
React UI -> Tauri command -> Rust service -> backend/storage/plugin/theme -> typed event -> React UI
```

The UI must not call media backends or SQLite directly. Rust services own playback state, persistence, plugin validation, and theme validation.
