# OpenPlayer Plugin And Theme Manifest Validation Design Spec

Date: 2026-05-14

## Purpose

This slice strengthens the plugin and theme manifest validation boundaries. It defines the V0 JSON manifest shape, Rust types, parse functions, validation rules, and documentation examples for plugins and themes without loading manifests from disk or applying them in the UI.

## Scope

Included:

- Plugin manifest JSON parsing from `&str`.
- Theme manifest JSON parsing from `&str`.
- Stronger plugin manifest schema and validation.
- Stronger theme manifest schema and validation.
- Documentation examples for V0 plugin and theme manifests.
- Unit tests for valid JSON, malformed JSON, and validation failures.

Excluded:

- Reading manifest files from disk.
- Scanning plugin or theme directories.
- Enabling/disabling plugins.
- Running plugin code or commands.
- Applying theme tokens to the React UI.
- Tauri commands for plugin/theme loading.

## Plugin Manifest Shape

The V0 plugin manifest remains application-level only. It must not expose decoder, renderer, filter, or media-pipeline extension points.

```json
{
  "id": "openplayer.metadata.local",
  "name": "Local Metadata",
  "version": "0.1.0",
  "description": "Adds local metadata helpers.",
  "entry": "builtIn",
  "permissions": ["metadata.read"],
  "contributes": {
    "commands": [
      {
        "id": "openplayer.metadata.refresh",
        "title": "Refresh Metadata"
      }
    ],
    "settingsPages": [],
    "metadataProviders": [
      {
        "id": "openplayer.metadata.local.provider",
        "title": "Local Metadata"
      }
    ],
    "subtitleSources": []
  }
}
```

Rust model:

- `PluginManifest`
  - `id: String`
  - `name: String`
  - `version: String`
  - `description: Option<String>`
  - `entry: PluginEntry`
  - `permissions: Vec<PluginPermission>`
  - `contributes: PluginContributions`
- `PluginEntry`
  - `BuiltIn`
- `PluginPermission`
  - `MetadataRead`
  - `SubtitleSearch`
  - `SettingsRead`
  - `SettingsWrite`
- `PluginContributions`
  - `commands: Vec<PluginCommandContribution>`
  - `settings_pages: Vec<PluginSettingsPageContribution>`
  - `metadata_providers: Vec<PluginMetadataProviderContribution>`
  - `subtitle_sources: Vec<PluginSubtitleSourceContribution>`

The contribution structs share an `id` and `title`. They stay data-only in this slice.

## Plugin Validation Rules

- `id`, `name`, and `version` must be non-empty.
- `id` must use stable lowercase dotted identifier syntax: segments separated by `.`, each segment starting with `a-z` and containing only `a-z`, `0-9`, or `-` after the first character.
- `version` must use simple semantic version syntax: `major.minor.patch`, numeric only.
- `description`, when present, must not be only whitespace.
- Permissions are enum-backed; unknown permission strings fail JSON parsing.
- All contribution IDs and titles must be non-empty.
- Contribution IDs must use the same dotted identifier syntax as plugin IDs.
- Contribution IDs must be unique across all contribution categories within one manifest.
- Unsupported extension categories are not part of the typed schema and should fail JSON parsing when unknown fields are denied.

## Theme Manifest Shape

Themes remain token manifests. This slice validates tokens but does not apply them to the UI.

```json
{
  "id": "studio-dark",
  "name": "Studio Dark",
  "version": "0.1.0",
  "tokens": {
    "surface": "#050607",
    "panel": "rgba(8,10,12,0.88)",
    "text": "#ece7dd",
    "muted": "#b9b0a3",
    "accent": "#caa05d",
    "danger": "#d78372",
    "border": "rgba(236,231,221,0.12)",
    "radius": "medium",
    "density": "comfortable"
  }
}
```

Rust model:

- `ThemeManifest`
  - `id: String`
  - `name: String`
  - `version: String`
  - `tokens: ThemeTokens`
