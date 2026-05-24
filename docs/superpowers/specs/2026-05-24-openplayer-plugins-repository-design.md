# OpenPlayer Official Plugins Repository Design

## Status

Approved for planning on 2026-05-24.

## Context

OpenPlayer now supports packaged plugins through `.opplugin` archives. The main
player repository currently contains the plugin system implementation, the
manifest documentation, and an example official plugin. Keeping official plugin
source code in the player repository will mix two release cadences: core player
releases and plugin releases.

## Decision

Create a separate GitHub repository named `openplayer-plugins`.

The repository is the official plugin collection for OpenPlayer. It owns
official plugin source code, example plugin projects, plugin-specific assets,
plugin package build scripts, CI validation, and plugin release artifacts.

The OpenPlayer player repository remains the owner of the plugin host API,
manifest schema behavior, runtime security model, and integration tests for the
host.

## Goals

- Keep official plugin development separate from core player development.
- Publish official `.opplugin` packages independently from player releases.
- Give future contributors a clean place to study and build plugins.
- Support multiple official plugins under one shared build and validation setup.
- Leave room for a future official plugin index without requiring it now.

## Non-Goals

- Do not move the plugin runtime implementation out of the player repository.
- Do not build an online plugin marketplace in this phase.
- Do not allow direct third-party native code execution through this repository.
- Do not require every future plugin to live in its own repository.

## Repository Layout

```text
openplayer-plugins/
  plugins/
    subtitle-typography/
      manifest.json
      README.md
      src/
      assets/
  scripts/
    package-plugin.mjs
    validate-manifest.mjs
  dist/
  .github/workflows/
    release.yml
  README.md
  LICENSE
```

`plugins/*` contains one plugin per directory. A plugin directory must contain a
root `manifest.json`. `src/` is optional and is used by plugins that need a
`webviewJs` runtime. `assets/` is optional and is copied into the package.

`dist/` is local build output and must not be tracked.

## Initial Plugin

The first migrated plugin is `subtitle-typography`.

It provides subtitle font family, font size, scale, letter spacing, vertical
position, color, outline, and shadow settings. The line spacing setting is not
included because the current mpv behavior is unreliable in OpenPlayer's host
path.

## Main Repository Changes

The player repository keeps `docs/plugins/README.md` as the canonical plugin API
and manifest documentation. It should link to `openplayer-plugins` for official
plugin examples and downloadable official packages.

The player repository should not keep official plugin source directories long
term. If tests need plugin manifests, they should use small fixtures that are
clearly marked as host tests rather than official plugin source.

## Build And Release

`openplayer-plugins` packages every plugin under `plugins/*` into a `.opplugin`
archive. The archive contains the plugin directory contents with `manifest.json`
at the package root.

Release assets are attached to GitHub Releases. Example:

```text
SubtitleTypography_1.0.0.opplugin
```

The release workflow validates every manifest before packaging. A failed plugin
manifest blocks the release.

## Validation

The plugin repository validates:

- Required manifest fields.
- Supported runtime kinds.
- Supported contribution placements.
- Safe package paths.
- Package archive contents.

The player repository validates host behavior:

- Importing `.opplugin` packages.
- Applying allowed plugin settings.
- Rejecting unsafe or unsupported plugin capabilities.
- Rendering plugin settings in the expected UI surfaces.

## Migration Plan

1. Create the new `openplayer-plugins` repository.
2. Add repository README, license, package scripts, and release workflow.
3. Move the current subtitle typography plugin into
   `plugins/subtitle-typography`.
4. Build and attach the first `.opplugin` release asset from the plugin
   repository.
5. Update the player repository plugin documentation to link to the new official
   plugin repository.
6. Replace any official plugin source in the player repository with minimal host
   fixtures where needed.

## Risks

- Plugin docs can drift from host behavior if schema changes are not coordinated.
  Mitigation: keep host schema documentation in the player repository and mirror
  validation checks in the plugin repository.
- Users may install stale plugin packages. Mitigation: plugin manifests should
  use semantic versions and clear release notes.
- Plugin release automation can diverge from player import behavior. Mitigation:
  package format tests should stay in both repositories at their respective
  boundaries.

## Success Criteria

- `openplayer-plugins` can build `SubtitleTypography_1.0.0.opplugin`.
- The generated package installs in OpenPlayer.
- The player repository no longer treats official plugin source code as part of
  the core app.
- Plugin API documentation remains available from the player repository.
