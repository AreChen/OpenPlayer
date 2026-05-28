# OpenPlayer Plugin Documentation Guidance

Use this directory as the source of truth when writing or reviewing OpenPlayer
plugins, SDK examples, and AI-facing plugin instructions.

## Current SDK

- Treat OpenPlayer 1.5.1 as the current SDK package generation.
- Use `docs/plugins/sdk-1.5-developer-guide.md` for the public SDK contract.
- Keep examples aligned with `openplayer-plugins/packages/sdk/index.d.ts`.
- Do not document APIs unless the host bridge, backend validation, official
  plugin examples, and tests already support them.

## Custom Views

- Prefer `presentation: "sidePanel"` for playlist-like right panels.
- Side panels should use a transparent document background and a
  semi-transparent themed app surface. When transparency should be user
  adjustable, declare a `pluginSettings` number setting and reference it with
  `frameOpacitySetting` instead of hard-coding host iframe opacity.
- Derive colors from host theme tokens such as `--op-accent`, `--op-panel`,
  `--op-panel-strong`, `--op-control`, `--op-text`, and `--op-line`.
- The host owns side panel margins, right alignment, height, and 14px rounded
  clipping. Plugin views should usually set their root app to `width: 100%`,
  `height: 100%`, and avoid extra outer padding.
- Use `color-mix(..., transparent)` when a plugin needs layered panel surfaces
  so video remains subtly visible through the plugin UI.

## Manifest Style

- Use `tv` for TV-like channel browsers and IPTV controls.
- Use `stream` for generic network stream entry points.
- Keep permissions minimal and feature-detect with
  `openplayer.capabilities.has(...)` or
  `openplayer.capabilities.hasPermission(...)`.

## Verification

For SDK/docs changes, run from `apps/desktop`:

```powershell
npm run verify:shell
npm run build
```

For official plugin examples, run from `openplayer-plugins`:

```powershell
npm test
npm run build
```
