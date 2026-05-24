# OpenPlayer Themes

OpenPlayer uses a built-in `Studio Dark` theme plus a user-selected accent
override. Additional themes can be imported through manifest-only theme plugins
and are persisted in the backend redb appearance store.

## Theme Manifest

```json
{
  "id": "studio-dark",
  "name": "Studio Dark",
  "version": "1.0.0",
  "tokens": {
    "surface": "#050607",
    "panel": "rgba(8, 10, 12, 0.72)",
    "panelStrong": "rgba(8, 10, 12, 0.88)",
    "text": "#ece7dd",
    "muted": "#b9b0a3",
    "faint": "#8f867a",
    "accent": "#78d5b3",
    "danger": "#d78372",
    "line": "rgba(236, 231, 221, 0.12)",
    "control": "rgba(18, 21, 25, 0.72)",
    "scrollbarThumb": "rgba(236, 231, 221, 0.22)",
    "scrollbarThumbHover": "rgba(120, 213, 179, 0.46)"
  }
}
```

## Tokens

- `surface`: base app surface.
- `panel`: translucent panel surface.
- `panelStrong`: stronger elevated panel surface.
- `text`: primary text color.
- `muted`: secondary text color.
- `faint`: low-emphasis text and icon color.
- `accent`: primary action, focus, and playback accent color.
- `danger`: destructive or error color.
- `line`: separators and subtle strokes.
- `control`: compact control surface.
- `scrollbarThumb`: scrollbar thumb color.
- `scrollbarThumbHover`: scrollbar thumb hover color.

## Validation Rules

- `id`, `name`, `version`, and `tokens` are required.
- Theme IDs may be a lowercase identifier such as `studio-dark` or a dotted
  lowercase identifier such as `dev.openplayer.theme.ocean.dark`.
- Identifier segments must start with `a-z` and may contain lowercase letters,
  digits, and `-`.
- Versions must use simple `major.minor.patch` numeric semver.
- Unknown manifest and token fields are rejected.
- Color tokens must use `#RGB`, `#RRGGBB`, or `rgba(r, g, b, a)`.
- `rgba` channels must be `0` through `255`; alpha must be `0` through `1`.

## Notes

- Theme plugin manifests are documented in [plugins](../plugins/README.md).
- Runtime theme and accent state is stored by
  `apps/desktop/src-tauri/src/appearance_store.rs`.
- The overlay applies theme values through CSS variables.