- `ThemeTokens`
  - `surface: String`
  - `panel: String`
  - `text: String`
  - `muted: String`
  - `accent: String`
  - `danger: String`
  - `border: String`
  - `radius: ThemeRadius`
  - `density: ThemeDensity`
- `ThemeRadius`
  - `None`
  - `Small`
  - `Medium`
  - `Large`
- `ThemeDensity`
  - `Compact`
  - `Comfortable`
  - `Spacious`

## Theme Validation Rules

- `id`, `name`, and `version` must be non-empty.
- `id` uses the same lowercase dotted identifier syntax, except single-segment IDs such as `studio-dark` are allowed.
- `version` must use simple semantic version syntax.
- Color tokens must be valid `#RGB`, `#RRGGBB`, or `rgba(r,g,b,a)` values.
- `rgba` channels must be within `0..=255`, and alpha must be within `0.0..=1.0`.
- Radius and density are enum-backed; unknown values fail JSON parsing.
- `studio_dark_manifest()` must use the full new token schema and pass validation.
- Broken themes fail validation and later slices should fall back to Studio Dark.

## JSON Parsing

Each crate exposes a parse-and-validate function:

- `parse_plugin_manifest_json(input: &str) -> Result<PluginManifest, PluginManifestError>`
- `parse_theme_manifest_json(input: &str) -> Result<ThemeManifest, ThemeManifestError>`

These functions parse JSON with `serde_json`, then call the crate validator. Malformed JSON maps to a typed `Json` error variant. Validation errors remain distinct from parse errors.

Unknown fields should be denied in manifest and contribution structs. This keeps the V0 boundary explicit and prevents accepting unsupported plugin or theme capabilities silently.

## Error Handling

Plugin errors should distinguish:

- malformed JSON
- empty required fields
- invalid identifier
- invalid version
- empty description
- empty contribution fields
- duplicate contribution IDs

Theme errors should distinguish:

- malformed JSON
- empty required fields
- invalid identifier
- invalid version
- invalid color token with token name

Both crates should keep errors UI-safe and deterministic. They should not include raw parse backtraces.

## Documentation

Update:

- `docs/plugins/README.md`
- `docs/themes/README.md`

The docs should include:

- V0 scope limits.
- One valid JSON manifest example.
- Important validation rules.
- Explicit note that file loading, directory scanning, plugin execution, and UI theme application are separate future slices.

## Testing Strategy

Plugin tests:

- valid built-in plugin manifest validates.
- valid plugin JSON parses and validates.
- malformed JSON returns plugin JSON error.
- empty id/name/version fail.
- invalid id fails.
- invalid version fails.
- whitespace description fails.
- unknown permission fails JSON parse.
- empty contribution id/title fails.
- duplicate contribution ID across categories fails.
- unsupported unknown field fails JSON parse.

Theme tests:

- `studio_dark_manifest()` validates under the full schema.
- valid theme JSON parses and validates.
- malformed JSON returns theme JSON error.
- empty id/name/version fail.
- invalid id fails.
- invalid version fails.
- invalid color tokens fail with token name.
- invalid rgba channel/alpha fail.
- unknown radius/density fails JSON parse.
- unsupported unknown field fails JSON parse.

Full verification:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `npm run verify:shell`
- `npm run build`

## Acceptance Criteria

- Plugin JSON manifests can be parsed and validated from strings.
- Theme JSON manifests can be parsed and validated from strings.
- Plugin validation rejects unsupported or ambiguous V0 plugin capabilities.
- Theme validation rejects unsupported or malformed token values.
- Default Studio Dark manifest remains valid.
- Plugin and theme docs show the supported V0 manifest shape.
- No UI loading, file IO, directory scanning, plugin execution, or theme application is implemented in this slice.

## Follow-Up Work

- Add single manifest file loading APIs.
- Add plugin/theme directory scanning and conflict handling.
- Persist active theme and plugin enablement through storage settings.
- Add a settings UI for choosing themes and viewing plugin manifests.
- Apply theme tokens to the React UI.
