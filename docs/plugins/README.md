# OpenPlayer Plugins

OpenPlayer currently supports manifest-only theme plugins. A plugin manifest can
contribute one or more themes, and the app stores imported plugin manifests and
enablement state in redb.

There is no JavaScript, WASM, native code, decoder, filter, renderer, or general
command plugin runtime yet.

## Manifest

```json
{
  "id": "dev.openplayer.theme.ocean",
  "name": "Ocean Theme Pack",
  "version": "1.0.0",
  "description": "Ocean themes for OpenPlayer.",
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

## Validation Rules

- `id`, `name`, `version`, `entry`, and `contributes.themes` are required.
- Plugin IDs must be dotted lowercase identifiers, such as
  `dev.openplayer.theme.ocean`.
- `entry` must be `"manifest"`.
- Versions must use simple `major.minor.patch` numeric semver.
- A plugin must contribute at least one theme.
- Theme IDs must be lowercase identifiers or dotted lowercase identifiers.
- Theme IDs must be unique within the plugin.
- Unknown fields are rejected.
- Theme token values must use supported color formats.

## Non-Goals

- No plugin directory scanning.
- No executable plugin code.
- No command, settings page, metadata provider, or subtitle provider plugin
  runtime.
- No media pipeline extension points.
