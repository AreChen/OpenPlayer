# OpenPlayer

OpenPlayer is a cross-platform, high-performance desktop media player built with Rust, Tauri v2, and React.

The project is in its first architecture milestone. Phase 1 establishes the repository, workspace, desktop shell, and core module boundaries. Media playback, SQLite persistence, application plugins, custom themes, and bundled libmpv support are designed in `docs/superpowers/specs/2026-05-13-openplayer-design.md` and will be implemented in follow-up phases.

## Goals

- Cross-platform desktop player foundation.
- Tauri v2 shell with a polished Studio Dark React UI.
- Rust workspace with focused crates for core services, media backends, storage, plugins, and themes.
- Future `libmpv` backend for broad media format, subtitle, and hardware decode support.
- MIT licensed public GitHub project.

## Development

This Task 1 checkpoint contains repository metadata and planning documents only. The buildable Rust workspace begins in Phase 1 Task 2, and desktop/frontend commands will be documented once `apps/desktop` is added in later Phase 1 tasks.

## License

OpenPlayer is licensed under the MIT license. See `LICENSE`.
