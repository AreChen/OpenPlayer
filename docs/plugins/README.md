# OpenPlayer Plugins

OpenPlayer plugin support is currently a manifest-only V0 contract. The Rust crate can parse and validate plugin manifest JSON strings, but the desktop app does not scan plugin folders, load files, execute plugins, or expose plugin UI yet.

## V0 Manifest

```json
{
  "id": "dev.openplayer.metadata",
  "name": "Metadata Helper",
  "version": "1.0.0",
  "description": "Adds metadata lookup commands.",
  "entry": "builtIn",
  "permissions": ["metadata.read", "settings.read"],
  "contributes": {
    "commands": [
      { "id": "dev.openplayer.metadata.refresh", "title": "Refresh metadata" }
    ],
    "settingsPages": [
      { "id": "dev.openplayer.metadata.settings", "title": "Metadata settings" }
    ],
    "metadataProviders": [
      { "id": "dev.openplayer.metadata.provider", "title": "Local metadata" }
    ],
    "subtitleSources": [
      { "id": "dev.openplayer.metadata.subtitles", "title": "Subtitle search" }
    ]
  }
}
```

## Validation Rules

- `id`, `name`, `version`, and `entry` are required; `id`, `name`, and `version` cannot be blank.
- `id` must be a dotted lowercase identifier with at least two segments, such as `dev.openplayer.metadata`.
- Identifier segments must start with `a-z` and may contain lowercase letters, digits, and `-`.
- `version` must use simple `major.minor.patch` numeric semver, such as `1.0.0`.
- V0 only accepts `"builtIn"` for `entry`.
- `description` is optional, but cannot be blank when present.
- Unknown manifest and contribution fields are rejected.
- Unknown permissions are rejected.
- Contribution `id` and `title` values cannot be blank.
- Contribution IDs must use the same dotted lowercase identifier syntax as plugin IDs.
- Contribution IDs must be unique across all contribution groups.

## Permissions

- `metadata.read`
- `subtitle.search`
- `settings.read`
- `settings.write`

## Contributions

- `commands`
- `settingsPages`
- `metadataProviders`
- `subtitleSources`

## Non-Goals For V0

- No plugin directory scanning.
- No manifest file IO.
- No JavaScript, WASM, or native plugin runtime.
- No decoder, filter, renderer, or media pipeline extension points.
- No Tauri command or desktop UI integration.
