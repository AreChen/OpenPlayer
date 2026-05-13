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

Prerequisites:

- Rust stable toolchain with edition 2024 support.
- Node.js 20 or newer.
- npm 10 or newer.
- Tauri v2 system dependencies for your platform.

Install frontend dependencies:

```powershell
Set-Location apps/desktop
npm install
```

Run Rust checks:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run frontend build:

```powershell
Set-Location apps/desktop
npm run build
```

Run the desktop app during development:

```powershell
Set-Location apps/desktop
npm run tauri:dev
```

## License

OpenPlayer is licensed under the MIT license. See `LICENSE`.
