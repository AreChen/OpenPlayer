# OpenPlayer Theme Plugin Settings Design

Date: 2026-05-22

## Goal

Add the first usable theme system while laying the plugin foundation for theme packs.

## Scope

Included:

- Built-in Studio Dark theme available from the desktop backend.
- Accent color override.
- Theme plugin manifests that contribute themes only.
- Redb-backed persistence for active theme, accent override, imported theme manifests, plugin manifests, and plugin enablement.
- Settings UI sections for appearance, theme plugins, and shortcuts.
- Runtime CSS variable application in the React overlay.

Excluded:

- JavaScript, WASM, or native plugin execution.
- Plugin access to mpv, files, network, commands, or renderer internals.
- Linux or macOS packaging changes.

## Backend Design

Create `apps/desktop/src-tauri/src/appearance_store.rs`.

The store uses `storage/openplayer-settings.redb` and keeps theme/plugin settings separate from playback history:

- `settings_kv`: current active theme and accent override.
- `theme_manifests`: imported plugin theme manifests by theme id.
- `plugin_manifests`: imported theme plugin manifests by plugin id.
- `plugin_enablement`: plugin id to enabled flag.

Expose Tauri commands:

- `appearance_state()`
- `appearance_set_theme(theme_id)`
- `appearance_set_accent_override(accent)`
- `appearance_import_theme_plugin(path)`
- `appearance_set_plugin_enabled(plugin_id, enabled)`
- `appearance_reset()`

## Manifest Contract

Theme plugins are manifest-only:

```json
{
  "id": "dev.openplayer.theme.ocean",
  "name": "Ocean Theme Pack",
  "version": "1.0.0",
  "description": "Ocean color themes for OpenPlayer.",
  "entry": "manifest",
  "contributes": {
    "themes": [
      {
        "id": "dev.openplayer.theme.ocean.dark",
        "name": "Ocean Dark",
        "version": "1.0.0",
        "tokens": {
          "surface": "#050607",
          "panel": "rgba(8, 10, 12, 0.72)",
          "panelStrong": "rgba(8, 10, 12, 0.88)",
          "text": "#ece7dd",
          "muted": "#b9b0a3",
          "faint": "#8f867a",
          "accent": "#62c7b7",
          "danger": "#d78372",
          "line": "rgba(236, 231, 221, 0.12)",
          "control": "rgba(18, 21, 25, 0.72)",
          "scrollbarThumb": "rgba(236, 231, 221, 0.22)",
          "scrollbarThumbHover": "rgba(98, 199, 183, 0.46)"
        }
      }
    ]
  }
}
```

Validation rejects unknown fields, blank ids/names/versions, invalid simple semver, invalid identifiers, invalid colors, non-`manifest` plugin entries, and duplicate theme ids.

## Frontend Design

Extend the settings dialog to use three sections:

- Appearance: theme cards, accent swatches, reset button.
- Plugins: imported theme plugin list, enable/disable controls, import JSON button.
- Shortcuts: current shortcut editor.

The selected theme tokens are applied as CSS variables on the overlay shell. Accent override changes `--accent` and scrollbar hover color without mutating the stored theme manifest.

## Testing

- Rust unit tests for built-in theme state, redb persistence, plugin manifest import, plugin enablement fallback, accent validation, and invalid manifests.
- Structural shell checks for redb settings storage, appearance Tauri commands, localStorage not used for themes, settings sections, and CSS variable application.
- Windows verification: `npm run verify:shell`, `npm run build`, `cargo fmt --all -- --check`, `cargo test -p openplayer-desktop`, `cargo clippy -p openplayer-desktop --all-targets -- -D warnings`.
