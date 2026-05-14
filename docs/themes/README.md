# OpenPlayer Themes

OpenPlayer theme support is currently a manifest-only V0 contract. The Rust crate can parse and validate theme manifest JSON strings and exposes the built-in Studio Dark manifest, but the desktop app does not scan theme folders, load files, or apply external themes yet.

## V0 Manifest

```json
{
  "id": "studio-dark",
  "name": "Studio Dark",
  "version": "1.0.0",
  "tokens": {
    "surface": "#050607",
    "panel": "rgba(8, 10, 12, 0.88)",
    "text": "#ece7dd",
    "muted": "#b9b0a3",
    "accent": "#caa05d",
    "danger": "#d78372",
    "border": "rgba(236, 231, 221, 0.12)",
    "radius": "medium",
    "density": "comfortable"
  }
}
```

## Validation Rules

- `id`, `name`, `version`, and `tokens` are required; `id`, `name`, and `version` cannot be blank.
- `id` may be a single lowercase identifier such as `studio-dark` or a dotted identifier such as `dev.openplayer.studio-dark`.
- Identifier segments must start with `a-z` and may contain lowercase letters, digits, and `-`.
- `version` must use simple `major.minor.patch` numeric semver, such as `1.0.0`.
- Unknown manifest and token fields are rejected.
- All color tokens must use `#RGB`, `#RRGGBB`, or `rgba(r, g, b, a)`.
- `rgba` channels must be `0` through `255`; alpha must be `0` through `1`.

## Tokens

- `surface`: base app surface color.
- `panel`: elevated or control surface color.
- `text`: primary text color.
- `muted`: secondary text color.
- `accent`: primary action and focus color.
- `danger`: destructive or error color.
- `border`: separator and subtle stroke color.
- `radius`: one of `none`, `small`, `medium`, or `large`.
- `density`: one of `compact`, `comfortable`, or `spacious`.

## Non-Goals For V0

- No theme directory scanning.
- No manifest file IO.
- No CSS generation.
- No React UI theme application.
- No Tauri command or desktop UI integration.
